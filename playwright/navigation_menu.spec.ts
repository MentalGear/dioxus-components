import { test, expect } from "./fixtures";
import { type Page } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from './axe';
import { BASE_URL } from './base-url';
import { gotoHydrated } from './hydration';

const URL = `${BASE_URL}/component/?name=navigation_menu&`;
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
  // The demo prefixes the router's base path (`/dioxus-components` on
  // Pages, empty on a root-served `dx serve`), so derive it from BASE_URL.
  const basePath = new globalThis.URL(BASE_URL).pathname.replace(/\/$/, '');
  await expect(docsLink).toHaveAttribute('href', `${basePath}/docs`);
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

test.describe('Panel layout matches shadcn (owner report: oversized panel, invisible featured card, mismatched list items)', () => {
  // Owner report (dark mode, "Getting started" open): the panel ran ~900px+
  // wide (a `width: max-content` box around `fr` tracks with no width
  // contract), the featured card was indistinguishable from the panel
  // (`--dx-muted` and `--dx-popover` are the same colour in dark), and
  // "Button"/"Input" -- bare-text links -- rendered at 16px/400 next to
  // "Introduction"'s 14px/500 title. Reference: shadcn's NavigationMenu demo,
  // shadcn-ui/ui@295a1f1 apps/v4/registry/new-york-v4/examples/
  // navigation-menu-demo.tsx (grid `md:w-[400px] lg:w-[500px]
  // lg:grid-cols-[.75fr_1fr]`, featured `from-muted/50 to-muted`, `ListItem`).
  const OPEN_PANEL = '.dx-navigation-menu-content[data-state="open"]';

  async function openPanel(page: Page, name: string) {
    const trigger = nav(page).getByRole('button', { name });
    await trigger.click();
    await expect(trigger).toHaveAttribute('aria-expanded', 'true');
    // Past the open animation (transform scale .98 -> 1), so rects are final.
    await expect
      .poll(() =>
        page.evaluate((sel) => {
          const el = document.querySelector(sel);
          return !!el && el.getAnimations().every((a) => a.playState === 'finished');
        }, OPEN_PANEL),
      )
      .toBe(true);
  }

  for (const scheme of ['light', 'dark'] as const) {
    test.describe(`${scheme} scheme, 1440px viewport`, () => {
      test.beforeEach(async ({ page }) => {
        await page.setViewportSize({ width: 1440, height: 900 });
        await page.emulateMedia({ colorScheme: scheme });
        // Hydrated, not just loaded: a panel opened before hydration attaches its listeners is read
        // mid-way (the featured title measured 16px/400, its pre-theme default, in ~10% of runs at
        // --workers=4); backlog row 109's class.
        await gotoHydrated(page, URL, GOTO);
        // The emulation must actually reach the theme's `--light`/`--dark`
        // toggle, or the dark half of this block silently re-tests light.
        expect(
          await page.evaluate(() => matchMedia('(prefers-color-scheme: dark)').matches),
        ).toBe(scheme === 'dark');
      });

      test('the panel is compact (<= 520px wide) and stays inside the viewport', async ({ page }) => {
        await openPanel(page, 'Getting started');
        const box = await page.evaluate((sel) => {
          const el = document.querySelector<HTMLElement>(sel)!;
          const r = el.getBoundingClientRect();
          return { width: el.offsetWidth, left: r.left, right: r.right, vw: innerWidth };
        }, OPEN_PANEL);
        // shadcn's grid is `lg:w-[500px]`; plus the panel's own padding.
        expect(box.width).toBeLessThanOrEqual(520);
        expect(box.left).toBeGreaterThanOrEqual(0);
        expect(box.right).toBeLessThanOrEqual(box.vw);
      });

      test('the Components panel is shadcn-sized too (lg:w-[600px], two columns)', async ({ page }) => {
        await openPanel(page, 'Components');
        const m = await page.evaluate((sel) => {
          const el = document.querySelector<HTMLElement>(sel)!;
          const grid = el.querySelector<HTMLElement>('.dx-navigation-menu-grid')!;
          return {
            width: el.offsetWidth,
            columns: getComputedStyle(grid).gridTemplateColumns.split(' ').length,
          };
        }, OPEN_PANEL);
        expect(m.width).toBeLessThanOrEqual(620);
        expect(m.columns).toBe(2);
      });

      test('the featured card is visibly a different surface from the panel', async ({ page }) => {
        await openPanel(page, 'Getting started');
        const m = await page.evaluate((sel) => {
          const panel = document.querySelector<HTMLElement>(sel)!;
          const card = panel.querySelector<HTMLElement>('.dx-navigation-menu-featured')!;
          // Normalise any CSS colour (rgb(), color(srgb ...), oklab(...)) to
          // sRGB bytes through a 1x1 canvas, so no colour-function syntax is
          // parsed by hand.
          const ctx = document.createElement('canvas').getContext('2d', { willReadFrequently: true })!;
          const rgba = (css: string) => {
            ctx.clearRect(0, 0, 1, 1);
            ctx.fillStyle = '#000';
            ctx.fillStyle = css;
            ctx.fillRect(0, 0, 1, 1);
            return Array.from(ctx.getImageData(0, 0, 1, 1).data);
          };
          const image = getComputedStyle(card).backgroundImage;
          const stops = image.match(/(?:rgba?|hsla?|hwb|lab|lch|oklab|oklch|color)\([^()]*\)/g) ?? [];
          return {
            image,
            // The gradient's last stop is where the card's text sits.
            cardBottom: stops.length ? rgba(stops[stops.length - 1]) : null,
            panel: rgba(getComputedStyle(panel).backgroundColor),
          };
        }, OPEN_PANEL);
        expect(m.image).toContain('linear-gradient');
        expect(m.cardBottom).not.toBeNull();
        const delta = Math.max(...m.cardBottom!.slice(0, 3).map((c, i) => Math.abs(c - m.panel[i])));
        // >= 6/255 on some channel: the card reads against the panel (shadcn's
        // muted on popover is 10 in light, ~15 in dark).
        expect(delta, `card ${m.cardBottom} vs panel ${m.panel}`).toBeGreaterThanOrEqual(6);
      });

      test('the featured copy sits at the top of the card, and its title stays on one line', async ({ page }) => {
        // Owner report: "the text [is] at the bottom" -- shadcn's `justify-end`
        // pinned the title to the foot of a card as tall as the three list rows.
        await openPanel(page, 'Getting started');
        const m = await page.evaluate((sel) => {
          const card = document.querySelector<HTMLElement>(`${sel} .dx-navigation-menu-featured`)!;
          const title = card.querySelector<HTMLElement>('.dx-navigation-menu-link-title')!;
          const t = getComputedStyle(title);
          return {
            titleOffset: title.getBoundingClientRect().top - card.getBoundingClientRect().top,
            titleHeight: title.getBoundingClientRect().height,
            titleSize: t.fontSize,
            titleWeight: t.fontWeight,
          };
        }, OPEN_PANEL);
        expect(m.titleOffset).toBeLessThanOrEqual(24);
        // text-lg line-height is 28px: one line, not two (a wrapped title is 56px).
        expect(m.titleHeight).toBeLessThanOrEqual(32);
        // text-lg font-medium
        expect([m.titleSize, m.titleWeight]).toEqual(['18px', '500']);
      });

      test('every list link has the same title size/weight and a same-sized description', async ({ page }) => {
        for (const name of ['Getting started', 'Components']) {
          await openPanel(page, name);
          const links = await page.evaluate((sel) => {
            const panel = document.querySelector(sel)!;
            return Array.from(panel.querySelectorAll<HTMLElement>('.dx-navigation-menu-link'))
              .filter((a) => !a.classList.contains('dx-navigation-menu-featured'))
              .map((a) => {
                // A bare-text link (no title wrapper) is measured as itself,
                // so it cannot hide behind a missing `-link-title`.
                const title = a.querySelector<HTMLElement>('.dx-navigation-menu-link-title') ?? a;
                const desc = a.querySelector<HTMLElement>('.dx-navigation-menu-link-description');
                const t = getComputedStyle(title);
                return {
                  text: a.textContent?.trim().slice(0, 24),
                  title: `${t.fontSize}/${t.fontWeight}`,
                  desc: desc ? getComputedStyle(desc).fontSize : null,
                };
              });
          }, OPEN_PANEL);
          expect(links.length, `${name}: list links found`).toBeGreaterThanOrEqual(3);
          expect(new Set(links.map((l) => l.title)).size, `${name} titles: ${JSON.stringify(links)}`).toBe(1);
          expect(links.every((l) => l.desc !== null), `${name}: every link has a description`).toBe(true);
          expect(new Set(links.map((l) => l.desc)).size, `${name} descriptions`).toBe(1);
          await page.keyboard.press('Escape');
          await expect(nav(page).getByRole('button', { name })).toHaveAttribute('aria-expanded', 'false');
        }
      });
    });
  }
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
