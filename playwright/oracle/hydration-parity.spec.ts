/**
 * ORACLE: hydration/deployment parity — fullstack SSG prerender vs. wasm
 * client hydration.
 *
 * Source: the WHATWG HTML Living Standard's hydration-relevant contract and
 * Dioxus's own fullstack hydration model:
 *   - The DOM the server sends and the DOM the client's first render
 *     produces must describe the *same* tree for hydration (attaching
 *     listeners to existing nodes rather than replacing them) to work at
 *     all -- Dioxus's hydration walks the server-rendered DOM by the
 *     `data-node-hydration` ids embedded in the markup below and expects
 *     the client's own render pass to produce a structurally identical
 *     tree at each id. A structural mismatch (a different element, a
 *     missing/extra attribute the client's event delegation depends on)
 *     does not "just" cause a one-time console warning the way React's
 *     reconciler-level hydration mismatch does -- Dioxus's event
 *     delegation is wired by walking the *hydrated* tree from those ids, so
 *     a mismatch there can silently leave event listeners unattached
 *     page-wide, not merely on the mismatched subtree.
 *   - The `popover` attribute and the top-layer promotion it implies:
 *     https://html.spec.whatwg.org/multipage/popover.html#the-popover-attribute
 *
 * ## The 2026-09-01 production incident this guards against
 *
 * The deployed site (https://mentalgear.github.io/dioxus-components/) is
 * built by CI as fullstack SSG (`.github/workflows/web.yml`: `ssg: true,
 * features: fullstack`). The SSG prerender runs the *server* binary -- a
 * host (non-wasm) build, not the wasm client. Before the axis fix this
 * spec guards (`docs/recommended-implementations.md` Caveat 1), every
 * overlay component split its rendered markup on
 * `#[cfg(target_family = "wasm")]`, which is false on that host binary --
 * so the SSG server rendered the *native* arm (plain `div`s, no `popover`
 * attributes, no `<dialog>` elements) while the wasm client then hydrated
 * against that structurally different markup. Empirical confirmation this
 * session reproduced locally, matching the live deployed site byte-for-byte
 * on the diagnostic marker: the always-mounted `ToastProvider` region
 * renders as `<div role="region" class="dx-toast-container-...">` with NO
 * `popover` attribute on both, where the wasm web arm's own render function
 * renders that same element with `popover="manual"`. The user-visible
 * symptom: "Dropdown Menu -> Open Menu: nothing happens" on a hard-loaded
 * page of the deployed site -- and, more broadly, broken events page-wide,
 * because Dioxus's event delegation is wired from the hydration walk (see
 * above), so one popover-shaped mismatch can take down listener attachment
 * for the whole page, not just the mismatched element.
 *
 * Fixture: no dedicated fixture -- this spec exercises the real home page
 * (`/`, `preview/src/main.rs`'s `Route::Home`) exactly as a real visitor's
 * browser would hard-load it from the deployed site, using the SSG static
 * build's own served HTML (see `playwright/ssg.local.config.ts`'s header
 * for how to build and serve it). Rule 3's Dropdown Menu demo is
 * `preview/src/components/dropdown_menu/variants/main/mod.rs`, embedded
 * directly on the home page's component gallery.
 *
 * Calibration: N/A (tier: hydration/deployment parity, not an HTML/APG/
 * Radix behavioural tier -- there is no meaningful "native reference"
 * for "did the server and client agree on markup").
 *
 * Rules:
 *   1. (served-markup invariant) The raw HTTP-served HTML of `/` --
 *      fetched via `request.get`, before any JS runs -- contains the
 *      always-mounted `ToastProvider` region carrying a `popover`
 *      attribute. This is the canary for "the SSG server rendered the web
 *      arm": the toast region mounts on every page regardless of which
 *      demo is visible, so its presence/attributes in the raw response are
 *      a direct read of what the *server* rendered, with zero client JS
 *      involved.
 *   2. (zero hydration errors) `page.goto("/")`, collecting console
 *      messages and `pageerror`s through load and a 2s settle. None may
 *      match /hydrat/i, and there must be no `pageerror`s. A structural
 *      server/client mismatch is exactly the class of defect Dioxus (like
 *      every hydrating framework) surfaces as a console warning/error
 *      naming "hydration" when it has to recover from one, or as an
 *      uncaught panic/exception when it can't.
 *   3. (post-hydration interactivity on a hard load) On the hard-loaded
 *      home page -- no client-side navigation, matching the real-world
 *      report this rule regression-tests -- clicking the Dropdown Menu
 *      demo's "Open Menu" trigger opens its menu. This is the literal
 *      user-reported symptom ("Dropdown Menu -> Open Menu: nothing is
 *      happening") on the deployed site's main page.
 *
 * STATUS AT WRITE TIME (this session, against pre-axis-fix `main`): all
 * three rules RED against the local SSG build -- see this session's report
 * for verbatim failure output. Rule 1 fails because the server-rendered
 * toast region carries no `popover` attribute at all (the native arm).
 * Rules 2 and 3 fail as direct consequences of rule 1's markup mismatch:
 * hydration cannot reconcile the wasm client's web-arm render against the
 * server's native-arm markup, so hydration errors surface and click
 * handlers page-wide (including the Dropdown Menu trigger) are never
 * attached.
 */

import { test, expect } from "@playwright/test";

const NAV_TIMEOUT = 20 * 60 * 1000; // first run compiles the app
const BASE = "http://127.0.0.1:8090";

test.describe("hydration parity — SSG server markup vs. wasm client", () => {
  test("Rule 1: served HTML's ToastProvider region carries popover (web-arm markup canary)", async ({
    request,
  }) => {
    const response = await request.get(`${BASE}/`, { timeout: NAV_TIMEOUT });
    expect(response.ok()).toBeTruthy();
    const html = await response.text();

    const containerMatch = html.match(/<div[^>]*class="dx-toast-container-[^"]*"[^>]*>/);
    expect(
      containerMatch,
      "expected a dx-toast-container element in the raw server-rendered HTML",
    ).not.toBeNull();

    const containerTag = containerMatch![0];
    expect(
      containerTag,
      `server-rendered ToastProvider region is missing popover — this is the ` +
        `native (non-web) render arm; raw tag: ${containerTag}`,
    ).toMatch(/\bpopover="manual"/);
  });

  test("Rule 2: zero hydration errors on a hard load of the home page", async ({ page }) => {
    const consoleMessages: string[] = [];
    const pageErrors: string[] = [];
    page.on("console", (msg) => consoleMessages.push(msg.text()));
    page.on("pageerror", (err) => pageErrors.push(err.message));

    await page.goto(`${BASE}/`, { timeout: NAV_TIMEOUT, waitUntil: "networkidle" });
    // Settle window: hydration mismatch recovery/errors can surface a tick
    // or two after the initial load event, not necessarily synchronously.
    await page.waitForTimeout(2000);

    const hydrationMessages = consoleMessages.filter((m) => /hydrat/i.test(m));
    expect(
      hydrationMessages,
      `console messages mentioning hydration:\n${hydrationMessages.join("\n")}`,
    ).toHaveLength(0);
    expect(
      pageErrors,
      `uncaught page errors during load:\n${pageErrors.join("\n")}`,
    ).toHaveLength(0);
  });

  test("Rule 3: Dropdown Menu 'Open Menu' opens its menu on a hard-loaded main page", async ({
    page,
  }) => {
    await page.goto(`${BASE}/`, { timeout: NAV_TIMEOUT, waitUntil: "networkidle" });

    const trigger = page.getByRole("button", { name: "Open Menu" });
    await expect(trigger).toBeVisible();
    await expect(trigger).toHaveAttribute("data-state", "closed");

    await trigger.click();

    await expect(
      trigger,
      "clicking 'Open Menu' on a hard-loaded page did not open the menu " +
        "(data-state never became 'open') — the literal user-reported " +
        "symptom this rule regression-tests",
    ).toHaveAttribute("data-state", "open");
    await expect(page.getByRole("option", { name: "Edit" })).toBeVisible();
  });
});
