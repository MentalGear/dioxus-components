/**
 * Carousel `vertical` demo: every visible slide's Card must lie fully
 * inside its own slide box AND inside the viewport's clip rect, no two
 * Cards may overlap, and Previous/Next must never sit on top of a slide.
 *
 * Regression for the owner report "vertical slider css is messed up": the
 * demo gave the (content-box) `Card` `height: 100%` of the slide's usable
 * block size (119px) and the Card's own `padding: 24px 0` + 1px borders
 * were added on top, so each Card rendered 169px tall inside a 135px slide.
 * Card 2 overlapped card 1 and the bottom Card was cut off by the clip
 * viewport. See `preview/src/components/carousel/variants/vertical/mod.rs`.
 *
 * The demo also uses the default `gap` (16px), the same as the horizontal
 * demos. An earlier 4px gap was filled by the Card's own shadow and read as
 * no gap at all (owner report), so the space between adjacent visible
 * cards must be clearly visible (>= MIN_GAP) and equal to the horizontal
 * default's.
 *
 * Rects are read in-page in one `evaluate` (never per-element CDP round
 * trips) and re-sampled with `expect.poll` so a still-settling scroll snap
 * cannot flake the assertions.
 */

import { test, expect } from "./fixtures";
import { type Page } from "@playwright/test";
import { BASE_URL } from "./base-url";
import { gotoHydrated } from "./hydration";

const GOTO_OPTS = { timeout: 20 * 60 * 1000 };
const EPS = 0.75;
/** Smallest card-to-card space that reads as a gap (the default is 16px). */
const MIN_GAP = 12;

type Rect = { x: number; y: number; w: number; h: number };
type Geometry = {
  viewport: Rect;
  prev: Rect;
  next: Rect;
  slides: { slide: Rect; card: Rect }[];
};

async function readGeometry(page: Page): Promise<Geometry> {
  return page.evaluate(() => {
    const frame = document.querySelector("#component-preview-frame-vertical")!;
    const R = (e: Element): Rect => {
      const r = e.getBoundingClientRect();
      return { x: r.x, y: r.y, w: r.width, h: r.height };
    };
    const viewport = frame.querySelector('[data-slot="carousel-viewport"]')!;
    const slides = [...frame.querySelectorAll('[aria-roledescription="slide"]')];
    return {
      viewport: R(viewport),
      prev: R(frame.querySelector(".dx-carousel-previous")!),
      next: R(frame.querySelector(".dx-carousel-next")!),
      slides: slides.map((s) => ({ slide: R(s), card: R(s.firstElementChild!) })),
    };
  });
}

/**
 * Card-to-card space between adjacent, currently visible slides of one demo
 * along the scroll axis ("y" vertical, "x" horizontal), read in one `evaluate`.
 */
async function cardSpacing(page: Page, variant: string, axis: "x" | "y"): Promise<number[]> {
  return page.evaluate(
    ([variant, axis]) => {
      const frame = document.querySelector(`#component-preview-frame-${variant}`)!;
      const vp = frame.querySelector('[data-slot="carousel-viewport"]')!.getBoundingClientRect();
      const lo = axis === "x" ? vp.left : vp.top;
      const hi = axis === "x" ? vp.right : vp.bottom;
      const cards = [...frame.querySelectorAll('[aria-roledescription="slide"]')]
        .map((s) => s.firstElementChild!.getBoundingClientRect())
        .map((r) => (axis === "x" ? { a: r.left, b: r.right } : { a: r.top, b: r.bottom }))
        .filter((c) => c.a >= lo - 0.75 && c.b <= hi + 0.75)
        .sort((p, q) => p.a - q.a);
      return cards.slice(1).map((c, i) => c.a - cards[i].b);
    },
    [variant, axis] as const,
  );
}

const intersects = (a: Rect, b: Rect) =>
  a.x < b.x + b.w - EPS && b.x < a.x + a.w - EPS && a.y < b.y + b.h - EPS && b.y < a.y + a.h - EPS;

const inside = (inner: Rect, outer: Rect) =>
  inner.x >= outer.x - EPS &&
  inner.y >= outer.y - EPS &&
  inner.x + inner.w <= outer.x + outer.w + EPS &&
  inner.y + inner.h <= outer.y + outer.h + EPS;

/** Every problem found, as human-readable strings (empty = geometry is sound). */
function problems(g: Geometry): string[] {
  const out: string[] = [];
  // "Visible" = the slide's own box meaningfully overlaps the clip rect.
  const visible = g.slides.filter((s) => intersects(s.slide, g.viewport));
  if (visible.length < 2) out.push(`expected >= 2 visible slides, saw ${visible.length}`);
  visible.forEach((s, i) => {
    if (!inside(s.card, s.slide)) {
      out.push(`card ${i} ${JSON.stringify(s.card)} exceeds its slide ${JSON.stringify(s.slide)}`);
    }
    if (!inside(s.card, g.viewport)) {
      out.push(`card ${i} ${JSON.stringify(s.card)} is clipped by the viewport ${JSON.stringify(g.viewport)}`);
    }
  });
  for (let i = 0; i < visible.length; i++) {
    for (let j = i + 1; j < visible.length; j++) {
      if (intersects(visible[i].card, visible[j].card)) out.push(`cards ${i} and ${j} overlap`);
    }
  }
  const byY = [...visible].sort((a, b) => a.card.y - b.card.y);
  for (let i = 0; i + 1 < byY.length; i++) {
    const space = byY[i + 1].card.y - (byY[i].card.y + byY[i].card.h);
    if (space < MIN_GAP) out.push(`only ${space}px between cards ${i} and ${i + 1} (< ${MIN_GAP}px)`);
  }
  for (const [name, btn] of [["Previous", g.prev], ["Next", g.next]] as const) {
    for (const [i, s] of visible.entries()) {
      if (intersects(btn, s.card)) out.push(`${name} button overlaps visible card ${i}`);
    }
    if (intersects(btn, g.viewport)) out.push(`${name} button overlaps the slide viewport`);
  }
  return out;
}

for (const width of [1280, 390]) {
  test.describe(`Carousel vertical geometry @${width}px`, () => {
    test.use({ viewport: { width, height: 900 } });

    test("whole cards, inside their slide and the clip, no overlap, buttons clear", async ({ page }) => {
      await gotoHydrated(page, `${BASE_URL}/component/?name=carousel&variant=vertical&`, GOTO_OPTS);
      const frame = page.locator("#component-preview-frame-vertical");
      await frame.scrollIntoViewIfNeeded();

      await expect.poll(async () => problems(await readGeometry(page))).toEqual([]);

      // Page to the end (Next disables at the last snap point): the last
      // slides must be whole too -- the bottom card was the one cut off by
      // the clip in the broken layout.
      const next = frame.getByRole("button", { name: "Next slide" });
      for (let i = 0; i < 4 && (await next.isEnabled()); i++) {
        await next.click();
        await page.waitForTimeout(400);
      }
      await expect(next).toBeDisabled();
      await expect
        .poll(async () => {
          const g = await readGeometry(page);
          const last = g.slides[g.slides.length - 1];
          // settled once the last slide has snapped inside the clip
          return inside(last.card, g.viewport) ? problems(g) : ["not settled"];
        })
        .toEqual([]);
    });

    test("vertical gap equals the horizontal default (the `sizes` demo)", async ({ page }) => {
      await gotoHydrated(page, `${BASE_URL}/component/?name=carousel&variant=vertical&`, GOTO_OPTS);
      await page.locator("#component-preview-frame-vertical").scrollIntoViewIfNeeded();
      await expect.poll(async () => (await cardSpacing(page, "vertical", "y")).length).toBeGreaterThan(0);
      const vertical = (await cardSpacing(page, "vertical", "y"))[0];

      await gotoHydrated(page, `${BASE_URL}/component/?name=carousel&variant=sizes&`, GOTO_OPTS);
      await page.locator("#component-preview-frame-sizes").scrollIntoViewIfNeeded();
      await expect.poll(async () => (await cardSpacing(page, "sizes", "x")).length).toBeGreaterThan(0);
      const horizontal = (await cardSpacing(page, "sizes", "x"))[0];

      expect(horizontal).toBeGreaterThanOrEqual(MIN_GAP);
      expect(Math.abs(vertical - horizontal)).toBeLessThanOrEqual(1);
    });
  });
}
