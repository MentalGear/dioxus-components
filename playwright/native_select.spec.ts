import { test, expect } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";

test("test", async ({ page }) => {
  await page.goto("http://127.0.0.1:8080/component/?name=native_select&", { timeout: 20 * 60 * 1000 });
  const select = page.locator("#component-preview-frame").first().getByRole("combobox");
  await expect(select).toHaveValue("apple");
  await select.selectOption("banana");
  await expect(select).toHaveValue("banana");
  await expect(page.getByText("Selected: banana")).toBeVisible();
});

test.describe("Axe automated scan", () => {
  // Native Select has no overlay/expand/select interaction -- it's a plain
  // native <select>, one state to scan.
  test("loaded has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto("http://127.0.0.1:8080/component/?name=native_select&", { timeout: 20 * 60 * 1000 });
    await expectNoAxeViolations(page, "native_select: loaded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});
