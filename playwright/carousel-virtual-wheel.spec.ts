/**
 * Carousel: trackpad/wheel scrolling ends cleanly on the virtualised
 * (`CarouselVirtualContent`) demos -- the owner's real-trackpad report
 * (MacBook, Firefox 156): "trackpad custom scroll end snap interferes with
 * looping/virtual slider".
 *
 * Two root causes, one test group each (see `primitives/src/carousel.rs`,
 * `CAROUSEL_SCROLL_TRACKING_JS`'s "Browser-idle gate" and
 * `item_basis_style`'s own docs):
 *
 * 1. OVERSIZED SNAP AREAS. The gap model puts each slide's gap inside its
 *    own box as `padding-inline-start` next to `flex-basis: 100%`, which is
 *    only a whole slide under `box-sizing: border-box`. That box-sizing came
 *    only from the themed `.dx-carousel-item` class, which the virtual
 *    slides (rendered by the primitive itself) never carry -- so each one
 *    was one gap (16px) wider than the snapport. A snap area larger than
 *    the snapport makes every position inside it a valid rest position, so
 *    the browser stopped snapping and the component's own re-centre then
 *    "snapped" the leftover 16px itself at scroll end, as an instant
 *    `scrollBy`.
 * 2. SCROLL-END REACTIONS FIRED BEFORE THE BROWSER WAS IDLE. The settle ran
 *    on the first `scrollend`, even with wheel input still arriving, and
 *    everything downstream of it (the `selected` update, the virtual
 *    window's re-anchor and re-render, the instant re-align `scrollBy` with
 *    snapping suspended) ran mid-gesture -- writing the scroll position
 *    while the browser was still scrolling (THE RULE, `carousel.rs` module
 *    doc).
 *
 * METHOD. A probe installed before the app boots wraps every scroll write
 * (`scrollBy`/`scrollTo`/`scroll`/`scrollIntoView` and the
 * `scrollLeft`/`scrollTop` setters) on `.dx-carousel-content` elements, so
 * each test can list exactly which writes the component made and when. All
 * timing, geometry and window state is sampled IN the page on every
 * animation frame (dev-docs/backlog.md row 112): nothing animated is read
 * across a CDP round trip. Trusted input comes from `page.mouse.wheel`
 * (these events really scroll); where a test needs "input is still
 * arriving" at a precise moment, it adds in-page synthetic `WheelEvent`s,
 * which never scroll (untrusted) but are exactly what the component's input
 * tracking sees. Headless Chromium cannot reproduce a real trackpad's
 * compositor momentum or Firefox's own scroll-end timing; these tests pin
 * the ordering contract, and the feel remains the owner's real-device
 * check.
 */

import { test, expect } from "./fixtures";
import type { Locator, Page } from "@playwright/test";
import { BASE_URL } from "./base-url";
import { gotoHydrated } from "./hydration";

type Variant = "virtual_loop" | "virtual_loop_rtl" | "virtual_many" | "main" | "rewind";

const GOTO_OPTS = { timeout: 20 * 60 * 1000 };

/** Installed before any page script runs: records the component's scroll writes. */
function installScrollWriteProbe(): void {
  const w = window as unknown as { __dxScrollWrites: { t: number; id: string; m: string; args: string }[] };
  w.__dxScrollWrites = [];
  const isScroller = (el: unknown): el is HTMLElement =>
    el instanceof HTMLElement && el.classList.contains("dx-carousel-content");
  for (const name of ["scrollBy", "scrollTo", "scroll", "scrollIntoView"] as const) {
    const orig = Element.prototype[name] as (...a: unknown[]) => void;
    (Element.prototype as unknown as Record<string, unknown>)[name] = function (this: Element, ...args: unknown[]) {
      if (isScroller(this)) w.__dxScrollWrites.push({ t: performance.now(), id: this.id, m: name, args: JSON.stringify(args) });
      return orig.apply(this, args);
    };
  }
  for (const prop of ["scrollLeft", "scrollTop"] as const) {
    const d = Object.getOwnPropertyDescriptor(Element.prototype, prop)!;
    Object.defineProperty(Element.prototype, prop, {
      configurable: true,
      get: d.get,
      set(this: Element, v: number) {
        if (isScroller(this)) w.__dxScrollWrites.push({ t: performance.now(), id: this.id, m: `${prop}=`, args: String(v) });
        d.set!.call(this, v);
      },
    });
  }
}

/** Demos that carry `CarouselAutoplay` -- rotation is switched off for these tests (below). */
const AUTOPLAY: Variant[] = ["virtual_loop", "virtual_loop_rtl"];

async function open(page: Page, variant: Variant): Promise<{ frame: Locator; content: Locator }> {
  await page.addInitScript(installScrollWriteProbe);
  // Autoplay would move the carousel on its own schedule. `CarouselAutoplay`
  // never starts under reduced motion (carousel.spec.ts's own autoplay
  // tests), which takes it out deterministically; nothing on the wheel/
  // scroll-end path under test depends on that preference (the re-align
  // is always instant, and the wheel band is off on these demos anyway).
  if (AUTOPLAY.includes(variant)) await page.emulateMedia({ reducedMotion: "reduce" });
  await gotoHydrated(page, `${BASE_URL}/component/?name=carousel&variant=${variant}&`, GOTO_OPTS);
  const frame = page.locator(variant === "main" ? "#component-preview-frame" : `#component-preview-frame-${variant}`);
  const content = frame.locator(".dx-carousel-content").first();
  await content.scrollIntoViewIfNeeded();
  // Park the pointer on the track: real wheel input targets what is under
  // it, and hovering also pauses the demos' autoplay (so nothing but the
  // input under test moves the carousel).
  const box = await content.boundingBox();
  if (!box) throw new Error("carousel content has no bounding box");
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  // At rest before any input: the position holds still for 400ms.
  await content.evaluate(async (node) => {
    const el = node as HTMLElement;
    let last = el.scrollLeft;
    let since = performance.now();
    while (performance.now() - since < 400) {
      await new Promise((r) => requestAnimationFrame(r));
      if (el.scrollLeft !== last) {
        last = el.scrollLeft;
        since = performance.now();
      }
    }
  });
  return { frame, content };
}

/** |distance| from the scroller's start edge to the nearest slide's start edge, px. */
async function restingOffset(content: Locator): Promise<number> {
  return content.evaluate((node) => {
    const el = node as HTMLElement;
    const c = el.getBoundingClientRect();
    const rtl = getComputedStyle(el).direction === "rtl";
    let best = Infinity;
    for (const child of Array.from(el.children)) {
      const r = child.getBoundingClientRect();
      best = Math.min(best, Math.abs(rtl ? r.right - c.right : r.left - c.left));
    }
    return best;
  });
}

type Frame = { t: number; pos: number; sel: string | null; aligned: string | null; off: number; win: string };
type Recording = {
  frames: Frame[];
  wheels: number[];
  scrollEnds: number[];
  windowChanges: number[];
  writes: { t: number; m: string; args: string }[];
};

/** Start the in-page recorder on this scroller (every frame, plus events). */
async function startRecording(content: Locator): Promise<void> {
  await content.evaluate((node) => {
    const el = node as HTMLElement;
    const rtl = getComputedStyle(el).direction === "rtl";
    const rec = {
      frames: [] as unknown[],
      wheels: [] as number[],
      scrollEnds: [] as number[],
      windowChanges: [] as number[],
      running: true,
      mo: null as MutationObserver | null,
      t0: performance.now(),
    };
    (window as unknown as { __dxRec: typeof rec }).__dxRec = rec;
    el.addEventListener("wheel", () => rec.wheels.push(performance.now()), { passive: true });
    el.addEventListener("scrollend", () => rec.scrollEnds.push(performance.now()), { passive: true });
    rec.mo = new MutationObserver(() => rec.windowChanges.push(performance.now()));
    rec.mo.observe(el, { childList: true });
    const tick = () => {
      const c = el.getBoundingClientRect();
      let aligned: Element | null = null;
      let off = Infinity;
      for (const child of Array.from(el.children)) {
        const r = child.getBoundingClientRect();
        const d = Math.abs(rtl ? r.right - c.right : r.left - c.left);
        if (d < off) {
          off = d;
          aligned = child;
        }
      }
      rec.frames.push({
        t: performance.now(),
        pos: el.scrollLeft,
        sel: el.querySelector('[data-selected="true"]')?.getAttribute("aria-label") ?? null,
        aligned: aligned?.getAttribute("aria-label") ?? null,
        off,
        win: Array.from(el.children)
          .map((ch) => (ch as HTMLElement).dataset.position)
          .join(","),
      });
      if (rec.running) requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);
  });
}

/** Keep recording for `settleMs` (in-page wall clock), then return everything. */
async function stopRecording(content: Locator, settleMs: number): Promise<Recording> {
  return content.evaluate(async (node, settleMs) => {
    const el = node as HTMLElement;
    await new Promise((r) => setTimeout(r, settleMs));
    const rec = (window as unknown as { __dxRec: { running: boolean; mo: MutationObserver; frames: Frame[]; wheels: number[]; scrollEnds: number[]; windowChanges: number[] } }).__dxRec;
    rec.running = false;
    rec.mo.disconnect();
    const writes = (window as unknown as { __dxScrollWrites: { t: number; id: string; m: string; args: string }[] }).__dxScrollWrites
      .filter((w) => w.id === el.id)
      .map(({ t, m, args }) => ({ t, m, args }));
    return { frames: rec.frames, wheels: rec.wheels, scrollEnds: rec.scrollEnds, windowChanges: rec.windowChanges, writes };
  }, settleMs);
}

/** Dispatch untrusted wheel events in-page every `dtMs` for `ms` (they never scroll). */
async function syntheticInputFor(content: Locator, ms: number, deltaX: number, dtMs = 16): Promise<void> {
  await content.evaluate(
    async (node, { ms, deltaX, dtMs }) => {
      const t0 = performance.now();
      while (performance.now() - t0 < ms) {
        node.dispatchEvent(new WheelEvent("wheel", { deltaX, bubbles: true, cancelable: true }));
        await new Promise((r) => setTimeout(r, dtMs));
      }
    },
    { ms, deltaX, dtMs },
  );
}

/** A trackpad-shaped flick: a ramp, then momentum decaying to ~1px, one trusted wheel event each. */
const FLICK = [4, 12, 30, 60, 90, 110, 100, 90, 80, 70, 62, 55, 48, 42, 37, 32, 28, 24, 21, 18, 15, 13, 11, 9, 7, 6, 5, 4, 3, 2, 2, 1, 1];

function during<T extends { t: number } | number>(xs: T[], from: number, to: number): T[] {
  return xs.filter((x) => {
    const t = typeof x === "number" ? x : x.t;
    return t >= from && t <= to;
  });
}

const VIRTUAL: Variant[] = ["virtual_loop", "virtual_loop_rtl", "virtual_many"];
/** +1 scrolls toward the next slide (physical direction flips under RTL). */
const FORWARD: Record<Variant, number> = { virtual_loop: 1, virtual_loop_rtl: -1, virtual_many: 1, main: 1, rewind: 1 };

test.describe("Carousel virtual content: every slide is exactly one snapport wide (no oversized snap areas)", () => {
  for (const variant of VIRTUAL) {
    test(`${variant}: each rendered slide's border box matches the scroller's own snapport`, async ({ page }) => {
      const { content } = await open(page, variant);
      const sizes = await content.evaluate((node) => {
        const el = node as HTMLElement;
        return {
          port: el.clientWidth,
          slides: Array.from(el.children).map((c) => (c as HTMLElement).getBoundingClientRect().width),
          boxSizing: Array.from(el.children).map((c) => getComputedStyle(c).boxSizing),
        };
      });
      for (const w of sizes.slides) expect(Math.abs(w - sizes.port), `slide ${w}px vs snapport ${sizes.port}px`).toBeLessThan(0.5);
      expect(new Set(sizes.boxSizing)).toEqual(new Set(["border-box"]));
      expect(await restingOffset(content), "at rest, a slide sits exactly on the snapport's start edge").toBeLessThan(1);
    });

    test(`${variant}: a small trusted wheel nudge at rest snaps back to the slide boundary (no free play)`, async ({ page }) => {
      const { content } = await open(page, variant);
      for (const d of [6, 5, 4]) {
        await page.mouse.wheel(FORWARD[variant] * d, 0);
        await page.waitForTimeout(40);
      }
      await startRecording(content);
      const rec = await stopRecording(content, 900);
      const last = rec.frames[rec.frames.length - 1];
      expect(last.off, "the browser's own mandatory snap brings it back to a boundary").toBeLessThan(1);
    });
  }
});

test.describe("Carousel: scroll-end reactions wait until the browser is truly idle", () => {
  for (const variant of VIRTUAL) {
    test(`${variant}: a trusted trackpad-shaped flick -- no component scroll write and no window re-render while input is arriving; clean rest after`, async ({
      page,
    }) => {
      const { content } = await open(page, variant);
      await startRecording(content);
      for (const d of FLICK) {
        await page.mouse.wheel(FORWARD[variant] * d, 0);
        await page.waitForTimeout(12);
      }
      const rec = await stopRecording(content, 1800);
      const first = rec.wheels[0];
      const last = rec.wheels[rec.wheels.length - 1];
      expect(rec.wheels.length).toBeGreaterThan(20);
      expect(during(rec.writes, first, last), "component scroll writes while wheel input was still arriving").toEqual([]);
      expect(during(rec.windowChanges, first, last), "virtual window re-rendered while wheel input was still arriving").toEqual([]);
      const sels = new Set(during(rec.frames, first, last).map((f) => f.sel));
      expect(sels.size, `selected changed mid-gesture: ${[...sels].join(" -> ")}`).toBe(1);

      // At rest: snapped, and the selected slide is the one actually showing.
      const tail = rec.frames.filter((f) => f.t > rec.frames[rec.frames.length - 1].t - 400);
      const end = tail[tail.length - 1];
      expect(end.off).toBeLessThan(1);
      expect(end.sel).toBe(end.aligned);
      // ...and nothing moves once it has come to rest.
      expect(new Set(tail.map((f) => `${Math.round(f.pos)}|${f.sel}|${f.win}`)).size, "the visible slide changed after rest").toBe(1);
    });
  }

  test("virtual_loop: a scrollend while input is still arriving does not settle; the settle follows once input goes quiet", async ({
    page,
  }) => {
    const { content } = await open(page, "virtual_loop");
    await startRecording(content);
    // One real wheel that moves past half a slide: the browser finishes that
    // scroll (and fires scrollend) while the in-page "momentum" below is still
    // arriving -- the Firefox trackpad shape (scrollend between the pan and
    // its momentum, or at a clamp) the owner's report points at.
    await page.mouse.wheel(220, 0);
    await syntheticInputFor(content, 700, 1);
    const rec = await stopRecording(content, 1200);
    const first = rec.wheels[0];
    const last = rec.wheels[rec.wheels.length - 1];
    expect(during(rec.scrollEnds, first, last).length, "the browser did report a scrollend mid-input").toBeGreaterThan(0);
    expect(during(rec.writes, first, last)).toEqual([]);
    expect(during(rec.windowChanges, first, last)).toEqual([]);
    const selAt = (t: number) => [...rec.frames].reverse().find((f) => f.t <= t)?.sel;
    expect(selAt(last), "selected must not move while input is arriving").toBe(selAt(first));
    // It does settle once quiet: the slide the browser came to rest on is selected.
    const end = rec.frames[rec.frames.length - 1];
    expect(end.sel).toBe(end.aligned);
    expect(end.sel).not.toBe(selAt(first));
    const settledAt = rec.frames.find((f) => f.t > last && f.sel === end.sel)!.t;
    expect(settledAt - last, "settle waits for the input-quiet window").toBeGreaterThanOrEqual(100);
  });

  test("main: a Previous/Next paging write requested while wheel input is still arriving waits until input is quiet", async ({
    page,
  }) => {
    const { frame, content } = await open(page, "main");
    await startRecording(content);
    const next = frame.getByRole("button", { name: "Next slide" });
    await Promise.all([syntheticInputFor(content, 600, 1), (async () => {
      await page.waitForTimeout(150);
      await next.evaluate((b) => (b as HTMLButtonElement).click());
    })()]);
    const rec = await stopRecording(content, 1200);
    const last = rec.wheels[rec.wheels.length - 1];
    expect(rec.writes.length, "Next still pages").toBeGreaterThan(0);
    expect(rec.writes[0].t - last, "the paging write waits for input to go quiet").toBeGreaterThanOrEqual(100);
    const end = rec.frames[rec.frames.length - 1];
    expect(end.sel).toBe("2 of 5");
    expect(end.off).toBeLessThan(1);
  });
});
