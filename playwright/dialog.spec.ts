import { test, expect } from '@playwright/test';
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from './axe';

test('test', async ({ page }) => {
  await page.goto('http://127.0.0.1:8080/component/?name=dialog&', { timeout: 20 * 60 * 1000 }); // Increase timeout to 20 minutes
  await page.getByRole('button', { name: 'Show Dialog' }).click();
  // Assert the dialog is open
  const dialog = page.getByRole('dialog');
  await expect(dialog).toBeVisible();
  // Assert the close button is focused. Scoped by name -- the dialog demo
  // also has an "Open Nested Dialog" button (see
  // playwright/oracle/tier3-radix/scroll-lock.spec.ts) so an unscoped
  // getByRole('button') would match more than one element here.
  const closeButton = dialog.getByRole('button', { name: 'Close' });
  await expect(closeButton).toBeFocused();
  // The dialog demo now also has an "Open Nested Dialog" button (see
  // playwright/oracle/tier3-radix/scroll-lock.spec.ts), so the focus trap's
  // tab cycle has two stops: Tab moves off the close button, and a second
  // Tab wraps back around to it.
  //
  // Phase 4.2 (docs/plan.md): the modal `Dialog` is now a native
  // `<dialog>` on the web arm, so this cycle goes through Chromium's own
  // focus trap rather than the vendored `FocusTrap`. Chromium's native trap
  // parks focus on `<body>` for exactly one Tab stop after the last
  // focusable element before wrapping to the first
  // (docs/phase4-spike-findings.md experiment 4a) -- invisible to the user
  // (no visible focus ring lands there) and it does not let focus escape
  // the dialog, so this is a harness correction for the new trap's
  // documented shape, not a behavior change under test.
  await page.keyboard.press('Tab');
  await expect(dialog.getByRole('button', { name: 'Open Nested Dialog' })).toBeFocused();
  await page.keyboard.press('Tab');
  await expect
    .poll(() => page.evaluate(() => document.activeElement === document.body))
    .toBe(true);
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

test.describe('Axe automated scan', () => {
  test('loaded (dialog closed) has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto('http://127.0.0.1:8080/component/?name=dialog&', { timeout: 20 * 60 * 1000 });
    // Wait for render before scanning -- see input.spec.ts's identical
    // comment for why (avoids a false pre-hydration "no main"/"no h1").
    await expect(page.getByRole('button', { name: 'Show Dialog' })).toBeVisible();
    await expectNoAxeViolations(page, 'dialog: loaded', { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });

  test('open has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto('http://127.0.0.1:8080/component/?name=dialog&', { timeout: 20 * 60 * 1000 });
    await page.getByRole('button', { name: 'Show Dialog' }).click();
    await expect(page.getByRole('dialog')).toBeVisible();
    await expectNoAxeViolations(page, 'dialog: open', { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});
