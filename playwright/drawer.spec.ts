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
import { BASE_URL } from './base-url';

const URL = `${BASE_URL}/component/?name=drawer&`;
const GOTO_OPTS = { timeout: 20 * 60 * 1000 };

// The entrance slide-in keyframe runs over `--dx-motion-duration-slow`
// (200ms, preview/assets/dx-components-theme.css) with `animation-fill-mode:
// forwards`; a geometry assertion taken before it finishes reads a mid-
// transform position, not the resting one (confirmed live: reading
// `getBoundingClientRect()` immediately after the trigger click, with no
// wait, showed the panel still at its pre-animation `translateY(100%)`
// position). 500ms is 2.5x that duration.
const ANIMATION_SETTLE_MS = 500;

// primitives/src/drawer.rs's own rubber-band constants for an overdrag PAST
// the fully-open resting position (dragging away from the dismiss
// direction). The construction that keeps that overdrag from ever exposing
// bare backdrop is two-part -- see preview/src/components/drawer/style.css's
// `[data-side]` rules' own comments -- so these tests check both halves
// rather than only the DOM box's own position:
//   1. the real, damped overdrag the box's own layout position can ever
//      reach is bounded by RUBBER_BAND_MAX_PX (a small, deliberate rubber
//      band -- a protected Rust unit test requires it be non-zero, so this
//      is NOT expected to be exactly 0), and
//   2. the CSS `filter: drop-shadow(...)` extension on that same side is
//      sized at least that large, so whatever sliver (1) allows is always
//      painted over with the panel's own background rather than exposing
//      the backdrop underneath.
const RUBBER_BAND_MAX_PX = 24;
// A little slack over the exact Rust constant for CI/animation-frame jitter
// in the boundingBox() sampling below (this is a bound, not an exact-value
// assertion).
const RUBBER_BAND_ASSERT_SLACK_PX = 6;
// Subpixel layout/rendering tolerance for an at-rest "flush with the edge"
// comparison (mirrors the +/-1 tolerance context-menu.spec.ts's own
// edge-clamping assertions use for the same reason).
const FLUSH_TOLERANCE_PX = 1;

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

test('the panel sits flush against its own edge at rest, bottom and top variants', async ({ page }) => {
  // Regression: the web arm's `DrawerContent` is a real, modal `<dialog>`
  // (primitives/src/dialog.rs), and Chromium's own `dialog`/`dialog:modal`
  // UA rules (`margin: auto` + `inset-block: 0` with no `inset-inline` +
  // `width`/`height: fit-content`) centered it -- both axes -- instead of
  // anchoring it to any edge. Measured live pre-fix: the bottom variant's
  // computed `margin` was `223px 496.594px` (equal top/bottom AND
  // left/right margins) on an 800x1280 viewport, i.e. a floating card, not
  // a bottom sheet.
  await page.goto(URL, GOTO_OPTS);
  const viewport = page.viewportSize();
  if (!viewport) throw new Error('no viewport size');

  await page.getByRole('button', { name: 'Move Goal' }).click();
  await page.waitForTimeout(ANIMATION_SETTLE_MS);
  const bottomBox = await page.locator('[data-slot="drawer-content"]').boundingBox();
  if (!bottomBox) throw new Error('drawer content has no bounding box');
  // Flush with the viewport's bottom edge: no gap below.
  expect(Math.abs(viewport.height - (bottomBox.y + bottomBox.height))).toBeLessThanOrEqual(FLUSH_TOLERANCE_PX);
  await page.keyboard.press('Escape');
  await expect(page.locator('[data-slot="drawer-root"]')).toHaveCount(0);

  await page.getByRole('button', { name: 'Open from Top' }).click();
  await page.waitForTimeout(ANIMATION_SETTLE_MS);
  const topBox = await page.locator('[data-slot="drawer-content"]').boundingBox();
  if (!topBox) throw new Error('drawer content has no bounding box');
  // Flush with the viewport's top edge: no gap above.
  expect(Math.abs(topBox.y)).toBeLessThanOrEqual(FLUSH_TOLERANCE_PX);
});

test('a drag away from the dismiss direction never exposes the viewport edge', async ({ page }) => {
  await page.goto(URL, GOTO_OPTS);
  const viewport = page.viewportSize();
  if (!viewport) throw new Error('no viewport size');

  await page.getByRole('button', { name: 'Move Goal' }).click();
  await page.waitForTimeout(ANIMATION_SETTLE_MS);

  const content = page.locator('[data-slot="drawer-content"]');
  const handleBox = await page.locator('[data-slot="drawer-handle"]').boundingBox();
  if (!handleBox) throw new Error('drawer handle has no bounding box');
  const startX = handleBox.x + handleBox.width / 2;
  const startY = handleBox.y + handleBox.height / 2;

  // Bottom drawer: dragging UP is away from the dismiss direction (the
  // rubber-band overdrag case). `damped_display_offset` maps a raw offset
  // to `(raw * RUBBER_BAND_RATIO).max(-RUBBER_BAND_MAX_PX)` -- with
  // RATIO=0.25, the cap only actually engages past 96px of raw movement
  // (24 / 0.25), so 160px raw comfortably pins the damped display offset
  // at its -24px cap for the later samples, not still mid-ramp.
  const rawOverdragPx = 160;
  await page.mouse.move(startX, startY);
  await page.mouse.down();
  let maxGapBelow = 0;
  for (let i = 1; i <= 6; i++) {
    await page.mouse.move(startX, startY - (rawOverdragPx * i) / 6, { steps: 2 });
    const box = await content.boundingBox();
    if (box) maxGapBelow = Math.max(maxGapBelow, viewport.height - (box.y + box.height));
  }
  await page.mouse.up();

  // Half 1: the real layout box never moves further than the documented,
  // deliberately small rubber-band cap.
  expect(maxGapBelow).toBeLessThanOrEqual(RUBBER_BAND_MAX_PX + RUBBER_BAND_ASSERT_SLACK_PX);

  // Half 2: the CSS extension that paints over exactly that sliver is
  // present and sized at least as large as the cap it must cover -- parsed
  // from the computed `filter`, not re-typed as a second magic number, so
  // this fails if the two ever drift apart. Chromium reports each
  // `drop-shadow(...)`'s offsets/color already resolved, e.g.
  // `drop-shadow(rgb(10, 10, 10) 0px 40px 0px)`.
  const filterValue = await content.evaluate((el) => getComputedStyle(el).filter);
  const offsets = [...filterValue.matchAll(/(-?[\d.]+)px/g)].map((m) => parseFloat(m[1]));
  expect(offsets.length).toBeGreaterThan(0);
  const maxAbsOffset = Math.max(...offsets.map(Math.abs));
  expect(maxAbsOffset).toBeGreaterThanOrEqual(RUBBER_BAND_MAX_PX);
});

test('dragging over the handle or the drawer content does not select text', async ({ page }) => {
  await page.goto(URL, GOTO_OPTS);
  await page.getByRole('button', { name: 'Move Goal' }).click();
  await page.waitForTimeout(ANIMATION_SETTLE_MS);

  // Drag #1: directly over DrawerHandle itself. The handle has no text of
  // its own, so this alone can't prove much (there's nothing there TO
  // select) -- included because it's the literal case the bug report
  // named, not because it's the strongest reproduction.
  const handleBox = await page.locator('[data-slot="drawer-handle"]').boundingBox();
  if (!handleBox) throw new Error('drawer handle has no bounding box');
  await page.mouse.move(handleBox.x + 2, handleBox.y + handleBox.height / 2);
  await page.mouse.down();
  await page.mouse.move(handleBox.x + handleBox.width - 2, handleBox.y + handleBox.height / 2, { steps: 5 });
  await page.mouse.up();
  const handleSelection = await page.evaluate(() => window.getSelection()?.toString() ?? '');
  expect(handleSelection).toBe('');

  // Drag #2: the actual reproduction. A drag gesture starts from a
  // `pointerdown` anywhere on the content not gated out
  // (primitives/src/drawer.rs's `use_drawer_drag_start_gate`), which
  // includes ordinary text -- sweep across the description paragraph the
  // same way a user's finger/mouse would while trying to grab the panel by
  // its content rather than precisely on the handle. Confirmed live
  // pre-fix: this exact sweep made `getSelection().toString()` return the
  // full sentence.
  await page.evaluate(() => window.getSelection()?.removeAllRanges());
  const desc = page.locator('[data-slot="drawer-description"]');
  const descBox = await desc.boundingBox();
  if (!descBox) throw new Error('drawer description has no bounding box');
  const midY = descBox.y + descBox.height / 2;

  await page.mouse.move(descBox.x + 2, midY);
  await page.mouse.down();
  for (let i = 1; i <= 8; i++) {
    await page.mouse.move(descBox.x + 2 + ((descBox.width - 4) * i) / 8, midY, { steps: 2 });
  }
  await page.mouse.up();

  const contentSelection = await page.evaluate(() => window.getSelection()?.toString() ?? '');
  expect(contentSelection).toBe('');
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
  // panel) and slow -- must snap back, not close.
  //
  // The velocity half of that needs the same care the long-drag test
  // above already takes, not just a pause *after* the move: drawer.rs's
  // `use_drawer_drag_move`'s velocity is `axis_delta / elapsed_ms` between
  // the last two pointermove samples ONLY, floored at 1ms
  // (`elapsed_ms.max(1.0)`, guarding the real division only) -- a pause
  // *after* the drag's one and only `mouse.move` call changes nothing
  // about the velocity already recorded from THAT move's own internal
  // `steps`, whose real inter-step timing is under no guarantee at all
  // (found by execution: a single `mouse.move(..., { steps: 2 })` here,
  // exactly as it read before this fix, reordered the drawer to "closed"
  // in roughly 1 of 3 runs even on an otherwise idle box, every time with
  // the identical `data-state` mismatch -- Playwright dispatches a
  // `{ steps: N }` move's synthetic events as fast as the page processes
  // them, occasionally under 1ms apart, which the `.max(1.0)` floor turns
  // into an artificially explosive velocity rather than a merely
  // undefined one). Splitting the same 15px into two explicit moves with
  // a real, measured pause between them -- the long-drag test's own
  // technique -- gives the last-two-samples velocity a genuine, timed
  // denominator instead of an incidental one.
  await page.mouse.move(startX, startY);
  await page.mouse.down();
  await page.mouse.move(startX, startY + 7, { steps: 1 });
  await page.waitForTimeout(120);
  await page.mouse.move(startX, startY + 15, { steps: 1 });
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

  test('top variant, dark mode, open has no automatically detectable a11y issues', async ({ page }) => {
    // Regression: `DrawerClose`'s default (no `as` override) rendering --
    // exactly the "Close" button in this component's own top-side demo --
    // had no CSS rule of its own at all, so its `background-color` was the
    // UA's `ButtonFace`, which never responds to `data-theme`. Measured
    // live pre-fix in dark mode: `color: rgb(212, 212, 212)` (correctly
    // inverted, inherited from `.dx-drawer`) on `background-color: rgb(239,
    // 239, 239)` (NOT inverted, the same value as light mode) -- light text
    // on a near-white background.
    await page.goto(`${URL}dark_mode=true`, GOTO_OPTS);
    await page.getByRole('button', { name: 'Open from Top' }).click();
    await expect(page.locator('[data-slot="drawer-root"]')).toHaveAttribute('data-state', 'open');
    await expectNoAxeViolations(page, 'drawer: top variant, dark mode, open', {
      excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT],
    });
  });
});
