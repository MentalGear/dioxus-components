import { test, expect } from "./fixtures";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from './axe';
import { BASE_URL } from './base-url';
import { gotoHydrated } from './hydration';
import { expectFadeKeepsTransform } from './assert-anchor-transform';
import { awaitAnimationsSettled } from './animations';

test('pointer navigation', async ({ page }) => {
  await page.goto(`${BASE_URL}/component/?name=context_menu&`, { timeout: 20 * 60 * 1000 }); // Increase timeout to 20 minutes
  await page.getByRole('button', { name: 'right click here' }).click({
    button: 'right'
  });

  // Assert the context menu is visible
  const contextMenu = page.getByRole('menu');
  await expect(contextMenu).toHaveAttribute('data-state', 'open');
  // Click on the "Edit" menu item
  await page.getByRole('menuitem', { name: 'Edit' }).click();
  // Assert the context menu is closed after clicking
  await expect(contextMenu).toHaveCount(0);
});

test('menu lands at the tap coordinates on touch long-press', async ({ page }) => {
  await page.goto(`${BASE_URL}/component/?name=context_menu&`, { timeout: 20 * 60 * 1000 });

  const trigger = page.getByRole('button', { name: 'right click here' });
  const contextMenu = page.getByRole('menu');

  // Pin the trigger to a fixed on-screen position instead of pushing it
  // down with padding on <main>/<body>. This test's whole point is a tap
  // point guaranteed to be nowhere near any viewport edge (so
  // `use_point_anchor_clamp`, top_layer.rs, never has a reason to act and
  // this test stays a clean "no clamping" baseline) -- but padding only
  // *adds* to whatever height the surrounding page content already has,
  // and that height isn't this test's to control or assume. It was 300px
  // of padding-top here, unchanged since this test was written; on this
  // component's actual current preview-page chrome, that pushed the
  // trigger's own center to y ~= 735 against a 720px-tall default
  // viewport -- 15px *past* the bottom edge, not "away from it" as the
  // comment this replaces claimed. That was already true on this branch's
  // parent commit (124c4d5, before use_point_anchor_clamp existed at all)
  // -- it simply had nothing to act on it yet, so a menu positioned
  // exactly at that tap point silently rendered a hundred-plus px past
  // the fold and this test never noticed (it only checks the menu's
  // top-left against the tap point, never full on-screen visibility).
  // use_point_anchor_clamp landing gave the clamp something to correct
  // for the first time, and correctly pulled the menu back on-screen --
  // a real behavior change, but the intended one, not a bug in the clamp
  // itself (confirmed by execution: instrumented logging on this exact
  // run showed a genuinely non-zero, already-`:popover-open` content size
  // measured before the clamp ran, and viewport/tap numbers -- vh=720,
  // tapY=734.97, content height=168 -- for which
  // `top + ch > vh - EDGE_MARGIN` is correctly true). Anchoring the
  // trigger's position directly, rather than adding to an unmeasured
  // ambient height, makes the "nowhere near an edge" premise true by
  // construction instead of by an increasingly-stale coincidence.
  await trigger.evaluate((el) => {
    (el as HTMLElement).style.position = 'fixed';
    (el as HTMLElement).style.top = '120px';
    (el as HTMLElement).style.left = '160px';
  });

  const box = await trigger.boundingBox();
  if (!box) throw new Error('trigger has no bounding box');
  const tapX = box.x + box.width / 2;
  const tapY = box.y + box.height / 2;
  const pointerId = 7777;

  await trigger.evaluate((el, { x, y, pointerId }) => {
    el.dispatchEvent(new PointerEvent('pointerdown', {
      pointerId,
      pointerType: 'touch',
      isPrimary: true,
      clientX: x,
      clientY: y,
      button: 0,
      buttons: 1,
      bubbles: true,
      cancelable: true,
    }));
  }, { x: tapX, y: tapY, pointerId });

  await expect(contextMenu).toHaveAttribute('data-state', 'open');
  // Measure the SETTLED box: the open fade scales the panel (0.95 -> 1 about its centre), so a box read
  // mid-fade sits up to ~5.5px right of the tap point on a 220px menu -- this failed on every load-starved
  // run, with the old `transform` keyframes and the new `scale` ones alike (dev-docs/backlog.md row 115).
  await awaitAnimationsSettled(contextMenu);
  const menuBox = await contextMenu.boundingBox();
  if (!menuBox) throw new Error('menu has no bounding box');
  // The menu's top-left should be at the tap coords (give or take a px for
  // sub-pixel rounding). If it's off by tens of pixels, a viewport coord
  // system is mismatched somewhere.
  expect(Math.abs(menuBox.x - tapX)).toBeLessThan(2);
  expect(Math.abs(menuBox.y - tapY)).toBeLessThan(2);
});

// docs/backlog.md row 10, item 5.2 ("ContextMenu viewport clamping"): unlike
// every anchored overlay in this crate (Select/Combobox/DropdownMenu/
// Menubar/etc, whose shift/size clamp against the viewport is covered by
// `playwright/oracle/tier2-html/top-layer.spec.ts`'s rules 11-15), that
// file's own header doc explicitly disclaims this exact case -- ContextMenu
// opens at a raw click point with no anchor element for
// `use_anchor_position_fallback` to key off of, so its own regression
// coverage lives here instead, alongside this file's other point-anchor
// tests, rather than in that anchor-keyed spec. Both of `ContextMenuTrigger`'s
// two `position.set(...)` call sites (the native `contextmenu` mouse handler,
// and the manual long-press timer for touch/pen) are exercised below, using
// the exact same synthetic-event technique the "menu lands at the tap
// coordinates" test above already uses to control click coordinates
// independent of the trigger element's own on-screen position.
test('clamps the menu into the viewport for a mouse right-click near a corner', async ({ page }) => {
  await page.goto(`${BASE_URL}/component/?name=context_menu&`, { timeout: 20 * 60 * 1000 });
  const trigger = page.getByRole('button', { name: 'right click here' });
  const contextMenu = page.getByRole('menu');
  const viewport = page.viewportSize();
  if (!viewport) throw new Error('no viewport size');

  // A synthetic `contextmenu` event dispatched directly on the trigger
  // reaches `ContextMenuTrigger`'s `oncontextmenu` handler exactly like a
  // real right-click would, with `clientX`/`clientY` set to whatever this
  // test wants regardless of where the trigger itself actually renders --
  // here, a couple of pixels inside the viewport's bottom-right corner, far
  // enough into the corner that an unclamped menu of any reasonable size
  // would overflow both edges.
  const x = viewport.width - 2;
  const y = viewport.height - 2;
  await trigger.evaluate((el, { x, y }) => {
    el.dispatchEvent(new MouseEvent('contextmenu', {
      clientX: x,
      clientY: y,
      bubbles: true,
      cancelable: true,
    }));
  }, { x, y });

  await expect(contextMenu).toHaveAttribute('data-state', 'open');
  const box = await contextMenu.boundingBox();
  if (!box) throw new Error('menu has no bounding box');

  // EDGE_MARGIN (top_layer.rs's `use_point_anchor_clamp`) is 4px; allow 1px
  // of rounding slack on each side of the assertion.
  const EDGE_MARGIN = 4;
  expect(box.x).toBeGreaterThanOrEqual(EDGE_MARGIN - 1);
  expect(box.y).toBeGreaterThanOrEqual(EDGE_MARGIN - 1);
  expect(box.x + box.width).toBeLessThanOrEqual(viewport.width - EDGE_MARGIN + 1);
  expect(box.y + box.height).toBeLessThanOrEqual(viewport.height - EDGE_MARGIN + 1);
});

test('clamps the menu into the viewport for a touch long-press near a corner', async ({ page }) => {
  await page.goto(`${BASE_URL}/component/?name=context_menu&`, { timeout: 20 * 60 * 1000 });
  const trigger = page.getByRole('button', { name: 'right click here' });
  const contextMenu = page.getByRole('menu');
  const viewport = page.viewportSize();
  if (!viewport) throw new Error('no viewport size');

  // Same technique as "menu lands at the tap coordinates" above, but
  // pointed at the top-left corner instead -- the long-press timer fires
  // ~500ms after this `pointerdown`, with no `pointerup` needed (see
  // `context_menu.rs`'s `ContextMenuTrigger::handle_pointer_down`).
  const x = 2;
  const y = 2;
  const pointerId = 8181;
  await trigger.evaluate((el, { x, y, pointerId }) => {
    el.dispatchEvent(new PointerEvent('pointerdown', {
      pointerId,
      pointerType: 'touch',
      isPrimary: true,
      clientX: x,
      clientY: y,
      button: 0,
      buttons: 1,
      bubbles: true,
      cancelable: true,
    }));
  }, { x, y, pointerId });

  await expect(contextMenu).toHaveAttribute('data-state', 'open');
  const box = await contextMenu.boundingBox();
  if (!box) throw new Error('menu has no bounding box');

  const EDGE_MARGIN = 4;
  expect(box.x).toBeGreaterThanOrEqual(EDGE_MARGIN - 1);
  expect(box.y).toBeGreaterThanOrEqual(EDGE_MARGIN - 1);
  expect(box.x + box.width).toBeLessThanOrEqual(viewport.width - EDGE_MARGIN + 1);
  expect(box.y + box.height).toBeLessThanOrEqual(viewport.height - EDGE_MARGIN + 1);
});


test('touch long-press opens the context menu', async ({ page }) => {
  // iOS Safari does not fire `contextmenu` on long press, so the menu must
  // open from a held touch instead. Reproduces issue #262.
  await page.goto(`${BASE_URL}/component/?name=context_menu&`, { timeout: 20 * 60 * 1000 });
  const trigger = page.getByRole('button', { name: 'right click here' });
  const contextMenu = page.getByRole('menu');

  const box = await trigger.boundingBox();
  if (!box) throw new Error('trigger has no bounding box');
  const x = box.x + box.width / 2;
  const y = box.y + box.height / 2;
  const pointerId = 4242;

  await trigger.evaluate((el, { x, y, pointerId }) => {
    el.dispatchEvent(new PointerEvent('pointerdown', {
      pointerId,
      pointerType: 'touch',
      isPrimary: true,
      clientX: x,
      clientY: y,
      button: 0,
      buttons: 1,
      bubbles: true,
      cancelable: true,
    }));
  }, { x, y, pointerId });

  await expect(contextMenu).toHaveAttribute('data-state', 'open');

  // Release the touch after the menu has opened; it should stay open.
  await trigger.evaluate((el, { x, y, pointerId }) => {
    el.dispatchEvent(new PointerEvent('pointerup', {
      pointerId,
      pointerType: 'touch',
      isPrimary: true,
      clientX: x,
      clientY: y,
      bubbles: true,
    }));
  }, { x, y, pointerId });

  await expect(contextMenu).toHaveAttribute('data-state', 'open');
});

test('pen long-press opens the context menu', async ({ page }) => {
  await page.goto(`${BASE_URL}/component/?name=context_menu&`, { timeout: 20 * 60 * 1000 });
  const trigger = page.getByRole('button', { name: 'right click here' });
  const contextMenu = page.getByRole('menu');

  const box = await trigger.boundingBox();
  if (!box) throw new Error('trigger has no bounding box');
  const x = box.x + box.width / 2;
  const y = box.y + box.height / 2;
  const pointerId = 4244;

  await trigger.evaluate((el, { x, y, pointerId }) => {
    el.dispatchEvent(new PointerEvent('pointerdown', {
      pointerId,
      pointerType: 'pen',
      isPrimary: true,
      clientX: x,
      clientY: y,
      button: 0,
      buttons: 1,
      bubbles: true,
      cancelable: true,
    }));
  }, { x, y, pointerId });

  await expect(contextMenu).toHaveAttribute('data-state', 'open');
});

test('mouse pointerdown does not arm the long-press timer', async ({ page }) => {
  await page.goto(`${BASE_URL}/component/?name=context_menu&`, { timeout: 20 * 60 * 1000 });
  const trigger = page.getByRole('button', { name: 'right click here' });

  const box = await trigger.boundingBox();
  if (!box) throw new Error('trigger has no bounding box');
  const x = box.x + box.width / 2;
  const y = box.y + box.height / 2;
  const pointerId = 4245;

  await trigger.evaluate((el, { x, y, pointerId }) => {
    el.dispatchEvent(new PointerEvent('pointerdown', {
      pointerId,
      pointerType: 'mouse',
      isPrimary: true,
      clientX: x,
      clientY: y,
      button: 0,
      buttons: 1,
      bubbles: true,
      cancelable: true,
    }));
  }, { x, y, pointerId });

  // Hold past the long-press threshold; the menu must remain closed because
  // mouse pointers should only open via the native `contextmenu` event.
  await page.waitForTimeout(700);
  await expect(page.getByRole('menu')).toHaveCount(0);
});

test('touch tap outside closes the open menu', async ({ page }) => {
  await page.goto(`${BASE_URL}/component/?name=context_menu&`, { timeout: 20 * 60 * 1000 });
  const trigger = page.getByRole('button', { name: 'right click here' });
  const contextMenu = page.getByRole('menu');

  await trigger.click({ button: 'right' });
  await expect(contextMenu).toHaveAttribute('data-state', 'open');

  // Tap near the bottom-right of the viewport, well outside the menu.
  // Inset 30px from each edge rather than flush against it (`width - 10`):
  // the scroll-lock's permanent `scrollbar-gutter: stable` baseline (see
  // `primitives/src/scroll_lock.rs`) reserves a strip at the viewport's
  // right/bottom edges that `document.elementFromPoint` legitimately
  // returns null for -- real UA chrome, not a scrollable area -- even
  // though `window.innerWidth`/`clientWidth` don't shrink to reflect it (a
  // 0-width-overlay-scrollbar engine's `innerWidth` reports the same value
  // whether or not that strip is reserved). A coordinate flush against the
  // edge can land in that strip and fail with "no element at outside
  // point" for a reason that has nothing to do with this test's actual
  // claim; the same false negative and the same fix are documented in
  // `docs/phase4-spike-findings.md`'s spike spec.
  const viewport = page.viewportSize();
  if (!viewport) throw new Error('no viewport');
  const farX = viewport.width - 30;
  const farY = viewport.height - 30;
  await page.evaluate(({ x, y }) => {
    const target = document.elementFromPoint(x, y);
    if (!target) throw new Error('no element at outside point');
    target.dispatchEvent(new PointerEvent('pointerdown', {
      pointerId: 5050,
      pointerType: 'touch',
      isPrimary: true,
      clientX: x,
      clientY: y,
      button: 0,
      buttons: 1,
      bubbles: true,
      cancelable: true,
    }));
  }, { x: farX, y: farY });

  await expect(contextMenu).toHaveCount(0);
});

test('pointerdown at the trigger location does not dismiss an open menu', async ({ page }) => {
  // Regression for the long-press dismiss bug: on iOS Safari a fresh
  // pointerdown could land at the original touch coordinates right after the
  // menu opened (either from a topology-change re-dispatch under the active
  // touch, or from compat-mouse promotion). The dismiss listener must treat
  // the trigger as "inside" the menu's root and ignore it.
  await page.goto(`${BASE_URL}/component/?name=context_menu&`, { timeout: 20 * 60 * 1000 });
  const trigger = page.getByRole('button', { name: 'right click here' });
  const contextMenu = page.getByRole('menu');

  await trigger.click({ button: 'right' });
  await expect(contextMenu).toHaveAttribute('data-state', 'open');

  await trigger.evaluate((el) => {
    const triggerRect = el.getBoundingClientRect();
    if (triggerRect.width === 0 || triggerRect.height === 0) {
      throw new Error('trigger has no bounding box');
    }

    const x = triggerRect.left + triggerRect.width / 2;
    const y = triggerRect.top + triggerRect.height / 2;

    if (
      x < triggerRect.left ||
      x > triggerRect.right ||
      y < triggerRect.top ||
      y > triggerRect.bottom
    ) {
      throw new Error('point is outside trigger bounds');
    }

    const root = el.parentElement;
    if (!root) throw new Error('trigger has no root');
    const rootRect = root.getBoundingClientRect();
    if (x < rootRect.left || x > rootRect.right || y < rootRect.top || y > rootRect.bottom) {
      throw new Error('point is outside context menu root bounds');
    }

    el.dispatchEvent(new PointerEvent('pointerdown', {
      pointerId: 6060,
      pointerType: 'touch',
      isPrimary: true,
      clientX: x,
      clientY: y,
      button: 0,
      buttons: 1,
      bubbles: true,
      cancelable: true,
    }));
  });

  await expect(contextMenu).toHaveAttribute('data-state', 'open');
});

test('touch released before long-press threshold does not open the menu', async ({ page }) => {
  await page.goto(`${BASE_URL}/component/?name=context_menu&`, { timeout: 20 * 60 * 1000 });
  const trigger = page.getByRole('button', { name: 'right click here' });

  const box = await trigger.boundingBox();
  if (!box) throw new Error('trigger has no bounding box');
  const x = box.x + box.width / 2;
  const y = box.y + box.height / 2;
  const pointerId = 4343;

  await trigger.evaluate((el, { x, y, pointerId }) => {
    el.dispatchEvent(new PointerEvent('pointerdown', {
      pointerId,
      pointerType: 'touch',
      isPrimary: true,
      clientX: x,
      clientY: y,
      button: 0,
      buttons: 1,
      bubbles: true,
      cancelable: true,
    }));
  }, { x, y, pointerId });

  // Quick tap — release well before the long-press threshold.
  await page.waitForTimeout(50);
  await trigger.evaluate((el, { x, y, pointerId }) => {
    el.dispatchEvent(new PointerEvent('pointerup', {
      pointerId,
      pointerType: 'touch',
      isPrimary: true,
      clientX: x,
      clientY: y,
      bubbles: true,
    }));
  }, { x, y, pointerId });

  // Wait past the long-press threshold; menu must remain closed.
  await page.waitForTimeout(700);
  await expect(page.getByRole('menu')).toHaveCount(0);
});

test('keyboard navigation', async ({ page }) => {
  await page.goto(`${BASE_URL}/component/?name=context_menu&`, { timeout: 20 * 60 * 1000 }); // Increase timeout to 20 minutes
  await page.getByRole('button', { name: 'right click here' }).click({
    button: 'right'
  });

  // Assert the context menu is visible
  const contextMenu = page.getByRole('menu');
  await expect(contextMenu).toHaveAttribute('data-state', 'open');
  // Hit escape to close the context menu
  await page.keyboard.press('Escape');
  // Assert the context menu is closed after pressing escape
  await expect(contextMenu).toHaveCount(0);

  // Reopen the context menu
  await page.getByRole('button', { name: 'right click here' }).click({
    button: 'right'
  });
  await page.keyboard.press('ArrowDown');
  // Assert the "Edit" menu item is focused
  await expect(page.getByRole('menuitem', { name: 'Edit' })).toBeFocused();
  await expect(page.getByRole('menuitem', { name: 'Undo' })).toHaveAttribute('data-disabled', 'true');
  // Move down to the "Duplicate" menu item
  await page.keyboard.press('ArrowDown');
  // Assert the "Duplicate" menu item is focused
  await expect(page.getByRole('menuitem', { name: 'Duplicate' })).toBeFocused();
  // Hit Enter to select the "Duplicate" menu item
  await page.keyboard.press('Enter');
  // Assert the context menu is closed after selection
  await expect(contextMenu).toHaveCount(0);
  // Assert the selected item is displayed
  await expect(page.getByText('Selected: Duplicate')).toBeVisible();
});

// --- Checkable items: ContextMenuCheckboxItem / ContextMenuRadioGroup / ContextMenuRadioItem ---
//
// shadcn's ContextMenu has CheckboxItem and RadioGroup + RadioItem; this one
// had neither. They are one construction shared with DropdownMenu and
// Menubar (`primitives/src/menu_item.rs`), graded here for ContextMenu's own
// wiring: role/aria contract, pointer (pointerdown-then-pointerup, like every
// ContextMenu item) and keyboard behaviour, close semantics, roving focus and
// typeahead. The shared role contract is also graded across all three hosts
// by `oracle/tier1-apg/menu-roles.spec.ts`; the label/indicator geometry by
// `menu-indicator-gap.spec.ts`.
//
// The `checkboxes` and `radio_group` variants keep the menu open after a
// toggle (`close_on_select: false`) -- the APG-optional behaviour for Space
// ("changes the state without closing the menu", menu-and-menubar-pattern.html,
// "Keyboard Interaction"). `main`'s "Show Bookmarks"/"Pedro Duarte" items use
// the primitive's default, Radix's `onSelect` default: selecting closes the menu.
const block = (variant: string) => `${BASE_URL}/component/block/?name=context_menu&variant=${variant}&`;

test.describe('Checkable items', () => {
  test('checkbox items: menuitemcheckbox with an always-present aria-checked; a click toggles, a disabled item does not, the menu stays open', async ({ page }) => {
    await gotoHydrated(page, block('checkboxes'));
    await page.getByRole('button', { name: 'Right click for checkboxes' }).click({ button: 'right' });
    const menu = page.getByRole('menu');
    await expect(menu).toHaveAttribute('data-state', 'open');

    await expect(page.getByRole('menuitemcheckbox'), 'three checkbox items').toHaveCount(3);
    await expect(page.getByRole('menuitem'), 'only the "More options" sub-trigger is a plain menuitem').toHaveCount(1);
    const bookmarks = page.getByRole('menuitemcheckbox', { name: 'Show Bookmarks Bar' });
    const fullUrls = page.getByRole('menuitemcheckbox', { name: 'Show Full URLs' });
    const devTools = page.getByRole('menuitemcheckbox', { name: 'Show Developer Tools' });

    await expect(bookmarks).toHaveAttribute('aria-checked', 'true');
    await expect(bookmarks).toHaveAttribute('data-state', 'checked');
    await expect(fullUrls).toHaveAttribute('aria-checked', 'false');
    await expect(fullUrls).toHaveAttribute('data-state', 'unchecked');
    await expect(devTools).toHaveAttribute('aria-checked', 'true');
    await expect(devTools).toHaveAttribute('aria-disabled', 'true');

    await fullUrls.click();
    await expect(fullUrls).toHaveAttribute('aria-checked', 'true');
    await expect(page.getByText('Bookmarks true, full URLs true')).toBeVisible();
    await expect(menu, 'close_on_select: false keeps the menu open').toHaveAttribute('data-state', 'open');

    await bookmarks.click();
    await expect(bookmarks).toHaveAttribute('aria-checked', 'false');

    // `force`: Playwright otherwise waits forever for an `aria-disabled`
    // element to become actionable.
    await devTools.click({ force: true });
    await expect(devTools, 'a disabled item cannot be toggled').toHaveAttribute('aria-checked', 'true');
    await expect(menu).toHaveAttribute('data-state', 'open');
  });

  test('checkbox items: arrow keys skip the disabled item, Space and Enter toggle in place, typeahead cycles the enabled ones', async ({ page }) => {
    await gotoHydrated(page, block('checkboxes'));
    await page.getByRole('button', { name: 'Right click for checkboxes' }).click({ button: 'right' });
    const menu = page.getByRole('menu');
    const bookmarks = page.getByRole('menuitemcheckbox', { name: 'Show Bookmarks Bar' });
    const fullUrls = page.getByRole('menuitemcheckbox', { name: 'Show Full URLs' });

    await page.keyboard.press('ArrowDown');
    await expect(bookmarks).toBeFocused();
    await page.keyboard.press('ArrowDown');
    await expect(fullUrls).toBeFocused();
    // Show Full URLs(1) -> Show Developer Tools(2, disabled, skipped) -> More
    // options(3, a sub-trigger) -> wraps to Bookmarks(0).
    await page.keyboard.press('ArrowDown');
    await expect(page.getByRole('menuitem', { name: 'More options' })).toBeFocused();
    await page.keyboard.press('ArrowDown');
    await expect(bookmarks).toBeFocused();
    await page.keyboard.press('ArrowDown');
    await expect(fullUrls).toBeFocused();

    // APG (Optional): Space on a menuitemcheckbox "changes the state without
    // closing the menu" -- what close_on_select: false gives.
    await page.keyboard.press('Space');
    await expect(fullUrls).toHaveAttribute('aria-checked', 'true');
    await expect(menu).toHaveAttribute('data-state', 'open');
    await expect(fullUrls, 'focus stays on the toggled item').toBeFocused();
    await page.keyboard.press('Enter');
    await expect(fullUrls).toHaveAttribute('aria-checked', 'false');
    await expect(menu).toHaveAttribute('data-state', 'open');

    // Typeahead: every label starts with "S", so repeating "s" cycles the
    // checkbox items -- skipping the disabled one.
    await page.keyboard.press('s');
    await expect(bookmarks).toBeFocused();
    await page.keyboard.press('s');
    await expect(fullUrls, "typeahead 's' must skip the disabled Show Developer Tools").toBeFocused();
  });

  test('a checkbox item inside a submenu registers in the submenu, not the root: keyboard opens it, Space toggles it in place, the root keeps its own roving focus', async ({ page }) => {
    await gotoHydrated(page, block('checkboxes'));
    await page.getByRole('button', { name: 'Right click for checkboxes' }).click({ button: 'right' });
    const menu = page.getByRole('menu');
    const bookmarks = page.getByRole('menuitemcheckbox', { name: 'Show Bookmarks Bar' });
    const subTrigger = page.getByRole('menuitem', { name: 'More options' });
    const wordWrap = page.getByRole('menuitemcheckbox', { name: 'Word Wrap' });

    // Bookmarks(0) -> Full URLs(1) -> Developer Tools(2, disabled, skipped) -> More options(3).
    for (let i = 0; i < 3; i++) await page.keyboard.press('ArrowDown');
    await expect(subTrigger).toBeFocused();
    await page.keyboard.press('ArrowRight');
    // The submenu's first item is `Word Wrap` (its own index 0). Had it
    // registered in the root collection it would have collided with `Show
    // Bookmarks Bar` (also index 0) and focus would not have landed on it.
    await expect(wordWrap).toBeVisible();
    await expect(wordWrap).toBeFocused();

    await page.keyboard.press('Space');
    await expect(wordWrap).toHaveAttribute('aria-checked', 'true');
    await expect(page.getByText('word wrap true')).toBeVisible();
    await expect(menu.first(), 'close_on_select: false keeps the whole menu open').toHaveAttribute('data-state', 'open');
    await expect(wordWrap, 'focus stays on the toggled item').toBeFocused();

    // ArrowLeft hands focus back to the sub-trigger; the root's own roving
    // focus is intact (ArrowDown from the last item wraps to Bookmarks).
    await page.keyboard.press('ArrowLeft');
    await expect(subTrigger).toBeFocused();
    await page.keyboard.press('ArrowDown');
    await expect(bookmarks).toBeFocused();
  });

  test('a checkbox item inside a submenu: a click toggles it, and by default it closes the whole menu tree', async ({ page }) => {
    await gotoHydrated(page, block('checkboxes'));
    await page.getByRole('button', { name: 'Right click for checkboxes' }).click({ button: 'right' });
    await page.getByRole('menuitem', { name: 'More options' }).click();
    const wordWrap = page.getByRole('menuitemcheckbox', { name: 'Word Wrap' });
    const minimap = page.getByRole('menuitemcheckbox', { name: 'Minimap' });
    await expect(wordWrap).toBeVisible();

    await wordWrap.click();
    await expect(wordWrap).toHaveAttribute('aria-checked', 'true');
    await expect(minimap, 'the submenu stays open').toBeVisible();

    // `Minimap` uses the primitive's default: choosing it closes the menu --
    // the entire tree, submenu included, like `ContextMenuSubItem`.
    await minimap.click();
    await expect(page.getByRole('menu'), 'the default close takes the whole tree with it').toHaveCount(0);
    await expect(page.getByText('minimap true')).toBeVisible();
  });

  test('radio groups: role=group named by its label, one checked item per group, a choice moves only its own group\'s check', async ({ page }) => {
    await gotoHydrated(page, block('radio_group'));
    await page.getByRole('button', { name: 'Right click for radio group' }).click({ button: 'right' });
    await expect(page.getByRole('menu')).toHaveAttribute('data-state', 'open');

    const people = page.getByRole('group', { name: 'People' });
    const theme = page.getByRole('group', { name: 'Theme' });
    await expect(people).toBeVisible();
    await expect(theme).toBeVisible();
    await expect(people.getByRole('menuitemradio')).toHaveCount(2);
    await expect(theme.getByRole('menuitemradio')).toHaveCount(3);
    const checked = (group: typeof people) => group.locator('[role="menuitemradio"][aria-checked="true"]');
    await expect(checked(people)).toHaveCount(1);
    await expect(checked(theme)).toHaveCount(1);
    await expect(people.getByRole('menuitemradio', { name: 'Pedro Duarte' })).toHaveAttribute('aria-checked', 'true');
    await expect(theme.getByRole('menuitemradio', { name: 'Light' })).toHaveAttribute('aria-checked', 'true');

    await people.getByRole('menuitemradio', { name: 'Colm Tuite' }).click();
    await expect(people.getByRole('menuitemradio', { name: 'Colm Tuite' })).toHaveAttribute('aria-checked', 'true');
    await expect(people.getByRole('menuitemradio', { name: 'Pedro Duarte' })).toHaveAttribute('aria-checked', 'false');
    await expect(checked(people)).toHaveCount(1);
    await expect(theme.getByRole('menuitemradio', { name: 'Light' }), "the other group is untouched").toHaveAttribute('aria-checked', 'true');
    await expect(page.getByRole('menu'), 'close_on_select: false keeps the menu open').toHaveAttribute('data-state', 'open');
  });

  test('radio groups: Space and Enter choose in place across both groups, typeahead falls back to the value', async ({ page }) => {
    await gotoHydrated(page, block('radio_group'));
    await page.getByRole('button', { name: 'Right click for radio group' }).click({ button: 'right' });
    const menu = page.getByRole('menu');
    const theme = page.getByRole('group', { name: 'Theme' });
    const dark = theme.getByRole('menuitemradio', { name: 'Dark' });
    const system = theme.getByRole('menuitemradio', { name: 'System' });

    // Pedro(0) Colm(1) | Light(2) Dark(3) System(4): one roving collection
    // across both groups.
    for (let i = 0; i < 4; i++) await page.keyboard.press('ArrowDown');
    await expect(dark).toBeFocused();
    await page.keyboard.press('Space');
    await expect(dark).toHaveAttribute('aria-checked', 'true');
    await expect(theme.getByRole('menuitemradio', { name: 'Light' })).toHaveAttribute('aria-checked', 'false');
    await expect(menu).toHaveAttribute('data-state', 'open');

    await page.keyboard.press('ArrowDown');
    await expect(system).toBeFocused();
    await page.keyboard.press('Enter');
    await expect(system).toHaveAttribute('aria-checked', 'true');
    await expect(dark).toHaveAttribute('aria-checked', 'false');
    await expect(menu).toHaveAttribute('data-state', 'open');

    // A radio item has no text_value here: typeahead falls back to its value.
    await page.keyboard.press('d');
    await expect(dark, "typeahead 'd' reaches Dark via its value").toBeFocused();
  });

  test('by default (Radix onSelect default) choosing a checkable item closes the menu and the new state is kept', async ({ page }) => {
    await gotoHydrated(page, block('main'));
    const trigger = page.getByRole('button', { name: 'right click here' });
    await trigger.click({ button: 'right' });
    const menu = page.getByRole('menu');
    await expect(menu).toHaveAttribute('data-state', 'open');

    const bookmarks = page.getByRole('menuitemcheckbox', { name: 'Show Bookmarks' });
    await expect(bookmarks).toHaveAttribute('aria-checked', 'true');
    await bookmarks.click();
    await expect(menu, 'a checkbox item closes the menu by default').toHaveCount(0);

    await trigger.click({ button: 'right' });
    await expect(bookmarks, 'the (controlled) state survived the close').toHaveAttribute('aria-checked', 'false');

    // Keyboard: Enter on a radio item chooses it and closes the menu. End
    // lands on the last item, "Colm Tuite".
    await page.keyboard.press('End');
    const colm = page.getByRole('menuitemradio', { name: 'Colm Tuite' });
    await expect(colm).toBeFocused();
    await page.keyboard.press('Enter');
    await expect(menu).toHaveCount(0);

    await trigger.click({ button: 'right' });
    await expect(colm).toHaveAttribute('aria-checked', 'true');
    await expect(page.getByRole('menuitemradio', { name: 'Pedro Duarte' })).toHaveAttribute('aria-checked', 'false');
  });
});

test.describe('Axe automated scan', () => {
  test('loaded (menu closed) has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=context_menu&`, { timeout: 20 * 60 * 1000 });
    // Wait for render before scanning -- see input.spec.ts's identical
    // comment for why (avoids a false pre-hydration "no main"/"no h1").
    await expect(page.getByRole('button', { name: 'right click here' })).toBeVisible();
    await expectNoAxeViolations(page, 'context-menu: loaded', { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });

  // docs/backlog.md row 25: ContextMenu's role="menu" popup carries no
  // aria-labelledby/aria-label at all, so an open menu has no accessible
  // name (APG menu-and-menubar pattern requires one) -- this is exactly the
  // class of defect this round's axe coverage is meant to surface.
  test('menu open has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=context_menu&`, { timeout: 20 * 60 * 1000 });
    await page.getByRole('button', { name: 'right click here' }).click({ button: 'right' });
    await expect(page.getByRole('menu')).toBeVisible();
    await expectNoAxeViolations(page, 'context-menu: menu open', { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });

  // Checkable items: role="menuitemcheckbox"/"menuitemradio" need aria-checked
  // and a menu/group context; a radio group needs to be a real `group`.
  test('menu with checkbox items open has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=context_menu&`, { timeout: 20 * 60 * 1000 });
    await page.getByRole('button', { name: 'Right click for checkboxes' }).click({ button: 'right' });
    await expect(page.getByRole('menuitemcheckbox', { name: 'Show Bookmarks Bar' })).toBeVisible();
    await expectNoAxeViolations(page, 'context-menu: checkboxes open', { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });

  test('menu with radio groups open has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=context_menu&`, { timeout: 20 * 60 * 1000 });
    await page.getByRole('button', { name: 'Right click for radio group' }).click({ button: 'right' });
    await expect(page.getByRole('menuitemradio', { name: 'Pedro Duarte' })).toBeVisible();
    await expectNoAxeViolations(page, 'context-menu: radio groups open', { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});

// ---------------------------------------------------------------------------
// `ContextMenuSub { open_on_hover: false }` -- click-activated submenu
// (preview/src/components/context_menu/variants/click_only_submenu/mod.rs;
// renders alongside `main` on this page under its own labels). The default
// hover-open / hover-leave-close contract is pinned, unchanged, by
// `oracle/tier1-apg/menu-submenu.spec.ts`'s "hover:" tests on `main`.
// ---------------------------------------------------------------------------
test.describe('ContextMenuSub open_on_hover: false (click activation)', () => {
  const HOVER_SETTLE_MS = 1000; // >> SUBMENU_OPEN_INTENT_DELAY / SUBMENU_CLOSE_GRACE_DELAY (200ms each)

  async function openMenu(page: import('@playwright/test').Page) {
    await page.goto(`${BASE_URL}/component/?name=context_menu&`, { timeout: 20 * 60 * 1000 });
    await page.getByRole('button', { name: 'secondary-click this zone' }).click({ button: 'right' });
    const menu = page.getByRole('menu').filter({ has: page.getByRole('menuitem', { name: 'Share' }) });
    await expect(menu).toBeVisible();
    return {
      sub: page.getByRole('menuitem', { name: 'Export as' }),
      submenu: page.getByRole('menu', { name: 'Export as' }),
    };
  }

  test('hovering the sub-trigger for a second does not open it; a click does', async ({ page }) => {
    const { sub, submenu } = await openMenu(page);
    await expect(sub).toHaveAttribute('aria-expanded', 'false');

    await sub.hover();
    await page.waitForTimeout(HOVER_SETTLE_MS);
    await expect(sub).toHaveAttribute('aria-expanded', 'false');
    await expect(submenu).toHaveCount(0);

    await sub.click();
    await expect(sub).toHaveAttribute('aria-expanded', 'true');
    await expect(submenu).toBeVisible();
    await expect(page.getByRole('menuitem', { name: 'PDF document' })).toBeFocused();
  });

  test('the pointer leaving a click-opened submenu does not close it', async ({ page }) => {
    const { sub, submenu } = await openMenu(page);
    await sub.click();
    await expect(submenu).toBeVisible();

    await page.getByRole('menuitem', { name: 'Share' }).hover();
    await page.waitForTimeout(HOVER_SETTLE_MS);
    await expect(sub).toHaveAttribute('aria-expanded', 'true');
    await expect(submenu).toBeVisible();

    await page.getByRole('menuitem', { name: 'Plain text' }).hover();
    await page.getByRole('menuitem', { name: 'Print' }).hover();
    await page.waitForTimeout(HOVER_SETTLE_MS);
    await expect(sub).toHaveAttribute('aria-expanded', 'true');
  });

  test('keyboard is unchanged: ArrowRight / Enter open, ArrowLeft / Escape close', async ({ page }) => {
    const { sub } = await openMenu(page);
    await page.keyboard.press('ArrowDown'); // Share
    await page.keyboard.press('ArrowDown'); // Export as
    await expect(sub).toBeFocused();

    await page.keyboard.press('ArrowRight');
    await expect(sub).toHaveAttribute('aria-expanded', 'true');
    await expect(page.getByRole('menuitem', { name: 'PDF document' })).toBeFocused();

    await page.keyboard.press('ArrowLeft');
    await expect(sub).toHaveAttribute('aria-expanded', 'false');
    await expect(sub).toBeFocused();

    await page.keyboard.press('Enter');
    await expect(page.getByRole('menuitem', { name: 'PDF document' })).toBeFocused();
    await page.keyboard.press('Escape');
    await expect(sub).toHaveAttribute('aria-expanded', 'false');
    await expect(sub).toBeFocused();
  });

  test('selecting a sub-item still closes the whole menu', async ({ page }) => {
    const { sub } = await openMenu(page);
    await sub.click();
    await page.getByRole('menuitem', { name: 'Plain text' }).click();
    await expect(page.getByRole('menuitem', { name: 'Share' })).toHaveCount(0);
    await expect(page.getByText('Chosen: text')).toBeVisible();
  });

  test('default (open_on_hover: true) is unchanged: hovering More tools still opens it', async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=context_menu&`, { timeout: 20 * 60 * 1000 });
    await page.getByRole('button', { name: 'right click here' }).click({ button: 'right' });
    const sub = page.getByRole('menuitem', { name: 'More tools' });
    await sub.hover();
    await expect(sub).toHaveAttribute('aria-expanded', 'true');
  });
});

// ---------------------------------------------------------------------------
// Open/close fade vs. the anchor's centring transform (2026-10-05). An anchored overlay centres itself
// with `transform: translateX(-50%)` (top_layer.rs's engine stylesheet); a `@keyframes` that sets
// `transform` REPLACES it for as long as it runs (the date picker played its fade 144px off-centre, the
// colour picker 133px). `ContextMenuContent` sets no `data-side`, so it has no centring to lose today -- the same
// shape, latent -- so this probes the rendered keyframes instead of sampling a position that could not
// move anyway: `assert-anchor-transform.ts` supplies an inline `transform`, seeks the element's own
// CSS animation to its start/midpoint/end, and asserts the transform survives while `scale`/`translate`
// do the moving. Source-level guard: scripts/check-anchored-keyframes.sh.
// ---------------------------------------------------------------------------
test.describe('Open/close animation', () => {
  test('the fade animates translate/scale and leaves transform alone (open and close)', async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=context_menu&`, { timeout: 20 * 60 * 1000 });
    await page.getByRole('button', { name: 'right click here' }).click({ button: 'right' });
    const content = page.locator('.dx-context-menu-content[data-state="open"]').first();
    await expect(content).toBeVisible();
    await expectFadeKeepsTransform(content, 'context menu content, open');
    await expectFadeKeepsTransform(content, 'context menu content, close', { 'data-state': 'closed' });
  });
});
