/**
 * ORACLE: tier 2 (HTML) -- a same-page section anchor lands where the page's own CSS says it must,
 * on a page whose cards are `content-visibility: auto`.
 *
 * Source: dev-docs/research/scroll-jank-2026-10-04.md section 7.4 ("Cost NOT fixed: first-visit
 * programmatic jumps land off-target") and section 8 (the construction). Rule sources:
 *   - HTML Standard, "scroll to the fragment" (navigate to a fragment): the element whose id is the
 *     fragment is scrolled into view with block `start`, which honours the scroll container's
 *     `scroll-padding` (the sticky navbar, `html { scroll-padding-top: var(--dx-navbar-height) }`), on
 *     every fragment navigation -- a link click, a fresh load of `/#id`, a back/forward to `#id`:
 *       https://html.spec.whatwg.org/multipage/browsing-the-web.html#scroll-to-the-fragment-identifier
 *   - CSS Containment Level 2, `content-visibility: auto`: a skipped box lays out at its
 *     `contain-intrinsic-size` (an estimate until it has been rendered once), then resizes when it is
 *     rendered, so the geometry a fragment scroll was computed against is not the final geometry:
 *       https://www.w3.org/TR/css-contain-2/#content-visibility
 *       https://www.w3.org/TR/css-sizing-4/#intrinsic-size-override
 *   - CSS Scroll Snap Level 1, `scroll-padding` (where a scrolled-to box must land):
 *       https://www.w3.org/TR/css-scroll-snap-1/#scroll-padding
 *
 * What it asserts: for EVERY same-page link in the docs sidebar (read from the DOM, not listed here, so a
 * new section is covered the day it is added) on Home (`/`: "Sample interfaces", "All components"; the
 * masonry of 14 blocks above "All components" is skipped on first visit) and Overview (`/docs`), the target
 * heading ends up where the page's scroll padding puts it (`scrollY` == `min(maxScroll, targetTop - pad)`,
 * +-2 px) once the page has settled (two rAFs and no scroll or layout change for 300 ms):
 *   1. after clicking the link (smooth scroll, as a user gets it, and instant),
 *   2. after a fresh load of `<page>#<id>`,
 *   3. after browser Back and Forward across them (`history.scrollRestoration` is `manual` under the
 *      Dioxus router, so the browser itself scrolls nowhere on a hash traversal),
 * every case on a first visit (a fresh page load per anchor, so no card has been rendered before and none
 * has a remembered size), and the forced-render window that makes this exact ends: `data-cv-settle` is
 * gone from `<html>` and the cards are skipped again.
 *
 * Chromium only. The fixtures' `scroll-behavior: auto !important` (see ../../fixtures.ts) is lifted after
 * load for the click cases so they run with the site's real `scroll-behavior: smooth`. Run:
 *   PLAYWRIGHT_BASE_URL=http://127.0.0.1:8083 npx playwright test --config=oracle.local.config.ts \
 *     ./oracle/tier2-html/anchor-landing.spec.ts
 *
 * Red proof (2026-10-04), the `dx serve` build on :8083 before the construction (cv on, no handler): a
 * fresh load of `/#id` does not scroll at all (client-rendered: the target is not in the DOM when the browser
 * looks for it), Back/Forward to a hash does not scroll (stays where it was), and clicking "All components"
 * lands ~35 px low. After: all green. Numbers: the research note's section 8.
 */
import { test, expect } from "../../fixtures";
import type { Page } from "@playwright/test";
import { BASE_URL } from "../../base-url";

const TOLERANCE_PX = 2;
const LOAD = { timeout: 20 * 60 * 1000 } as const;
/** A dev (debug wasm) page load is several seconds, and a case loads the page once per anchor. */
const SLOW_TEST_MS = 5 * 60 * 1000;

const PAGES = [
  { name: "Home", path: "/", ready: ".dx-component-card", min: 60, anchors: 2 },
  { name: "Overview", path: "/docs", ready: ".dx-docs-page", min: 1, anchors: 3 },
] as const;
type PageSpec = (typeof PAGES)[number];

async function waitReady(page: Page, p: PageSpec) {
  await page.waitForFunction(([sel, min]) => document.querySelectorAll(sel).length >= min, [p.ready, p.min] as const, { timeout: 5 * 60 * 1000 });
  // Hydration finished enough that the sidebar's links exist.
  await page.waitForSelector('[data-slot="sidebar"] a[href^="#"]:not([href="#"])', { state: "attached", timeout: 5 * 60 * 1000 });
  await page.waitForTimeout(800);
}

async function open(page: Page, p: PageSpec, hash = "") {
  await page.goto("about:blank"); // a real navigation every time, never a same-document hash jump
  await page.goto(`${BASE_URL}${p.path}${hash}`, LOAD);
  await waitReady(page, p);
}

/** The ids the sidebar links to on this page (read from the DOM: a new section is covered automatically). */
async function sectionIds(page: Page): Promise<string[]> {
  return page.evaluate(() =>
    [...document.querySelectorAll<HTMLAnchorElement>('[data-slot="sidebar"] a[href^="#"]')].map((a) => a.getAttribute("href")!.slice(1)).filter(Boolean),
  );
}

/** Real users get `html { scroll-behavior: smooth }`; the fixtures force `auto`. Restore either for the page. */
async function scrollBehavior(page: Page, behavior: "smooth" | "auto") {
  await page.evaluate((b) => document.documentElement.style.setProperty("scroll-behavior", b, "important"), behavior);
}

async function clickSectionLink(page: Page, id: string) {
  const collapsed = await page.evaluate(() => document.querySelector('[data-slot="sidebar"]')?.getAttribute("data-state") === "collapsed");
  if (collapsed) {
    await page.locator('[data-slot="sidebar-trigger"]').first().click();
    await page.waitForTimeout(500);
  }
  await page.locator(`[data-slot="sidebar"] a[href="#${id}"]`).first().click();
}

interface Landing {
  scrollY: number;
  want: number;
  top: number;
  pad: number;
  max: number;
  err: number;
  waited: number;
}

/** Wait until two rAFs have run and neither scrollY, the target's top nor the page height moved for 300 ms. */
async function settle(page: Page, id: string): Promise<Landing> {
  return page.evaluate(
    (id) =>
      new Promise<Landing>((resolve) => {
        const sample = () => {
          const t = document.getElementById(id);
          const top = t ? t.getBoundingClientRect().top : NaN;
          return [Math.round(scrollY * 10) / 10, Math.round(top * 10) / 10, document.documentElement.scrollHeight] as const;
        };
        let last = sample();
        const start = performance.now();
        let since = start;
        requestAnimationFrame(() =>
          requestAnimationFrame(function tick() {
            const s = sample();
            const now = performance.now();
            if (s.join() !== last.join()) {
              last = s;
              since = now;
            }
            if (now - since >= 300 || now - start > 15_000) {
              const pad = parseFloat(getComputedStyle(document.documentElement).scrollPaddingTop) || 0;
              const max = document.documentElement.scrollHeight - innerHeight;
              const t = document.getElementById(id)!;
              const abs = t.getBoundingClientRect().top + scrollY;
              const want = Math.min(max, Math.max(0, abs - pad));
              return resolve({ scrollY, want, top: t.getBoundingClientRect().top, pad, max, err: scrollY - want, waited: Math.round(now - start) });
            }
            requestAnimationFrame(tick);
          }),
        );
      }),
    id,
  );
}

function expectLanded(l: Landing, what: string) {
  expect(
    Math.abs(l.err),
    `${what}: scrollY ${l.scrollY.toFixed(1)}, want ${l.want.toFixed(1)} (target top ${l.top.toFixed(1)} px, padding ${l.pad} px, max scroll ${l.max.toFixed(0)}): off by ${l.err.toFixed(1)} px`,
  ).toBeLessThanOrEqual(TOLERANCE_PX);
}

for (const p of PAGES) {
  test.describe(`${p.name} (${p.path}): section anchors land on target`, () => {
    test("the sidebar lists the section anchors (the cases below are not vacuous)", async ({ page }) => {
      await open(page, p);
      expect((await sectionIds(page)).length).toBeGreaterThanOrEqual(p.anchors);
    });

    for (const behavior of ["smooth", "auto"] as const) {
      test(`1. click, first visit, scroll-behavior ${behavior}`, async ({ page }) => {
        test.setTimeout(SLOW_TEST_MS);
        let ids: string[] = [];
        for (let i = 0; i === 0 || i < ids.length; i++) {
          await open(page, p); // a first visit every time: no card has been rendered, none has a remembered size
          if (i === 0) ids = await sectionIds(page);
          await scrollBehavior(page, behavior);
          await clickSectionLink(page, ids[i]);
          expectLanded(await settle(page, ids[i]), `click #${ids[i]}`);
        }
      });
    }

    test("2. fresh load of <page>#id", async ({ page }) => {
      test.setTimeout(SLOW_TEST_MS);
      await open(page, p);
      const ids = await sectionIds(page);
      for (const id of ids) {
        await open(page, p, `#${id}`);
        expectLanded(await settle(page, id), `fresh load ${p.path}#${id}`);
      }
    });

    test("3. back and forward across the anchors", async ({ page }) => {
      test.setTimeout(SLOW_TEST_MS);
      await open(page, p);
      const ids = await sectionIds(page);
      await scrollBehavior(page, "smooth");
      for (const id of ids) {
        await clickSectionLink(page, id);
        expectLanded(await settle(page, id), `click #${id}`);
      }
      // Back through every earlier anchor (the last one is where we are), then forward again.
      for (const id of ids.slice(0, -1).reverse()) {
        await page.goBack();
        expectLanded(await settle(page, id), `back to #${id}`);
      }
      for (const id of ids.slice(1)) {
        await page.goForward();
        expectLanded(await settle(page, id), `forward to #${id}`);
      }
    });

    test("4. the forced-render window ends: data-cv-settle is gone and the cards are skipped again", async ({ page }) => {
      await open(page, p);
      const id = (await sectionIds(page)).at(-1)!;
      await scrollBehavior(page, "smooth");
      await clickSectionLink(page, id);
      await settle(page, id);
      await expect.poll(() => page.evaluate(() => document.documentElement.hasAttribute("data-cv-settle")), { timeout: 8000, message: "html[data-cv-settle] is removed" }).toBe(false);
      if (p.path === "/") {
        // The gallery below "All components" is skipped again (the perf win is not traded away).
        const r = await page.evaluate(() => {
          const prev = [...document.querySelectorAll<HTMLElement>(".dx-component-card-preview")];
          return { n: prev.length, skipped: prev.filter((e) => !e.checkVisibility({ contentVisibilityAuto: true })).length };
        });
        expect(r.skipped, `of ${r.n} gallery cards, ${r.skipped} are skipped after the anchor landed`).toBeGreaterThanOrEqual(Math.floor(r.n * 0.5));
      }
    });
  });
}
