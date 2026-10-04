import { test, expect } from "./fixtures";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from './axe';
import { BASE_URL } from './base-url';
import { gotoHydrated } from './hydration';

test('test', async ({ page }) => {
  await page.goto(`${BASE_URL}/component/?name=checkbox&`, { timeout: 20 * 60 * 1000, waitUntil: 'networkidle' }); // Increase timeout to 20 minutes
  let checkbox = page.getByRole('checkbox', { name: 'Demo Checkbox' });
  await expect(checkbox).toBeVisible();
  // The checkbox should not be checked initially
  await expect(checkbox).toHaveAttribute('data-state', 'unchecked');
  // Clicking the checkbox should check it
  await checkbox.click();
  await expect(checkbox).toHaveAttribute('data-state', 'checked');
  // Pressing space should also toggle the checkbox.
  // // Use locator.press so the element is focused before the keystroke —
  // // webkit does not always retain focus on a button after a synthetic click.
  // await checkbox.press('Space');
  await page.keyboard.press('Space');
  await expect(checkbox).toHaveAttribute('data-state', 'unchecked');
});

test.describe('Axe automated scan', () => {
  test('loaded (unchecked) has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=checkbox&`, { timeout: 20 * 60 * 1000, waitUntil: 'networkidle' });
    await expectNoAxeViolations(page, 'checkbox: unchecked', { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });

  test('checked has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=checkbox&`, { timeout: 20 * 60 * 1000, waitUntil: 'networkidle' });
    await page.getByRole('checkbox', { name: 'Demo Checkbox' }).click();
    await expectNoAxeViolations(page, 'checkbox: checked', { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});

test('the box and its indicator keep one constant size whether or not it is checked', async ({ page }) => {
  // The primitive mounts the indicator's children (the check icon) only while the box is
  // checked. A content-sized indicator was therefore 0x0 unchecked and 16x16 checked -- the
  // inside of the box changed shape on every toggle. The indicator now always fills the box.
  await gotoHydrated(page, `${BASE_URL}/component/?name=checkbox&`, { timeout: 20 * 60 * 1000 });
  const checkbox = page.getByRole('checkbox', { name: 'Demo Checkbox' });
  await expect(checkbox).toBeVisible();

  const boxes = () =>
    checkbox.evaluate((el) => {
      const rect = (e: Element) => {
        const r = e.getBoundingClientRect();
        return { x: r.x, y: r.y, width: r.width, height: r.height };
      };
      return { checkbox: rect(el), indicator: rect(el.querySelector(':scope > span')!), display: getComputedStyle(el).display };
    });

  const unchecked = await boxes();
  await checkbox.click();
  await expect(checkbox).toHaveAttribute('data-state', 'checked');
  const checked = await boxes();

  for (const [state, b] of [['unchecked', unchecked], ['checked', checked]] as const) {
    expect(b.checkbox.width, `${state}: checkbox width`).toBeCloseTo(16, 1);
    expect(b.checkbox.height, `${state}: checkbox height`).toBeCloseTo(16, 1);
    expect(b.indicator, `${state}: indicator fills the checkbox`).toEqual(b.checkbox);
    // A flex box, not an inline-block: an inline-block button sits on the line box's baseline
    // (2px above a table cell's centre) -- see data_table.spec.ts.
    expect(b.display, `${state}: display`).toBe('flex');
  }
  expect(checked.checkbox, 'checking does not move or resize the checkbox').toEqual(unchecked.checkbox);
});
