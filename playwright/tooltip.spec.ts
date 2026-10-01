import { test, expect } from "./fixtures";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";
import { sampleCloseFade, assertFadesOutThenUnmounts } from "./assert-fade-out";
import { BASE_URL } from "./base-url";
import { gotoHydrated } from "./hydration";

test("test", async ({ page }) => {
  await gotoHydrated(page, `${BASE_URL}/component/?name=tooltip&`);
  let tooltip = page.getByRole("tooltip");
  // tabbing to the trigger element should show the tooltip
  await page.locator("#component-preview-frame").focus();
  await page.keyboard.press("Tab");
  await expect(tooltip).toBeVisible();
  // tabbing out of the trigger element should hide the tooltip
  await page.keyboard.press("Tab");
  await expect(tooltip).toHaveCount(0);

  // hovering over the trigger element should show the tooltip
  await page.locator("#component-preview-frame").first().getByText("Rich content").hover();
  await expect(tooltip).toBeVisible();

  // moving the mouse away from the trigger element should hide the tooltip
  await page.mouse.move(0, 0);
  await expect(tooltip).toHaveCount(0);
});

// dev-docs/backlog.md row 115 -- the product bug behind this file's load-only
// "tabbing out of the trigger element should hide the tooltip" flake. The
// popover's `toggle` event is dispatched asynchronously, AFTER `showPopover()`
// returns; `use_popover_shown_while_mounted` used to mirror that event's
// `newState === "open"` back into the `open` signal. When the focus-out (or
// mouse-leave) landed before that queued event -- input events outrank it
// under main-thread load -- the stale "open" echo re-opened the content with
// nothing left to close it (stuck open, forever). Made deterministic here by
// blurring synchronously inside `showPopover()`, i.e. strictly before its
// `toggle` task can run.
test("a focus-out in the same beat as the open stays closed (no stale toggle echo reopening it)", async ({ page }) => {
  await gotoHydrated(page, `${BASE_URL}/component/?name=tooltip&`);
  await page.evaluate(() => {
    const original = HTMLElement.prototype.showPopover;
    HTMLElement.prototype.showPopover = function (...args) {
      const result = original.apply(this, args);
      (document.activeElement as HTMLElement | null)?.blur();
      return result;
    };
  });
  await page.locator("#component-preview-frame").focus();
  await page.keyboard.press("Tab");
  await expect(page.getByRole("tooltip")).toHaveCount(0);
});

test.describe("Axe automated scan", () => {
  test("loaded (tooltip closed) has no automatically detectable a11y issues", async ({ page }) => {
    await gotoHydrated(page, `${BASE_URL}/component/?name=tooltip&`);
    await expectNoAxeViolations(page, "tooltip: loaded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });

  test("tooltip open has no automatically detectable a11y issues", async ({ page }) => {
    await gotoHydrated(page, `${BASE_URL}/component/?name=tooltip&`);
    await page.locator("#component-preview-frame").first().getByText("Rich content").hover();
    await expect(page.getByRole("tooltip")).toBeVisible();
    await expectNoAxeViolations(page, "tooltip: open", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});

// dev-docs/backlog.md rows 19 and 7: on the web (popover) arm,
// `use_popover_sync` used to call `hidePopover()` the instant `open` went
// false, and the UA's `[popover]:not(:popover-open) { display: none }`
// rule then hid the content before `dx-tooltip-fade-out`
// (`../preview/src/components/tooltip/style.css`, cherry-picked from
// upstream PR #293) ever got a chance to play -- RED before both fixes
// landed (reasoned through, not executed, since this lane cannot run
// Playwright: `display: none` from the very first sample, so
// `assertFadesOutThenUnmounts` would fail both the "no display:none while
// mounted" invariant and the "a few frames of decreasing opacity" check
// immediately). GREEN once `primitives/src/top_layer.rs`'s
// `use_popover_shown_while_mounted` (row 19) keeps the popover shown
// through the animation that CSS defines (row 7).
test.describe("Close-fade animation (docs/backlog.md rows 19, 7)", () => {
  test("content fades out (opacity -> 0, still popover-open) before unmounting", async ({ page }) => {
    await gotoHydrated(page, `${BASE_URL}/component/?name=tooltip&`, { timeout: 20 * 60 * 1000 });
    const trigger = page.locator("#component-preview-frame").first().getByText("Rich content");
    await trigger.hover();
    const tooltip = page.getByRole("tooltip");
    await expect(tooltip).toBeVisible();
    const contentId = await tooltip.getAttribute("id");
    if (!contentId) throw new Error("tooltip content has no id to sample");

    // `sampleCloseFade` settles the OPEN fade-in first (`toBeVisible()` passes
    // at opacity 0), starts sampling before the close trigger so the first
    // frames are never missed, fires the trigger and returns the capture --
    // see assert-fade-out.ts.
    const capture = await sampleCloseFade(page, contentId, () =>
      page.mouse.move(0, 0), // same close trigger as this file's own "test" above
    );

    assertFadesOutThenUnmounts(capture);
  });
});
