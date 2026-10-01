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
import { replayWheel, recordingsOfKind, TRANSLATE_X_EXPR } from "./wheel-replay";

type Variant = "virtual_loop" | "virtual_loop_rtl" | "virtual_many" | "main" | "rewind" | "rtl" | "vertical";

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
const FORWARD: Record<Variant, number> = { virtual_loop: 1, virtual_loop_rtl: -1, virtual_many: 1, main: 1, rewind: 1, rtl: -1, vertical: 1 };

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

/*
 * The wheel band only engages at a TRUE end of the data -- the owner's
 * approved rule, one predicate (`isTrueEnd`) in `CAROUSEL_WHEEL_BAND_JS`,
 * read per gesture from `data-loop`/`data-slide-count` on the scroller and
 * the `data-index` of the slide at that physical edge:
 * - any looping carousel (`r#loop`, any `LoopMode`, plain or virtual): never;
 * - virtual, non-looping: only when its real first/last item is the slide at
 *   the edge the scroller rests at -- never at the rendered slice's edge
 *   mid-list (the browser clamps there; the window re-centres once idle);
 * - plain, non-looping: unchanged (both ends; carousel.spec.ts's own mode-D
 *   block, e.g. "real (trusted) wheel input at the start and end edges").
 *
 * HOLDING A SLICE EDGE. A virtual window only rests at its slice's edge
 * mid-list while input is still arriving -- once idle, it re-centres. The
 * tests below reproduce that state deliberately: an in-page keep-alive
 * stream of VERTICAL synthetic wheel events (off-axis, so the band ignores
 * them entirely, but the browser-idle gate counts every wheel event) keeps
 * the window from re-centring while the scroller is placed at the slice edge
 * and pushed with trusted input.
 */


/** A trackpad-shaped push: ramp, then decay (px per event). */
const PUSH = [3, 9, 21, 35, 50, 60, 62, 58, 54, 50, 46, 43, 40, 37, 34, 31, 28, 25];

async function enableWheelDebug(page: Page): Promise<void> {
  await page.addInitScript(() => {
    try {
      window.localStorage.setItem("dx-carousel-debug", "1");
    } catch {
      // No storage: the telemetry assertions below then fail loudly.
    }
  });
}

type BandRecord = { id: string; startedAtEdge: string; falseEnd: string | null; owned: boolean | null };

async function bandRecords(content: Locator): Promise<BandRecord[]> {
  return content.evaluate((el) => {
    const log = (window as unknown as { __dxCarouselWheel?: BandRecord[] }).__dxCarouselWheel ?? [];
    return JSON.parse(JSON.stringify(log.filter((r) => r.id === el.id)));
  });
}

/** Trusted wheel push (one real event per step) while sampling |band| in-page every frame; returns the max. */
async function trustedPush(page: Page, content: Locator, sign: number): Promise<number> {
  await content.evaluate((node) => {
    const el = node as HTMLElement;
    const w = window as unknown as { __dxBandMax: number; __dxBandOn: boolean };
    w.__dxBandMax = 0;
    w.__dxBandOn = true;
    const tick = () => {
      const m = el.style.transform.match(/translate[XY]\(([-\d.]+)px\)/);
      w.__dxBandMax = Math.max(w.__dxBandMax, m ? Math.abs(Number.parseFloat(m[1])) : 0);
      if (w.__dxBandOn) requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);
  });
  for (const d of PUSH) {
    await page.mouse.wheel(sign * d, 0);
    await page.waitForTimeout(12);
  }
  return content.evaluate(async () => {
    await new Promise((r) => setTimeout(r, 250));
    const w = window as unknown as { __dxBandMax: number; __dxBandOn: boolean };
    w.__dxBandOn = false;
    return w.__dxBandMax;
  });
}

/** Start/stop the off-axis keep-alive stream (see this block's header). */
async function holdInputOpen(content: Locator): Promise<void> {
  await content.evaluate((node) => {
    const w = window as unknown as { __dxKeepAlive?: number };
    const tick = () => node.dispatchEvent(new WheelEvent("wheel", { deltaY: 1, bubbles: true, cancelable: true }));
    // The first event goes out NOW: an interval's first tick is 16ms away,
    // and a placement inside that window would find input quiet and let the
    // window re-centre (found live -- it did, 2ms after the placement).
    tick();
    w.__dxKeepAlive = window.setInterval(tick, 16);
  });
}
async function releaseInput(content: Locator): Promise<void> {
  await content.evaluate((node) => {
    const w = window as unknown as { __dxKeepAlive?: number; __dxSnapSaved?: string };
    window.clearInterval(w.__dxKeepAlive);
    // Snapping was suspended by `placeAtSliceEdge`; give it back now.
    if (w.__dxSnapSaved !== undefined) {
      (node as HTMLElement).style.scrollSnapType = w.__dxSnapSaved;
      delete w.__dxSnapSaved;
    }
  });
}

/** Still at rest against the slice's `side` edge (sub-0.5px, as the band's own rest test)? */
async function expectStillAtEdge(content: Locator, side: "min" | "max"): Promise<void> {
  const gap = await content.evaluate((node, side) => {
    const el = node as HTMLElement;
    const c = el.getBoundingClientRect();
    const xs = Array.from(el.children).map((ch) => ch.getBoundingClientRect());
    return side === "min" ? Math.min(...xs.map((r) => r.left)) - c.left : Math.max(...xs.map((r) => r.right)) - c.right;
  }, side);
  expect(Math.abs(gap), `still at rest against the slice's ${side} edge right before the push (gap ${gap})`).toBeLessThan(0.5);
}

/**
 * Put the scroller at rest against its rendered slice's physical `side` edge
 * (instantly). Test setup only: snapping (with `scroll-snap-stop: always`)
 * holds the scroller against a multi-slide instant jump, so it is suspended
 * for the placement AND kept suspended until `releaseInput` -- restoring it
 * straight away lets Chrome re-snap to the previously snapped slide (found
 * live: the scroller moved back 672px with no write of ours, before the
 * push). The push itself happens at a clamped edge, where snapping has no
 * say. Returns the `data-index` of the slide now sitting at that edge.
 */
async function placeAtSliceEdge(content: Locator, side: "min" | "max"): Promise<number> {
  const r = await content.evaluate((node, side) => {
    const el = node as HTMLElement;
    const measure = () => {
      const c = el.getBoundingClientRect();
      let pick: HTMLElement | null = null;
      let best = side === "min" ? Infinity : -Infinity;
      for (const child of Array.from(el.children) as HTMLElement[]) {
        const rr = child.getBoundingClientRect();
        const v = side === "min" ? rr.left : rr.right;
        if (side === "min" ? v < best : v > best) {
          best = v;
          pick = child;
        }
      }
      return { gap: best - (side === "min" ? c.left : c.right), index: Number(pick?.dataset.index) };
    };
    const w = window as unknown as { __dxSnapSaved?: string };
    // Input must be "still arriving" at the very moment of the jump.
    el.dispatchEvent(new WheelEvent("wheel", { deltaY: 1, bubbles: true, cancelable: true }));
    if (w.__dxSnapSaved === undefined) w.__dxSnapSaved = el.style.scrollSnapType;
    el.style.scrollSnapType = "none";
    el.scrollBy({ left: measure().gap, behavior: "instant" });
    return measure();
  }, side);
  expect(Math.abs(r.gap), `at rest against the slice's ${side} edge (gap ${r.gap})`).toBeLessThan(0.5);
  return r.index;
}

/** Page with Next under reduced motion (instant) until `label` is selected, then restore motion. */
async function pageTo(page: Page, frame: Locator, label: string): Promise<void> {
  await page.emulateMedia({ reducedMotion: "reduce" });
  const next = frame.getByRole("button", { name: "Next slide" }).first();
  await next.evaluate(async (nextBtn, label) => {
    const root = nextBtn.closest("[aria-roledescription='carousel']") as HTMLElement;
    const prevBtn = root.querySelector('button[aria-label="Previous slide"]') as HTMLButtonElement;
    const num = (l: string | null | undefined) => Number((l ?? "").split(" of ")[0]);
    // Slides are role=group, or role=tabpanel when the demo has indicators.
    const selected = () =>
      root.querySelector(':is([role="group"], [role="tabpanel"])[data-selected="true"]')?.getAttribute("aria-label");
    for (let i = 0; i < 400 && selected() !== label; i++) {
      const before = selected();
      // Step toward the target: Previous when it is behind (a non-looping
      // list cannot get there with Next), otherwise Next.
      const btn = num(label) < num(before) && !prevBtn.disabled ? prevBtn : (nextBtn as HTMLButtonElement);
      btn.click();
      const t0 = performance.now();
      while (selected() === before && performance.now() - t0 < 2000) await new Promise((r) => requestAnimationFrame(r));
    }
  }, label);
  await expect(frame.locator(':is([role="group"], [role="tabpanel"])[data-selected="true"]').first()).toHaveAttribute(
    "aria-label",
    label,
  );
  // Let the instant re-align and the settle land before the next step.
  await page.waitForTimeout(300);
  await page.emulateMedia({ reducedMotion: "no-preference" });
}

test.describe("Carousel wheel band: only at a true end of the data", () => {
  test("virtual_many at its real first item (start) bands on a trusted push", async ({ page }) => {
    await enableWheelDebug(page);
    const { content } = await open(page, "virtual_many");
    expect(await content.getAttribute("data-loop")).toBe("false");
    expect(await content.getAttribute("data-slide-count")).toBe("200");
    expect(await trustedPush(page, content, -1)).toBeGreaterThan(5);
    const [rec] = (await bandRecords(content)).slice(-1);
    expect(rec.startedAtEdge).toBe("left");
    expect(rec.owned).toBe(true);
  });

  test("virtual_many at its real last item (end) bands on a trusted push", async ({ page }) => {
    test.setTimeout(180_000);
    await enableWheelDebug(page);
    const { frame, content } = await open(page, "virtual_many");
    await pageTo(page, frame, "200 of 200");
    await holdInputOpen(content);
    const edgeIndex = await placeAtSliceEdge(content, "max");
    expect(edgeIndex, "the real last item is the slide at the edge").toBe(199);
    await page.waitForTimeout(150);
    await expectStillAtEdge(content, "max");
    expect(await trustedPush(page, content, 1)).toBeGreaterThan(5);
    await releaseInput(content);
    const [rec] = (await bandRecords(content)).slice(-1);
    expect(rec.startedAtEdge).toBe("right");
    expect(rec.owned).toBe(true);
  });

  test("virtual_many resting mid-list at its rendered slice's edge does NOT band; the window re-centres once idle", async ({
    page,
  }) => {
    test.setTimeout(120_000);
    await enableWheelDebug(page);
    const { frame, content } = await open(page, "virtual_many");
    await pageTo(page, frame, "50 of 200");
    for (const [side, sign, name] of [
      ["min", -1, "left"],
      ["max", 1, "right"],
    ] as const) {
      await holdInputOpen(content);
      const edgeIndex = await placeAtSliceEdge(content, side);
      expect(edgeIndex, "mid-list: the edge slide is neither the first nor the last item").toBeGreaterThan(0);
      expect(edgeIndex).toBeLessThan(199);
      await page.waitForTimeout(150); // a fresh gesture (> GAP_MS since any on-axis input)
      await expectStillAtEdge(content, side);
      expect(await trustedPush(page, content, sign), `pushing into the slice's ${side} edge`).toBe(0);
      const recs = await bandRecords(content);
      const [rec] = recs.slice(-1);
      expect(rec.falseEnd, `the gesture started at rest against that edge and was refused: ${JSON.stringify(recs.slice(-4))}`).toBe(name);
      expect(rec.owned).not.toBe(true);
      await releaseInput(content);
      // Once idle, the window re-centres: the scroller is no longer at the slice edge.
      await expect
        .poll(() =>
          content.evaluate((node, side) => {
            const el = node as HTMLElement;
            const c = el.getBoundingClientRect();
            const xs = Array.from(el.children).map((ch) => ch.getBoundingClientRect());
            return side === "min"
              ? Math.min(...xs.map((r) => r.left)) - c.left
              : c.right - Math.max(...xs.map((r) => r.right));
          }, side),
        )
        .toBeLessThan(-1);
    }
  });

  for (const variant of ["rewind", "virtual_loop", "virtual_loop_rtl"] as const) {
    test(`${variant}: a looping carousel never bands -- start, middle and end, at both slice edges`, async ({ page }) => {
      test.setTimeout(180_000);
      await enableWheelDebug(page);
      const { frame, content } = await open(page, variant);
      expect(await content.getAttribute("data-loop")).toBe("true");
      const n = variant === "rewind" ? 5 : 12;
      for (const label of [`1 of ${n}`, `${Math.ceil(n / 2)} of ${n}`, `${n} of ${n}`]) {
        await pageTo(page, frame, label);
        for (const [side, sign] of [
          ["min", -1],
          ["max", 1],
        ] as const) {
          // Rewind renders every slide, so its physical edges are its only
          // edges; the virtual loops' slice edges are held open as above.
          await holdInputOpen(content);
          await placeAtSliceEdge(content, side);
          await page.waitForTimeout(150);
          await expectStillAtEdge(content, side);
          expect(await trustedPush(page, content, sign), `${label}, pushing into the ${side} edge`).toBe(0);
          await releaseInput(content);
          await page.waitForTimeout(250);
        }
      }
      const refused = (await bandRecords(content)).filter((r) => r.falseEnd);
      expect(refused.length, "pushes really started at rest against an edge").toBeGreaterThan(0);
      expect((await bandRecords(content)).some((r) => r.owned === true)).toBe(false);
    });
  }

  test("RTL and vertical: the predicate reads the physical edge slide's data-index (true end bands; a rewritten mid-list index does not)", async ({
    page,
  }) => {
    for (const [variant, axis, side] of [
      ["rtl", "x", "max"],
      ["vertical", "y", "min"],
    ] as const) {
      await page.goto("about:blank");
      const { content } = await open(page, variant);
      // Slide 1 rests at the physical right under RTL, at the top when vertical.
      const push = (sign: number) =>
        content.evaluate(async (node, { axis, sign }) => {
          const el = node as HTMLElement;
          let max = 0;
          // A push, then its momentum tail (so it releases by smooth decay).
          for (const d of [3, 9, 21, 35, 50, 60, 62, 58, 54, 50, 46, 43, 40, 37, 34, 31, 29, 27, 25, 23, 21, 19, 18, 16]) {
            el.dispatchEvent(
              new WheelEvent("wheel", { deltaX: axis === "x" ? sign * d : 0, deltaY: axis === "y" ? sign * d : 0, bubbles: true }),
            );
            const m = el.style.transform.match(/translate[XY]\(([-\d.]+)px\)/);
            max = Math.max(max, m ? Math.abs(Number.parseFloat(m[1])) : 0);
            await new Promise((r) => setTimeout(r, 16));
          }
          // Released and home before the next push, so no leftover is measured.
          const t0 = performance.now();
          while (el.style.transform !== "" && performance.now() - t0 < 3000) await new Promise((r) => setTimeout(r, 50));
          return max;
        }, { axis, sign });
      const sign = side === "max" ? 1 : -1;
      expect(await push(sign), `${variant}: slide 1 is a true end`).toBeGreaterThan(5);
      // Rewrite the rendered markup so the same edge slide reads as item 41
      // of 100 -- exactly what a virtual slice edge mid-list looks like.
      await content.evaluate((node, side) => {
        const el = node as HTMLElement;
        const kids = Array.from(el.children) as HTMLElement[];
        const key = (r: DOMRect) => (side === "max" ? r.right + r.bottom : -(r.left + r.top));
        const edge = kids.reduce((a, b) => (key(b.getBoundingClientRect()) > key(a.getBoundingClientRect()) ? b : a));
        edge.dataset.index = "40";
        el.dataset.slideCount = "100";
      }, side);
      await page.waitForTimeout(200);
      expect(await push(sign), `${variant}: a mid-list index at the same edge`).toBe(0);
    }
  });

  test("recorded owned-push replays: band at virtual_many's real start, none on the loop demos", async ({ page }) => {
    const [rec] = recordingsOfKind("owned-push");
    for (const [variant, expectBand] of [
      ["virtual_many", true],
      ["virtual_loop", false],
      ["rewind", false],
    ] as const) {
      await page.goto("about:blank");
      const { content } = await open(page, variant);
      const r = await replayWheel(content, rec, {
        sample: { band: TRANSLATE_X_EXPR },
        settleMs: 400,
        jump: { atIndex: 0, scrollLeft: "start" },
      });
      const max = Math.max(0, ...r.samples.map((s) => Math.abs(s.band)));
      if (expectBand) expect(max, variant).toBeGreaterThan(5);
      else expect(max, variant).toBe(0);
    }
  });
});

/*
 * The mouse/pen drag rubber-band follows the SAME true-end rule as the wheel
 * band, through the same shared predicate (`carousel_true_end_js!` in
 * `primitives/src/carousel.rs`, spliced into both bridges). A drag decides
 * once per edge per gesture, the first time it overdrags past that edge; a
 * refused edge behaves like a drag that simply cannot pass it (the scroller
 * clamps, no transform, the slide settles normally, and a virtual window
 * re-centres once idle).
 */

type DragResult = { band: number; gapAtHold: number };

/**
 * Real mouse drag from the track's centre, pushing content INTO the physical
 * `side` edge (`min` = left/top, `max` = right/bottom) by `distance` px, with
 * |band| sampled in-page every frame and the edge gap measured while the
 * button is still held; then release.
 */
async function dragIntoEdge(
  page: Page,
  content: Locator,
  side: "min" | "max",
  axis: "x" | "y",
  distance: number,
): Promise<DragResult> {
  await content.evaluate((node) => {
    const el = node as HTMLElement;
    const w = window as unknown as { __dxDragBand: number; __dxDragOn: boolean };
    w.__dxDragBand = 0;
    w.__dxDragOn = true;
    const tick = () => {
      const m = el.style.transform.match(/translate[XY]\(([-\d.]+)px\)/);
      w.__dxDragBand = Math.max(w.__dxDragBand, m ? Math.abs(Number.parseFloat(m[1])) : 0);
      if (w.__dxDragOn) requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);
  });
  const box = await content.boundingBox();
  if (!box) throw new Error("content has no bounding box");
  const x0 = box.x + box.width / 2;
  const y0 = box.y + box.height / 2;
  // Content moves WITH the pointer: into the min edge = pointer toward +axis.
  const sign = side === "min" ? 1 : -1;
  await page.mouse.move(x0, y0);
  await page.mouse.down();
  const steps = 24;
  for (let i = 1; i <= steps; i++) {
    const d = (sign * distance * i) / steps;
    await page.mouse.move(axis === "x" ? x0 + d : x0, axis === "y" ? y0 + d : y0);
    await page.waitForTimeout(16);
  }
  const gapAtHold = await content.evaluate((node, { side, axis }) => {
    const el = node as HTMLElement;
    const c = el.getBoundingClientRect();
    const rs = Array.from(el.children).map((ch) => ch.getBoundingClientRect());
    return side === "min"
      ? Math.min(...rs.map((r) => (axis === "x" ? r.left : r.top))) - (axis === "x" ? c.left : c.top)
      : Math.max(...rs.map((r) => (axis === "x" ? r.right : r.bottom))) - (axis === "x" ? c.right : c.bottom);
  }, { side, axis });
  await page.mouse.up();
  const band = await content.evaluate(async () => {
    await new Promise((r) => setTimeout(r, 100));
    const w = window as unknown as { __dxDragBand: number; __dxDragOn: boolean };
    w.__dxDragOn = false;
    return w.__dxDragBand;
  });
  return { band, gapAtHold };
}

/** After a drag: no stale transform, snapped, and `selected` is the slide actually showing. */
async function expectCleanRest(content: Locator): Promise<void> {
  await expect.poll(() => content.evaluate((el) => (el as HTMLElement).style.transform), { timeout: 3000 }).toBe("");
  await expect
    .poll(
      () =>
        content.evaluate((node) => {
          const el = node as HTMLElement;
          const c = el.getBoundingClientRect();
          const rtl = getComputedStyle(el).direction === "rtl";
          const vertical = el.getAttribute("data-orientation") === "vertical";
          let aligned: Element | null = null;
          let off = Infinity;
          for (const child of Array.from(el.children)) {
            const r = child.getBoundingClientRect();
            const d = Math.abs(vertical ? r.top - c.top : rtl ? r.right - c.right : r.left - c.left);
            if (d < off) {
              off = d;
              aligned = child;
            }
          }
          const sel = el.querySelector('[data-selected="true"]')?.getAttribute("aria-label");
          return off < 1 && sel === aligned?.getAttribute("aria-label") ? "ok" : `off=${off} sel=${sel} aligned=${aligned?.getAttribute("aria-label")}`;
        }),
      { timeout: 3000 },
    )
    .toBe("ok");
}

/** Once idle, a virtual window has re-centred: the scroller is no longer against `side`'s slice edge. */
async function expectRecentred(content: Locator, side: "min" | "max"): Promise<void> {
  await expect
    .poll(() =>
      content.evaluate((node, side) => {
        const el = node as HTMLElement;
        const c = el.getBoundingClientRect();
        const xs = Array.from(el.children).map((ch) => ch.getBoundingClientRect());
        return side === "min"
          ? Math.min(...xs.map((r) => r.left)) - c.left
          : c.right - Math.max(...xs.map((r) => r.right));
      }, side),
    )
    .toBeLessThan(-1);
}

async function pitchOf(content: Locator): Promise<number> {
  return content.evaluate((el) => (el as HTMLElement).clientWidth);
}

test.describe("Carousel drag band: only at a true end of the data", () => {
  test("plain main: dragging past the first slide still bands (unchanged), then rests cleanly", async ({ page }) => {
    const { content } = await open(page, "main");
    const r = await dragIntoEdge(page, content, "min", "x", (await pitchOf(content)) * 1.2);
    expect(r.band).toBeGreaterThan(5);
    await expectCleanRest(content);
  });

  test("rewind: dragging past its first and last slide never bands; it clamps and settles", async ({ page }) => {
    test.setTimeout(120_000);
    const { frame, content } = await open(page, "rewind");
    const pitch = await pitchOf(content);
    for (const [label, side] of [
      ["1 of 5", "min"],
      ["5 of 5", "max"],
    ] as const) {
      await pageTo(page, frame, label);
      const r = await dragIntoEdge(page, content, side, "x", pitch * 1.2);
      expect(Math.abs(r.gapAtHold), `${label}: clamped at the ${side} edge while held`).toBeLessThan(1);
      expect(r.band, `${label}: no band`).toBe(0);
      await expectCleanRest(content);
      await expect(frame.locator('[role="group"][data-selected="true"]').first()).toHaveAttribute("aria-label", label);
    }
  });

  for (const variant of ["virtual_loop", "virtual_loop_rtl"] as const) {
    test(`${variant}: dragging into either slice edge never bands, at start, middle and end; the window re-centres`, async ({
      page,
    }) => {
      test.setTimeout(180_000);
      const { frame, content } = await open(page, variant);
      const pitch = await pitchOf(content);
      for (const label of ["1 of 12", "6 of 12", "12 of 12"]) {
        await pageTo(page, frame, label);
        for (const side of ["min", "max"] as const) {
          // radius 2: the slice edge is two slides away, so 3 pitches reaches it.
          const r = await dragIntoEdge(page, content, side, "x", pitch * 3);
          expect(Math.abs(r.gapAtHold), `${label}: reached the ${side} slice edge while held`).toBeLessThan(1);
          expect(r.band, `${label}, ${side}: no band`).toBe(0);
          await expectCleanRest(content);
          await expectRecentred(content, side);
        }
      }
    });
  }

  test("virtual_many mid-list: a drag against the rendered slice's edge does not band, and the window re-centres once idle", async ({
    page,
  }) => {
    test.setTimeout(120_000);
    const { frame, content } = await open(page, "virtual_many");
    await pageTo(page, frame, "50 of 200");
    const pitch = await pitchOf(content);
    for (const side of ["min", "max"] as const) {
      const r = await dragIntoEdge(page, content, side, "x", pitch * 3);
      expect(Math.abs(r.gapAtHold), `reached the ${side} slice edge while held`).toBeLessThan(1);
      expect(r.band, `${side}: no band`).toBe(0);
      await expectCleanRest(content);
      await expectRecentred(content, side);
    }
  });

  test("virtual_many at its real first and last item: a drag past it bands", async ({ page }) => {
    test.setTimeout(180_000);
    const { frame, content } = await open(page, "virtual_many");
    const pitch = await pitchOf(content);
    const first = await dragIntoEdge(page, content, "min", "x", pitch * 1.2);
    expect(first.band, "real first item").toBeGreaterThan(5);
    await expectCleanRest(content);
    await pageTo(page, frame, "200 of 200");
    const last = await dragIntoEdge(page, content, "max", "x", pitch * 1.2);
    expect(last.band, "real last item").toBeGreaterThan(5);
    await expectCleanRest(content);
  });

  test("RTL and vertical: the shared predicate on the drag path (true end bands; a rewritten mid-list index does not)", async ({
    page,
  }) => {
    for (const [variant, axis, side] of [
      ["rtl", "x", "max"],
      ["vertical", "y", "min"],
    ] as const) {
      await page.goto("about:blank");
      const { content } = await open(page, variant);
      const pitch = await content.evaluate((el) =>
        el.getAttribute("data-orientation") === "vertical" ? (el as HTMLElement).clientHeight : (el as HTMLElement).clientWidth,
      );
      // Slide 1 rests at the physical right under RTL, at the top when vertical.
      const control = await dragIntoEdge(page, content, side, axis, pitch * 1.2);
      expect(control.band, `${variant}: slide 1 is a true end`).toBeGreaterThan(5);
      await expectCleanRest(content);
      // Rewrite the edge slide's rendered markup to read as item 41 of 100.
      await content.evaluate((node, side) => {
        const el = node as HTMLElement;
        const kids = Array.from(el.children) as HTMLElement[];
        const key = (r: DOMRect) => (side === "max" ? r.right + r.bottom : -(r.left + r.top));
        const edge = kids.reduce((a, b) => (key(b.getBoundingClientRect()) > key(a.getBoundingClientRect()) ? b : a));
        edge.dataset.index = "40";
        el.dataset.slideCount = "100";
      }, side);
      const refused = await dragIntoEdge(page, content, side, axis, pitch * 1.2);
      expect(Math.abs(refused.gapAtHold), `${variant}: still clamped at the edge`).toBeLessThan(1);
      expect(refused.band, `${variant}: a mid-list index at the same edge`).toBe(0);
      await expect.poll(() => content.evaluate((el) => (el as HTMLElement).style.transform)).toBe("");
    }
  });
});

/*
 * Fast trackpad flicks: the owner's real-trackpad report ("quick trackpad
 * scrolls still produce an 'end of slides' rubberband effect when there are
 * still more slides to come"). Measured: a flick shaped like the owner's
 * hardest recordings holds a virtual window's scroller clamped against its
 * rendered slice's physical end mid-list (and a looping carousel's against
 * its physical ends) for ~1s -- where macOS Firefox draws its own native
 * bounce whenever `overscroll-behavior` is `auto`. This crate's own band
 * never fired there. The rule under test (`content_overscroll_style` in
 * `primitives/src/carousel.rs`): the scroll axis is `overscroll-behavior:
 * none` unless BOTH ends of the rendered slides are true ends, so wherever a
 * flick can clamp at a false end, the platform cannot bounce. (Whether
 * Firefox then shows no bounce is the owner's real-device check: headless
 * Chromium draws no elastic overscroll at all, so the rendered style is what
 * can be proven here.)
 */

type FlickFrame = {
  band: number;
  atMin: boolean;
  atMax: boolean;
  minTrue: boolean;
  maxTrue: boolean;
  overscroll: string;
};

/** One trusted flick (a recording's deltas and timing, one real wheel event each), sampled in-page every frame. */
async function flick(page: Page, content: Locator, recordingId: string, dir: 1 | -1): Promise<FlickFrame[]> {
  const rec = recordingsOfKind("end-edge")
    .concat(recordingsOfKind("owned-push"))
    .find((r) => r.id === recordingId)!;
  await content.evaluate((node) => {
    const el = node as HTMLElement;
    const vertical = el.getAttribute("data-orientation") === "vertical";
    const trueEnd = (child: HTMLElement | null) => {
      if (!child || el.dataset.loop === "true") return false;
      const i = Number(child.dataset.index);
      return i === 0 || i === Number(el.dataset.slideCount) - 1;
    };
    const w = window as unknown as { __dxFlick: { frames: unknown[]; run: boolean } };
    w.__dxFlick = { frames: [], run: true };
    const tick = () => {
      const c = el.getBoundingClientRect();
      const kids = Array.from(el.children) as HTMLElement[];
      const lead = (r: DOMRect) => (vertical ? r.top : r.left);
      const trail = (r: DOMRect) => (vertical ? r.bottom : r.right);
      let lo: HTMLElement | null = null;
      let hi: HTMLElement | null = null;
      for (const k of kids) {
        if (!lo || lead(k.getBoundingClientRect()) < lead(lo.getBoundingClientRect())) lo = k;
        if (!hi || trail(k.getBoundingClientRect()) > trail(hi.getBoundingClientRect())) hi = k;
      }
      const m = el.style.transform.match(/translate[XY]\(([-\d.]+)px\)/);
      const cs = getComputedStyle(el);
      w.__dxFlick.frames.push({
        band: m ? Math.abs(Number.parseFloat(m[1])) : 0,
        atMin: !!lo && lead(lo.getBoundingClientRect()) - lead(c) >= -0.5,
        atMax: !!hi && trail(hi.getBoundingClientRect()) - trail(c) <= 0.5,
        minTrue: trueEnd(lo),
        maxTrue: trueEnd(hi),
        overscroll: vertical ? cs.overscrollBehaviorY : cs.overscrollBehaviorX,
      });
      if (w.__dxFlick.run) requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);
  });
  const vertical = (await content.getAttribute("data-orientation")) === "vertical";
  const t0 = Date.now();
  for (const e of rec.events) {
    const wait = e.t - (Date.now() - t0);
    if (wait > 0) await page.waitForTimeout(wait);
    const d = dir * Math.abs(e.dx || e.dy);
    await page.mouse.wheel(vertical ? 0 : d, vertical ? d : 0);
  }
  return content.evaluate(async () => {
    await new Promise((r) => setTimeout(r, 1200));
    const w = window as unknown as { __dxFlick: { frames: FlickFrame[]; run: boolean } };
    w.__dxFlick.run = false;
    return w.__dxFlick.frames;
  });
}

/** Frames where the scroller is clamped against a physical end that is NOT a true end of the data. */
function falseEndClamps(frames: FlickFrame[]): FlickFrame[] {
  return frames.filter((f) => (f.atMin && !f.minTrue) || (f.atMax && !f.maxTrue));
}

test.describe("Carousel fast flicks: the platform may only bounce at a true end", () => {
  for (const [variant, startLabel] of [
    ["virtual_many", "50 of 200"],
    ["virtual_many", "1 of 200"],
    ["virtual_loop", "6 of 12"],
    ["virtual_loop_rtl", "6 of 12"],
    ["rewind", "3 of 5"],
  ] as const) {
    test(`${variant} from ${startLabel}: hard flicks both ways -- any false-end clamp has overscroll none, and our band never draws there`, async ({
      page,
    }) => {
      test.setTimeout(180_000);
      const { frame, content } = await open(page, variant);
      let clamps = 0;
      for (const dir of [1, -1] as const) {
        await pageTo(page, frame, startLabel);
        const frames = await flick(page, content, "end-edge", dir);
        const bad = falseEndClamps(frames);
        clamps += bad.length;
        for (const f of bad) expect(f.overscroll, `${variant} dir ${dir}: clamped at a false end`).toBe("none");
        const bandAway = frames.filter((f) => f.band > 0.5 && !((f.atMin && f.minTrue) || (f.atMax && f.maxTrue)));
        expect(bandAway.length, `${variant} dir ${dir}: our band away from a true end`).toBe(0);
      }
      // Not vacuous: these flicks really do reach a false end on this demo
      // (the virtual 1-of-200 start reaches only its far, false end forward).
      expect(clamps, "the flicks reached a false end at least once").toBeGreaterThan(0);
    });
  }

  test("plain main: both ends are true, so the platform default is untouched and flicks never clamp at a false end", async ({
    page,
  }) => {
    const { content } = await open(page, "main");
    expect(await content.evaluate((el) => getComputedStyle(el).overscrollBehaviorX)).toBe("auto");
    for (const dir of [1, -1] as const) {
      const frames = await flick(page, content, "end-edge", dir);
      expect(falseEndClamps(frames).length).toBe(0);
      expect(frames.every((f) => f.overscroll === "auto")).toBe(true);
    }
  });

  test("virtual_many: overscroll follows the rendered window -- none mid-list, and none at a real end whose other slice end is false", async ({
    page,
  }) => {
    const { frame, content } = await open(page, "virtual_many");
    const axis = () => content.evaluate((el) => getComputedStyle(el).overscrollBehaviorX);
    expect(await axis(), "at item 1 of 200: its far slice end is false").toBe("none");
    await pageTo(page, frame, "50 of 200");
    expect(await axis()).toBe("none");
    // The band still engages at a real end (a transform, unaffected by
    // overscroll-behavior) -- carousel-virtual-wheel's own "virtual_many at
    // its real first item" tests pin that.
  });
});
