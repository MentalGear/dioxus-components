/**
 * ORACLE: tier 2 (HTML) — text-entry font-size floor on touch devices.
 *
 * Source: this is documented WebKit/Apple **platform behaviour, not a W3C
 * rule** -- no HTML or CSS spec mandates it, and no other engine does it.
 * It is filed under tier 2 (rather than a fourth, Apple-only tier) because
 * its rule source and its fix's justification both cite standards
 * documents even though the *triggering* behaviour is vendor-specific:
 *
 *   (a) THE BEHAVIOUR (WebKit/Apple, not W3C): Mobile Safari zooms the page
 *       when a focused text-entry element -- an `input` of a text-like
 *       type, `textarea`, `select`, or `[contenteditable]` -- has a
 *       computed `font-size` below 16px. Documented by Apple's own Safari
 *       Web Content Guide: "If a text field's font size is less than
 *       16px, Safari will zoom in when the field is focused, and zoom back
 *       out when the field is blurred":
 *       https://developer.apple.com/library/archive/documentation/AppleApplications/Reference/SafariWebContent/UsingtheViewport/UsingtheViewport.html
 *       This is the same auto-zoom mechanism widely tracked as a WebKit
 *       quirk (see e.g. the long-standing community documentation at
 *       https://webkit.org/blog/ and countless duplicate reports); no
 *       WHATWG or CSSWG specification defines or requires it. Chromium and
 *       Gecko do not reproduce this behaviour.
 *
 *   (b) WHY `maximum-scale=1`/`user-scalable=no` IS NOT AN ACCEPTABLE FIX:
 *       WCAG 2.1 Success Criterion 1.4.4 "Resize Text" requires that text
 *       can be resized up to 200% without loss of content or
 *       functionality, with no exception for form fields:
 *       https://www.w3.org/WAI/WCAG21/Understanding/resize-text.html
 *       Disabling pinch-zoom via the viewport meta tag to suppress the
 *       auto-zoom instead blocks a low-vision user's own zoom globally --
 *       trading one accessibility defect for a worse one. This repo's
 *       `preview/index.html` viewport meta stays zoomable; do not add
 *       `maximum-scale` or `user-scalable=no` to "fix" this rule.
 *
 *   (c) THE CONSTRUCTION (consensus, not just this repo's opinion): shadcn's
 *       `Input` component ships `text-base md:text-sm` -- 16px below the
 *       `md` breakpoint, 14px at and above it -- precisely to floor
 *       touch-viewport font-size without changing the intentional desktop
 *       look: https://ui.shadcn.com/docs/components/input (component
 *       source: `components/ui/input.tsx` in `shadcn-ui/ui`). This repo's
 *       `Input`/`Textarea` already followed that construction (see
 *       `preview/src/components/input/style.css` and `.../textarea/
 *       style.css`); this file is what proves every *other* text-entry
 *       surface in the app does too.
 *
 * Rule: at a touch/coarse-pointer viewport, every text-entry element
 * matched by SELECTOR below, on every route in this app, computes
 * `font-size >= 16px`.
 *
 * Calibration (docs/conformance-harness.md, "Calibration"): the first test
 * below is a CALIBRATION test -- it asserts the iPhone 13 emulation this
 * whole file relies on actually yields `(pointer: coarse)` / `(hover:
 * none)`, matching real Mobile Safari's media-query environment. If that
 * test fails, every other test in this file is meaningless (wrong tool),
 * not a component defect.
 *
 * Enumeration (docs/plan.md-style oracle prep, 2026-09-02): every
 * `preview/src/components/*` route was visited at this viewport with a
 * one-off script (not part of this suite) to answer "what else does this"
 * before this file was written. Findings, verbatim (RED on the tree before
 * this file's paired fix commit):
 *
 *   - `Combobox` (`combobox/style.css`, "Switch workspace" widget included)
 *     -- 0.875rem (14px). THEMED component gap.
 *   - The navbar's language `<select class="dx-language-select">`, present
 *     on every single route -- 13.3333px (Chromium's UA default for
 *     unstyled form controls, which do not inherit `body`'s font-size the
 *     way ordinary text does). RAW element, app-wide.
 *   - `top_layer`'s two background-inertness probe inputs
 *     (`#dialog-inert-bg-input`, `#popover-modal-inert-bg-input`) and its
 *     clip-fixture Combobox-primitive trigger (`#clip-combobox-trigger`,
 *     `type="text"`) -- 13.3333px. RAW elements (this fixture composes
 *     `dioxus_primitives::` directly, per its own header comment; see
 *     `docs/preview-composition.md`).
 *   - `form`'s two native-reference `<select>` elements (`#fruit-native`,
 *     `#fruit-required-native`) -- 13.3333px. RAW elements, deliberately
 *     (they exist *as* the tier-2 form-participation native control).
 *   - `Calendar`'s month/year navigation -- `.dx-calendar-month-select`/
 *     `.dx-calendar-year-select`, an invisible (`opacity: 0`) native
 *     `<select>` overlaid on the visible `-value` span, the standard
 *     construction for a styled select whose native picker a touch device
 *     should still open on tap -- 13.3333px. THEMED component gap (found
 *     by execution; not visible from a `grep` for `input`, since it is a
 *     `select`).
 *   - Dashboard email client's inline reply box,
 *     `.ec-thread-compose-row [data-slot="textarea"]` -- 15px (0.9375rem),
 *     an app-specific override in `email_client.css` whose specificity
 *     beats the themed `Textarea`'s own `.dx-textarea` floor. THEMED
 *     component (`Textarea`) + app-layer override gap, needing its own fix
 *     in `email_client.css` (component-layer alone does not win the
 *     cascade here).
 *   - Already passing at 16px: `Input`, `Textarea` everywhere else,
 *     `DatePicker`'s `contenteditable` segments (inherit the page's
 *     unset-so-16px default -- no bug found, but given an explicit floor
 *     anyway; see that component's `style.css` comment), and every raw
 *     checkbox/radio input in `form` (excluded by SELECTOR: these are not
 *     text-entry).
 *   - Not applicable (no text-entry element exists at all, or the one that
 *     does is never user-focusable so the platform behaviour cannot
 *     trigger): `Select`'s visible trigger is a `<button>`, but it does
 *     also render a hidden native `<select data-slot="select-native">`
 *     for form participation (docs/plan.md Phase 1.3, the "BubbleSelect"
 *     pattern) -- `tabindex="-1"` + `pointer-events: none` + `opacity: 0`,
 *     so it is never reachable by tab or tap and excluded from SELECTOR
 *     below on that basis, not ignored; `Calendar`'s day cells are
 *     `<button>`s; `TagGroup` has no text-input affordance in this
 *     codebase's implementation; `ColorPicker`'s 2D-area thumb inputs are
 *     `type="range"` (excluded by SELECTOR) and its hex field is a themed
 *     `Input`, covered above.
 *
 * Fixture routes: this app's own routes, per `preview/src/main.rs`'s
 * `Route` enum and `preview/src/components/mod.rs`'s `examples!` list --
 * no vendored reference page exists for a rule this specific to one app's
 * inventory of text-entry surfaces.
 */

import { test, expect, devices, type Page } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "../../axe";

// Playwright's iPhone descriptors default to WebKit (`defaultBrowserType:
// "webkit"`), which this repo's local lanes do not ship; the rule under test
// is about the coarse-pointer media state and computed font-size, both of
// which Chromium's mobile emulation reproduces faithfully, so run the
// descriptor on Chromium here. A real WebKit/Mobile Safari project remains the
// missing calibration (docs/backlog.md row 4).
test.use({ ...devices["iPhone 13"], defaultBrowserType: "chromium" });

const BASE = "http://127.0.0.1:8080";

// Matches every element WebKit's auto-zoom rule cares about: text-like
// `input`s, `textarea`, `select`, and editable `contenteditable` hosts.
// Excluded `input` types are never rendered as a zoomable text caret on
// iOS (checkbox/radio/range paint their own control; button/submit/reset
// are buttons; color/file open their own picker UI; hidden is invisible).
// `[tabindex="-1"]` is excluded too, found by execution: `Select`'s hidden
// native `<select data-slot="select-native">` (the "BubbleSelect" form-
// participation pattern, docs/plan.md Phase 1.3) sets it alongside
// `pointer-events: none` and `opacity: 0`, so a real user can never tab or
// tap into it -- the platform behaviour this file is about cannot trigger
// on an element nothing can focus.
const SELECTOR =
  "input:not([type=checkbox]):not([type=radio]):not([type=range]):not([type=button]):not([type=submit]):not([type=reset]):not([type=color]):not([type=file]):not([type=hidden]):not([tabindex='-1']), textarea:not([tabindex='-1']), select:not([tabindex='-1']), [contenteditable]:not([contenteditable=false]):not([tabindex='-1'])";

type Row = {
  tag: string;
  id: string;
  cls: string;
  fontSize: string;
  label: string;
};

async function scan(page: Page): Promise<Row[]> {
  return page.$$eval(SELECTOR, (els) =>
    els.map((el) => {
      const cs = getComputedStyle(el as HTMLElement);
      return {
        tag: el.tagName.toLowerCase(),
        id: (el as HTMLElement).id || "",
        cls: (el.getAttribute("class") || "").slice(0, 60),
        fontSize: cs.fontSize,
        label:
          el.getAttribute("aria-label") ||
          el.getAttribute("placeholder") ||
          el.getAttribute("name") ||
          "",
      };
    }),
  );
}

function assertAllAtLeast16(rows: Row[], route: string) {
  const under = rows.filter((r) => parseFloat(r.fontSize) < 16);
  expect(
    under,
    `text-entry element(s) on ${route} compute below the 16px touch floor:\n` +
      under.map((r) => `  <${r.tag}> id="${r.id}" class="${r.cls}" label="${r.label}" font-size=${r.fontSize}`).join("\n"),
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

test("home page: every text-entry element is >= 16px", async ({ page }) => {
  await page.goto(`${BASE}/`, { timeout: 60000, waitUntil: "networkidle" });
  assertAllAtLeast16(await scan(page), "/");
});

test("docs page: every text-entry element is >= 16px", async ({ page }) => {
  await page.goto(`${BASE}/docs?`, { timeout: 60000, waitUntil: "networkidle" });
  assertAllAtLeast16(await scan(page), "/docs");
});

test("demos page: every text-entry element is >= 16px (blocks closed)", async ({ page }) => {
  await page.goto(`${BASE}/demos?`, { timeout: 60000, waitUntil: "networkidle" });
  assertAllAtLeast16(await scan(page), "/demos");
});

// Overlay-gated checks below run with a coarse pointer but a DESKTOP-width
// viewport: the rule is about `(pointer: coarse)` (what makes WebKit zoom),
// not about the mobile layout, and at 390px the dashboard sidebar collapses
// and touch-synthesised clicks on some triggers never open their overlay.
// `isMobile`/`hasTouch` from the iPhone descriptor keep `pointer: coarse`
// true whatever the viewport size (calibrated by the first test above).
test.describe("overlay-gated elements (coarse pointer, desktop-width viewport)", () => {
  test.use({ viewport: { width: 1024, height: 768 } });

  test("demos page: BlockColorPalette's hex field is >= 16px once its popover opens", async ({ page }) => {
    await page.goto(`${BASE}/demos?`, { timeout: 60000, waitUntil: "domcontentloaded" });
    await page.waitForTimeout(3000);
    const trigger = page.getByRole("button", { name: /Color picker/i }).first();
    test.skip((await trigger.count()) === 0, "no BlockColorPalette trigger rendered on /demos in this build");
    await trigger.scrollIntoViewIfNeeded();
    await trigger.focus();
    await page.keyboard.press("Enter");
    await expect(page.getByRole("dialog")).toBeVisible();
    assertAllAtLeast16(await scan(page), "/demos (BlockColorPalette popover open)");
  });

  test("dashboard email client: search input and compose form are >= 16px", async ({ page }) => {
    await page.goto(`${BASE}/dashboard/email-client?`, { timeout: 60000, waitUntil: "domcontentloaded" });
    await page.waitForTimeout(3000);
    assertAllAtLeast16(await scan(page), "/dashboard/email-client");

    const compose = page.getByRole("button", { name: /Compose/ }).first();
    await compose.focus();
    await page.keyboard.press("Enter");
    await expect(page.getByRole("dialog")).toBeVisible();
    assertAllAtLeast16(await scan(page), "/dashboard/email-client (compose open)");
  });

  // axe (static rules) -- no component spec reaches /dashboard/email-client
  // at all (it's a full dashboard demo, not a component gallery page), so
  // both its default and compose-open states are new coverage here, per
  // docs/backlog.md row 34.
  // KNOWN RED, filed rather than fixed (docs/backlog.md): each message row
  // (`ListPane`) renders `role="button"` on a `div` that also contains its
  // own further-interactive controls (star/flag toggles), which axe's
  // `nested-interactive` rule flags -- a real structural markup question
  // (how the row's own click-to-open should coexist with per-row controls)
  // that this round's "small, clearly correct" fix bar does not cover.
  test("dashboard email client: default state has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto(`${BASE}/dashboard/email-client?`, { timeout: 60000, waitUntil: "domcontentloaded" });
    // Wait for the inbox list to actually render before scanning, rather
    // than a fixed timeout -- see input.spec.ts's identical convention.
    await expect(page.getByRole("button", { name: /Compose/ }).first()).toBeVisible();
    await expectNoAxeViolations(page, "dashboard/email-client: loaded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });

  test("dashboard email client: compose open has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto(`${BASE}/dashboard/email-client?`, { timeout: 60000, waitUntil: "domcontentloaded" });
    const compose = page.getByRole("button", { name: /Compose/ }).first();
    await expect(compose).toBeVisible();
    await compose.focus();
    await page.keyboard.press("Enter");
    await expect(page.getByRole("dialog")).toBeVisible();
    await expectNoAxeViolations(page, "dashboard/email-client: compose open", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });

  test('overlay: combobox listbox open ("Switch workspace" input stays >= 16px)', async ({ page }) => {
    const route = `${BASE}/component/?name=combobox&`;
    await page.goto(route, { timeout: 60000, waitUntil: "domcontentloaded" });
    await page.waitForTimeout(3000);
    // Open from the keyboard, as combobox.spec.ts's keyboard test does.
    // By accessible name: the navbar's language <select> also has the
    // combobox role and precedes the Combobox input in DOM order.
    const trigger = page.getByRole("combobox", { name: "Select framework" });
    await trigger.focus();
    await page.keyboard.press("ArrowDown");
    await expect(page.locator("[role='listbox'][data-state='open']")).toBeVisible();
    assertAllAtLeast16(await scan(page), `${route} (list open)`);
  });
});


// Every `preview/src/components/*` page, per `preview/src/components/mod.rs`'s
// `examples!` list -- each page renders its "main" variant plus every listed
// variant inline (`ComponentHighlight`/`ComponentVariantHighlight` in
// `preview/src/main.rs`), so one page load covers every variant.
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
  test(`component "${name}": every text-entry element is >= 16px`, async ({ page }) => {
    const route = `${BASE}/component/?name=${name}&`;
    await page.goto(route, { timeout: 60000, waitUntil: "networkidle" });
    assertAllAtLeast16(await scan(page), route);
  });
}

// Overlay-gated text-entry elements: opened the way the component's own
// spec does (combobox.spec.ts, color-picker.spec.ts, sheet is opened via
// its own "Right" trigger button in its preview demo).
test("overlay: color_picker popover open (hex field stays >= 16px)", async ({ page }) => {
  const route = `${BASE}/component/?name=color_picker&`;
  await page.goto(route, { timeout: 60000, waitUntil: "networkidle" });
  const trigger = page.getByRole("button", { name: /Color picker/i }).first();
  await trigger.click({ timeout: 10000 });
  await expect(page.getByRole("dialog")).toBeVisible();
  assertAllAtLeast16(await scan(page), `${route} (popover open)`);
});

test("overlay: date_picker calendar popover open (segments stay >= 16px)", async ({ page }) => {
  const route = `${BASE}/component/?name=date_picker&`;
  await page.goto(route, { timeout: 60000, waitUntil: "networkidle" });
  const trigger = page.getByRole("button", { name: "Show Calendar" }).first();
  await trigger.click({ timeout: 10000 });
  await expect(page.getByRole("dialog")).toBeVisible();
  assertAllAtLeast16(await scan(page), `${route} (calendar open)`);
});

test("overlay: sheet open (its Input fields stay >= 16px)", async ({ page }) => {
  const route = `${BASE}/component/?name=sheet&`;
  await page.goto(route, { timeout: 60000, waitUntil: "networkidle" });
  await page.getByRole("button", { name: "Right" }).first().click({ timeout: 10000 });
  await page.waitForTimeout(300);
  assertAllAtLeast16(await scan(page), `${route} (sheet open)`);
});
