import { test, expect } from './fixtures';
import type { Page } from '@playwright/test';
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from './axe';
import { BASE_URL } from './base-url';
import { expectFadeKeepsTransform, probeAnimations } from './assert-anchor-transform';

test('hover navigation', async ({ page }) => {
  await page.goto(`${BASE_URL}/component/?name=navbar&`, { timeout: 20 * 60 * 1000 }); // Increase timeout to 20 minutes
  // wait for the styles to load
  await expect(page.getByRole('menuitem', { name: 'Inputs' })).toHaveCSS('border-width', '0px');
  const inputsNav = page.getByRole('menu').filter({ has: page.getByRole('menuitem', { name: 'Inputs' }) }).first();
  await inputsNav.hover();
  await expect(inputsNav).toHaveAttribute('data-state', 'open');
  const calendar = page.getByRole('menuitem', { name: 'Calendar' });
  await expect(calendar).toBeVisible();
  await calendar.evaluate((element) => {
    (element as HTMLElement).click();
  });
  // Assert the url changed to the calendar component. Path-segment form
  // (dev-docs/backlog.md row 46): `navbar`'s own demo fixture
  // (preview/src/components/navbar/variants/main/mod.rs) links via
  // `Route::component`, which now builds the canonical, SSG-enumerable
  // `/component/<name>/` route rather than the legacy `?name=...` query
  // form (still supported -- it redirects client-side to this same path).
  await expect(page).toHaveURL(/\/component\/calendar\//);
});

test('mobile navigation', async ({ page }) => {
  await page.goto(`${BASE_URL}/component/?name=navbar&`, { timeout: 20 * 60 * 1000 }); // Increase timeout to 20 minutes
  await page.getByRole('menuitem', { name: 'Inputs' }).tap();
  await page.getByRole('menuitem', { name: 'Calendar' }).tap();
  // Assert the url changed to the calendar component (path-segment form,
  // see "hover navigation"'s identical comment above).
  await expect(page).toHaveURL(/\/component\/calendar\//);
});

test('keyboard navigation', async ({ page }) => {
  await page.goto(`${BASE_URL}/component/?name=navbar&`, { timeout: 20 * 60 * 1000 }); // Increase timeout to 20 minutes
  await page.locator('#component-preview-frame').first().getByRole('menubar').focus();
  // Go right with the keyboard
  await page.keyboard.press('ArrowRight');
  // Assert the focus is on the information menu item
  await expect(page.getByRole('menuitem', { name: 'Information' })).toBeFocused();
  // Go left with the keyboard
  await page.keyboard.press('ArrowLeft');
  // Assert the focus is on the inputs menu item
  await expect(page.getByRole('menuitem', { name: 'Inputs' })).toBeFocused();
  await page.keyboard.press('ArrowDown');
  // Assert the focus is on the calendar menu item
  await expect(page.getByRole('menuitem', { name: 'Calendar' })).toBeFocused();
  await expect(page.getByRole('menuitem', { name: 'Slider' })).toHaveAttribute('data-disabled', 'true');
  await page.keyboard.press('ArrowDown');
  // Assert the disabled slider item is skipped
  await expect(page.getByRole('menuitem', { name: 'Checkbox' })).toBeFocused();
  // Click the focused menu item
  await page.keyboard.press('Enter');
  // Assert the url changed to the checkbox component (path-segment form,
  // see "hover navigation"'s identical comment above).
  await expect(page).toHaveURL(/\/component\/checkbox\//);
});

test.describe('Close animation is not skipped (docs/backlog.md row 77 follow-up)', () => {
  // row 77 landed the identical fix for Navigation Menu and flagged --
  // but never investigated -- that `navbar.rs` pairs the same
  // `use_animated_open` + `use_popover_sync` combination, so it likely
  // carries the same dead exit-animation defect. Confirmed by execution
  // (an isolated dev server, this lane): sampling `.dx-navbar-content`
  // every animation frame across a real close showed `opacity`/`transform`
  // jump straight from the open resting value to the closed target between
  // two consecutive frames, with the content already disconnected from the
  // DOM one frame later -- `use_popover_sync`'s synchronous `hidePopover()`
  // (called the instant `open` went `false`) removed the element before
  // `use_animated_open`'s own rAF-deferred `getAnimations()` check ever
  // ran, so the ~150ms fade-and-scale-down plus its ~250ms settle hold
  // (`primitives/src/lib.rs`'s `use_animated_open`) were skipped outright.
  // This block asserts the construction directly: the content must stay
  // mounted and mid-animation for a real stretch of frames after closing,
  // not vanish within one or two.
  type Sample = { t: number; opacity: string; dataState: string | null; connected: boolean };

  declare global {
    interface Window {
      __navbarCloseSamples?: Sample[];
      __navbarCloseSampling?: boolean;
    }
  }

  // Two-step, not a single `page.evaluate` returning a not-yet-awaited
  // promise: starting the rAF loop and *awaiting that start* before the
  // real key press below is what guarantees the sampler's first frame is
  // already scheduled before the close fires -- an evaluate whose own
  // Promise executor is still in flight when a following Node-side action
  // fires is a real, observed race (an earlier draft of this test, with no
  // synchronizing step in between, intermittently sampled zero frames at
  // all -- `document.querySelector` had nothing to find yet by the time it
  // ran). The loop itself still runs entirely in-page via `requestAnimationFrame`
  // (row 112: sample in-page, not across await round trips) -- only its
  // start is synchronized with Node.
  async function startSampling(page: Page) {
    await page.evaluate(() => {
      const el = document.querySelector('.dx-navbar-content') as HTMLElement | null;
      window.__navbarCloseSamples = [];
      window.__navbarCloseSampling = !!el;
      if (!el) return;
      function tick() {
        if (!window.__navbarCloseSampling) return;
        window.__navbarCloseSamples!.push({
          t: performance.now(),
          opacity: getComputedStyle(el!).opacity,
          dataState: el!.getAttribute('data-state'),
          connected: el!.isConnected,
        });
        requestAnimationFrame(tick);
      }
      requestAnimationFrame(tick);
    });
  }

  async function stopSampling(page: Page): Promise<Sample[]> {
    return page.evaluate(() => {
      window.__navbarCloseSampling = false;
      return window.__navbarCloseSamples ?? [];
    });
  }

  test('the panel stays mounted and connected while its close animation plays', async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=navbar&`, { timeout: 20 * 60 * 1000 });
    const trigger = page.getByRole('menuitem', { name: 'Inputs' });
    await expect(trigger).toHaveCSS('border-width', '0px');
    await trigger.hover();
    const content = page.locator('.dx-navbar-content[data-state="open"]');
    await expect(content).toBeVisible();
    await expect(page.getByRole('menuitem', { name: 'Calendar' })).toBeVisible();

    // Close via Escape (`Navbar`'s own root `onkeydown`, `primitives/src/
    // navbar.rs`), which calls `set_open_nav.call(None)` directly and
    // `prevent_default()`s the key event -- a script-driven close, not one
    // the browser's own native `popover="auto"` light-dismiss algorithm
    // ever gets to intervene in. This is deliberate, not an easier
    // substitute for "click elsewhere"/"hover a different nav": those two
    // *do* go through native forced-hide (per WHATWG, showing a sibling
    // `auto` popover -- or an outside pointerdown -- hides this one
    // synchronously, at the browser level, before any Rust effect ever
    // runs) -- a separate, pre-existing limit of `PopoverKind::Auto` that
    // no hook swap on the `open` signal can reach (`use_popover_shown_
    // while_mounted`'s own doc calls this an accepted limit of the plain
    // Popover API). Escape isolates the exact class row 77 flagged and
    // this fix closes: `use_popover_sync`'s own synchronous `hidePopover()`
    // racing `use_animated_open`'s exit animation.
    await page.locator('[role="menubar"]').first().focus();

    // Start the in-page rAF sampling loop, THEN close, THEN collect --
    // per row 112, the actual per-frame sampling always happens in-page.
    await startSampling(page);
    await page.keyboard.press('Escape');
    await page.waitForTimeout(700);
    const samples = await stopSampling(page);

    expect(samples.length).toBeGreaterThan(10);

    // Real close animations are never instant: some meaningful stretch of
    // frames must show the content still connected to the DOM with
    // `data-state="closed"` (the animating-out phase `use_animated_open`
    // holds it in during) before it actually unmounts. Pre-fix, this
    // window was one frame wide (often zero, since the very next sample
    // already read `connected: false`) -- this asserts a real hold, not a
    // one-frame technicality.
    const closedWhileConnected = samples.filter((s) => s.dataState === 'closed' && s.connected);
    expect(closedWhileConnected.length).toBeGreaterThan(3);

    // The fade must be gradual: at least one sampled frame mid-close reads
    // an opacity strictly between the open (1) and closed (0) resting
    // values -- impossible if `opacity` jumps directly from one to the
    // other between two consecutive frames, exactly the pre-fix defect.
    const midFadeOpacities = closedWhileConnected
      .map((s) => Number.parseFloat(s.opacity))
      .filter((o) => Number.isFinite(o) && o > 0 && o < 1);
    expect(midFadeOpacities.length).toBeGreaterThan(0);

    // It still does eventually unmount (the fix must not leak the element
    // forever) -- the last sampled frame(s), past the animation's own
    // duration plus its settle hold, should show it gone.
    const lastSample = samples[samples.length - 1];
    expect(lastSample.connected).toBe(false);
  });
});

test.describe('Axe automated scan', () => {
  test('loaded (dropdown closed) has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=navbar&`, { timeout: 20 * 60 * 1000 });
    // Wait for render before scanning -- see input.spec.ts's identical
    // comment for why (avoids a false pre-hydration "no main"/"no h1").
    await expect(page.getByRole('menuitem', { name: 'Inputs' })).toBeVisible();
    await expectNoAxeViolations(page, 'navbar: loaded', { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });

  test('Inputs dropdown open has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=navbar&`, { timeout: 20 * 60 * 1000 });
    const inputsNav = page.getByRole('menu').filter({ has: page.getByRole('menuitem', { name: 'Inputs' }) }).first();
    await inputsNav.hover();
    await expect(inputsNav).toHaveAttribute('data-state', 'open');
    await expectNoAxeViolations(page, 'navbar: Inputs dropdown open', { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});

// ---------------------------------------------------------------------------
// `open_on_hover: false` -- click activation
// (preview/src/components/navbar/variants/click_only/mod.rs, which renders
// alongside `main` on this page under its own labels, so the hover-driven
// tests above can never match it). The default behaviour (hover opens,
// hovering another switches, leaving closes) is the unchanged "hover
// navigation" test at the top of this file.
// ---------------------------------------------------------------------------
test.describe('open_on_hover: false (click activation)', () => {
  const HOVER_SETTLE_MS = 1000;

  // A `NavbarNav` is a `role="menu"` wrapper around its trigger + content;
  // its `data-state` is what flips open/closed.
  function clickNav(page: Page, trigger: string) {
    return page
      .getByRole('menu')
      .filter({ has: page.getByRole('menuitem', { name: trigger, exact: true }) })
      .first();
  }

  test('hovering a trigger for a second does not open it; a click does', async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=navbar&`, { timeout: 20 * 60 * 1000 });
    // Wait for the app to render (and its styles to load) before reading
    // state -- `goto` only waits for the document, not the wasm app.
    await expect(page.getByRole('menuitem', { name: 'Layout', exact: true })).toBeVisible({ timeout: 60_000 });
    const layout = clickNav(page, 'Layout');
    await expect(layout).toHaveAttribute('data-state', 'closed');

    await page.getByRole('menuitem', { name: 'Layout', exact: true }).hover();
    await page.waitForTimeout(HOVER_SETTLE_MS);
    await expect(layout).toHaveAttribute('data-state', 'closed');

    await page.getByRole('menuitem', { name: 'Layout', exact: true }).click();
    await expect(layout).toHaveAttribute('data-state', 'open');
    await expect(page.getByRole('menuitem', { name: 'Card', exact: true })).toBeVisible();
  });

  test('the pointer leaving a click-opened dropdown does not close it', async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=navbar&`, { timeout: 20 * 60 * 1000 });
    const layout = clickNav(page, 'Layout');
    await page.getByRole('menuitem', { name: 'Layout', exact: true }).click();
    await expect(layout).toHaveAttribute('data-state', 'open');

    await page.getByRole('heading', { level: 1 }).hover();
    await page.waitForTimeout(HOVER_SETTLE_MS);
    await expect(layout).toHaveAttribute('data-state', 'open');
  });

  test('hovering another trigger while one is open does not switch; clicking it does', async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=navbar&`, { timeout: 20 * 60 * 1000 });
    const layout = clickNav(page, 'Layout');
    const overlays = clickNav(page, 'Overlays');
    await page.getByRole('menuitem', { name: 'Layout', exact: true }).click();
    await expect(layout).toHaveAttribute('data-state', 'open');

    await page.getByRole('menuitem', { name: 'Overlays', exact: true }).hover();
    await page.waitForTimeout(HOVER_SETTLE_MS);
    await expect(layout).toHaveAttribute('data-state', 'open');
    await expect(overlays).toHaveAttribute('data-state', 'closed');

    await page.getByRole('menuitem', { name: 'Overlays', exact: true }).click();
    await expect(overlays).toHaveAttribute('data-state', 'open');
    await expect(layout).toHaveAttribute('data-state', 'closed');
  });

  test('keyboard still opens and Escape closes', async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=navbar&`, { timeout: 20 * 60 * 1000 });
    const layout = clickNav(page, 'Layout');
    await page.getByRole('menuitem', { name: 'Layout', exact: true }).focus();
    await page.keyboard.press('ArrowDown');
    await expect(layout).toHaveAttribute('data-state', 'open');
    await expect(page.getByRole('menuitem', { name: 'Card', exact: true })).toBeFocused();
    await page.keyboard.press('Escape');
    await expect(layout).toHaveAttribute('data-state', 'closed');
  });

  test('a press outside closes a click-opened dropdown', async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=navbar&`, { timeout: 20 * 60 * 1000 });
    const layout = clickNav(page, 'Layout');
    await page.getByRole('menuitem', { name: 'Layout', exact: true }).click();
    await expect(layout).toHaveAttribute('data-state', 'open');
    await page.getByRole('heading', { level: 1 }).click();
    await expect(layout).toHaveAttribute('data-state', 'closed');
  });
});

// ---------------------------------------------------------------------------
// Open/close fade vs. the anchor's centring transform (2026-10-05). An anchored overlay centres itself
// with `transform: translateX(-50%)` (top_layer.rs's engine stylesheet); a `@keyframes` that sets
// `transform` REPLACES it for as long as it runs (the date picker played its fade 144px off-centre, the
// colour picker 133px). `NavbarContent` sets no `data-side`, so it has no centring to lose today -- the same
// shape, latent -- so this probes the rendered keyframes instead of sampling a position that could not
// move anyway: `assert-anchor-transform.ts` supplies an inline `transform`, seeks the element's own
// CSS animation to its start/midpoint/end, and asserts the transform survives while `scale`/`translate`
// do the moving. Source-level guard: scripts/check-anchored-keyframes.sh.
// ---------------------------------------------------------------------------
test.describe('Open/close animation', () => {
  test('the fade animates translate/scale and leaves transform alone (open, and every close direction)', async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=navbar&`, { timeout: 20 * 60 * 1000 });
    const trigger = page.getByRole('menuitem', { name: 'Inputs' });
    await expect(trigger).toHaveCSS('border-width', '0px');
    await trigger.hover();
    const content = page.locator('.dx-navbar-content[data-state="open"]');
    await expect(content).toBeVisible();
    await expectFadeKeepsTransform(content, 'navbar content, open');
    // One keyframe pair per `data-open-menu-direction` (navbar/style.css).
    for (const direction of ['closed', 'start', 'end']) {
      await expectFadeKeepsTransform(content, `navbar content, close (${direction})`, {
        'data-state': 'closed',
        'data-open-menu-direction': direction,
      });
    }
  });

  // The slide offsets are physical (`translate: -100% 0` / `100% 0`, "rtl-physical" in the stylesheet): a
  // transform or `translate` is never direction-aware, so each `start`/`end` close variant has an explicit
  // `[dir="rtl"]` twin with the sign flipped (`dx-navbar-content-animate-out-{start,end}-rtl`).
  // scripts/check-css-logical-properties.sh scans the `translate` property for exactly this since
  // 2026-10-05; this proves the twins really are selected and really mirror, and keep `transform` alone.
  test('RTL: the start/end close slides mirror under dir=rtl and the twins leave transform alone', async ({ page }) => {
    // One page load per direction, with `dir` set while every nav is still closed: flipping it under an open
    // nav reverses the row beneath a stationary pointer, which hovers a SIBLING nav, closes this one and opens
    // that one (observed in about half of the runs, and the sibling then stays open).
    const slideEndX = async (rtl: boolean, direction: string): Promise<string> => {
      await page.goto(`${BASE_URL}/component/?name=navbar&`, { timeout: 20 * 60 * 1000 });
      const trigger = page.getByRole('menuitem', { name: 'Inputs' });
      await expect(trigger).toHaveCSS('border-width', '0px');
      if (rtl) await page.evaluate(() => document.documentElement.setAttribute('dir', 'rtl'));
      await trigger.hover();
      const content = page.locator('.dx-navbar-content[data-state="open"]');
      await expect(content).toBeVisible();
      const attrs = { 'data-state': 'closed', 'data-open-menu-direction': direction };
      const fades = (await probeAnimations(content, attrs)).filter((a) => a.keyframeProps.includes('opacity'));
      expect(fades.length, `navbar close (${direction}, ${rtl ? 'rtl' : 'ltr'}): the fade animation`).toBe(1);
      // The twins are fades too: they must leave `transform` alone like the LTR keyframes.
      await expectFadeKeepsTransform(content, `navbar content, close (${direction}, ${rtl ? 'rtl' : 'ltr'})`, attrs);
      return fades[0].end.translate;
    };
    // The x offset's sign, from the computed `translate` ("-100%", "100%" or "-224px 0px").
    const sign = (translate: string) => (translate.trim().startsWith('-') ? -1 : 1);

    const ltr = { start: await slideEndX(false, 'start'), end: await slideEndX(false, 'end') };
    expect(sign(ltr.start), `LTR start slides out to the start side: ${JSON.stringify(ltr)}`).toBe(-1);
    expect(sign(ltr.end), `LTR end slides out to the end side: ${JSON.stringify(ltr)}`).toBe(1);

    const rtl = { start: await slideEndX(true, 'start'), end: await slideEndX(true, 'end') };
    expect(sign(rtl.start), `RTL start is the mirror of LTR start: ${JSON.stringify({ ltr, rtl })}`).toBe(1);
    expect(sign(rtl.end), `RTL end is the mirror of LTR end: ${JSON.stringify({ ltr, rtl })}`).toBe(-1);
  });
});
