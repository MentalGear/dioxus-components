import { test, expect } from '@playwright/test';
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from './axe';

test('test', async ({ page }) => {
  await page.goto('http://127.0.0.1:8080/component/?name=dropdown_menu&');
  let menuElement = page.getByRole('button', { name: 'Open Menu' });
  // The menu should not be open initially
  await expect(menuElement).toHaveAttribute('data-state', 'closed');
  // Clicking the menu should open it
  await menuElement.click();
  await expect(menuElement).toHaveAttribute('data-state', 'open');
  // Pressing down should focus the first item
  await page.keyboard.press('ArrowDown');
  await expect(page.getByRole('menuitem', { name: 'Edit' })).toBeFocused();
  await expect(page.getByRole('menuitem', { name: 'Undo' })).toHaveAttribute('data-disabled', 'true');
  await page.keyboard.press('ArrowDown');
  await expect(page.getByRole('menuitem', { name: 'Duplicate' })).toBeFocused();
  // The menu should close after selecting an item
  await page.keyboard.press('Enter');
  await expect(menuElement).toHaveAttribute('data-state', 'closed');
  // The selected item should be displayed
  await expect(page.getByText('Selected: Duplicate')).toBeVisible();

  // Reopen the menu
  await menuElement.click();
  await expect(menuElement).toHaveAttribute('data-state', 'open');
  // Pressing Escape should close the menu
  await page.keyboard.press('Escape');
  await expect(menuElement).toHaveAttribute('data-state', 'closed');

  // Reopen the menu
  await menuElement.click();
  await expect(menuElement).toHaveAttribute('data-state', 'open');
  // Pressing Tab should close the menu
  await page.keyboard.press('Tab');
  await expect(menuElement).toHaveAttribute('data-state', 'closed');

  // Reopen the menu
  await menuElement.click();
  await expect(menuElement).toHaveAttribute('data-state', 'open');
  // Clicking outside the menu should close it
  await page.locator('body').click({ position: { x: 0, y: 0 } });
  await expect(menuElement).toHaveAttribute('data-state', 'closed');

  // Reopen the menu
  await menuElement.click();
  await expect(menuElement).toHaveAttribute('data-state', 'open');
  // Clicking an item should close the menu. Scoped to the open menu
  // specifically (rather than a bare page-wide `getByRole('menuitem', ...)`)
  // because the preceding outside-click step above (`body` at (0, 0)) lands
  // on the site's own top-left nav-brand link and triggers an unrelated,
  // pre-existing SPA-navigation defect in `preview/src/main.rs`: the
  // previous route's DOM (this exact DropdownMenu content included) is not
  // unmounted when the client-side router navigates, so another page's own
  // "Edit"-labeled control (e.g. a leaked Menubar/Navbar demo, both of which
  // also use role="menuitem") can end up in the accessibility tree
  // alongside this one, colliding on name+role. Filed as a fresh backlog
  // candidate (found verifying `docs/backlog.md` row 24's fix) rather than
  // fixed here -- out of scope for an ARIA-role migration.
  await page
    .locator('[role="menu"][data-state="open"]')
    .getByRole('menuitem', { name: 'Edit' })
    .click();
  await expect(menuElement).toHaveAttribute('data-state', 'closed');
});

// Regression guard for the 2026-09-04 user report ("dropdown menu has full
// page width"): `DropdownMenuContent` is a fit-content popover positioned
// under its trigger, not a full-viewport-width one. See
// `oracle/tier2-html/top-layer.spec.ts`'s "Rule 13" for the full mechanism
// this guards against (a duplicated/un-folded `style` attribute letting the
// popover UA stylesheet's centering-trap default, or a stray caller
// `width`, stretch the content) and why every anchored content's own
// stylesheet authors `min-width` only, never `width`.
test('open content is fit-content width, not full page width, and sits near its trigger', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.goto('http://127.0.0.1:8080/component/?name=dropdown_menu&');
  const trigger = page.getByRole('button', { name: 'Open Menu' });
  const triggerBox = await trigger.boundingBox();
  await trigger.click();
  const content = page.getByRole('menu');
  await expect(content).toBeVisible();
  const box = await content.boundingBox();
  if (!triggerBox || !box) {
    throw new Error(`expected both trigger and content boxes, got trigger=${triggerBox} content=${box}`);
  }
  const viewport = page.viewportSize();
  const debug = JSON.stringify({ triggerBox, box, viewport });
  expect(box.width, debug).toBeLessThan((viewport?.width ?? 1280) * 0.6);
  // Left-aligned under its trigger (DropdownMenu's `side="bottom"
  // align="start"` contract) -- generous tolerance for viewport-edge
  // clamping, just enough to catch full detachment to the viewport's own
  // left edge.
  expect(Math.abs(box.x - triggerBox.x), debug).toBeLessThan(200);
});

test.describe('Axe automated scan', () => {
  test('loaded (menu closed) has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto('http://127.0.0.1:8080/component/?name=dropdown_menu&');
    // Wait for render before scanning -- see input.spec.ts's identical
    // comment for why (avoids a false pre-hydration "no main"/"no h1").
    await expect(page.getByRole('button', { name: 'Open Menu' })).toBeVisible();
    await expectNoAxeViolations(page, 'dropdown-menu: loaded', { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });

  test('menu open has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto('http://127.0.0.1:8080/component/?name=dropdown_menu&');
    await page.getByRole('button', { name: 'Open Menu' }).click();
    await expect(page.getByRole('menu')).toBeVisible();
    await expectNoAxeViolations(page, 'dropdown-menu: menu open', { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});
