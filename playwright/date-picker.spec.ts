/**
 * DatePicker — calendar popover anchoring (live-site report, item 3,
 * 2026-09-01) and a minimal smoke suite. No date-picker spec existed
 * before this file (docs/backlog.md row 20) -- this closes part of that
 * backlog row (an e2e spec exists now); the rest of row 20's scope
 * (segment typing/arrows/backspace, range picker) is not covered here and
 * remains open, see this session's report.
 *
 * Root cause of the live-site report ("the calendar doesn't anchor to its
 * trigger"), confirmed by execution against this repo's dev server:
 *   1. `date_picker/style.css` never carried its own copy of the
 *      `@supports (anchor-name: --a)` block every other non-modal-arm
 *      consumer in this workspace has (grep turned up zero matches) --
 *      the same shape of gap fixed for `color_picker/style.css`.
 *   2. That alone was not sufficient: `date_picker::DatePickerPopover`
 *      (`primitives/src/date_picker.rs`) rendered its calendar on the
 *      *modal* popover arm unconditionally -- its own `is_modal` prop was
 *      declared but never forwarded to the `PopoverRoot` it renders. The
 *      modal arm's DOM-relative "centering trick" positions the popup
 *      under its *positioned ancestor* (the narrow date-input group), not
 *      its trigger, with no edge/collision avoidance -- confirmed by
 *      execution, a 276px-wide calendar centered under a ~178px-wide
 *      ancestor rendered ~138px off the left edge of the viewport on this
 *      repo's dev server, which is the actual shape of "doesn't anchor to
 *      its trigger."
 * Fixed by forwarding `is_modal` in the primitive, setting
 * `is_modal: false` on `preview/src/components/date_picker/component.rs`'s
 * `DatePickerPopover` calls (matching `ColorPickerPopover`'s own
 * `is_modal: false`), and adding the `@supports` block to this page's
 * `style.css` -- see each fix's own comment for the full account.
 */

import { test, expect, type Page } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";

const NAV_TIMEOUT = 20 * 60 * 1000; // first run compiles the app
const PAGE_URL = "http://127.0.0.1:8080/component/?name=date_picker&";

async function gotoDatePicker(page: Page) {
  await page.goto(PAGE_URL, { timeout: NAV_TIMEOUT, waitUntil: "networkidle" });
}

function trigger(page: Page) {
  return page.getByRole("button", { name: "Show Calendar" }).first();
}

function content(page: Page) {
  return page.getByRole("dialog");
}

async function rectOf(page: Page, locator: ReturnType<typeof trigger>) {
  return locator.evaluate((el) => {
    const r = (el as HTMLElement).getBoundingClientRect();
    return { top: r.top, left: r.left, right: r.right, bottom: r.bottom, width: r.width, height: r.height };
  });
}

test("opens the calendar popover on trigger click", async ({ page }) => {
  await gotoDatePicker(page);
  await expect(content(page)).toBeHidden();

  await trigger(page).click();

  await expect(content(page)).toBeVisible();
});

test("anchors the popup next to its trigger, not viewport-centered or offset", async ({ page }) => {
  await gotoDatePicker(page);
  await trigger(page).click();
  await expect(content(page)).toBeVisible();
  // Let the open-state fade-in settle (`dx-date-picker-popover-fade-in`,
  // `.15s ease-out`) before reading geometry -- same reasoning as
  // top-layer.spec.ts Rule 8's ColorPicker case: mid-animation reads can
  // catch a not-yet-settled box, unrelated to anchoring but noisy for it.
  await page.waitForTimeout(200);

  const t = await rectOf(page, trigger(page));
  const c = await rectOf(page, content(page));
  const viewport = await page.evaluate(() => ({ width: window.innerWidth, height: window.innerHeight }));
  const debug = JSON.stringify({ trigger: t, content: c, viewport });

  // Below the trigger (this fixture's default side is "bottom"), not
  // centered in the viewport -- the pre-fix modal arm rendered far enough
  // left of the trigger to spill off the viewport edge entirely, so
  // "roughly centered on the trigger's own horizontal position" is the
  // discriminating assertion, not merely "somewhere on screen."
  expect(c.top, debug).toBeGreaterThan(t.bottom);
  const triggerCenterX = (t.left + t.right) / 2;
  const contentCenterX = (c.left + c.right) / 2;
  expect(Math.abs(contentCenterX - triggerCenterX), debug).toBeLessThan(2);
  // And not the viewport-centered placement the broken modal arm fell
  // back to for content wider than its positioning ancestor: the
  // viewport's own horizontal center would coincide with the trigger's
  // only by accident on this fixture's layout, so assert against it
  // directly using a page-specific fixed offset would be fragile --
  // instead this is covered by the anchor-to-trigger assertion above,
  // which a viewport-centered box could only satisfy if the trigger
  // itself happened to sit at viewport center (it does not, on this
  // fixture's layout, confirmed by execution: trigger center vs. viewport
  // center differ by well over the 2px tolerance used above).
  expect(Math.abs(triggerCenterX - viewport.width / 2)).toBeGreaterThan(20);
});

test("uses the CSS-anchor path, not the JS-measured fallback (no inline top/left)", async ({ page }) => {
  await gotoDatePicker(page);
  // A raw DOM click (bypassing Playwright's own actionability auto-scroll)
  // deliberately keeps `window.scrollY` at 0 for this open: confirmed by
  // execution (this session's diagnosis) that this repo's Chromium build
  // computes a `[popover]` + `position: fixed` element's *very first*
  // `anchor()`-resolved position incorrectly whenever the document is
  // already scrolled at the moment the popover is shown -- off by exactly
  // the scroll offset, as if measured document-relative instead of
  // viewport-relative -- and that wrong value never self-corrects for the
  // life of that popover instance, even once real further scrolling
  // happens. `use_anchor_position_fallback`'s own fallback correctly
  // detects and compensates for this (this repo's actual, existing
  // protection against exactly this failure mode -- see its doc), so
  // end-user positioning is never wrong; this test's whole point, though,
  // is to isolate the *CSS-only* path specifically, which this Chromium
  // quirk can only be kept out of by not scrolling before the open it is
  // asserting about. `trigger(page).click()` here would implicitly
  // scroll the (below-the-fold) trigger into view first and trip this
  // every time.
  await trigger(page).evaluate((el) => (el as HTMLElement).click());
  await expect(content(page)).toBeVisible();

  // `use_anchor_position_fallback` (primitives/src/top_layer.rs) is the
  // only thing that ever writes an inline `top` on this element -- an
  // empty style here confirms the CSS-native `anchor()` path engaged
  // first, the same signal top-layer.spec.ts Rule 8 checks for its
  // ColorPicker case.
  const inlineTop = await content(page).evaluate((el) => (el as HTMLElement).style.top);
  expect(inlineTop).toBe("");
  const marker = await content(page).evaluate((el) => el.className.includes("dx-anchor-popover"));
  expect(marker).toBe(true);
});

test("offset to trigger is unchanged after scrolling (CSS anchor tracks scroll natively)", async ({ page }) => {
  await gotoDatePicker(page);
  // Raw DOM click, scrollY kept at 0 for the open -- see the identical
  // note on the "uses the CSS-anchor path" test above for why: this
  // repo's Chromium miscomputes the very first `anchor()`-resolved
  // position whenever the page is already scrolled at open time, which
  // would put this test on the (already-covered, and already correct)
  // fallback path instead of the CSS-native one this test exists to
  // isolate. The test's own scroll -- what is actually under test here --
  // still happens for real, afterward.
  await trigger(page).evaluate((el) => (el as HTMLElement).click());
  await expect(content(page)).toBeVisible();
  await page.waitForTimeout(200);

  const offsetOf = async () =>
    page.evaluate(() => {
      const c = document.querySelector('[class*="dx-anchor-popover"]')!.getBoundingClientRect();
      const t = document.querySelector('[style*="anchor-name"]')!.getBoundingClientRect();
      return { top: c.top - t.bottom, left: c.left - t.left };
    });

  const before = await offsetOf();
  await page.evaluate(() => window.scrollBy(0, 150));
  // No JS fallback is active on this path (CSS anchor positioning
  // recomputes natively on scroll), but a short wait keeps this robust
  // against any incidental reflow/paint delay, same tolerance top-layer.
  // spec.ts's Rule 8 uses for its own scroll-tracking cases.
  await page.waitForTimeout(150);
  const after = await offsetOf();

  const debug = JSON.stringify({ before, after });
  expect(after.top, debug).toBeCloseTo(before.top, 0);
  expect(after.left, debug).toBeCloseTo(before.left, 0);
});

test("Escape closes the popup", async ({ page }) => {
  await gotoDatePicker(page);
  await trigger(page).click();
  await expect(content(page)).toBeVisible();

  await page.keyboard.press("Escape");

  await expect(content(page)).toBeHidden();
});

test.describe("Axe automated scan", () => {
  test("loaded (popover closed) has no automatically detectable a11y issues", async ({ page }) => {
    await gotoDatePicker(page);
    await expectNoAxeViolations(page, "date-picker: loaded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });

  test("calendar popover open has no automatically detectable a11y issues", async ({ page }) => {
    await gotoDatePicker(page);
    await trigger(page).click();
    await expect(content(page)).toBeVisible();
    await expectNoAxeViolations(page, "date-picker: calendar popover open", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});
