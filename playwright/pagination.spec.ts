import { test, expect } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";
import { BASE_URL } from "./base-url";

// Every `PaginationLink`/`PaginationPrevious`/`PaginationNext` renders a real
// `<a href="#">` (`preview/src/components/pagination/component.rs`) --
// clicking one navigates the page (a hash change still counts as a
// navigation) and breaks the rest of the test, so every test below only
// hovers/reads attributes, never clicks. Every locator below is scoped to
// `#component-preview-frame` (the page's own nav/sidebar chrome carries
// dozens of unrelated `<a>` links -- `menubar.spec.ts`'s "keyboard
// navigation" test uses the same scoping for the identical reason).

test("current page is marked aria-current=\"page\"; Previous/Next have accessible names", async ({ page }) => {
  await page.goto(`${BASE_URL}/component/?name=pagination&`, { waitUntil: "networkidle" });
  const preview = page.locator("#component-preview-frame").first();

  const current = preview.locator('a[aria-current="page"]');
  await expect(current).toBeVisible();
  await expect(current).toHaveText("2");

  await expect(preview.getByRole("link", { name: "Go to previous page" })).toBeVisible();
  await expect(preview.getByRole("link", { name: "Go to next page" })).toBeVisible();
  await expect(preview.getByRole("link", { name: "1", exact: true })).toBeVisible();
  await expect(preview.getByRole("link", { name: "3", exact: true })).toBeVisible();
});

// docs/backlog.md row 89's own watch-list item: "pagination's own equivalent
// rule pair has a subtle token mismatch left on the watch list." The
// mismatch is real -- `[data-active="true"]`'s own background
// (`var(--light, var(--primary-color)) var(--dark, var(--primary-color-3))`)
// and `[data-active="true"]:hover`'s (`var(--primary-color-4)`) are
// different token-family members, not a deliberate one-step-lighter pairing
// -- but it does NOT let a user lose the current-page indication on hover:
// `[data-active="true"]`'s own `border` (`var(--primary-color-6)`) is set by
// a rule `:hover` never touches at all, so it persists unconditionally, and
// no non-active link has a border in any state. Confirmed by the exact
// computed values below (both themes) -- this is the "genuinely benign"
// outcome the investigation allowed for, not a manufactured fix: the
// background does shift by a few RGB points on hover, but the border --
// the one property no other link's hover ever sets -- never changes.
for (const dark of [false, true]) {
  test(`hovering the current page keeps its border (the actual "you are here" signal) unchanged (${dark ? "dark" : "light"} mode)`, async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=pagination&${dark ? "dark_mode=true" : ""}`, { waitUntil: "networkidle" });
    const preview = page.locator("#component-preview-frame").first();
    const current = preview.locator('a[aria-current="page"]');
    const other = preview.getByRole("link", { name: "1", exact: true });

    await page.locator("body").hover({ position: { x: 0, y: 0 } });
    // Wait past the CSS transition (--dx-motion-duration-slow, 200ms)
    // before reading computed style -- a mid-transition read returns an
    // interpolated, neither-old-nor-new value.
    await page.waitForTimeout(400);
    const atRest = await current.evaluate((el) => {
      const s = getComputedStyle(el);
      return { bg: s.backgroundColor, border: s.borderColor, width: s.borderTopWidth };
    });

    await current.hover();
    await page.waitForTimeout(400);
    expect(await current.evaluate((el) => el.matches(":hover"))).toBe(true);
    const hovered = await current.evaluate((el) => {
      const s = getComputedStyle(el);
      return { bg: s.backgroundColor, border: s.borderColor, width: s.borderTopWidth };
    });

    // The border -- the one visual cue no other link's hover rule ever
    // sets -- is untouched by hovering the current page.
    expect(hovered.border, JSON.stringify({ atRest, hovered })).toBe(atRest.border);
    expect(hovered.width, JSON.stringify({ atRest, hovered })).toBe(atRest.width);
    expect(hovered.width, "the border must actually be present, not 0px").not.toBe("0px");

    // A non-active link never has this border, hovered or not -- so the
    // current page stays visually distinguishable from every other link
    // regardless of which one the pointer happens to rest on.
    await other.hover();
    await page.waitForTimeout(400);
    const otherHoveredWidth = await other.evaluate((el) => getComputedStyle(el).borderTopWidth);
    expect(otherHoveredWidth, "a non-active link must not gain a border on hover").toBe("0px");
  });
}

test.describe("Axe automated scan", () => {
  test("loaded has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=pagination&`, { waitUntil: "networkidle" });
    await expectNoAxeViolations(page, "pagination: loaded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});
