/**
 * Replay real trackpad wheel recordings against a scroller and sample what
 * it does -- both entirely IN the page.
 *
 * WHY IN-PAGE (dev-docs/backlog.md row 112): an animated value (a rubber-band
 * transform, a spring-back tween) must never be sampled across `await`
 * round trips. Each CDP hop costs several ms of jitter and lands at an
 * arbitrary phase of the animation, so a test that reads the value between
 * `page.mouse.wheel()` calls asserts on a race, not on the engine. So this
 * module makes exactly ONE `evaluate` per replay: the page itself schedules
 * the events, samples on every animation frame, and resolves once the
 * settle window after the last event has elapsed.
 *
 * SCHEDULING: one requestAnimationFrame loop dispatches every event whose
 * recorded time has arrived and then samples. That is faithful to the
 * source: Firefox delivers wheel events aligned to frames (the recordings
 * average ~17ms per event), and it puts a dispatch and the sample that
 * follows it in the same frame. (A watchdog timer drives the same loop if
 * rAF is throttled, e.g. a background tab, so a replay cannot hang.)
 *
 * SYNTHETIC EVENTS DO NOT SCROLL: `dispatchEvent(new WheelEvent(...))` is an
 * untrusted event, so the browser never performs the default wheel scroll
 * for it. The scroller only moves if the test moves it. That is exactly what
 * the "native arrival" recordings need to emulate: real momentum carried the
 * scroller to the edge on its own, which no synthetic event can do (and
 * `page.mouse.wheel`'s trusted events are also unreliable here: headless
 * Chromium smooth-scrolls each one and `scroll-snap-stop: always` stops a
 * long gesture at every slide). `ReplayOptions.jump` therefore places the
 * scroller at the edge at a chosen event index, right before that event is
 * dispatched -- the emulation is documented at each call site.
 */

import fs from "node:fs";
import path from "node:path";
import type { Locator } from "@playwright/test";

export interface RecordedWheelEvent {
  /** ms since the first event of the recording. */
  t: number;
  dx: number;
  dy: number;
}

export type RecordingKind =
  | "owned-push"
  | "slow-push"
  | "native-arrival"
  | "native-arrival-repush"
  | "end-edge"
  | "notched"
  | "vertical-only";

export interface WheelRecording {
  id: string;
  /** `<session id>#<burst index>`, or "SYNTHETIC". */
  source: string;
  kind: RecordingKind;
  synthetic?: boolean;
  /** Where the scroller should rest when the replay starts. */
  startsAt: "start-edge" | "end-edge" | "mid-track";
  /** The edge the recording pushes toward. */
  edge: "start" | "end";
  deltaMode: 0 | 1 | 2;
  events: RecordedWheelEvent[];
  note: string;
  /** Native kinds: event index at which momentum is assumed to reach the edge. */
  arrivalIndex?: number;
  /** native-arrival-repush: index of the first event of the re-push. */
  repushIndex?: number;
}

/** Where to put the scroller, and when, during a replay. */
export interface ScrollJump {
  /** Applied immediately BEFORE the event with this index is dispatched. */
  atIndex: number;
  /** CSS selector (document-wide) of the scroller; default: the replay target. */
  scroller?: string;
  /** Absolute px, or the scroll extremes. */
  scrollLeft: number | "start" | "end";
}

export interface ReplayOptions {
  /**
   * Named JS expressions, evaluated on every animation frame with `target`
   * (the element the events are dispatched on) in scope; each must yield a
   * number. E.g. `{ band: TRANSLATE_X_EXPR }`. Default: `{ band: TRANSLATE_X_EXPR }`.
   */
  sample?: Record<string, string>;
  /** How long to keep sampling after the last event. Default 1500ms. */
  settleMs?: number;
  /** Emulate the browser's own momentum reaching an edge (see file doc). */
  jump?: ScrollJump;
}

export interface WheelSample {
  /** ms relative to the LAST event's dispatch (negative while events are still arriving). */
  t: number;
  [name: string]: number;
}

export interface ReplayResult {
  samples: WheelSample[];
  /** Dispatch time of each event, ms relative to the last event (so <= 0). */
  eventTimes: number[];
  /** Actual duration of the first-to-last event span, ms. */
  spanMs: number;
  /** Number of `jump` applications (0 or 1) -- proves the emulation ran. */
  jumped: boolean;
}

/**
 * Signed px of the target's inline `translateX(...)`/`translateY(...)`
 * (0 when the inline transform is empty). The carousel writes its band as an
 * inline style, never through a rendered attribute.
 */
export const TRANSLATE_X_EXPR =
  "(() => { const m = /translate[XY]?\\(\\s*(-?[\\d.]+)px/.exec(target.style.transform); return m ? parseFloat(m[1]) : 0; })()";

const RECORDINGS_PATH = path.resolve(__dirname, "fixtures", "carousel-wheel-recordings.json");

export function loadRecordings(): WheelRecording[] {
  const raw = JSON.parse(fs.readFileSync(RECORDINGS_PATH, "utf8"));
  return raw.recordings as WheelRecording[];
}

export function recordingsOfKind(kind: RecordingKind): WheelRecording[] {
  return loadRecordings().filter((r) => r.kind === kind);
}

/** Set the scroller's `scrollLeft` instantly (no smooth animation). */
export async function setScrollLeft(scroller: Locator, to: number | "start" | "end"): Promise<void> {
  await scroller.evaluate((el, to) => {
    const e = el as HTMLElement;
    const left = to === "start" ? 0 : to === "end" ? e.scrollWidth - e.clientWidth : to;
    e.scrollTo({ left, behavior: "instant" });
  }, to);
}

/**
 * Dispatch `recording`'s events on `target` with their recorded relative
 * timing and sample `opts.sample` on every animation frame until
 * `opts.settleMs` after the last event. One `evaluate`; see the file doc.
 */
export async function replayWheel(
  target: Locator,
  recording: WheelRecording,
  opts: ReplayOptions = {},
): Promise<ReplayResult> {
  const sample = opts.sample ?? { band: TRANSLATE_X_EXPR };
  const args = {
    events: recording.events,
    deltaMode: recording.deltaMode,
    sample,
    settleMs: opts.settleMs ?? 1500,
    jump: opts.jump ?? null,
  };
  return target.evaluate((el, a) => {
    return new Promise<ReplayResult>((resolve) => {
      const fns = Object.entries(a.sample).map(
        ([name, expr]) => [name, new Function("target", `return (${expr});`) as (t: Element) => unknown] as const,
      );
      const read = () => {
        const out: Record<string, number> = {};
        for (const [name, fn] of fns) {
          try {
            const v = Number(fn(el));
            out[name] = Number.isFinite(v) ? v : Number.NaN;
          } catch {
            out[name] = Number.NaN;
          }
        }
        return out;
      };
      const start = performance.now();
      const raw: Array<{ at: number; v: Record<string, number> }> = [];
      const dispatchedAt: number[] = [];
      let next = 0;
      let jumped = false;
      let lastTick = start;
      let done = false;

      const applyJump = () => {
        const j = a.jump!;
        const sc = (j.scroller ? document.querySelector(j.scroller) : el) as HTMLElement | null;
        if (!sc) throw new Error(`replayWheel: jump scroller ${j.scroller} not found`);
        const left =
          j.scrollLeft === "start" ? 0 : j.scrollLeft === "end" ? sc.scrollWidth - sc.clientWidth : j.scrollLeft;
        sc.scrollTo({ left, behavior: "instant" });
        jumped = true;
      };

      const tick = () => {
        if (done) return;
        const now = performance.now() - start;
        lastTick = performance.now();
        while (next < a.events.length && a.events[next].t <= now) {
          if (a.jump && !jumped && a.jump.atIndex === next) applyJump();
          const e = a.events[next];
          el.dispatchEvent(
            new WheelEvent("wheel", {
              deltaX: e.dx,
              deltaY: e.dy,
              deltaMode: a.deltaMode,
              bubbles: true,
              cancelable: true,
            }),
          );
          dispatchedAt.push(now);
          next++;
        }
        raw.push({ at: now, v: read() });
        const lastAt = next === a.events.length ? dispatchedAt[dispatchedAt.length - 1] : null;
        if (lastAt !== null && now >= lastAt + a.settleMs) {
          done = true;
          window.clearInterval(watchdog);
          resolve({
            samples: raw.map((r) => ({ t: r.at - lastAt, ...r.v })),
            eventTimes: dispatchedAt.map((t) => t - lastAt),
            spanMs: lastAt - dispatchedAt[0],
            jumped,
          });
          return;
        }
        schedule();
      };
      let framePending = false;
      const schedule = () => {
        if (framePending) return;
        framePending = true;
        window.requestAnimationFrame(() => {
          framePending = false;
          tick();
        });
      };
      // rAF is the driver; the watchdog only steps in when rAF stalls.
      const watchdog = window.setInterval(() => {
        if (performance.now() - lastTick > 50) tick();
      }, 25);
      schedule();
    });
  }, args);
}

/* ---------------------------------------------------------------- analysis */

/** Largest |value| of `key` over the whole replay. */
export function peakAbs(r: ReplayResult, key = "band"): number {
  return r.samples.reduce((m, s) => Math.max(m, Math.abs(s[key])), 0);
}

/** Largest |value| among samples with `t >= sinceMs` (relative to the last event). */
export function peakAbsSince(r: ReplayResult, sinceMs: number, key = "band"): number {
  return r.samples.filter((s) => s.t >= sinceMs).reduce((m, s) => Math.max(m, Math.abs(s[key])), 0);
}

/** Value of `key` at the last sample taken at or before `tMs` (relative to the last event). */
export function valueAt(r: ReplayResult, tMs: number, key = "band"): number {
  let v = r.samples[0][key];
  for (const s of r.samples) {
    if (s.t > tMs) break;
    v = s[key];
  }
  return v;
}

/**
 * Milliseconds after the last event at which |value| is at most `eps` and
 * stays there for the rest of the replay: `0` if it already was, `null` if it
 * never got (and stayed) there within the settle window.
 */
export function settledAfter(r: ReplayResult, eps = 0.5, key = "band"): number | null {
  const after = r.samples.filter((s) => s.t >= 0);
  let lastAbove = -1;
  after.forEach((s, i) => {
    if (Math.abs(s[key]) > eps) lastAbove = i;
  });
  if (lastAbove === -1) return 0;
  if (lastAbove === after.length - 1) return null;
  return Math.max(0, after[lastAbove + 1].t);
}

/** One-line human summary, used in failure messages and annotations. */
export function summarize(r: ReplayResult, key = "band"): string {
  const settled = settledAfter(r, 0.5, key);
  return (
    `peak=${peakAbs(r, key).toFixed(1)}px at-last-event=${valueAt(r, 0, key).toFixed(1)}px ` +
    `+400ms=${valueAt(r, 400, key).toFixed(1)}px settled=${settled === null ? "never" : `${Math.round(settled)}ms`} ` +
    `frames=${r.samples.length}`
  );
}
