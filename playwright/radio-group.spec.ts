import { test, expect } from "@playwright/test";
import { expectNoAxeViolations, CONTRAST_TRACKED_ELSEWHERE } from "./axe";

test("test", async ({ page }) => {
  await page.goto("http://127.0.0.1:8080/component/?name=radio_group&", { timeout: 20 * 60 * 1000 }); // Increase timeout to 20 minutes
  await page.getByRole('radio', { name: 'Blue' }).click();
  await page.keyboard.press('ArrowDown');
  await expect(page.getByRole('radio', { name: 'Red' })).toBeFocused();
  await page.keyboard.press('ArrowDown');
  await expect(page.getByRole('radio', { name: 'Blue' })).toBeFocused();
});

test.describe("Axe automated scan", () => {
  test("loaded (none selected) has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto("http://127.0.0.1:8080/component/?name=radio_group&", { timeout: 20 * 60 * 1000 });
    // Wait for render before scanning -- see input.spec.ts's identical
    // comment for why (avoids a false pre-hydration "no main"/"no h1").
    await expect(page.getByRole('radio', { name: 'Blue' })).toBeVisible();
    await expectNoAxeViolations(page, "radio-group: loaded", { exclude: [CONTRAST_TRACKED_ELSEWHERE] });
  });

  test("an item selected has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto("http://127.0.0.1:8080/component/?name=radio_group&", { timeout: 20 * 60 * 1000 });
    await page.getByRole('radio', { name: 'Blue' }).click();
    await expectNoAxeViolations(page, "radio-group: Blue selected", { exclude: [CONTRAST_TRACKED_ELSEWHERE] });
  });
});
