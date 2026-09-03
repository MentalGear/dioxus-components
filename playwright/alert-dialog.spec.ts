import { test, expect } from '@playwright/test';
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from './axe';

test('test', async ({ page }) => {
  await page.goto('http://127.0.0.1:8080/component/?name=alert_dialog&', { timeout: 20 * 60 * 1000 }); // Increase timeout to 20 minutes
  await page.getByRole('button', { name: 'Show Alert Dialog' }).click();
  // Assert the dialog is open
  const dialog = page.getByRole('alertdialog');
  await expect(dialog).toBeVisible();
  // Assert the cancel button is focused
  const cancelButton = page.getByRole('button', { name: 'Cancel' });
  await expect(cancelButton).toBeFocused();
  // Hitting tab should move to the confirm button
  await page.keyboard.press('Tab');
  const confirmButtonForTab = page.getByRole('button', { name: 'Delete' });
  await expect(confirmButtonForTab).toBeFocused();
  // Phase 4.2 (docs/plan.md): the always-modal AlertDialog is now a native
  // `<dialog>` on the web arm. Chromium's own focus trap parks focus on
  // `<body>` for exactly one Tab stop after the last focusable element
  // before wrapping to the first (docs/phase4-spike-findings.md experiment
  // 4a) -- harness correction for the new trap's documented shape, same as
  // dialog.spec.ts's identical fix.
  await page.keyboard.press('Tab');
  await expect
    .poll(() => page.evaluate(() => document.activeElement === document.body))
    .toBe(true);
  // Hitting tab again should move focus back to the cancel button
  await page.keyboard.press('Tab');
  await expect(cancelButton).toBeFocused();
  // Hitting escape should close the dialog
  await page.keyboard.press('Escape');
  // Assert the dialog is closed
  await expect(dialog).toHaveCount(0);

  // Reopen the dialog
  await page.getByRole('button', { name: 'Show Alert Dialog' }).click();
  // Assert the dialog is open again
  await expect(dialog).toBeVisible();
  // Click the confirm button
  const confirmButton = page.getByRole('button', { name: 'Delete' });
  await confirmButton.click();
  // Assert the dialog is closed after confirming
  await expect(dialog).toHaveCount(0);
});

test.describe('Axe automated scan', () => {
  test('loaded (dialog closed) has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto('http://127.0.0.1:8080/component/?name=alert_dialog&', { timeout: 20 * 60 * 1000 });
    await expectNoAxeViolations(page, 'alert-dialog: loaded', { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });

  test('open has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto('http://127.0.0.1:8080/component/?name=alert_dialog&', { timeout: 20 * 60 * 1000 });
    await page.getByRole('button', { name: 'Show Alert Dialog' }).click();
    await expect(page.getByRole('alertdialog')).toBeVisible();
    await expectNoAxeViolations(page, 'alert-dialog: open', { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});
