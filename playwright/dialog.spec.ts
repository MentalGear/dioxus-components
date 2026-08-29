import { test, expect } from '@playwright/test';

test('test', async ({ page }) => {
  await page.goto('http://127.0.0.1:8080/component/?name=dialog&', { timeout: 20 * 60 * 1000 }); // Increase timeout to 20 minutes
  await page.getByRole('button', { name: 'Show Dialog' }).click();
  // Assert the dialog is open
  const dialog = page.getByRole('dialog');
  await expect(dialog).toBeVisible();
  // Assert the close button is focused
  const closeButton = dialog.getByRole('button');
  await expect(closeButton).toBeFocused();
  // Hitting tab should keep focus on the close button
  await page.keyboard.press('Tab');
  await expect(closeButton).toBeFocused();
  // Hitting escape should close the dialog
  await page.keyboard.press('Escape');
  // Assert the dialog can no longer be found
  await expect(dialog).toHaveCount(0);

  // Reopen the dialog
  await page.getByRole('button', { name: 'Show Dialog' }).click();
  // Assert the dialog is open again
  await expect(dialog).toBeVisible();
  // Click the close button
  await closeButton.click();
  // Assert the dialog is closed after clicking close
  await expect(dialog).toHaveCount(0);

  // Reopen the dialog
  await page.getByRole('button', { name: 'Show Dialog' }).click();
  await expect(dialog).toBeVisible();
  // Clicking far outside the dialog content should dismiss it.
  await page.mouse.click(2, 2);
  await expect(dialog).toHaveCount(0);
});

test('dialog stays open when clicking non-focusable content inside it', async ({ page }) => {
  // Regression: `use_outside_dismiss` (shared with Popover) served pointerdown
  // and focusin with one handler. Clicking a non-focusable region inside the
  // dialog (e.g. this demo's "Item information" title) blurs the currently
  // focused control, and the browser moves focus to the nearest focusable
  // *ancestor* -- outside the dialog's root while still containing it. The
  // shared handler read that as focus leaving and closed the dialog.
  await page.goto('http://127.0.0.1:8080/component/?name=dialog&', { timeout: 20 * 60 * 1000 });
  await page.getByRole('button', { name: 'Show Dialog' }).click();
  const dialog = page.getByRole('dialog');
  await expect(dialog).toBeVisible();

  await dialog.getByText('Item information').click();

  await expect(dialog).toBeVisible();
});
