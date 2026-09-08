import { test, expect } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";

test("test", async ({ page }) => {
  await page.goto("http://127.0.0.1:8080/component/?name=field&", { timeout: 20 * 60 * 1000 });
  // Label/control association: clicking the label focuses its input.
  const username = page.getByLabel("Username");
  await expect(username).toBeVisible();
  await username.click();
  await expect(username).toBeFocused();

  // The invalid field's error message is present and associated by role.
  await expect(page.getByRole("alert")).toHaveText("Enter a valid email address.");

  // The horizontal field: checkbox beside its label/description.
  await expect(page.getByRole("checkbox", { name: "Marketing emails" })).toBeVisible();
});

test.describe("Axe automated scan", () => {
  // Field has no overlay/expand/select interaction -- one state to scan.
  test("loaded has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto("http://127.0.0.1:8080/component/?name=field&", { timeout: 20 * 60 * 1000 });
    await expectNoAxeViolations(page, "field: loaded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});
