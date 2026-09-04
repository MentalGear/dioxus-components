import { test, expect, type Page } from '@playwright/test';
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from './axe';

test('test', async ({ page }) => {
  await page.goto('http://127.0.0.1:8080/component/?name=toast&');
  // Create a toast
  await page.getByRole('button', { name: 'Info (60s)' }).click();
  // Create another toast
  await page.getByRole('button', { name: 'Info (60s)' }).click();
  // exact: true — the default substring match also catches the docs
  // sidebar's "Close navigation" button whenever the stylesheet that hides
  // it at desktop widths has not fully applied yet (its @import blocks the
  // rest of the cascade on slow networks), sending both clicks to it
  // instead of a toast.
  const toast_close_buttons = page.getByRole('button', { name: 'close', exact: true });
  // Hover and close the first toast
  await toast_close_buttons.first().hover();
  await toast_close_buttons.first().click();
  await expect(toast_close_buttons).toHaveCount(1);

  // Hover and close the second toast
  await toast_close_buttons.first().hover();
  await toast_close_buttons.first().click();
  await expect(toast_close_buttons).toHaveCount(0);
});

/**
 * docs/backlog.md row 44 -- "top_layer fixture's 'Add toast' appears dead
 * on iOS 18" (device report 2026-09-03), root-caused to an unstyled toast
 * getting whatever box the engine's own popover UA-stylesheet defaults
 * happened to produce, which differs enough between engines to matter
 * (Chromium ~105x40px on-screen; WebKit's default was the unverified
 * suspect for the device report). `toBeVisible()` alone -- the only check
 * the tests above and `oracle/tier2-html/top-layer.spec.ts`'s Rule 10 ran
 * before this -- does not catch that: it only checks that an element has
 * *some* box and no `visibility: hidden`/`display: none`, never that the
 * box is a real, on-screen size, which is exactly why this escaped.
 * `boundingBox()`, asserted non-null/non-zero/within-viewport, is the
 * oracle row 44 specifies, run on both the themed demo (this file's own
 * subject) and the `top_layer` fixture
 * (`preview/src/components/top_layer/component.rs`,
 * `#toast-stack-trigger`/"Add toast") -- the fix landed at the primitive
 * level (`primitives/src/toast.rs`'s `ensure_toast_base_styles`, a
 * minimal, always-injected fallback stylesheet -- see its own doc), not in
 * either page's own markup, so both lanes are worth covering independently
 * rather than trusting one to stand in for the other.
 */
test.describe('Toast bounding box (docs/backlog.md row 44)', () => {
  async function assertToastOnScreen(page: Page) {
    const toast = page.getByRole('alertdialog').first();
    await expect(toast).toBeVisible();
    const box = await toast.boundingBox();
    const viewport = page.viewportSize();
    if (!box || !viewport) {
      throw new Error(
        `expected both a toast bounding box and a viewport, got box=${JSON.stringify(box)} viewport=${JSON.stringify(viewport)}`,
      );
    }
    const debug = `box=${JSON.stringify(box)} viewport=${JSON.stringify(viewport)}`;
    // Non-zero: a collapsed (0x0, or near-zero from an unstyled
    // `fit-content` box in an engine that resolves it differently) toast is
    // exactly the "appears dead" failure mode this row is about.
    expect(box.width, debug).toBeGreaterThan(0);
    expect(box.height, debug).toBeGreaterThan(0);
    // Inside the viewport: a real box positioned off-screen (e.g. still
    // carrying the UA popover stylesheet's un-reset `inset`/`margin`
    // centering-trap this file's own primitive doc describes) would pass
    // the two checks above while still being unreachable to a real user.
    expect(box.x, debug).toBeGreaterThanOrEqual(0);
    expect(box.y, debug).toBeGreaterThanOrEqual(0);
    expect(box.x + box.width, debug).toBeLessThanOrEqual(viewport.width);
    expect(box.y + box.height, debug).toBeLessThanOrEqual(viewport.height);
  }

  test('themed demo: the toast renders a real, on-screen box', async ({ page }) => {
    await page.goto('http://127.0.0.1:8080/component/?name=toast&');
    await page.getByRole('button', { name: 'Info (60s)' }).click();
    await assertToastOnScreen(page);
  });

  test('top_layer fixture: "Add toast" renders a real, on-screen box', async ({ page }) => {
    await page.goto('http://127.0.0.1:8080/component/?name=top_layer&');
    // The trigger is a normal in-flow element well down this fixture page
    // (see `preview/src/components/top_layer/component.rs`'s own comment
    // on this section) -- scroll it into view before clicking, mirroring
    // `oracle/tier2-html/top-layer.spec.ts`'s Rule 10 test for the same
    // trigger.
    await page.locator('#toast-stack-trigger').scrollIntoViewIfNeeded();
    await page.locator('#toast-stack-trigger').click();
    await assertToastOnScreen(page);
  });

  // Bug 2 (docs/backlog.md row 44), separate from the bounding-box checks
  // above: the fixture's own `ToastProvider { style: "position: fixed;
  // ..." }` used to replace the primitive's own `style: "--toast-count:
  // {length}"` outright (`merge_attributes`'s plain-`style` rule is
  // last-wins, not concatenation), so the region's computed
  // `--toast-count` silently went missing whenever a caller supplied its
  // own `style` -- unlike the bounding-box checks above, this one does not
  // depend on any engine's popover-default box and is red in this
  // sandbox's own Chromium before the fix, not just on the unavailable-here
  // WebKit lane.
  test('top_layer fixture: --toast-count survives the fixture\'s own style override', async ({ page }) => {
    await page.goto('http://127.0.0.1:8080/component/?name=top_layer&');
    await page.locator('#toast-stack-trigger').scrollIntoViewIfNeeded();
    await page.locator('#toast-stack-trigger').click();
    const toastCount = await page.evaluate(() => {
      const region = document.querySelector('[role="region"][popover]');
      return region ? getComputedStyle(region).getPropertyValue('--toast-count').trim() : null;
    });
    expect(toastCount).toBe('1');
  });
});

test.describe('Axe automated scan', () => {
  test('loaded (no toast) has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto('http://127.0.0.1:8080/component/?name=toast&');
    await expectNoAxeViolations(page, 'toast: loaded', { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });

  test('a toast shown has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto('http://127.0.0.1:8080/component/?name=toast&');
    await page.getByRole('button', { name: 'Info (60s)' }).click();
    await expect(page.getByRole('button', { name: 'close', exact: true }).first()).toBeVisible();
    await expectNoAxeViolations(page, 'toast: toast shown', { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});
