import { test, expect } from '@playwright/test';
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from './axe';
import { BASE_URL } from './base-url';

test('test', async ({ page }) => {
  await page.goto(`${BASE_URL}/component/?name=toggle&`, { timeout: 20 * 60 * 1000, waitUntil: 'networkidle' }); // Increase timeout to 20 minutes

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

// HARDENING (docs/backlog.md row 89), not a reproduced defect: `.dx-toggle:hover`
// and `.dx-toggle[data-state="on"]` tie at specificity (0,2,0) -- unlike
// calendar/tabs's own row 89 fix (an accidental extra-specificity bump from
// a stray `:not(...)`), this was a genuine tie broken only by source order,
// and `[data-state="on"]` already happened to be declared after `:hover` --
// so the pressed background already survived hover on the unmodified tree
// (verified, both themes, exact values below). `:where(:hover)` makes that
// unconditional (specificity, not source order) so reordering the rules --
// or `:hover` later gaining an extra clause the way calendar/tabs's did --
// can no longer silently flip the winner. Runs in both themes since the
// tokens differ per theme.
for (const dark of [false, true]) {
  test(`a pressed toggle keeps its "on" background while hovered (${dark ? 'dark' : 'light'} mode)`, async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=toggle&${dark ? 'dark_mode=true' : ''}`, { waitUntil: 'networkidle' });
    const toggle = page.getByRole('button', { name: 'B', exact: true });

    await toggle.click();
    await expect(toggle).toHaveAttribute('data-state', 'on');
    await page.locator('body').hover({ position: { x: 0, y: 0 } });
    // Wait past the CSS transition (--dx-motion-duration-base, 150ms) before
    // reading computed style -- a mid-transition read returns an
    // interpolated, neither-old-nor-new color.
    await page.waitForTimeout(400);
    const onAtRest = await toggle.evaluate((el) => getComputedStyle(el).backgroundColor);

    await toggle.hover();
    await page.waitForTimeout(400);
    expect(await toggle.evaluate((el) => el.matches(':hover'))).toBe(true);
    const onAndHovered = await toggle.evaluate((el) => getComputedStyle(el).backgroundColor);

    // Exact values confirmed identical before and after the `:where()` fix
    // (light: rgb(176, 176, 176); dark: rgb(62, 62, 62)) -- this asserts the
    // invariant the fix makes durable, not a color that changed.
    expect(onAndHovered, `onAtRest=${onAtRest} onAndHovered=${onAndHovered}`).toBe(onAtRest);
  });
}

test.describe('Axe automated scan', () => {
  test('loaded (off) has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=toggle&`, { timeout: 20 * 60 * 1000, waitUntil: 'networkidle' });
    await expectNoAxeViolations(page, 'toggle: off', { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });

  test('on has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=toggle&`, { timeout: 20 * 60 * 1000, waitUntil: 'networkidle' });
    await page.getByRole('button', { name: 'B', exact: true }).click();
    await expectNoAxeViolations(page, 'toggle: on', { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});
