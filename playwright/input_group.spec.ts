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
