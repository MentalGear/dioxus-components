/**
 * Carousel trackpad edge-band: replay of REAL trackpad recordings.
 *
 * Source data: `fixtures/carousel-wheel-recordings.json` -- wheel bursts the
 * owner recorded on a MacBook trackpad in Firefox 156 while tuning the
 * overscroll bench (dev-docs/research/carousel-overscroll-bench.html), plus
 * two clearly-marked SYNTHETIC ones (a notched mouse wheel and vertical page
 * scrolling) for which no real data exists. The JSON's top-level `timing`
 * note says how event spacing was reconstructed (the telemetry has no
 * per-event timestamps).
 *
 * TWO DESCRIBE BLOCKS, ONE SET OF RECORDINGS:
 *
 *  (a) REFERENCE -- the bench's own mode D ("onewriter", rev 40, copied to
 *      `fixtures/carousel-overscroll-bench-rev40.html`). It is the
 *      known-good engine the recordings were tuned against, so this block
 *      proves the replay harness reproduces what the owner saw (a band on an
 *      owned push, none on a native arrival, a band after a re-push).
 *
 *  (b) COMPONENT -- the shipped carousel's `.dx-carousel-content` transform,
 *      asserted against the component's INTENDED contract. Written before the
 *      "mode D" edge-band engine lands in `primitives/src/carousel.rs`; it is
 *      expected to fail against the old engine on main (see the per-test
 *      comments for which assertions did, when this file was added).
 *
 * WHAT IS EMULATED, AND WHY: `dispatchEvent(new WheelEvent(...))` is
 * untrusted, so the browser never scrolls for it. Real trackpad momentum
 * carried the scroller from mid-track to the edge on its own; nothing here
 * can (and `page.mouse.wheel` cannot either: headless Chromium smooth-scrolls
 * each trusted event and `scroll-snap-stop: always` pins a long gesture at
 * every slide). So for the native kinds (`mid-track`) the scroller starts at
 * slide 2 (or the second-to-last slide) and `replayWheel`'s `jump` moves it
 * to the edge, instantly, right before event `arrivalIndex` -- "momentum
 * arrives here". The real index was not recorded; it is a documented
 * approximation stored in the JSON. Edge-start kinds need no emulation.
 *
 * SAMPLING (dev-docs/backlog.md row 112): the band is an animated value, so it
 * is sampled in-page on every animation frame by `replayWheel`, never across
 * awaits. Times are relative to the LAST event of the recording.
 */

import { test, expect } from "./fixtures";
import { type Page, type Locator } from "@playwright/test";
import path from "node:path";
import { BASE_URL } from "./base-url";
import { gotoHydrated } from "./hydration";
import {
  loadRecordings,
  replayWheel,
  peakAbs,
  peakAbsSince,
  valueAt,
  settledAfter,
  summarize,
  TRANSLATE_X_EXPR,
  type WheelRecording,
  type ReplayResult,
  type ScrollJump,
} from "./wheel-replay";

const RECORDINGS = loadRecordings();
const byKind = (kind: WheelRecording["kind"]) => RECORDINGS.filter((r) => r.kind === kind);

/** A band above this is "a band"; below it, sub-pixel noise / rounding. */
const BAND_MIN_PX = 3;
/** Anything at or below this counts as "no band at all". */
const NO_BAND_PX = 0.5;

/**
 * Put the scroller where the recording starts. `mid-track` is one slide in
 * from the edge the recording is heading for (slide 2, or the second-to-last).
 */
async function place(scroller: Locator, rec: WheelRecording): Promise<void> {
  await scroller.evaluate(
    (el, { startsAt, edge }) => {
      const e = el as HTMLElement;
      const max = e.scrollWidth - e.clientWidth;
      const kids = e.children;
      const pitch = kids[1].getBoundingClientRect().left - kids[0].getBoundingClientRect().left;
      const left = startsAt === "start-edge" ? 0 : startsAt === "end-edge" ? max : edge === "start" ? pitch : max - pitch;
      e.scrollTo({ left, behavior: "instant" });
    },
    { startsAt: rec.startsAt, edge: rec.edge },
  );
  // Let the engine's own scroll/scrollend handlers run before the replay starts.
  await scroller.page().evaluate(
    () => new Promise<void>((resolve) => requestAnimationFrame(() => requestAnimationFrame(() => setTimeout(resolve, 200)))),
  );
}

/** The emulated momentum arrival for native kinds (see the file doc). */
function momentumArrival(rec: WheelRecording): ScrollJump | undefined {
  if (rec.startsAt !== "mid-track") return undefined;
  if (rec.arrivalIndex === undefined) throw new Error(`${rec.id}: native recording without arrivalIndex`);
  return { atIndex: rec.arrivalIndex, scrollLeft: rec.edge === "start" ? "start" : "end" };
}

async function widthOf(el: Locator): Promise<number> {
  return el.evaluate((e) => (e as HTMLElement).clientWidth);
}

/** Time (relative to the last event) at which the recording's `index`-th event was dispatched. */
function eventTime(r: ReplayResult, index: number): number {
  return r.eventTimes[index];
}

/* ------------------------------------------------------------------------ */
/* (a) Reference: the bench's own mode D                                     */
/* ------------------------------------------------------------------------ */

const BENCH_URL = "file://" + path.resolve(__dirname, "fixtures", "carousel-overscroll-bench-rev40.html");

async function openBench(page: Page): Promise<{ track: Locator }> {
  // The bench links a Google Fonts stylesheet; keep the run hermetic and fast.
  await page.route("https://fonts.googleapis.com/**", (route) => route.abort());
  await page.goto(BENCH_URL, { waitUntil: "domcontentloaded" });
  await page.locator('button.mode[data-mode="onewriter"]').click();
  await expect(page.locator('button.mode[data-mode="onewriter"]')).toHaveAttribute("aria-pressed", "true");
  const track = page.locator("#track");
  await expect(track).toBeVisible();
  await expect(page.locator("#rSel")).toHaveText("1 of 15");
  return { track };
}

test.describe("Carousel wheel replay: reference (the bench's own mode D, rev 40)", () => {
  /** Both the inline transform and the bench's own `#rOver` readout. */
  const benchSample = {
    band: TRANSLATE_X_EXPR,
    rOver: "(parseFloat(document.getElementById('rOver').textContent) || 0)",
  };

  for (const rec of byKind("owned-push")) {
    test(`owned-push ${rec.id}: draws a bounded band, back to 0 within ~1s of the last event`, async ({ page }) => {
      const { track } = await openBench(page);
      await place(track, rec);
      const width = await widthOf(track);
      const r = await replayWheel(track, rec, { sample: benchSample });
      const summary = summarize(r);
      expect(peakAbs(r), `band appeared (${summary})`).toBeGreaterThan(BAND_MIN_PX);
      expect(peakAbs(r), `band <= 0.5 x track width ${width} (${summary})`).toBeLessThanOrEqual(0.5 * width);
      // The bench's own readout tracks the transform it draws.
      expect(peakAbs(r, "rOver")).toBeGreaterThan(0);
      const settled = settledAfter(r);
      expect(settled, `returned to ~0 (${summary})`).not.toBeNull();
      expect(settled!, summary).toBeLessThanOrEqual(1000);
    });
  }

  for (const rec of byKind("end-edge")) {
    test(`end-edge ${rec.id}: same contract at the end edge (band toward the other side)`, async ({ page }) => {
      const { track } = await openBench(page);
      await place(track, rec);
      const width = await widthOf(track);
      const r = await replayWheel(track, rec);
      const summary = summarize(r);
      expect(peakAbs(r), summary).toBeGreaterThan(BAND_MIN_PX);
      expect(peakAbs(r), summary).toBeLessThanOrEqual(0.5 * width);
      // The end edge translates the content the other way (negative px).
      expect(Math.min(...r.samples.map((s) => s.band)), summary).toBeLessThan(0);
      expect(Math.max(...r.samples.map((s) => s.band)), summary).toBeLessThanOrEqual(NO_BAND_PX);
      const settled = settledAfter(r);
      expect(settled, summary).not.toBeNull();
      expect(settled!, summary).toBeLessThanOrEqual(1000);
    });
  }

  for (const rec of byKind("slow-push")) {
    test(`slow-push ${rec.id}: spring-back begins within ~400ms of the last event`, async ({ page }) => {
      const { track } = await openBench(page);
      await place(track, rec);
      const r = await replayWheel(track, rec);
      const summary = summarize(r);
      expect(peakAbs(r), `a slow push still draws a band (${summary})`).toBeGreaterThan(NO_BAND_PX);
      const settled = settledAfter(r);
      const atLast = Math.abs(valueAt(r, 0));
      const at400 = Math.abs(valueAt(r, 400));
      // Either already home, or visibly on its way there 400ms after the last event.
      expect(settled !== null || at400 < atLast - 0.25, summary).toBe(true);
      expect(settled, `fully home within the window (${summary})`).not.toBeNull();
    });
  }

  for (const rec of byKind("native-arrival")) {
    test(`native-arrival ${rec.id}: momentum reaching the edge draws no band`, async ({ page }) => {
      const { track } = await openBench(page);
      await place(track, rec);
      const jump = momentumArrival(rec);
      const r = await replayWheel(track, rec, { jump });
      expect(r.jumped, "the momentum-arrival emulation ran").toBe(true);
      expect(peakAbs(r), summarize(r)).toBeLessThanOrEqual(NO_BAND_PX);
    });
  }

  for (const rec of byKind("native-arrival-repush")) {
    test(`native-arrival-repush ${rec.id}: no band while coasting in, a band after the re-push at event ${rec.repushIndex}`, async ({
      page,
    }) => {
      const { track } = await openBench(page);
      await place(track, rec);
      const width = await widthOf(track);
      const r = await replayWheel(track, rec, { jump: momentumArrival(rec) });
      const summary = summarize(r);
      expect(r.jumped).toBe(true);
      const tRepush = eventTime(r, rec.repushIndex!);
      const before = r.samples.filter((s) => s.t < tRepush - 1);
      expect(Math.max(...before.map((s) => Math.abs(s.band))), `no band before the re-push (${summary})`).toBeLessThanOrEqual(
        NO_BAND_PX,
      );
      expect(peakAbsSince(r, tRepush - 1), `band after the re-push (${summary})`).toBeGreaterThan(BAND_MIN_PX);
      expect(peakAbs(r), summary).toBeLessThanOrEqual(0.5 * width);
      const settled = settledAfter(r);
      expect(settled, summary).not.toBeNull();
      expect(settled!, summary).toBeLessThanOrEqual(1000);
    });
  }

  /*
   * DOCUMENTED DIFFERENCES -- NOT ASSERTED. The bench predates the notched
   * and axis rules the component deliberately adds: it converts deltaMode 1
   * lines to px (x16) and treats any wheel event as horizontal input
   * (`raw = deltaX !== 0 ? deltaX : deltaY`), so it neither releases a
   * notched click on a short timer nor ignores a vertical page scroll. The
   * component will deliberately differ (see block (b)), so the bench's actual
   * behaviour is only recorded as a test annotation, never asserted.
   *
   * Measured when this file was added (Chromium, 806px track): the notched
   * recording (4 x 3 lines) draws a 29px band that is held until the bench's
   * 1500ms idle backstop and is home ~1.8s after the last click (component
   * contract: <= 300ms); the vertical-only recording (8 x dy -100) draws a
   * 99px band, home ~1.8s after the last event (component contract: no band).
   */
  for (const kind of ["notched", "vertical-only"] as const) {
    for (const rec of byKind(kind)) {
      test(`${kind} ${rec.id}: records the bench's actual behaviour (documented difference, not asserted)`, async ({
        page,
      }) => {
        const { track } = await openBench(page);
        await place(track, rec);
        const r = await replayWheel(track, rec);
        test.info().annotations.push({ type: `bench-actual (${kind})`, description: summarize(r) });
        // Harness sanity only.
        expect(r.samples.length).toBeGreaterThan(0);
        expect(r.eventTimes).toHaveLength(rec.events.length);
      });
    }
  }
});

/* ------------------------------------------------------------------------ */
/* (b) Component: the shipped carousel's intended contract                   */
/* ------------------------------------------------------------------------ */

async function openComponent(page: Page): Promise<{ content: Locator }> {
  await gotoHydrated(page, `${BASE_URL}/component/?name=carousel&variant=main&`, { timeout: 20 * 60 * 1000 });
  const frame = page.locator("#component-preview-frame");
  // The same element carousel.spec.ts's `readContentTransform` reads: it is
  // the scroll-snap track AND the element the band's transform is written to.
  const content = frame.locator(".dx-carousel-content");
  await expect(frame.getByRole("group", { name: "1 of 5" })).toHaveAttribute("data-selected", "true");
  // Scrolling the demo into view is left to the browser's own instant scroll
  // (fixtures.ts forces `scroll-behavior: auto`).
  await content.scrollIntoViewIfNeeded();
  return { content };
}

/*
 * The component's intended contract. RED against main's old engine when this
 * file was added (`CAROUSEL_WHEEL_BOUNCE_JS`'s plateau/velocity release, no
 * owned-vs-native split, no notched or axis rule) and green once mode D's
 * edge-band engine is ported -- nothing here is skipped. Baseline on main
 * (430c52b, release SSG build, Chromium, 336px-wide track, bound = 168px):
 *   - owned-push-flick / -hard, end-edge: FAIL the bound (peaks 234 / 264 /
 *     288px); the release itself is fine (home within 300ms).
 *   - native-arrival-a / -b: FAIL, the old engine draws a band (167 / 146px)
 *     for momentum arriving at the edge.
 *   - native-arrival-repush-start / -end: FAIL "no band before the re-push"
 *     (228 / 214px drawn while still coasting in).
 *   - notched: FAIL, settles 400ms after the last click (contract 300ms).
 *   - vertical-only: FAIL, draws a 169px band from dy-only events.
 *   - slow-push-a / -b: PASS (spring-back already under way by +400ms).
 */
test.describe("Carousel wheel replay: component (intended contract)", () => {
  for (const rec of byKind("owned-push")) {
    test(`owned-push ${rec.id}: band bounded to 0.5 x width, returns to 0`, async ({ page }) => {
      const { content } = await openComponent(page);
      await place(content, rec);
      const width = await widthOf(content);
      const r = await replayWheel(content, rec);
      const summary = summarize(r);
      expect(peakAbs(r), `band appeared (${summary})`).toBeGreaterThan(BAND_MIN_PX);
      expect(peakAbs(r), `band <= 0.5 x width ${width} (${summary})`).toBeLessThanOrEqual(0.5 * width);
      const settled = settledAfter(r);
      expect(settled, `returned to 0 (${summary})`).not.toBeNull();
      expect(settled!, summary).toBeLessThanOrEqual(1000);
    });
  }

  for (const rec of byKind("end-edge")) {
    test(`end-edge ${rec.id}: band toward the end side, bounded, returns to 0`, async ({ page }) => {
      const { content } = await openComponent(page);
      await place(content, rec);
      const width = await widthOf(content);
      const r = await replayWheel(content, rec);
      const summary = summarize(r);
      expect(peakAbs(r), summary).toBeGreaterThan(BAND_MIN_PX);
      expect(peakAbs(r), summary).toBeLessThanOrEqual(0.5 * width);
      expect(Math.min(...r.samples.map((s) => s.band)), summary).toBeLessThan(0);
      expect(Math.max(...r.samples.map((s) => s.band)), summary).toBeLessThanOrEqual(NO_BAND_PX);
      const settled = settledAfter(r);
      expect(settled, summary).not.toBeNull();
      expect(settled!, summary).toBeLessThanOrEqual(1000);
    });
  }

  for (const rec of byKind("slow-push")) {
    test(`slow-push ${rec.id}: releases within ~400ms of the last event`, async ({ page }) => {
      const { content } = await openComponent(page);
      await place(content, rec);
      const r = await replayWheel(content, rec);
      const summary = summarize(r);
      expect(peakAbs(r), `a slow push still draws a band (${summary})`).toBeGreaterThan(NO_BAND_PX);
      const settled = settledAfter(r);
      const atLast = Math.abs(valueAt(r, 0));
      const at400 = Math.abs(valueAt(r, 400));
      expect(settled !== null || at400 < atLast - 0.25, `spring-back under way by +400ms (${summary})`).toBe(true);
      expect(settled, `fully home (${summary})`).not.toBeNull();
    });
  }

  for (const rec of byKind("native-arrival")) {
    test(`native-arrival ${rec.id}: momentum reaching the edge draws no band`, async ({ page }) => {
      const { content } = await openComponent(page);
      await place(content, rec);
      const r = await replayWheel(content, rec, { jump: momentumArrival(rec) });
      expect(r.jumped, "the momentum-arrival emulation ran").toBe(true);
      expect(peakAbs(r), summarize(r)).toBeLessThanOrEqual(NO_BAND_PX);
    });
  }

  for (const rec of byKind("native-arrival-repush")) {
    test(`native-arrival-repush ${rec.id}: no band while coasting in, a band after the re-push at event ${rec.repushIndex}`, async ({
      page,
    }) => {
      const { content } = await openComponent(page);
      await place(content, rec);
      const width = await widthOf(content);
      const r = await replayWheel(content, rec, { jump: momentumArrival(rec) });
      const summary = summarize(r);
      expect(r.jumped).toBe(true);
      const tRepush = eventTime(r, rec.repushIndex!);
      const before = r.samples.filter((s) => s.t < tRepush - 1);
      expect(Math.max(...before.map((s) => Math.abs(s.band))), `no band before the re-push (${summary})`).toBeLessThanOrEqual(
        NO_BAND_PX,
      );
      expect(peakAbsSince(r, tRepush - 1), `band after the re-push (${summary})`).toBeGreaterThan(BAND_MIN_PX);
      expect(peakAbs(r), summary).toBeLessThanOrEqual(0.5 * width);
      const settled = settledAfter(r);
      expect(settled, summary).not.toBeNull();
      expect(settled!, summary).toBeLessThanOrEqual(1000);
    });
  }

  for (const rec of byKind("notched")) {
    test(`notched ${rec.id} (synthetic): a mouse-wheel click releases within ~300ms of the last click`, async ({ page }) => {
      const { content } = await openComponent(page);
      await place(content, rec);
      const width = await widthOf(content);
      const r = await replayWheel(content, rec);
      const summary = summarize(r);
      expect(peakAbs(r), summary).toBeLessThanOrEqual(0.5 * width);
      const settled = settledAfter(r);
      expect(settled, `released (${summary})`).not.toBeNull();
      expect(settled!, summary).toBeLessThanOrEqual(300);
    });
  }

  for (const rec of byKind("vertical-only")) {
    test(`vertical-only ${rec.id} (synthetic): vertical page scrolling over the carousel draws NO band`, async ({ page }) => {
      const { content } = await openComponent(page);
      await place(content, rec);
      const r = await replayWheel(content, rec);
      expect(peakAbs(r), summarize(r)).toBeLessThanOrEqual(NO_BAND_PX);
    });
  }
});
