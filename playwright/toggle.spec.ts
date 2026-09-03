import { test, expect } from '@playwright/test';
import { expectNoAxeViolations, CONTRAST_TRACKED_ELSEWHERE } from './axe';

test('test', async ({ page }) => {
  await page.goto('http://127.0.0.1:8080/component/?name=toggle&', { timeout: 20 * 60 * 1000, waitUntil: 'networkidle' }); // Increase timeout to 20 minutes

  let toggleElement = page.getByRole('button', { name: 'B', exact: true });
  await expect(toggleElement).toBeVisible();
  // The toggle should not be checked initially
  await expect(toggleElement).toHaveAttribute('data-state', 'off');
  // // Clicking the toggle should check it
  await toggleElement.click();
  await expect(toggleElement).toHaveAttribute('data-state', 'on');
  // Pressing space should also toggle the toggle.
  // Use locator.press so the element is focused before the keystroke —
  // webkit does not always retain focus on a button after a synthetic click.
  // await toggleElement.press('Space');
  // await toggleElement.press('Space');
  await page.keyboard.press('Space');
  await expect(toggleElement).toHaveAttribute('data-state', 'off');
});

test.describe('Axe automated scan', () => {
  test('loaded (off) has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto('http://127.0.0.1:8080/component/?name=toggle&', { timeout: 20 * 60 * 1000, waitUntil: 'networkidle' });
    await expectNoAxeViolations(page, 'toggle: off', { exclude: [CONTRAST_TRACKED_ELSEWHERE] });
  });

  test('on has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto('http://127.0.0.1:8080/component/?name=toggle&', { timeout: 20 * 60 * 1000, waitUntil: 'networkidle' });
    await page.getByRole('button', { name: 'B', exact: true }).click();
    await expectNoAxeViolations(page, 'toggle: on', { exclude: [CONTRAST_TRACKED_ELSEWHERE] });
  });
});
