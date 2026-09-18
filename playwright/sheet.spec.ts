import { test, expect } from '@playwright/test';
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from './axe';

test('sheet basic interactions', async ({ page }) => {
  await page.goto('http://127.0.0.1:8080/component/?name=sheet&', { timeout: 20 * 60 * 1000 });

  // Open sheet from Right button
  await page.getByRole('button', { name: 'Right' }).click();

  // Assert the sheet is open
  const sheet = page.locator('[data-slot="sheet-root"]');
  await expect(sheet).toHaveAttribute('data-state', 'open');

  // Assert the first input is focused (focus trap)
  const nameInput = page.locator('#sheet-demo-name');
  await expect(nameInput).toBeFocused();

  // Tab through focusable elements and verify focus cycles
  // Tab: name input -> username input -> Save button -> Cancel button -> close button -> name input
  await page.keyboard.press('Tab');
  const usernameInput = page.locator('#sheet-demo-username');
  await expect(usernameInput).toBeFocused();

  await page.keyboard.press('Tab');
  const saveButton = page.getByRole('button', { name: 'Save changes' });
  await expect(saveButton).toBeFocused();

  await page.keyboard.press('Tab');
  const cancelButton = page.getByRole('button', { name: 'Cancel' });
  await expect(cancelButton).toBeFocused();

  await page.keyboard.press('Tab');
  const closeButton = sheet.getByRole('button').last();
  await expect(closeButton).toBeFocused();

  // Phase 4.2 (docs/plan.md): Sheet composes the modal Dialog primitive,
  // now a native `<dialog>` on the web arm. Chromium's own focus trap parks
  // focus on `<body>` for exactly one Tab stop after the last focusable
  // element before wrapping to the first
  // (docs/phase4-spike-findings.md experiment 4a) -- harness correction for
  // the new trap's documented shape, same fix as dialog.spec.ts.
  await page.keyboard.press('Tab');
  await expect
    .poll(() => page.evaluate(() => document.activeElement === document.body))
    .toBe(true);

  // One more Tab cycles back to the first input.
  await page.keyboard.press('Tab');
  await expect(nameInput).toBeFocused();

  // Hitting escape should close the sheet
  await page.keyboard.press('Escape');
  await expect(sheet).toHaveCount(0);

  // Reopen the sheet
  await page.getByRole('button', { name: 'Right' }).click();
  await expect(sheet).toHaveAttribute('data-state', 'open');

  // Click the close button
  await closeButton.click();
  await expect(sheet).toHaveCount(0);
});

test('sheet opens from different sides', async ({ page }) => {
  await page.goto('http://127.0.0.1:8080/component/?name=sheet&', { timeout: 20 * 60 * 1000 });

  const sheet = page.locator('[data-slot="sheet-root"]');
  const sheetContent = page.locator('[data-slot="sheet-content"]');

  // Test Top
  await page.getByRole('button', { name: 'Top' }).click();
  await expect(sheet).toHaveAttribute('data-state', 'open');
  await expect(sheetContent).toHaveAttribute('data-side', 'top');
  await page.keyboard.press('Escape');
  await expect(sheet).toHaveCount(0);

  // Test Bottom
  await page.getByRole('button', { name: 'Bottom' }).click();
  await expect(sheet).toHaveAttribute('data-state', 'open');
  await expect(sheetContent).toHaveAttribute('data-side', 'bottom');
  await page.keyboard.press('Escape');
  await expect(sheet).toHaveCount(0);

  // Test Left
  await page.getByRole('button', { name: 'Left' }).click();
  await expect(sheet).toHaveAttribute('data-state', 'open');
  await expect(sheetContent).toHaveAttribute('data-side', 'left');
  await page.keyboard.press('Escape');
  await expect(sheet).toHaveCount(0);
});

test('right sheet content geometry matches shadcn spec at desktop and near-breakpoint widths', async ({ page }) => {
  // shadcn: `inset-y-0 h-full w-3/4 border-l sm:max-w-sm` -- 384px (24rem)
  // only from a 640px viewport up; below that, plain 75% with no cap.
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto('http://127.0.0.1:8080/component/?name=sheet&', { timeout: 20 * 60 * 1000 });

  const sheet = page.locator('[data-slot="sheet-root"]');
  const content = page.locator('[data-slot="sheet-content"]');

  await page.getByRole('button', { name: 'Right' }).click();
  await expect(sheet).toHaveAttribute('data-state', 'open');

  // At 1280px, 75vw (960px) is well past the 384px cap, so the panel should
  // land exactly on min(75vw, 384px) = 384px.
  let box = await content.boundingBox();
  expect(box).not.toBeNull();
  expect(box!.width).toBeCloseTo(384, 0);

  await page.keyboard.press('Escape');
  await expect(sheet).toHaveCount(0);

  // Below the 640px breakpoint the cap does not apply -- the panel should
  // track ~75% of the viewport instead of staying pinned at 384px. 600px is
  // chosen so the two formulas clearly disagree (75% of 600 is 450; 384 is
  // what an (incorrectly) unconditional cap would still produce). The
  // bounds below are deliberately a range rather than one exact pixel value:
  // this app reserves a permanent scrollbar gutter
  // (primitives/src/scroll_lock.rs's `ensure_scrollbar_gutter_baseline`),
  // which is also this fixed-position percentage's real containing block,
  // so the precise result is a few px under a naive `0.75 * 600` depending
  // on the platform's scrollbar width -- immaterial to what this assertion
  // is actually checking (that the cap does not apply here at all).
  await page.setViewportSize({ width: 600, height: 800 });
  await page.getByRole('button', { name: 'Right' }).click();
  await expect(sheet).toHaveAttribute('data-state', 'open');
  box = await content.boundingBox();
  expect(box).not.toBeNull();
  expect(box!.width).toBeGreaterThan(400); // clearly past the 384px cap
  expect(box!.width).toBeLessThan(460); // still recognizably ~75%, not e.g. 100%

  await page.keyboard.press('Escape');
  await expect(sheet).toHaveCount(0);
});

// Regression coverage for a native-`<dialog>` centering defect shared with
// Drawer: Chromium's `dialog:modal` UA rule ships `inset: 0; margin: auto;
// width/height: fit-content`. Overriding only the one edge a side needs
// (e.g. `right: 0`) used to leave the UA's own `inset: 0` still supplying
// the OPPOSITE edge uncontested, over-constraining the box against a
// definite `width`/`height` and making the UA center it into its own auto
// margins instead of anchoring it -- a right/left sheet rendered as a
// floating centered card, and a top/bottom sheet stretched to the full
// viewport height instead of sizing to its content. Checks, per side, that
// the panel's own edge coincides with the matching viewport edge (not
// merely that its `data-side` attribute is correct, which the "opens from
// different sides" test above already covers) and that the cross-axis
// dimension is content-sized, not a full-viewport stretch.
test('sheet content is anchored flush to its own edge, not centered', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto('http://127.0.0.1:8080/component/?name=sheet&', { timeout: 20 * 60 * 1000 });

  const sheet = page.locator('[data-slot="sheet-root"]');
  const content = page.locator('[data-slot="sheet-content"]');
  // The real, available layout width/height (this app reserves a permanent
  // scrollbar gutter -- see the geometry test above), not the raw viewport
  // size: an edge-anchored panel's far edge lines up with THIS, not
  // necessarily with `page.viewportSize()`.
  const avail = await page.evaluate(() => ({
    w: document.documentElement.clientWidth,
    h: document.documentElement.clientHeight,
  }));

  // Tolerance covers this app's permanent scrollbar-gutter reservation
  // (primitives/src/scroll_lock.rs), which shifts the panel's fixed-position
  // containing block a little short of the raw `clientWidth`/`clientHeight`
  // read above (observed ~15px) -- not a pixel-perfect fit, but the whole
  // point of this assertion is "flush against its own edge, not centered",
  // a difference measured in hundreds of pixels, so a generous tolerance
  // loses none of the check's actual power.
  const TOLERANCE = 20;
  function closeTo(actual: number, expected: number) {
    expect(Math.abs(actual - expected)).toBeLessThan(TOLERANCE);
  }

  async function openAndMeasure(side: string) {
    await page.getByRole('button', { name: side, exact: true }).click();
    await expect(sheet).toHaveAttribute('data-state', 'open');
    // Content slides in over 500ms (style.css's `dx-slide-in-*` animations);
    // `getBoundingClientRect()` (what `boundingBox()` uses) reflects the
    // CURRENT, still-animating transform, not the final resting position --
    // measuring immediately after the `data-state` flip catches the panel
    // still near its off-screen starting `translate`, not where it settles.
    await page.waitForTimeout(700);
    const box = (await content.boundingBox())!;
    expect(box).not.toBeNull();
    await page.keyboard.press('Escape');
    await expect(sheet).toHaveCount(0);
    return box;
  }

  const right = await openAndMeasure('Right');
  closeTo(right.x + right.width, avail.w); // flush right
  expect(right.x).toBeGreaterThan(avail.w / 2); // not centered
  closeTo(right.height, avail.h); // h-full: stretches top-to-bottom

  const left = await openAndMeasure('Left');
  closeTo(left.x, 0); // flush left
  closeTo(left.height, avail.h); // h-full

  const top = await openAndMeasure('Top');
  closeTo(top.y, 0); // flush top
  closeTo(top.x + top.width, avail.w); // inset-x-0: full width
  expect(top.height).toBeLessThan(avail.h * 0.9); // h-auto: content-sized, not a full-height stretch

  const bottom = await openAndMeasure('Bottom');
  closeTo(bottom.y + bottom.height, avail.h); // flush bottom
  closeTo(bottom.x + bottom.width, avail.w); // inset-x-0: full width
  expect(bottom.height).toBeLessThan(avail.h * 0.9); // h-auto: content-sized
});

test('close button matches shadcn spec: 16px icon, opacity-70, rounded-xs, top-right', async ({ page }) => {
  await page.goto('http://127.0.0.1:8080/component/?name=sheet&', { timeout: 20 * 60 * 1000 });

  const sheet = page.locator('[data-slot="sheet-root"]');
  await page.getByRole('button', { name: 'Right' }).click();
  await expect(sheet).toHaveAttribute('data-state', 'open');

  const content = page.locator('[data-slot="sheet-content"]');
  const closeButton = sheet.getByRole('button').last();

  // shadcn's close icon is `size-4` (16px).
  const iconBox = await closeButton.locator('svg').first().boundingBox();
  expect(iconBox).not.toBeNull();
  expect(iconBox!.width).toBeCloseTo(16, 0);
  expect(iconBox!.height).toBeCloseTo(16, 0);

  // shadcn: `opacity-70` default state, `rounded-xs` (2px) corners.
  const opacity = await closeButton.evaluate((el) => getComputedStyle(el).opacity);
  expect(opacity).toBe('0.7');
  const borderRadius = await closeButton.evaluate((el) => getComputedStyle(el).borderRadius);
  expect(borderRadius).toBe('2px');

  // shadcn: `absolute top-4 right-4` -- close to the content's own top-right
  // corner (within one button-box's worth of margin), not flush against the
  // viewport edge or hanging off the panel.
  const contentBox = await content.boundingBox();
  const closeBox = await closeButton.boundingBox();
  expect(contentBox).not.toBeNull();
  expect(closeBox).not.toBeNull();
  const rightGap = contentBox!.x + contentBox!.width - (closeBox!.x + closeBox!.width);
  const topGap = closeBox!.y - contentBox!.y;
  expect(rightGap).toBeGreaterThanOrEqual(0);
  expect(rightGap).toBeLessThan(24);
  expect(topGap).toBeGreaterThanOrEqual(0);
  expect(topGap).toBeLessThan(24);
});

test.describe('Axe automated scan', () => {
  test('loaded (sheet closed) has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto('http://127.0.0.1:8080/component/?name=sheet&', { timeout: 20 * 60 * 1000 });
    // Wait for render before scanning -- see input.spec.ts's identical
    // comment for why (avoids a false pre-hydration "no main"/"no h1").
    await expect(page.getByRole('button', { name: 'Right' })).toBeVisible();
    await expectNoAxeViolations(page, 'sheet: loaded', { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });

  test('open has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto('http://127.0.0.1:8080/component/?name=sheet&', { timeout: 20 * 60 * 1000 });
    await page.getByRole('button', { name: 'Right' }).click();
    await expect(page.locator('[data-slot="sheet-root"]')).toHaveAttribute('data-state', 'open');
    await expectNoAxeViolations(page, 'sheet: open', { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});
