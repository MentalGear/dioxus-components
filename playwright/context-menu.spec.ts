import { test, expect } from '@playwright/test';
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from './axe';

test('pointer navigation', async ({ page }) => {
  await page.goto('http://127.0.0.1:8080/component/?name=context_menu&', { timeout: 20 * 60 * 1000 }); // Increase timeout to 20 minutes
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
  await page.goto('http://127.0.0.1:8080/component/?name=context_menu&', { timeout: 20 * 60 * 1000 });
  // Push the trigger down so the tap point isn't at viewport (0, 0) — any
  // misalignment will then have a non-zero direction to detect.
  await page.evaluate(() => {
    const main = document.querySelector('main') ?? document.body;
    (main as HTMLElement).style.paddingTop = '300px';
    (main as HTMLElement).style.paddingLeft = '120px';
  });

  const trigger = page.getByRole('button', { name: 'right click here' });
  const contextMenu = page.getByRole('menu');
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
  await page.goto('http://127.0.0.1:8080/component/?name=context_menu&', { timeout: 20 * 60 * 1000 });
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
  await page.goto('http://127.0.0.1:8080/component/?name=context_menu&', { timeout: 20 * 60 * 1000 });
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
  await page.goto('http://127.0.0.1:8080/component/?name=context_menu&', { timeout: 20 * 60 * 1000 });
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
  await page.goto('http://127.0.0.1:8080/component/?name=context_menu&', { timeout: 20 * 60 * 1000 });
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
  await page.goto('http://127.0.0.1:8080/component/?name=context_menu&', { timeout: 20 * 60 * 1000 });
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
  await page.goto('http://127.0.0.1:8080/component/?name=context_menu&', { timeout: 20 * 60 * 1000 });
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
  await page.goto('http://127.0.0.1:8080/component/?name=context_menu&', { timeout: 20 * 60 * 1000 });
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
  await page.goto('http://127.0.0.1:8080/component/?name=context_menu&', { timeout: 20 * 60 * 1000 });
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
  await page.goto('http://127.0.0.1:8080/component/?name=context_menu&', { timeout: 20 * 60 * 1000 }); // Increase timeout to 20 minutes
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

test.describe('Axe automated scan', () => {
  test('loaded (menu closed) has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto('http://127.0.0.1:8080/component/?name=context_menu&', { timeout: 20 * 60 * 1000 });
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
    await page.goto('http://127.0.0.1:8080/component/?name=context_menu&', { timeout: 20 * 60 * 1000 });
    await page.getByRole('button', { name: 'right click here' }).click({ button: 'right' });
    await expect(page.getByRole('menu')).toBeVisible();
    await expectNoAxeViolations(page, 'context-menu: menu open', { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});
