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

/**
 * Class-level guards for the two defects reported on the deployed
 * `indicators`/`api` and `autoplay` demos (dark theme, carousel page):
 *
 *  1. Demo content must never extend past its preview frame, on ANY carousel
 *     demo (`.dx-component-preview-frame` is a plain flex box with a
 *     `min-height` and no clipping; a regression that gives it a fixed
 *     height / `overflow: hidden`, or lets a demo's own layout outgrow it,
 *     shows up here as content past the frame's border box or a clipping
 *     ancestor). Checked at both viewport widths, light and dark, for every
 *     frame the page renders -- not only the two the owner happened to see.
 *  2. The custom dot picker's dots must all keep the same shape/radius
 *     before AND after a state change. The shipped defect (square dots after
 *     the first click) was dioxus-interpreter-js 0.7.9 stripping `var()`
 *     shorthands (`border-radius`, `background`) out of an inline `style`
 *     the first time it was re-set, so the assertion that matters is the one
 *     made AFTER a click, not the initial render.
 */
const ALL_CAROUSEL_URL = `${BASE_URL}/component/carousel/`;

for (const vp of VIEWPORTS) {
  for (const scheme of SCHEMES) {
    test.describe(`carousel demo frames @ ${vp.name} ${scheme}`, () => {
      test.use({ viewport: { width: vp.width, height: vp.height }, colorScheme: scheme });

      test("every carousel demo fits inside its preview frame and no ancestor clips it", async ({ page }) => {
        await gotoHydrated(page, ALL_CAROUSEL_URL, GOTO_OPTS);
        const result = await page.evaluate(() => {
          const frames = [...document.querySelectorAll<HTMLElement>('[id^="component-preview-frame"]')].filter(
            (f) => f.querySelector(".dx-carousel"),
          );
          const problems: string[] = [];
          for (const frame of frames) {
            const fr = frame.getBoundingClientRect();
            const cs = getComputedStyle(frame);
            for (const el of frame.querySelectorAll<HTMLElement>("*")) {
              // Slides live inside a scroller whose overflow is the point of a carousel.
              if (el.closest(".dx-carousel-content") && !el.classList.contains("dx-carousel-content")) continue;
              if (el.closest("svg") && el.tagName.toLowerCase() !== "svg") continue;
              const r = el.getBoundingClientRect();
              if (r.width === 0 || r.height === 0) continue;
              const style = getComputedStyle(el);
              if (style.position === "fixed" || style.visibility === "hidden") continue;
              if (r.bottom > fr.bottom + 0.5 || r.top < fr.top - 0.5 || r.right > fr.right + 0.5 || r.left < fr.left - 0.5) {
                problems.push(
                  `${frame.id}: <${el.tagName.toLowerCase()} class="${el.className}"> box ` +
                    `[${r.left.toFixed(1)},${r.top.toFixed(1)},${r.right.toFixed(1)},${r.bottom.toFixed(1)}] ` +
                    `exceeds frame [${fr.left.toFixed(1)},${fr.top.toFixed(1)},${fr.right.toFixed(1)},${fr.bottom.toFixed(1)}]`,
                );
              }
            }
            if (frame.scrollHeight > frame.clientHeight + 1) {
              problems.push(`${frame.id}: scrollHeight ${frame.scrollHeight} > clientHeight ${frame.clientHeight}`);
            }
            if (frame.scrollWidth > frame.clientWidth + 1) {
              problems.push(`${frame.id}: scrollWidth ${frame.scrollWidth} > clientWidth ${frame.clientWidth}`);
            }
            if (cs.overflowY !== "visible") problems.push(`${frame.id}: overflow-y is ${cs.overflowY}`);
            for (let a = frame.parentElement; a && a !== document.documentElement; a = a.parentElement) {
              const o = getComputedStyle(a);
              if (o.overflowY !== "visible" || o.overflowX === "hidden") {
                problems.push(
                  `${frame.id}: ancestor <${a.tagName.toLowerCase()} class="${a.className}"> clips (overflow ${o.overflowX}/${o.overflowY})`,
                );
              }
            }
          }
          return { count: frames.length, problems };
        });
        // The page ships every carousel demo; a silent "found none" would pass vacuously.
        expect(result.count).toBeGreaterThanOrEqual(10);
        expect(result.problems, result.problems.join("\n")).toEqual([]);
      });

      test("autoplay: the rotation control sits inside the track's top-start corner, not above the slide", async ({
        page,
      }) => {
        await gotoHydrated(page, ALL_CAROUSEL_URL, GOTO_OPTS);
        const frame = page.locator("#component-preview-frame-autoplay");
        await frame.scrollIntoViewIfNeeded();
        const m = await frame.evaluate((f) => {
          const btn = f.querySelector(".dx-carousel-rotation-control")!.getBoundingClientRect();
          const track = f.querySelector(".dx-carousel-content")!.getBoundingClientRect();
          const root = f.querySelector(".dx-carousel")!.getBoundingClientRect();
          return { btn: btn.toJSON(), track: track.toJSON(), root: root.toJSON() };
        });
        // Inside the slide area on both axes (nothing above the track)...
        expect(m.btn.top).toBeGreaterThanOrEqual(m.track.top);
        expect(m.btn.bottom).toBeLessThanOrEqual(m.track.bottom);
        expect(m.btn.left).toBeGreaterThanOrEqual(m.track.left - 0.5);
        expect(m.btn.right).toBeLessThanOrEqual(m.track.right);
        // ...at the top-start corner, with a real inset from the card edge.
        expect(m.btn.top - m.track.top).toBeGreaterThanOrEqual(4);
        expect(m.btn.left - m.track.left).toBeGreaterThanOrEqual(4);
        expect(m.btn.top - m.track.top).toBeLessThanOrEqual(16);
        // The carousel root is no taller than the slides: no control row above them.
        expect(m.track.top - m.root.top).toBeLessThanOrEqual(1);
      });

      test("api: every custom-picker dot keeps the same shape and radius, before and after paging", async ({ page }) => {
        await gotoHydrated(page, ALL_CAROUSEL_URL, GOTO_OPTS);
        const frame = page.locator("#component-preview-frame-api");
        await frame.scrollIntoViewIfNeeded();
        const dots = frame.locator('button[aria-label^="Go to slide"]');
        await expect(dots).toHaveCount(5);

        const read = () =>
          dots.evaluateAll((els) =>
            els.map((b) => {
              const cs = getComputedStyle(b);
              const r = b.getBoundingClientRect();
              return {
                w: r.width,
                h: r.height,
                radius: cs.borderTopLeftRadius,
                active: b.getAttribute("data-active"),
                bg: cs.backgroundColor,
              };
            }),
          );
        const assertUniform = (rows: Awaited<ReturnType<typeof read>>, label: string) => {
          expect(rows.filter((r) => r.active === "true"), `${label}: exactly one active dot`).toHaveLength(1);
          for (const r of rows) {
            expect(r.w, `${label}: width == height`).toBeCloseTo(r.h, 1);
            expect(r.radius, `${label}: same radius on every dot`).toBe(rows[0].radius);
            expect(parseFloat(r.radius), `${label}: rounded, not 0`).toBeGreaterThan(0);
            expect(r.bg, `${label}: has a painted background`).not.toBe("rgba(0, 0, 0, 0)");
          }
          const inactive = rows.filter((r) => r.active !== "true");
          expect(new Set(inactive.map((r) => r.bg)).size, `${label}: inactive dots share one colour`).toBe(1);
          expect(rows.find((r) => r.active === "true")!.bg, `${label}: active differs from inactive`).not.toBe(
            inactive[0].bg,
          );
        };

        assertUniform(await read(), "initial");
        await frame.locator(".dx-carousel-next").click();
        await frame.locator(".dx-carousel-next").click();
        await expect(frame.getByText("Slide 3 of 5")).toBeVisible();
        await expect(dots.nth(2)).toHaveAttribute("data-active", "true");
        assertUniform(await read(), "after two Next clicks");
        await dots.nth(0).click();
        await expect(dots.nth(0)).toHaveAttribute("data-active", "true");
        assertUniform(await read(), "after picking dot 1");
      });

      test("indicators: tab dots share one height and radius (the active pill is the only wider one) after paging", async ({
        page,
      }) => {
        await gotoHydrated(page, ALL_CAROUSEL_URL, GOTO_OPTS);
        const frame = page.locator("#component-preview-frame-indicators");
        await frame.scrollIntoViewIfNeeded();
        const tabs = frame.locator('[role="tab"]');
        await expect(tabs).toHaveCount(5);
        await tabs.nth(2).click();
        await expect(tabs.nth(2)).toHaveAttribute("aria-selected", "true");
        const readRows = () =>
          tabs.evaluateAll((els) =>
            els.map((b) => {
              const cs = getComputedStyle(b);
              const r = b.getBoundingClientRect();
              return { w: r.width, h: r.height, radius: cs.borderTopLeftRadius, selected: b.getAttribute("aria-selected") };
            }),
          );
        // The previously-selected pill is still narrowing (width transition) right after the click.
        await expect
          .poll(async () => new Set((await readRows()).filter((r) => r.selected !== "true").map((r) => r.w)).size)
          .toBe(1);
        const rows = await readRows();
        for (const r of rows) {
          expect(r.h).toBeCloseTo(rows[0].h, 1);
          expect(r.radius).toBe(rows[0].radius);
          expect(parseFloat(r.radius)).toBeGreaterThan(0);
        }
        expect(new Set(rows.filter((r) => r.selected !== "true").map((r) => r.w)).size).toBe(1);
      });
    });
  }
}
