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
 *   4. (no duplicated attribute names in served markup) The raw HTTP-served
 *      HTML of `/` contains NO start tag with the same attribute name
 *      written twice. (`/component/?name=top_layer&` and
 *      `/component/?name=dialog&` were the URLs this rule was specified
 *      against, but `name` is a *query* param
 *      (`#[route("/component/?:name&:iframe&:dark_mode")]`,
 *      `preview/src/main.rs`) and this app's `dx build --ssg` only
 *      prerenders a fixed path list that does not vary by query string --
 *      confirmed empirically: the built `server` binary run standalone
 *      serves byte-identical markup for `/component/?name=button&`,
 *      `/component/?name=top_layer&`, and no query at all, all three
 *      being literally the `name=""` ("Component not found") prerendered
 *      page, and the documented local SSG lane (`docs/conformance-
 *      harness.md`) serves the same static snapshot even more plainly, via
 *      a bare `python3 -m http.server`. There is no locally-servable URL
 *      under this app's SSG build where `/component/?name=X&`'s *own*
 *      markup differs by `X`. `/` does not have this problem and is where
 *      the fixtures actually live pre-JS: the whole component gallery,
 *      `top_layer` fixture included, is embedded directly on the home page
 *      (Rule 3's Dropdown Menu is the same pattern) -- it is also, not
 *      coincidentally, the exact page Finding 1's own evidence came from.)
 *      Rule source: the WHATWG HTML parsing
 *      spec's tokenizer step for start tags --
 *      https://html.spec.whatwg.org/multipage/parsing.html#attribute-name-state
 *      -- "When the user agent leaves the attribute name state ... the
 *      complete attribute's name must be compared to the other attributes
 *      on the same token; if there is already an attribute on the token
 *      with the exact same name, then this is a duplicate-attribute parse
 *      error and the new attribute must be removed from the token." I.e.
 *      on a real duplicate, the browser's HTML parser silently keeps the
 *      *first* occurrence and drops every later one. This makes any
 *      component that SSR-serializes an explicit attribute and then also
 *      serializes a caller override of the same name (rather than merging
 *      them into one value before rendering) a hydration hazard: Dioxus's
 *      web (CSR/post-hydration) DOM path applies attributes sequentially,
 *      so there the *last*-applied value wins -- the exact opposite of
 *      what the parser above does with the *served* HTML. Server and
 *      client then structurally agree on "one aria-label attribute" but
 *      disagree on its *value*, which is invisible to Rules 1-3 (they
 *      check tree shape and interactivity, not computed accessible names)
 *      but fails axe's `landmark-unique` on the SSG lane wherever two
 *      landmarks are meant to be told apart by a caller-supplied name
 *      (`preview.spec.ts`, the top-layer fixture's toast region vs. the
 *      app shell's). Found 2026-09-01 in `ToastRegionRendered`
 *      (`primitives/src/toast.rs`): its `aria_label: "{length}
 *      notifications"` followed by `..attributes` serialized BOTH the
 *      primitive's default and the top-layer fixture's
 *      `aria_label: "Top-layer fixture notifications"` override into the
 *      same `<div>` start tag. Also asserts directly that the fixture
 *      region's *effective* (first-wins, per the parse rule quoted above)
 *      served `aria-label` is the caller's override, not the primitive's
 *      default -- the concrete, human-readable form of "no duplicate" for
 *      this exact regression.
 *
 * STATUS AT WRITE TIME (this session, against pre-axis-fix `main`): rules
 * 1-3 RED against the local SSG build -- see this session's report for
 * verbatim failure output. Rule 1 fails because the server-rendered toast
 * region carries no `popover` attribute at all (the native arm). Rules 2
 * and 3 fail as direct consequences of rule 1's markup mismatch: hydration
 * cannot reconcile the wasm client's web-arm render against the server's
 * native-arm markup, so hydration errors surface and click handlers
 * page-wide (including the Dropdown Menu trigger) are never attached.
 *
 * Rule 4 was added and confirmed RED separately (2026-09-01, against a
 * build with rules 1-3 already green): 15 duplicate-attribute start tags on
 * `/`, across five components, every one of them a real caller override in
 * `preview/src/components/top_layer/component.rs`'s oracle fixture (plus
 * one on a dashboard `Progress` bar) colliding with that primitive's own
 * explicit attribute of the same name. Verbatim (one representative per
 * component; the fixture's `id="clip-*"`/`id="scroll-*"`/`id="edge-*"`/
 * `id="popover-modal-*"` triggers repeat the same popover/context-menu/
 * select shape 5/2/1 more times respectively):
 *
 *   duplicate "style" in a dashboard Progress bar:
 *     <div role="progressbar" ... style="--progress-value: 68%"
 *          aria-label="Toward Q2 target" style="width: 100%;" ...>
 *   duplicate "style" in ContextMenuTrigger (`context_menu.rs`):
 *     <div id="dxc-82" ... style="-webkit-touch-callout: none; ..."
 *          style="padding:20px;...;cursor:context-menu;..." ...>
 *   duplicate "id" in PopoverTrigger (`popover.rs`):
 *     <button id="dxc-238" type="button" style="anchor-name: --dxa-dxc-240;"
 *             id="clip-popover-trigger" ...>
 *   duplicate "id" in ContextMenuRoot (`context_menu.rs`):
 *     <div id="dxc-232" tabindex=0 data-state="closed" data-disabled=false
 *          id="clip-context-menu-root" ...>
 *   duplicate "id" in ContextMenuTrigger (`context_menu.rs`):
 *     <div id="dxc-233" tabindex="-1" role="button" aria-haspopup="menu"
 *          aria-expanded=false style="-webkit-touch-callout: none; ..."
 *          id="clip-context-menu-trigger" ...>
 *   duplicate "id" in SelectTrigger (`select/components/trigger.rs`):
 *     <button id="dxc-226" style="anchor-name: --dxa-dxc-226;" type="button"
 *             aria-haspopup="listbox" aria-expanded=false
 *             id="clip-select-trigger" ...>
 *   duplicate "style"+"aria-label" in ToastRegionRendered (`toast.rs`, the
 *   original Finding 1 evidence):
 *     <div id="dxc-216" role="region" aria-label="0 notifications"
 *          tabindex="-1" popover="manual" style="--toast-count: 0"
 *          style="position: fixed; top: auto; right: 0; bottom: 0; left:
 *          auto; margin: 0;" aria-label="Top-layer fixture notifications"
 *          data-node-hydration="2062">
 *
 * Fixed by construction in all five files (`toast.rs`'s `ToastRegionRendered`
 * both arms, `progress.rs`'s `Progress`, `context_menu.rs`'s
 * `ContextMenuRoot` and `ContextMenuTrigger`, `popover.rs`'s
 * `PopoverTrigger`, `select/components/trigger.rs`'s `SelectTrigger`):
 * `merge_attributes` now combines each component's own default attributes
 * with the caller's `attributes` into a single deduped list (caller wins)
 * before the `rsx!` call, so every tag is only ever built with one value
 * per attribute name -- neither lane can ever emit a duplicate, because
 * there is only ever one attribute *to* emit. `PopoverTrigger`'s `id` is
 * the one exception carried over unmerged, on its own single explicit
 * binding (its pre-existing `use_effect`-based caller-id-to-`ctx.labelledby`
 * sync, unchanged, already makes it end up holding the caller's value) --
 * every other `id` above is caller-wins like the rest, matching each
 * element's pre-existing client (CSR, post-hydration) behavior exactly
 * (spread order already made the caller's id win there), not newly
 * protected against override the way `toast.rs`'s own `id` is (see that
 * function's doc): nothing else here does a fixed-expectation
 * `document.getElementById` lookup against its *own* id winning --
 * `use_outside_dismiss`/`use_refocus_on_close_unless`'s lookups already
 * silently no-op whenever a caller's override id makes them miss, on both
 * lanes, unchanged by this fix (a real, separate, pre-existing behavior gap
 * once a caller overrides one of these ids -- out of scope here, which is
 * only about the served markup agreeing with the DOM).
 */

import { test, expect } from "@playwright/test";

const NAV_TIMEOUT = 20 * 60 * 1000; // first run compiles the app
const BASE = "http://127.0.0.1:8090";

/**
 * Rule 4 support: a minimal HTML start-tag tokenizer, just enough to answer
 * "does this tag have the same attribute name twice, and if so what value
 * does WHATWG's first-wins duplicate-attribute rule leave in effect."
 *
 * Deliberately not a full HTML parser -- text-node content in this app's
 * served markup is always entity-escaped by Dioxus's SSR renderer (rsx text
 * children are never written as raw `<`/`>`), so a naive "look for the next
 * `<`" scan never mistakes text for a tag boundary here. It does track
 * quote state so a `>` inside a quoted attribute value (e.g. an inline
 * `style` string) never truncates a tag early.
 */
type StartTag = {
  raw: string;
  name: string;
  /** Attribute names in document order, lower-cased, one entry per occurrence. */
  attrNames: string[];
  /** name -> value of the FIRST occurrence only (WHATWG duplicate-attribute rule: later ones are dropped). */
  effectiveValues: Map<string, string | null>;
};

function extractStartTags(html: string): StartTag[] {
  const tags: StartTag[] = [];
  let i = 0;
  const n = html.length;
  while (i < n) {
    if (html[i] !== "<") {
      i++;
      continue;
    }
    if (html.startsWith("<!--", i)) {
      const end = html.indexOf("-->", i + 4);
      i = end === -1 ? n : end + 3;
      continue;
    }
    if (html[i + 1] === "/" || html[i + 1] === "!" || html[i + 1] === "?") {
      const end = html.indexOf(">", i);
      i = end === -1 ? n : end + 1;
      continue;
    }
    if (!/[a-zA-Z]/.test(html[i + 1] ?? "")) {
      // Not actually a tag start (stray '<'); move on one char.
      i++;
      continue;
    }
    let j = i + 1;
    let inSingle = false;
    let inDouble = false;
    while (j < n) {
      const c = html[j];
      if (inSingle) {
        if (c === "'") inSingle = false;
      } else if (inDouble) {
        if (c === '"') inDouble = false;
      } else if (c === "'") {
        inSingle = true;
      } else if (c === '"') {
        inDouble = true;
      } else if (c === ">") {
        break;
      }
      j++;
    }
    const raw = html.slice(i, j + 1);
    const nameMatch = raw.match(/^<([a-zA-Z][a-zA-Z0-9-]*)/);
    if (nameMatch) {
      const name = nameMatch[1].toLowerCase();
      let body = raw.slice(1 + nameMatch[1].length, raw.length - 1);
      if (body.endsWith("/")) body = body.slice(0, -1);
      const attrNames: string[] = [];
      const effectiveValues = new Map<string, string | null>();
      const attrRe =
        /([^\s"'=<>`/]+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'=<>`]+)))?/g;
      let m: RegExpExecArray | null;
      while ((m = attrRe.exec(body)) !== null) {
        const attrName = m[1].toLowerCase();
        const value = m[2] ?? m[3] ?? m[4] ?? null;
        attrNames.push(attrName);
        if (!effectiveValues.has(attrName)) {
          effectiveValues.set(attrName, value); // first occurrence wins, per WHATWG
        }
      }
      tags.push({ raw, name, attrNames, effectiveValues });
    }
    i = j + 1;
  }
  return tags;
}

function duplicateAttrTags(html: string): { raw: string; dup: string }[] {
  const out: { raw: string; dup: string }[] = [];
  for (const tag of extractStartTags(html)) {
    const seen = new Set<string>();
    for (const a of tag.attrNames) {
      if (seen.has(a)) {
        out.push({ raw: tag.raw, dup: a });
        break;
      }
      seen.add(a);
    }
  }
  return out;
}

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

  // `/component/?name=top_layer&`/`/component/?name=dialog&` are not
  // included: `name` is a query param this app's SSG build does not
  // prerender per-value (see Rule 4's doc above) -- `/` already embeds
  // every fixture, `top_layer` included, directly.
  const RULE4_URLS = [`${BASE}/`];

  test("Rule 4: no start tag in served HTML has a duplicated attribute name (WHATWG HTML duplicate-attribute parse error)", async ({
    request,
  }) => {
    for (const url of RULE4_URLS) {
      const response = await request.get(url, { timeout: NAV_TIMEOUT });
      expect(response.ok()).toBeTruthy();
      const html = await response.text();

      const dupes = duplicateAttrTags(html);
      expect(
        dupes,
        `${url}: found start tag(s) with a duplicated attribute name. Per WHATWG HTML's ` +
          `attribute-name parsing state, a duplicate is a parse error and the browser keeps ` +
          `only the FIRST occurrence -- but Dioxus's web (CSR/hydrated) DOM path applies ` +
          `attributes sequentially, so there the LAST-applied value wins. Server and client ` +
          `then disagree about which value is in effect:\n` +
          dupes.map((d) => `  duplicate "${d.dup}" in: ${d.raw}`).join("\n"),
      ).toHaveLength(0);
    }
  });

  test("Rule 4b: the top-layer fixture's toast region served accessible name is the caller's override, not the primitive's default", async ({
    request,
  }) => {
    const response = await request.get(`${BASE}/`, { timeout: NAV_TIMEOUT });
    expect(response.ok()).toBeTruthy();
    const html = await response.text();

    // Both the app shell's own ToastProvider and this fixture's own
    // ToastProvider render a `popover="manual"` region (same component);
    // find the one carrying the fixture's `aria_label` override text so
    // this assertion is unambiguous about which region it is checking.
    const regionTags = extractStartTags(html).filter(
      (t) => t.name === "div" && t.effectiveValues.get("popover") === "manual",
    );
    const fixtureRegion = regionTags.find((t) =>
      t.raw.includes("Top-layer fixture notifications"),
    );
    expect(
      fixtureRegion,
      `expected a popover="manual" toast region in served HTML whose markup mentions the ` +
        `fixture's aria_label override ("Top-layer fixture notifications"); found ${regionTags.length} ` +
        `popover="manual" region(s): ${regionTags.map((t) => t.raw).join("\n")}`,
    ).toBeDefined();

    const effective = fixtureRegion!.effectiveValues.get("aria-label");
    expect(
      effective,
      `the fixture's toast region's EFFECTIVE served aria-label (WHATWG first-wins on a ` +
        `duplicate) should be the caller's override, not the primitive's own default -- raw ` +
        `tag: ${fixtureRegion!.raw}`,
    ).toBe("Top-layer fixture notifications");
  });
});
