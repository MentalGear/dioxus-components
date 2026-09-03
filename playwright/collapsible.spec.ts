import { test, expect } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";

test("test", async ({ page }) => {
  await page.goto("http://127.0.0.1:8080/component/?name=collapsible&", { timeout: 20 * 60 * 1000 }); // Increase timeout to 20 minutes
  const preview = page.locator("#component-preview-frame").first();
  await page.getByRole("button", { name: "Recent Activity" }).click();
  await expect(preview.getByText("Fixed a bug in the collapsible component")).toBeVisible();
});

test.describe("Axe automated scan", () => {
  test("loaded (collapsed) has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto("http://127.0.0.1:8080/component/?name=collapsible&", { timeout: 20 * 60 * 1000 });
    await expectNoAxeViolations(page, "collapsible: collapsed", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });

  test("expanded has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto("http://127.0.0.1:8080/component/?name=collapsible&", { timeout: 20 * 60 * 1000 });
    const preview = page.locator("#component-preview-frame").first();
    await page.getByRole("button", { name: "Recent Activity" }).click();
    await expect(preview.getByText("Fixed a bug in the collapsible component")).toBeVisible();
    await expectNoAxeViolations(page, "collapsible: expanded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});
