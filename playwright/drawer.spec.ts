/**
 * Rule sources (docs/conformance-harness.md tiers):
 *
 * - WHATWG `<dialog>` modal semantics (focus trap, `showModal()`/`close()`
 *   focus restore, inertness, top layer) are inherited unchanged from
 *   `primitives/src/dialog.rs` and already covered by
 *   `oracle/tier2-html/native-dialog.spec.ts` against the unmodified
 *   `Dialog` primitive `Drawer` composes -- not re-covered here.
 * - APG "Dialog (Modal)" Pattern (tier 2): Escape closes the dialog and
 *   focus returns to the element that invoked it. Exercised here because
 *   it's the *drawer's own* integration (the trigger button, the specific
 *   first-focusable content) that needs checking, not the primitive's own
 *   escape-key/focus-restore mechanics again.
 * - shadcn/ui Drawer (Vaul)'s drag-to-dismiss gesture -- distance ratio
 *   and release-velocity thresholds, rubber-band overdrag -- is tier 3
 *   "opinion": there is no APG/WHATWG rule for a pointer drag gesture,
 *   only this library's (and Vaul's) own convention. See
 *   `primitives/src/drawer.rs`'s `DISMISS_DRAG_RATIO`/
 *   `DISMISS_VELOCITY_PX_PER_MS` doc comments for the exact values these
 *   tests assume.
 */
import { test, expect } from '@playwright/test';
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from './axe';

const URL = 'http://127.0.0.1:8080/component/?name=drawer&';
const GOTO_OPTS = { timeout: 20 * 60 * 1000 };

test('opens and traps focus, Escape closes and restores focus to the trigger', async ({ page }) => {
  await page.goto(URL, GOTO_OPTS);
  const trigger = page.getByRole('button', { name: 'Move Goal' });
  await trigger.click();

  const root = page.locator('[data-slot="drawer-root"]');
  await expect(root).toHaveAttribute('data-state', 'open');

  // First focusable element inside the content in tree order -- WHATWG
  // `showModal()`'s own autofocus algorithm. `DrawerHandle` is
  // `aria-hidden` and not focusable, so the "Decrease goal" icon button
  // (the drawer's own first real control) is the expected stop.
  await expect(page.getByRole('button', { name: 'Decrease goal' })).toBeFocused();

  // APG Dialog (Modal): Escape closes and returns focus to the invoker.
  await page.keyboard.press('Escape');
  await expect(root).toHaveCount(0);
  await expect(trigger).toBeFocused();
});

test('data-state transitions from closed to open to closed', async ({ page }) => {
  await page.goto(URL, GOTO_OPTS);
  const root = page.locator('[data-slot="drawer-root"]');
  const content = page.locator('[data-slot="drawer-content"]');

  await expect(root).toHaveCount(0);

  await page.getByRole('button', { name: 'Move Goal' }).click();
  await expect(root).toHaveAttribute('data-state', 'open');
  await expect(content).toHaveAttribute('data-state', 'open');
  await expect(content).toHaveAttribute('data-side', 'bottom');

  await page.keyboard.press('Escape');
  await expect(root).toHaveCount(0);
});

test('opens from the top side with side-specific data attributes', async ({ page }) => {
  await page.goto(URL, GOTO_OPTS);
  await page.getByRole('button', { name: 'Open from Top' }).click();

  const content = page.locator('[data-slot="drawer-content"]');
  await expect(content).toHaveAttribute('data-side', 'top');
  await expect(content).toHaveAttribute('data-state', 'open');

  await page.keyboard.press('Escape');
  await expect(page.locator('[data-slot="drawer-root"]')).toHaveCount(0);
});

test('a long, slow drag past the dismiss threshold closes the drawer', async ({ page }) => {
  await page.goto(URL, GOTO_OPTS);
  await page.getByRole('button', { name: 'Move Goal' }).click();

  const root = page.locator('[data-slot="drawer-root"]');
  const content = page.locator('[data-slot="drawer-content"]');
  await expect(root).toHaveAttribute('data-state', 'open');

  const contentBox = await content.boundingBox();
  const handleBox = await page.locator('[data-slot="drawer-handle"]').boundingBox();
  if (!contentBox || !handleBox) throw new Error('drawer content/handle has no bounding box');

  const startX = handleBox.x + handleBox.width / 2;
  const startY = handleBox.y + handleBox.height / 2;
  // Bottom drawer: dragging DOWN (toward its own edge) dismisses.
  // Comfortably past DISMISS_DRAG_RATIO (25% of the panel's own height).
  const dragDistance = contentBox.height * 0.4;

  await page.mouse.move(startX, startY);
  await page.mouse.down();
  // Several separately-timed intermediate moves with a real pause between
  // each -- this drag must exercise the *distance* release path, not the
  // *velocity* one (shadcn/ui Drawer (Vaul) closes on either), so the
  // per-step speed has to stay well under DISMISS_VELOCITY_PX_PER_MS
  // (0.5px/ms) throughout, including the last step before release.
  const steps = 6;
  for (let i = 1; i <= steps; i++) {
    await page.mouse.move(startX, startY + (dragDistance * i) / steps, { steps: 2 });
    await page.waitForTimeout(120);
  }
  // A real pause with no further movement right before release: no new
  // pointermove sample arrives, so the release velocity is whatever the
  // slow stepped movement above already made it -- not inflated by a
  // fast final flick.
  await page.waitForTimeout(200);
  await page.mouse.up();

  await expect(content).toHaveCount(0);
  await expect(root).toHaveCount(0);
});

test('a short, slow drag snaps back without closing the drawer', async ({ page }) => {
  await page.goto(URL, GOTO_OPTS);
  await page.getByRole('button', { name: 'Move Goal' }).click();

  const root = page.locator('[data-slot="drawer-root"]');
  const content = page.locator('[data-slot="drawer-content"]');
  await expect(root).toHaveAttribute('data-state', 'open');

  const handleBox = await page.locator('[data-slot="drawer-handle"]').boundingBox();
  if (!handleBox) throw new Error('drawer handle has no bounding box');
  const startX = handleBox.x + handleBox.width / 2;
  const startY = handleBox.y + handleBox.height / 2;

  // Short (well under the 25% distance threshold for any reasonably-sized
  // panel) and slow (the pause before release keeps velocity low too) --
  // must snap back, not close.
  await page.mouse.move(startX, startY);
  await page.mouse.down();
  await page.mouse.move(startX, startY + 15, { steps: 2 });
  await page.waitForTimeout(150);
  await page.mouse.up();

  await expect(root).toHaveAttribute('data-state', 'open');
  await expect(content).toHaveCount(1);
  // The live drag offset is carried by an inline `translate` (see
  // primitives/src/drawer.rs's module doc for why not `transform`) --
  // snapping back resets it to exactly 0.
  await expect.poll(async () => content.getAttribute('style')).toContain('translate: 0 0px');
});

test.describe('Axe automated scan', () => {
  test('loaded (drawer closed) has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto(URL, GOTO_OPTS);
    // Wait for render before scanning -- see input.spec.ts's identical
    // comment for why (avoids a false pre-hydration "no main"/"no h1").
    await expect(page.getByRole('button', { name: 'Move Goal' })).toBeVisible();
    await expectNoAxeViolations(page, 'drawer: loaded', { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });

  test('open has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto(URL, GOTO_OPTS);
    await page.getByRole('button', { name: 'Move Goal' }).click();
    await expect(page.locator('[data-slot="drawer-root"]')).toHaveAttribute('data-state', 'open');
    // Named via DrawerTitle so axe's `aria-dialog-name` rule passes -- see
    // primitives/src/drawer.rs's `DrawerTitle` doc for the lesson this
    // repeats from `primitives/src/command.rs`'s `CommandDialog`.
    await expectNoAxeViolations(page, 'drawer: open', { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});
