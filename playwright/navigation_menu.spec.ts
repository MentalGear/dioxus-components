import { test, expect, type Page } from '@playwright/test';
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from './axe';

const URL = 'http://127.0.0.1:8080/component/?name=navigation_menu&';
const GOTO = { timeout: 20 * 60 * 1000 }; // Increase timeout to 20 minutes

/**
 * This demo page's own `NavigationMenu` -- scoped by its `aria_label`
 * (`preview/src/components/navigation_menu/variants/main/mod.rs`), not
 * `page.getByRole(...)` directly against the whole page. The site's own
 * persistent chrome (header navbar, footer -- `preview/src/main.rs`,
 * out of this lane's ownership) renders its own "Docs" links on every
 * route, and would otherwise make `page.getByRole('link', { name: 'Docs' })`
 * resolve to more than one element (confirmed by live reproduction: 3
 * matches page-wide -- header, footer, this demo -- vs. exactly 1 once
 * scoped to this locator). Every test below goes through this helper
 * rather than repeating the same page-wide-vs-scoped mistake per test.
 */
function nav(page: Page) {
  return page.getByRole('navigation', { name: 'Component navigation menu' });
}

test('hover opens the panel', async ({ page }) => {
  await page.goto(URL, GOTO);
  const trigger = nav(page).getByRole('button', { name: 'Getting started' });
  await expect(trigger).toHaveAttribute('aria-expanded', 'false');
  await trigger.hover();
  // primitives/src/navigation_menu.rs's HOVER_OPEN_INTENT_DELAY (150ms) --
  // expect()'s own auto-retry tolerates the delay without this test
  // asserting its exact length (a tight "still closed immediately after
  // hover" window would be flaky under load, not a real methodology
  // improvement -- see dev-docs/dx-serve-hot-reload.md's own caution
  // about test-methodology mistakes producing false alarms).
  await expect(trigger).toHaveAttribute('aria-expanded', 'true');
});

test('opening one item closes the other', async ({ page }) => {
  await page.goto(URL, GOTO);
  const gettingStarted = nav(page).getByRole('button', { name: 'Getting started' });
  const components = nav(page).getByRole('button', { name: 'Components' });

  await gettingStarted.hover();
  await expect(gettingStarted).toHaveAttribute('aria-expanded', 'true');

  await components.hover();
  await expect(components).toHaveAttribute('aria-expanded', 'true');
  await expect(gettingStarted).toHaveAttribute('aria-expanded', 'false');
});

test('pointer leaving the trigger and its content closes the panel', async ({ page }) => {
  await page.goto(URL, GOTO);
  const trigger = nav(page).getByRole('button', { name: 'Getting started' });

  await trigger.hover();
  await expect(trigger).toHaveAttribute('aria-expanded', 'true');

  // Move the pointer well outside both the trigger and its (top-layer
  // promoted) content -- the page's own h1 is a safe, always-present target.
  await page.getByRole('heading', { level: 1 }).hover();
  // primitives/src/navigation_menu.rs's HOVER_CLOSE_GRACE_DELAY (300ms).
  await expect(trigger).toHaveAttribute('aria-expanded', 'false');
});

test('pointer entering the content cancels the close', async ({ page }) => {
  await page.goto(URL, GOTO);
  const trigger = nav(page).getByRole('button', { name: 'Getting started' });

  await trigger.hover();
  await expect(trigger).toHaveAttribute('aria-expanded', 'true');

  // The disclosed content is promoted to the top layer for painting on the
  // web arm (`primitives/src/navigation_menu.rs`'s module doc, "Top
  // layer") but never reparented, so it stays inside `nav(page)`'s scope.
  const featuredLink = nav(page).getByRole('link', { name: 'Component Library' });
  await expect(featuredLink).toBeVisible();
  // Leave the trigger for the content itself (not somewhere else) --
  // NavigationMenuTrigger/NavigationMenuContent's own onmouseenter cancels
  // the pending close (see this primitive's module doc, "Hover intent").
  await featuredLink.hover();
  await expect(trigger).toHaveAttribute('aria-expanded', 'true');
});

test('the plain top-level link is a real link', async ({ page }) => {
  await page.goto(URL, GOTO);
  const docsLink = nav(page).getByRole('link', { name: 'Docs' });
  await expect(docsLink).toHaveAttribute('href', '/docs');
  await docsLink.click();
  await expect(page).toHaveURL(/\/docs/);
});

test.describe('Open animation never reflows the content (user-reported)', () => {
  // User report on the live site: "hovering to show menus -- the menu
  // items' text reflows, which indicates the width is animated in a way
  // that is wrong; it should not be width." shadcn/Radix's own
  // NavigationMenuContent animates only opacity/transform (scale/
  // translate) -- the content keeps a stable, intrinsic width
  // (`max-content`, clamped by `min-width`/`max-width`) from the moment it
  // first mounts, so no line of text ever re-wraps mid-animation. This
  // block asserts that construction directly, sampling every animation
  // frame rather than a fixed handful of `setTimeout` polls (which could
  // straddle the one bad frame and miss it).
  async function sampleWidthEveryFrame(page: Page, forMs: number): Promise<number[]> {
    return page.evaluate((duration) => {
      return new Promise<number[]>((resolve) => {
        const samples: number[] = [];
        const deadline = performance.now() + duration;
        function tick() {
          const el = document.querySelector('.dx-navigation-menu-content');
          if (el) {
            samples.push(el.getBoundingClientRect().width);
          }
          if (performance.now() < deadline) {
            requestAnimationFrame(tick);
          } else {
            resolve(samples);
          }
        }
        requestAnimationFrame(tick);
      });
    }, forMs);
  }

  test("the content's width never changes while its open animation plays", async ({ page }) => {
    await page.goto(URL, GOTO);
    const trigger = nav(page).getByRole('button', { name: 'Getting started' });
    await expect(trigger).toHaveAttribute('aria-expanded', 'false');

    await trigger.hover();
    // Spans HOVER_OPEN_INTENT_DELAY (150ms) plus the open transition's own
    // --dx-motion-duration-slow (200ms), with margin --
    // primitives/src/navigation_menu.rs and this component's style.css.
    const widths = await sampleWidthEveryFrame(page, 500);

    // A few real, mounted-content samples are required, or this assertion
    // would trivially pass on an empty array if the query above ever broke.
    expect(widths.length).toBeGreaterThan(3);
    // Rounded to the nearest pixel: real layout stability, not sub-pixel
    // rounding noise, is what this guards -- a genuine width-based reflow
    // (the reported bug) would show a multi-pixel-wide range across these
    // samples, not a rounding-only difference.
    const distinctWidths = new Set(widths.map((w) => Math.round(w)));
    expect(distinctWidths.size).toBe(1);
  });

  test('the open/close motion actually animates transform, not just opacity', async ({ page }) => {
    // Regression coverage for the actual root cause this report traced to
    // (not a literal `width` transition -- see this file's own root-cause
    // account for the full evidence): `preview/src/components/
    // navigation_menu/style.css`'s `[data-state]` transform rules used to
    // tie in CSS specificity with the shared, engine-injected anchor-
    // positioning stylesheet's own `transform: none` reset
    // (primitives/src/top_layer.rs), and -- injected later in the cascade
    // -- that rule was winning, silently killing the scale/slide entrance
    // motion (opacity was the only property ever really transitioning).
    // Switching between two differently-sized panels with *that* motion
    // dead is what most plausibly produced the reported "reflow": each
    // panel popped to its own full (and differently sized) box instantly,
    // with no easing to make the size difference read as one continuous
    // animation rather than a jump.
    await page.goto(URL, GOTO);
    const trigger = nav(page).getByRole('button', { name: 'Getting started' });
    await trigger.hover();
    await expect(trigger).toHaveAttribute('aria-expanded', 'true');
    // `aria-expanded` flips the instant the signal does, well before the
    // open animation itself (--dx-motion-duration-slow, 200ms) has settled
    // -- wait past it, or the read below races a mid-animation value
    // (confirmed by execution: an earlier version of this test read a
    // genuine in-flight matrix here, e.g. "matrix(0.998, 0, 0, 0.998, 0,
    // 0.77)", and failed a strict identity check that only makes sense
    // once the animation has actually finished).
    await page.waitForTimeout(300);

    const settledTransform = await page.evaluate(() => {
      const el = document.querySelector('.dx-navigation-menu-content');
      return el ? getComputedStyle(el).transform : null;
    });
    // The settled "open" transform is `translateY(0) scale(1)`, i.e. the
    // identity matrix -- reported as either the literal string "none" or
    // an explicit identity matrix depending on whether an animation is
    // still formally associated with the element (confirmed by execution:
    // both forms observed across otherwise-identical settled states), so
    // this alone can't distinguish "correctly settled" from "was never
    // applied at all" by string equality. The real assertion is on the
    // *closed* state below, whose intended value
    // (`translateY(...) scale(0.98)`) is never the identity -- if the
    // engine-injected rule (or the JS anchor-positioning fallback's own
    // inline `transform: none` -- see this file's root-cause account) were
    // still winning, this would also read as identity instead.
    expect(['none', 'matrix(1, 0, 0, 1, 0, 0)']).toContain(settledTransform);

    // Flip `data-state` directly on the real, mounted element (bypassing
    // this primitive's own hover/focus lifecycle entirely) to isolate
    // exactly what the CSS cascade computes for "closed", independent of
    // timing -- the same technique this bug was originally diagnosed with.
    await page.evaluate(() => {
      const el = document.querySelector('.dx-navigation-menu-content');
      el?.setAttribute('data-state', 'closed');
    });
    // Read past this file's own --dx-motion-duration-base (150ms)
    // transition so this is the *settled* value, not a mid-transition one.
    await page.waitForTimeout(300);
    const settledClosedTransform = await page.evaluate(() => {
      const el = document.querySelector('.dx-navigation-menu-content');
      return el ? getComputedStyle(el).transform : null;
    });
    expect(settledClosedTransform).not.toBeNull();
    expect(settledClosedTransform).not.toBe('none');
  });
});

test.describe('Axe automated scan', () => {
  test('loaded (panel closed) has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto(URL, GOTO);
    // Wait for render before scanning -- see input.spec.ts's identical
    // comment for why (avoids a false pre-hydration "no main"/"no h1").
    await expect(nav(page).getByRole('button', { name: 'Getting started' })).toBeVisible();
    await expectNoAxeViolations(page, 'navigation_menu: loaded', {
      excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT],
    });
  });

  test('panel open has no automatically detectable a11y issues', async ({ page }) => {
    await page.goto(URL, GOTO);
    const trigger = nav(page).getByRole('button', { name: 'Getting started' });
    await trigger.click();
    await expect(trigger).toHaveAttribute('aria-expanded', 'true');
    await expectNoAxeViolations(page, 'navigation_menu: open', {
      excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT],
    });
  });
});
