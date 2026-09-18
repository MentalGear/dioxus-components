import { test, expect } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";
import { BASE_URL } from "./base-url";

test("test", async ({ page }) => {
  await page.goto(`${BASE_URL}/component/?name=spinner&`, { timeout: 20 * 60 * 1000 });
  await expect(page.getByRole("status", { name: "Loading" })).toBeVisible();
  await expect(page.getByRole("status", { name: "Saving" })).toBeVisible();
});

test.describe("Axe automated scan", () => {
  // Spinner has no overlay/expand/select interaction -- one state to scan.
  test("loaded has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=spinner&`, { timeout: 20 * 60 * 1000 });
    await expectNoAxeViolations(page, "spinner: loaded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});
