/**
 * ORACLE: tier 2 (HTML) — double-tap-to-zoom suppression on touch devices.
 *
 * Source: the CSS `touch-action` property is defined by the Pointer Events
 * specification (a W3C Recommendation), not by any single browser vendor:
 *   https://w3c.github.io/pointerevents/#the-touch-action-css-property
 * Its own prose explicitly ties the property to gesture suppression: "the
 * touch-action CSS property ... is used to specify whether and how a given
 * region can be manipulated by the user (...). Common uses of this
 * property include the elimination of double-tap-to-zoom or delays for
 * click event handling." WebKit documents Mobile Safari's own double-tap
 * gesture in exactly these terms and names `touch-action: manipulation` as
 * the fix authors must opt an element into: a "fast tap"/"300ms tap delay"
 * elimination technique that also disables double-tap-to-zoom on the
 * element it is set on, without disabling one-finger panning or pinch-zoom
 * the way `touch-action: none` would (WebKit's own worked example is
 * exactly a `role="button"` custom control, the same shape this app's
 * `[role="button"]`/menu-item/tab/etc. controls are):
 *   https://webkit.org/blog/5610/more-responsive-tapping-on-ios/
 *
 * Reported behaviour this guards against (user device review, iOS 18
 * Safari): "If the user is quickly tapping buttons, the double-tap zoom is
 * triggered." Two rapid taps on/near an interactive control that has not
 * opted out of the gesture are read by Mobile Safari as a double-tap
 * zoom request instead of two independent activations.
 *
 * Fix, two layers (same construction as `touch-focus-zoom.spec.ts`'s
 * font-size floor):
 *   - APP LAYER (`preview/assets/main.css`): one `!important` rule
 *     covering every interactive element/role this app renders, as a
 *     catch-all for raw elements (`top_layer`'s/`form`'s
 *     `dioxus_primitives::` fixtures — docs/preview-composition.md — and
 *     any future raw control added anywhere in `preview/`).
 *   - COMPONENT LAYER (`preview/src/components/<name>/style.css`): the same
 *     property, unconditionally, on every themed component's own
 *     interactive root class — load-bearing for a consumer who copies a
 *     component out via `dx components add` and never pulls in
 *     `main.css` at all.
 *
 * `touch-action: none` is NEVER an acceptable value here (it additionally
 * disables one-finger panning, breaking scroll) — this file treats `none`
 * as a hard failure distinct from (and worse than) the UA default `auto`.
 * `pan-x pan-y` (or either individually combined with the other) is
 * accepted as equivalent to `manipulation` for this rule's purpose: per
 * the Pointer Events spec, `manipulation` is defined as "the union of
 * pan-x, pan-y, and pinch-zoom... this value is a legacy expression" of a
 * pan+pinch-zoom set that still leaves double-tap-to-zoom suppressed the
 * same way -- so an element that spells the same permission set out
 * explicitly (`pan-x pan-y` -- pinch-zoom is implied on touch UAs even
 * without an explicit token, per the same spec section's note) is not a
 * defect distinct from one that uses the `manipulation` keyword.
 *
 * SELECTOR mirrors `main.css`'s app-layer rule exactly, one list shared by
 * both the fix and this oracle by construction: `button`, `[role="button"]`,
 * `a[href]`, `input`, `select`, `textarea`, `summary`,
 * `[tabindex]:not([tabindex="-1"])`, `[role="menuitem"]`, `[role="option"]`,
 * `[role="tab"]`, `[role="switch"]`, `[role="checkbox"]`, `[role="radio"]`,
 * `[role="slider"]`. This already reaches "anything with a click handler"
 * indirectly: every interactive control in this codebase that has an
 * `onclick`/tap handler also carries a native interactive tag, an
 * ARIA widget role from the list above, or a `tabindex` making it
 * keyboard-focusable (APG's own baseline requirement for a custom
 * control) -- a clickable element with none of those would already be a
 * separate, pre-existing a11y defect (unreachable by keyboard), not a
 * gap unique to this rule.
 *
 * Calibration (docs/conformance-harness.md, "Calibration"): the first test
 * below asserts the iPhone 13 emulation this file relies on actually
 * yields `(pointer: coarse)` -- identical calibration to
 * `touch-focus-zoom.spec.ts`'s own first test (both files rely on the same
 * Chromium mobile-emulation environment; WebKit itself is not available
 * locally -- docs/backlog.md row 4).
 *
 * RED-BEFORE-FIX EVIDENCE (captured this session by reverting
 * `preview/assets/main.css` and every `preview/src/components/<name>/style.css`
 * touch-action addition, restarting the dev server, and re-running this
 * file against the reverted tree -- see the paired fix commit for the
 * patch): **56 of 57 tests failed** -- every test except the calibration
 * one. `computed touch-action: auto` (the UA default that permits the
 * double-tap gesture) on literally every element matched by SELECTOR,
 * everywhere: no element anywhere in the app declared `touch-action`
 * before this round. Concretely, this means every single route failed --
 * even a component page with no interactive markup of its own (e.g.
 * `/component/?name=avatar&`) still failed, because this app's own shared
 * page chrome, present on every route, is itself built from unfixed
 * interactive elements: the footer's five links (`.dx-footer-brand-link`,
 * `.dx-footer-link` x4), the component page's own "DEMO"/"CODE" tabs
 * (`.dx-tabs-trigger`, `role="tab"`), its per-code-block "Copy code"
 * button, and (route-dependent) the navbar's language `<select>`/GitHub
 * link. Representative offenders from the captured failure output (the
 * home page, `/`, and the `virtual_list` component page, which additionally
 * renders 100+ `role="listitem"`-adjacent interactive rows in its demo):
 *
 *   <a class="dx-footer-link"> "Docs" touch-action=auto
 *   <button class="dx-tabs-trigger-..."> role="tab" "DEMO" touch-action=auto
 *   <button class="dx-copy-button"> "Copy code" touch-action=auto
 *
 * GREEN AFTER FIX: all 57 tests pass, confirmed by re-applying the patch,
 * restarting the dev server, and re-running this file against the fixed
 * tree.
 */

import { test, expect, devices, type Page } from "@playwright/test";

// Same reasoning as touch-focus-zoom.spec.ts: run the iPhone 13 device
// descriptor on Chromium (this repo's local lanes ship no WebKit project).
// The rule under test is about the computed `touch-action` value, which
// Chromium reproduces faithfully; only the underlying gesture itself needs
// real Mobile Safari to observe directly (docs/backlog.md row 4).
test.use({ ...devices["iPhone 13"], defaultBrowserType: "chromium" });

const BASE = "http://127.0.0.1:8080";

// Mirrors preview/assets/main.css's app-layer selector list exactly -- see
// that rule's own comment for why each entry is there.
const SELECTOR =
  'button, [role="button"], a[href], input, select, textarea, summary, [tabindex]:not([tabindex="-1"]), [role="menuitem"], [role="option"], [role="tab"], [role="switch"], [role="checkbox"], [role="radio"], [role="slider"]';

type Row = {
  tag: string;
  id: string;
  cls: string;
  role: string;
  touchAction: string;
  label: string;
};

function isAcceptable(touchAction: string): boolean {
  // `manipulation` is the primary construction; `pan-x pan-y` (either
  // token order) is the spec-equivalent explicit spelling of the same
  // permission set (see this file's header doc) -- both suppress
  // double-tap-to-zoom while still allowing panning.
  return (
    touchAction === "manipulation" ||
    touchAction === "pan-x pan-y" ||
    touchAction === "pan-y pan-x"
  );
}

async function scan(page: Page): Promise<Row[]> {
  return page.$$eval(SELECTOR, (els) =>
    els.map((el) => {
      const cs = getComputedStyle(el as HTMLElement);
      return {
        tag: el.tagName.toLowerCase(),
        id: (el as HTMLElement).id || "",
        cls: (el.getAttribute("class") || "").slice(0, 60),
        role: el.getAttribute("role") || "",
        touchAction: cs.touchAction,
        label:
          el.getAttribute("aria-label") ||
          el.getAttribute("placeholder") ||
          el.textContent?.trim().slice(0, 30) ||
          "",
      };
    }),
  );
}

function assertAllManipulation(rows: Row[], route: string) {
  const offenders = rows.filter((r) => !isAcceptable(r.touchAction));
  const noneOffenders = offenders.filter((r) => r.touchAction === "none");
  expect(
    offenders,
    `interactive element(s) on ${route} do not suppress double-tap-to-zoom ` +
      `(computed touch-action must be "manipulation" or the equivalent ` +
      `"pan-x pan-y", never left at the UA default "auto"` +
      (noneOffenders.length > 0
        ? `, and NEVER "none" -- ${noneOffenders.length} element(s) here compute "none", which additionally breaks scrolling and must not be used`
        : "") +
      `):\n` +
      offenders
        .map(
          (r) =>
            `  <${r.tag}${r.role ? ` role="${r.role}"` : ""}> id="${r.id}" class="${r.cls}" label="${r.label}" touch-action=${r.touchAction}`,
        )
        .join("\n"),
  ).toEqual([]);
}

test("CALIBRATION: iPhone 13 emulation yields coarse pointer / no hover", async ({ page }) => {
  await page.goto(`${BASE}/`, { timeout: 60000, waitUntil: "networkidle" });
  const media = await page.evaluate(() => ({
    coarse: matchMedia("(pointer: coarse)").matches,
    noHover: matchMedia("(hover: none)").matches,
  }));
  expect(media, "iPhone 13 device profile must emulate a touch (coarse pointer, no hover) environment").toEqual({
    coarse: true,
    noHover: true,
  });
});

test("home page: every interactive element suppresses double-tap zoom", async ({ page }) => {
  await page.goto(`${BASE}/`, { timeout: 60000, waitUntil: "networkidle" });
  assertAllManipulation(await scan(page), "/");
});

test("docs page: every interactive element suppresses double-tap zoom", async ({ page }) => {
  await page.goto(`${BASE}/docs?`, { timeout: 60000, waitUntil: "networkidle" });
  assertAllManipulation(await scan(page), "/docs");
});

test("demos page: every interactive element suppresses double-tap zoom", async ({ page }) => {
  await page.goto(`${BASE}/demos?`, { timeout: 60000, waitUntil: "networkidle" });
  assertAllManipulation(await scan(page), "/demos");
});

test("dashboard email client: every interactive element suppresses double-tap zoom", async ({ page }) => {
  await page.goto(`${BASE}/dashboard/email-client?`, { timeout: 60000, waitUntil: "domcontentloaded" });
  await page.waitForTimeout(3000);
  assertAllManipulation(await scan(page), "/dashboard/email-client");
});

// Overlay-gated checks: opened the same way each component's own spec
// opens them (combobox.spec.ts, color-picker.spec.ts, date-picker.spec.ts,
// sheet's own "Right" trigger demo) -- these states are unreachable from a
// plain page load, so they need their own scan after opening.
test.describe("overlay-gated elements", () => {
  test('overlay: combobox listbox open ("Switch workspace" options suppress double-tap zoom)', async ({ page }) => {
    const route = `${BASE}/component/?name=combobox&`;
    await page.goto(route, { timeout: 60000, waitUntil: "domcontentloaded" });
    await page.waitForTimeout(3000);
    const trigger = page.getByRole("combobox", { name: "Select framework" });
    await trigger.focus();
    await page.keyboard.press("ArrowDown");
    await expect(page.locator("[role='listbox'][data-state='open']")).toBeVisible();
    assertAllManipulation(await scan(page), `${route} (list open)`);
  });

  test("overlay: color_picker popover open (hue/area thumbs suppress double-tap zoom)", async ({ page }) => {
    const route = `${BASE}/component/?name=color_picker&`;
    await page.goto(route, { timeout: 60000, waitUntil: "networkidle" });
    const trigger = page.getByRole("button", { name: /Color picker/i }).first();
    await trigger.click({ timeout: 10000 });
    await expect(page.getByRole("dialog")).toBeVisible();
    assertAllManipulation(await scan(page), `${route} (popover open)`);
  });

  test("overlay: date_picker calendar popover open (day cells suppress double-tap zoom)", async ({ page }) => {
    const route = `${BASE}/component/?name=date_picker&`;
    await page.goto(route, { timeout: 60000, waitUntil: "networkidle" });
    const trigger = page.getByRole("button", { name: "Show Calendar" }).first();
    await trigger.click({ timeout: 10000 });
    await expect(page.getByRole("dialog")).toBeVisible();
    assertAllManipulation(await scan(page), `${route} (calendar open)`);
  });

  test("overlay: sheet open (its controls suppress double-tap zoom)", async ({ page }) => {
    const route = `${BASE}/component/?name=sheet&`;
    await page.goto(route, { timeout: 60000, waitUntil: "networkidle" });
    await page.getByRole("button", { name: "Right" }).first().click({ timeout: 10000 });
    await page.waitForTimeout(300);
    assertAllManipulation(await scan(page), `${route} (sheet open)`);
  });

  test("overlay: dropdown menu open (menu items suppress double-tap zoom)", async ({ page }) => {
    const route = `${BASE}/component/?name=dropdown_menu&`;
    await page.goto(route, { timeout: 60000, waitUntil: "networkidle" });
    const trigger = page.getByRole("button", { name: /Open Menu/i }).first();
    await trigger.click({ timeout: 10000 });
    await expect(page.locator('[role="menu"][data-state="open"]')).toBeVisible();
    assertAllManipulation(await scan(page), `${route} (menu open)`);
  });

  // Every library overlay type open at once, on the largest single probe
  // surface in the app -- the same fixture top-layer.spec.ts's own Rule 1
  // opens every one of, reused here rather than re-deriving open steps.
  test("top_layer fixture: clipping-escape popovers open (content suppresses double-tap zoom)", async ({ page }) => {
    const route = `${BASE}/component/?name=top_layer&`;
    await page.goto(route, { timeout: 60000, waitUntil: "networkidle" });
    await page.locator("#clip-popover-trigger").click();
    await expect(page.locator("#clip-popover-content")).toBeVisible();
    assertAllManipulation(await scan(page), `${route} (popover open)`);
  });
});

// Every `preview/src/components/*` page, per `preview/src/components/mod.rs`'s
// `examples!` list -- mirrors touch-focus-zoom.spec.ts's own COMPONENTS
// enumeration exactly (same rationale: one page load covers every listed
// variant, since `ComponentHighlight` renders them all inline).
const COMPONENTS = [
  "accordion", "alert_dialog", "aspect_ratio", "avatar", "badge", "button",
  "calendar", "card", "checkbox", "collapsible", "color_picker", "combobox",
  "context_menu", "date_picker", "dialog", "drag_and_drop_list",
  "dropdown_menu", "form", "hover_card", "input", "item", "label", "menubar",
  "navbar", "pagination", "popover", "progress", "radio_group", "scroll_area",
  "select", "separator", "sheet", "sidebar", "skeleton", "slider", "switch",
  "tabs", "tag_group", "textarea", "toast", "toggle", "toggle_group",
  "toolbar", "tooltip", "top_layer", "virtual_list",
];

for (const name of COMPONENTS) {
  test(`component "${name}": every interactive element suppresses double-tap zoom`, async ({ page }) => {
    const route = `${BASE}/component/?name=${name}&`;
    await page.goto(route, { timeout: 60000, waitUntil: "networkidle" });
    assertAllManipulation(await scan(page), route);
  });
}
