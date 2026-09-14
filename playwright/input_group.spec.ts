import { test, expect } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";

test("test", async ({ page }) => {
  await page.goto("http://127.0.0.1:8080/component/?name=input_group&", { timeout: 20 * 60 * 1000 });
  const search = page.getByPlaceholder("Search...");
  await expect(search).toBeVisible();
  await search.fill("hello");
  await expect(search).toHaveValue("hello");

  await expect(page.getByText("$", { exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Convert" })).toBeVisible();
});

test.describe("Axe automated scan", () => {
  // Input Group has no overlay/expand/select interaction -- one state to scan.
  test("loaded has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto("http://127.0.0.1:8080/component/?name=input_group&", { timeout: 20 * 60 * 1000 });
    await expectNoAxeViolations(page, "input_group: loaded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});

/*
 * The wrapped input carries no border/radius of its own -- regression
 * guard for a real bug found and fixed 2026-09-14. `Input`'s component
 * used to render `class: "dx-input"` as a literal RSX attribute and then
 * spread `..attributes` after it; Dioxus's plain "later duplicate
 * attribute wins" rule meant `InputGroupInput`'s own `class:
 * "dx-input-group-control"` fully *replaced* `"dx-input"` rather than
 * being combined with it, so the rendered `<input>` carried only
 * `"dx-input-group-control"` and never got `"dx-input"`'s own UA-style
 * reset. `.dx-input-group .dx-input-group-control`'s
 * `border-radius: 0; box-shadow: none;` rule (this file's own style.css)
 * never had anything to override at the browser's *native* default input
 * chrome, which showed through inside `.dx-input-group`'s own CSS-drawn
 * border -- a visible double-border/"pill within a pill" seam. Fixed by
 * having `Input` compose its base class via `merge_attributes` (which
 * concatenates `class` values space-joined) instead of a plain spread.
 * Asserts both halves directly: the class union survived the merge, and
 * the *effect* that class union is supposed to produce (no border/radius
 * of the input's own) actually holds.
 */
test("the input inside the group keeps its base class and shows no border of its own", async ({ page }) => {
  await page.goto("http://127.0.0.1:8080/component/?name=input_group&", { timeout: 20 * 60 * 1000 });
  const search = page.getByPlaceholder("Search...");
  await expect(search).toBeVisible();

  const className = await search.evaluate((el) => el.className);
  expect(className.split(/\s+/)).toEqual(expect.arrayContaining(["dx-input", "dx-input-group-control"]));

  const style = await search.evaluate((el) => {
    const cs = getComputedStyle(el);
    return { boxShadow: cs.boxShadow, borderRadius: cs.borderRadius };
  });
  expect(style.boxShadow, JSON.stringify(style)).toBe("none");
  expect(parseFloat(style.borderRadius), JSON.stringify(style)).toBe(0);
});
