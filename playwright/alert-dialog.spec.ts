import { test, expect } from "./fixtures";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from './axe';
import { BASE_URL } from './base-url';
import { assertSharedBackdrop, captureBackdrop, resolveOverlayMs } from "./assert-backdrop-fade";

test('test', async ({ page }) => {
  await page.goto(`${BASE_URL}/component/?name=alert_dialog&`, { timeout: 20 * 60 * 1000 }); // Increase timeout to 20 minutes
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
    await page.goto(`${BASE_URL}/component/?name=alert_dialog&`, { timeout: 20 * 60 * 1000 });
    await expectNoAxeViolations(page, 'alert-dialog: loaded', { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });

  test('open has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=alert_dialog&`, { timeout: 20 * 60 * 1000 });
    await page.getByRole('button', { name: 'Show Alert Dialog' }).click();
    await expect(page.getByRole('alertdialog')).toBeVisible();
    await expectNoAxeViolations(page, 'alert-dialog: open', { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});

// The scrim every modal overlay shares (assert-backdrop-fade.ts has the full
// claim; popover/dialog/alert-dialog/sheet/drawer specs all run the identical
// assertion): the dialog's own `::backdrop` is the only painter, black/10 with a
// 4px blur, and it fades in and out for the same length. Before this, AlertDialog
// stacked its own animated wrapper scrim on the UA's un-animated one and the
// exit snapped.
test.describe("Scrim", () => {
  test("AlertDialog paints the shared scrim and fades it in and out", async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=alert_dialog&`, { timeout: 20 * 60 * 1000 });
    const trigger = page.getByRole("button", { name: "Show Alert Dialog", exact: true });
    await expect(trigger).toBeVisible();
    const capture = await captureBackdrop(
      page,
      () => trigger.click(),
      () => page.keyboard.press("Escape"),
    );
    assertSharedBackdrop(capture, await resolveOverlayMs(page), "alertdialog");
  });
});
