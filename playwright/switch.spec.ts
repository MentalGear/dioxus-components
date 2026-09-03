import { test, expect } from '@playwright/test';
import { expectNoAxeViolations, CONTRAST_TRACKED_ELSEWHERE } from './axe';

test('test', async ({ page }) => {
  await page.goto('http://127.0.0.1:8080/component/?name=switch&', { timeout: 20 * 60 * 1000 }); // Increase timeout to 20 minutes
  await page.waitForLoadState('networkidle');
  let switchElement = page.getByRole('switch', { name: 'Switch Demo' });
  await expect(switchElement).toBeVisible();
  // The switch should not be checked initially
  await expect(switchElement).toHaveAttribute('data-state', 'unchecked');
  // Clicking the switch should check it
  await switchElement.click();
  await expect(switchElement).toHaveAttribute('data-state', 'checked');
  // Pressing space should also toggle the switch.
  // Use locator.press so the element is focused before the keystroke —
  // webkit does not always retain focus on a button after a synthetic click.
  await switchElement.press('Space');
  await expect(switchElement).toHaveAttribute('data-state', 'unchecked');
});

test.describe('Axe automated scan', () => {
  test('loaded (unchecked) has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto('http://127.0.0.1:8080/component/?name=switch&', { timeout: 20 * 60 * 1000, waitUntil: 'networkidle' });
    await expectNoAxeViolations(page, 'switch: unchecked', { exclude: [CONTRAST_TRACKED_ELSEWHERE] });
  });

  test('checked has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto('http://127.0.0.1:8080/component/?name=switch&', { timeout: 20 * 60 * 1000, waitUntil: 'networkidle' });
    await page.getByRole('switch', { name: 'Switch Demo' }).click();
    await expectNoAxeViolations(page, 'switch: checked', { exclude: [CONTRAST_TRACKED_ELSEWHERE] });
  });
});
