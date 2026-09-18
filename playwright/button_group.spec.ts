import { test, expect } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";
import { BASE_URL } from "./base-url";

test("test", async ({ page }) => {
  await page.goto(`${BASE_URL}/component/?name=button_group&`, { timeout: 20 * 60 * 1000 });
  // Scope to the demo frame: the component page renders its own source in a
  // code viewer and carries sitewide chrome, either of which can contribute
  // another role="group" and make an unscoped locator ambiguous.
  const groups = page.locator("#component-preview-frame").first().getByRole("group");
  await expect(groups).toHaveCount(2);
  await expect(groups.first().getByRole("button", { name: "Today" })).toBeVisible();
  await expect(groups.last().getByRole("button", { name: "More" })).toBeVisible();
});

test.describe("Axe automated scan", () => {
  // Button Group has no overlay/expand/select interaction -- one state to scan.
  test("loaded has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=button_group&`, { timeout: 20 * 60 * 1000 });
    await expectNoAxeViolations(page, "button_group: loaded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});
