/**
 * Carousel demo geometry: the `indicators` and `api` demos' own layout.
 *
 * Owner-reported on the deployed build: (1) the `indicators` demo's dots
 * rendered ABOVE the slides, squeezed against (and partly hidden by) the
 * viewport's top edge, because the APG tabbed pattern puts the tablist
 * BEFORE the slides in DOM order (tier-1 oracle R9) and nothing reordered it
 * visually; (2) the `api` demo's slide was a 64x83 sliver and its caption
 * wrapped to "Slide 1 / of 5", because the wrapper was 10rem wide before
 * `.dx-carousel`'s own 6rem `padding-inline` reservation came out of it.
 *
 * Geometry is read in-page (`getBoundingClientRect`), at desktop and phone
 * widths, in light and dark, so a regression in any of those four corners is
 * caught -- not just the one a human happened to look at.
 */

import { test, expect } from "./fixtures";
import { type Page } from "@playwright/test";
import { BASE_URL } from "./base-url";
import { gotoHydrated } from "./hydration";

const GOTO_OPTS = { timeout: 20 * 60 * 1000 };

const VIEWPORTS = [
  { name: "1280", width: 1280, height: 900 },
  { name: "390", width: 390, height: 844 },
] as const;
const SCHEMES = ["light", "dark"] as const;

async function open(page: Page, variant: "indicators" | "api") {
  await gotoHydrated(
    page,
    `${BASE_URL}/component/?name=carousel&variant=${variant}&`,
    GOTO_OPTS,
  );
  await page.locator(`#component-preview-frame-${variant} .dx-carousel`).first().scrollIntoViewIfNeeded();
}

for (const vp of VIEWPORTS) {
  for (const scheme of SCHEMES) {
    test.describe(`carousel demos @ ${vp.name} ${scheme}`, () => {
      test.use({ viewport: { width: vp.width, height: vp.height }, colorScheme: scheme });

      test("indicators: dots sit below the slides, inside the frame, tablist still first in DOM", async ({
        page,
      }) => {
        await open(page, "indicators");
        const m = await page.evaluate(() => {
          const frame = document.querySelector("#component-preview-frame-indicators")!;
          const root = frame.querySelector(".dx-carousel")!;
          const tablist = root.querySelector('[role="tablist"]')!;
          const viewport = root.querySelector(".dx-carousel-content")!;
          const first = (root.querySelector('[role="tabpanel"]') ?? viewport)!;
          const r = (el: Element) => el.getBoundingClientRect();
          return {
            tablist: r(tablist).toJSON(),
            viewport: r(viewport).toJSON(),
            root: r(root).toJSON(),
            frame: r(frame).toJSON(),
            tabs: [...tablist.querySelectorAll('[role="tab"]')].map((t) => r(t).toJSON()),
            tablistFirstInDom: !!(
              tablist.compareDocumentPosition(first) & Node.DOCUMENT_POSITION_FOLLOWING
            ),
          };
        });

        expect(m.tablistFirstInDom, "APG/R9: tablist precedes the slides in DOM order").toBe(true);
        // Visually below the slides, with real breathing room.
        expect(m.tablist.top - m.viewport.bottom).toBeGreaterThanOrEqual(8);
        // Inside the frame and the carousel root; no dot is clipped.
        expect(m.tablist.bottom).toBeLessThanOrEqual(m.root.bottom + 0.5);
        expect(m.tablist.bottom).toBeLessThanOrEqual(m.frame.bottom + 0.5);
        for (const t of m.tabs) {
          expect(t.left).toBeGreaterThanOrEqual(m.root.left - 0.5);
          expect(t.right).toBeLessThanOrEqual(m.root.right + 0.5);
          expect(t.height).toBeGreaterThan(0);
        }
        // Centred under the carousel (the active pill is wider than the other
        // dots, so compare the dots' overall extent, not individual centres;
        // the root, not the scroller, is the symmetric reference -- the
        // scroller's own box is offset by the slides' leading gap padding).
        const tabsMid = (Math.min(...m.tabs.map((t) => t.left)) + Math.max(...m.tabs.map((t) => t.right))) / 2;
        const rootMid = m.root.left + m.root.width / 2;
        expect(Math.abs(tabsMid - rootMid)).toBeLessThanOrEqual(1.5);
      });

      test("api: slide card is square and not tiny, caption is one centred line", async ({ page }) => {
        await open(page, "api");
        const m = await page.evaluate(() => {
          const frame = document.querySelector("#component-preview-frame-api")!;
          const root = frame.querySelector(".dx-carousel")!;
          const card = root.querySelector(".dx-card")!;
          const viewport = root.querySelector(".dx-carousel-content")!;
          const caption = [...root.querySelectorAll("p")].find((p) => /^Slide \d+ of \d+$/.test(p.textContent ?? ""))!;
          const r = (el: Element) => el.getBoundingClientRect().toJSON();
          const range = document.createRange();
          range.selectNodeContents(caption);
          const tr = range.getBoundingClientRect();
          return {
            card: r(card),
            root: r(root),
            captionTextMid: tr.left + tr.width / 2,
            captionTextHeight: tr.height,
            viewport: r(viewport),
            caption: caption ? r(caption) : null,
            captionText: caption?.textContent ?? null,
            lineHeight: caption ? parseFloat(getComputedStyle(caption).lineHeight) : NaN,
            fontSize: caption ? parseFloat(getComputedStyle(caption).fontSize) : NaN,
          };
        });

        expect(m.card.width).toBeGreaterThanOrEqual(200);
        expect(Math.abs(m.card.width - m.card.height)).toBeLessThanOrEqual(1);
        expect(m.captionText).toBe("Slide 1 of 5");
        // Single line: its height is one line-height (normal line-height
        // resolves to NaN above -> fall back to ~1.5 x font-size).
        const line = Number.isNaN(m.lineHeight) ? m.fontSize * 1.5 : m.lineHeight;
        // (the text's own Range box, so the caption's block padding is not counted).
        expect(m.captionTextHeight).toBeLessThanOrEqual(line + 2);
        // Horizontally centred under the carousel (text-align centre inside a
        // full-width block, so compare the root's centre with the caption's
        // own text centre via a Range, not the block's box).
        expect(Math.abs(m.captionTextMid - (m.root.left + m.root.width / 2))).toBeLessThanOrEqual(1.5);
        // And below the slides.
        expect(m.caption!.top).toBeGreaterThanOrEqual(m.viewport.bottom);
      });
    });
  }
}
