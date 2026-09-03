import { test, expect } from '@playwright/test';
import { expectNoAxeViolations, CONTRAST_TRACKED_ELSEWHERE } from './axe';

test('hover navigation', async ({ page }) => {
  await page.goto('http://127.0.0.1:8080/component/?name=navbar&', { timeout: 20 * 60 * 1000 }); // Increase timeout to 20 minutes
  // wait for the styles to load
  await expect(page.getByRole('menuitem', { name: 'Inputs' })).toHaveCSS('border-width', '0px');
  const inputsNav = page.getByRole('menu').filter({ has: page.getByRole('menuitem', { name: 'Inputs' }) }).first();
  await inputsNav.hover();
  await expect(inputsNav).toHaveAttribute('data-state', 'open');
  const calendar = page.getByRole('menuitem', { name: 'Calendar' });
  await expect(calendar).toBeVisible();
  await calendar.evaluate((element) => {
    (element as HTMLElement).click();
  });
  // Assert the url changed to the calendar component
  await expect(page).toHaveURL(/.*name=calendar/);
});

test('mobile navigation', async ({ page }) => {
  await page.goto('http://127.0.0.1:8080/component/?name=navbar&', { timeout: 20 * 60 * 1000 }); // Increase timeout to 20 minutes
  await page.getByRole('menuitem', { name: 'Inputs' }).tap();
  await page.getByRole('menuitem', { name: 'Calendar' }).tap();
  // Assert the url changed to the calendar component
  await expect(page).toHaveURL(/.*name=calendar/);
});

test('keyboard navigation', async ({ page }) => {
  await page.goto('http://127.0.0.1:8080/component/?name=navbar&', { timeout: 20 * 60 * 1000 }); // Increase timeout to 20 minutes
  await page.getByRole('menubar').focus();
  // Go right with the keyboard
  await page.keyboard.press('ArrowRight');
  // Assert the focus is on the information menu item
  await expect(page.getByRole('menuitem', { name: 'Information' })).toBeFocused();
  // Go left with the keyboard
  await page.keyboard.press('ArrowLeft');
  // Assert the focus is on the inputs menu item
  await expect(page.getByRole('menuitem', { name: 'Inputs' })).toBeFocused();
  await page.keyboard.press('ArrowDown');
  // Assert the focus is on the calendar menu item
  await expect(page.getByRole('menuitem', { name: 'Calendar' })).toBeFocused();
  await expect(page.getByRole('menuitem', { name: 'Slider' })).toHaveAttribute('data-disabled', 'true');
  await page.keyboard.press('ArrowDown');
  // Assert the disabled slider item is skipped
  await expect(page.getByRole('menuitem', { name: 'Checkbox' })).toBeFocused();
  // Click the focused menu item
  await page.keyboard.press('Enter');
  // Assert the url changed to the checkbox component
  await expect(page).toHaveURL(/.*name=checkbox/);
});

test.describe('Axe automated scan', () => {
  test('loaded (dropdown closed) has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto('http://127.0.0.1:8080/component/?name=navbar&', { timeout: 20 * 60 * 1000 });
    // Wait for render before scanning -- see input.spec.ts's identical
    // comment for why (avoids a false pre-hydration "no main"/"no h1").
    await expect(page.getByRole('menuitem', { name: 'Inputs' })).toBeVisible();
    await expectNoAxeViolations(page, 'navbar: loaded', { exclude: [CONTRAST_TRACKED_ELSEWHERE] });
  });

  test('Inputs dropdown open has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto('http://127.0.0.1:8080/component/?name=navbar&', { timeout: 20 * 60 * 1000 });
    const inputsNav = page.getByRole('menu').filter({ has: page.getByRole('menuitem', { name: 'Inputs' }) }).first();
    await inputsNav.hover();
    await expect(inputsNav).toHaveAttribute('data-state', 'open');
    await expectNoAxeViolations(page, 'navbar: Inputs dropdown open', { exclude: [CONTRAST_TRACKED_ELSEWHERE] });
  });
});
