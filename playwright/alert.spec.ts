import { test, expect } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";

test("test", async ({ page }) => {
  await page.goto("http://127.0.0.1:8080/component/?name=alert&", { timeout: 20 * 60 * 1000 });
  const alerts = page.getByRole("alert");
  await expect(alerts).toHaveCount(2);
  await expect(alerts.first()).toContainText("You can add components to your app");
  await expect(alerts.last()).toContainText("Unable to process your payment.");
});

test.describe("Axe automated scan", () => {
  // Alert has no overlay/expand/select interaction -- one state to scan.
  test("loaded has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto("http://127.0.0.1:8080/component/?name=alert&", { timeout: 20 * 60 * 1000 });
    await expectNoAxeViolations(page, "alert: loaded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});
