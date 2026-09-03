import { test, expect } from '@playwright/test';
import { expectNoAxeViolations, CONTRAST_TRACKED_ELSEWHERE } from './axe';

test('test', async ({ page }) => {
  await page.goto('http://127.0.0.1:8080/component/?name=toast&');
  // Create a toast
  await page.getByRole('button', { name: 'Info (60s)' }).click();
  // Create another toast
  await page.getByRole('button', { name: 'Info (60s)' }).click();
  // exact: true — the default substring match also catches the docs
  // sidebar's "Close navigation" button whenever the stylesheet that hides
  // it at desktop widths has not fully applied yet (its @import blocks the
  // rest of the cascade on slow networks), sending both clicks to it
  // instead of a toast.
  const toast_close_buttons = page.getByRole('button', { name: 'close', exact: true });
  // Hover and close the first toast
  await toast_close_buttons.first().hover();
  await toast_close_buttons.first().click();
  await expect(toast_close_buttons).toHaveCount(1);

  // Hover and close the second toast
  await toast_close_buttons.first().hover();
  await toast_close_buttons.first().click();
  await expect(toast_close_buttons).toHaveCount(0);
});

test.describe('Axe automated scan', () => {
  test('loaded (no toast) has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto('http://127.0.0.1:8080/component/?name=toast&');
    await expectNoAxeViolations(page, 'toast: loaded', { exclude: [CONTRAST_TRACKED_ELSEWHERE] });
  });

  test('a toast shown has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto('http://127.0.0.1:8080/component/?name=toast&');
    await page.getByRole('button', { name: 'Info (60s)' }).click();
    await expect(page.getByRole('button', { name: 'close', exact: true }).first()).toBeVisible();
    await expectNoAxeViolations(page, 'toast: toast shown', { exclude: [CONTRAST_TRACKED_ELSEWHERE] });
  });
});
