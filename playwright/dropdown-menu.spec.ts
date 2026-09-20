import { test, expect } from '@playwright/test';
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from './axe';
import { BASE_URL } from './base-url';

test('test', async ({ page }) => {
  await page.goto(`${BASE_URL}/component/?name=dropdown_menu&`);
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
  // Clicking an item should close the menu.
  await page.getByRole('menuitem', { name: 'Edit' }).click();
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
  await page.goto(`${BASE_URL}/component/?name=dropdown_menu&`);
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

// REGRESSION (docs/backlog.md row 85): a raw DOM `.focus()` call landing
// directly on a menu item -- the exact thing a Playwright test driving
// focus programmatically does, and an assistive-technology focus-
// restoration path plausibly could -- used to close the whole menu and
// revert focus to the trigger, instead of simply focusing the item in
// place. Root cause: `DropdownMenuTrigger`'s own `onblur` decided "did
// focus leave the whole menu" by synchronously reading `ctx.focus.
// any_focused()`, a Rust-tracked roving-focus signal this crate's own
// click/keyboard code always updates *before* moving DOM focus -- but a
// raw external `.focus()` call updates nothing, so the guard read stale
// "nothing focused" state and closed. Confirmed RED on the unmodified
// tree: `trigger` ended up `data-state="closed"` after this exact
// sequence. Fixed by construction: `DropdownMenuContentRendered` now
// calls `use_outside_dismiss` (the same DOM-truth-based "did focus/a
// click land outside" check `ContextMenu`'s own root already used, never
// vulnerable to this because it asks `element.contains(event.target)`
// rather than trusting a signal), and the fragile `onblur` branch is
// removed. See `DropdownMenuSubTrigger`'s identical fix
// (`oracle/tier1-apg/menu-submenu.spec.ts`) for the sub-trigger instance
// of the same class.
test('a raw .focus() call on a plain item does not close the menu (row 85)', async ({ page }) => {
  await page.goto(`${BASE_URL}/component/?name=dropdown_menu&`);
  const trigger = page.getByRole('button', { name: 'Open Menu' });
  // Mouse-click-opened, deliberately with NO keyboard navigation
  // afterward -- this leaves `ctx.focus` (the root's own roving-focus
  // collection) never populated, exactly the precondition the bug needs.
  await trigger.click();
  await expect(trigger).toHaveAttribute('data-state', 'open');
  const editItem = page.getByRole('menuitem', { name: 'Edit' });
  await expect(editItem).toBeVisible();

  await editItem.focus();
  // Give any (correctly, now, no-op) reactive close a moment to have
  // fired if it were going to -- avoids a false green from asserting
  // before a real regression's own close would have completed.
  await page.waitForTimeout(150);

  await expect(trigger, 'the whole menu must not close').toHaveAttribute('data-state', 'open');
  await expect(editItem, 'the item must simply be focused in place').toBeFocused();
});

test.describe('Axe automated scan', () => {
  test('loaded (menu closed) has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=dropdown_menu&`);
    // Wait for render before scanning -- see input.spec.ts's identical
    // comment for why (avoids a false pre-hydration "no main"/"no h1").
    await expect(page.getByRole('button', { name: 'Open Menu' })).toBeVisible();
    await expectNoAxeViolations(page, 'dropdown-menu: loaded', { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });

  test('menu open has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=dropdown_menu&`);
    await page.getByRole('button', { name: 'Open Menu' }).click();
    await expect(page.getByRole('menu')).toBeVisible();
    await expectNoAxeViolations(page, 'dropdown-menu: menu open', { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});
