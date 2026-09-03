import { test, expect } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";

test("test", async ({ page }) => {
  await page.goto("http://127.0.0.1:8080/component/?name=input&", {
    timeout: 20 * 60 * 1000,
  }); // Increase timeout to 20 minutes

  await page.getByRole('textbox', { name: 'Enter your name' }).fill('name');
  await expect(page.locator('#input-greeting')).toContainText('Hello, name!');
});

test.describe("Axe automated scan", () => {
  // Input has no overlay/expand/select interaction -- one state to scan.
  test("loaded has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto("http://127.0.0.1:8080/component/?name=input&", { timeout: 20 * 60 * 1000 });
    // Wait for the wasm app to actually render before scanning -- without
    // this, axe can catch the pre-hydration document shell (no <main>, no
    // h1 yet) and report a false "page has no main landmark"/"no h1" that
    // has nothing to do with Input.
    await expect(page.getByRole('textbox', { name: 'Enter your name' })).toBeVisible();
    await expectNoAxeViolations(page, "input: loaded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});
