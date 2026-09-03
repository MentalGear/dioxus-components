import { test, expect } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";

test("test", async ({ page }) => {
  await page.goto("http://127.0.0.1:8080/component/?name=tooltip&");
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

test.describe("Axe automated scan", () => {
  test("loaded (tooltip closed) has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto("http://127.0.0.1:8080/component/?name=tooltip&");
    await expectNoAxeViolations(page, "tooltip: loaded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });

  test("tooltip open has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto("http://127.0.0.1:8080/component/?name=tooltip&");
    await page.locator("#component-preview-frame").first().getByText("Rich content").hover();
    await expect(page.getByRole("tooltip")).toBeVisible();
    await expectNoAxeViolations(page, "tooltip: open", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});
