/**
 * Main-thread-responsiveness measurement helper for
 * `oracle/tier2-html/main-thread.spec.ts` (dev-docs/backlog.md row 45).
 * Paired 1:1 with that spec the way `assert-close-animation.ts` is paired
 * with `accordion-animation.spec.ts` -- the spec file's own header carries
 * the full narrative (why this exists, the subject inventory, the
 * robustness policy, the red-proof method); this file is the mechanism.
 *
 * What it does: installs a `PerformanceObserver` for the W3C Long Tasks
 * API and the W3C Event Timing API in the page before any of the page's
 * own scripts run, buffers what they report, and gives the spec a small
 * measure-one-gesture-at-a-time API.
 *
 * Field/threshold sources (both verified reachable, HTTP 200, checked
 * 2026-09-18):
 *   - Long Tasks API -- `PerformanceLongTaskTiming.attribution`
 *     (`TaskAttributionTiming`'s `containerType`/`containerSrc`/
 *     `containerId`/`containerName`):
 *     https://www.w3.org/TR/longtasks-1/#sec-PerformanceLongTaskTiming
 *     https://www.w3.org/TR/longtasks-1/#sec-TaskAttributionTiming
 *   - Event Timing API -- `durationThreshold` on `PerformanceObserverInit`,
 *     `interactionId`, `processingStart`/`processingEnd` on
 *     `PerformanceEventTiming`. The spec clamps `durationThreshold` itself
 *     ("Should add PerformanceEventTiming" algorithm): "Let minDuration be
 *     the maximum between 16 and options's durationThreshold value" -- so
 *     `EVENT_DURATION_THRESHOLD_MS` below (16) is already the floor, not
 *     an arbitrary choice:
 *     https://www.w3.org/TR/event-timing/#dom-performanceobserverinit-durationthreshold
 *     https://www.w3.org/TR/event-timing/#dom-performanceeventtiming-interactionid
 *
 * Why `page.addInitScript`, not `page.evaluate` after navigation: an
 * observer installed after the page's own scripts start running would
 * miss whatever already happened before that call landed.
 * `addInitScript` runs before any script on the page, on every
 * navigation, so a fresh `page.goto` always gets a fresh, from-the-start
 * observer with no gap. Also pass `buffered: true` on both observers
 * (dev-docs/backlog.md row 45's own note) as a second line of defense --
 * belt and braces, not a substitute for installing early.
 *
 * Excluding what the page did not cause (first-load tasks, and anything
 * before the gesture being measured): `withGesture` resets the buffer
 * immediately before running the gesture AND filters the entries it
 * returns to `startTime >= t0`, where `t0` is a fresh `performance.now()`
 * mark taken right after that reset -- so double-covered even if the
 * reset call and the mark do not land in the exact same microtask. This
 * is why a test never needs to (and should not) read the report before
 * its own gesture -- wasm-boot/hydration long tasks are startup, not an
 * interaction, and this oracle is about interactions.
 */
import { test, type Page } from "@playwright/test";

/** https://www.w3.org/TR/longtasks-1/#sec-TaskAttributionTiming */
export interface LongTaskAttribution {
  containerType: string;
  containerSrc: string;
  containerId: string;
  containerName: string;
}

/** https://www.w3.org/TR/longtasks-1/#sec-PerformanceLongTaskTiming */
export interface LongTaskEntry {
  name: string;
  startTime: number;
  duration: number;
  attribution: LongTaskAttribution[];
}

/** https://www.w3.org/TR/event-timing/#sec-performance-event-timing */
export interface EventTimingEntry {
  name: string;
  startTime: number;
  duration: number;
  processingStart: number;
  processingEnd: number;
  interactionId: number;
}

export interface GestureReport {
  longTasks: LongTaskEntry[];
  interactions: EventTimingEntry[];
  maxTaskMs: number;
  maxInteractionMs: number;
}

/**
 * dev-docs/recommended-implementations.md §11 (kciter, "The Browser's Main
 * Thread Is Expensive"): "any single task over 50 ms is a 'long task'".
 * This is a reference value from that article, not a tuning knob -- never
 * lower it to make a test pass; see main-thread.spec.ts's header.
 */
export const LONG_TASK_THRESHOLD_MS = 50;

/**
 * dev-docs/backlog.md row 45: "no interaction over 200 ms". Same rule as
 * above -- never lower it.
 */
export const INTERACTION_THRESHOLD_MS = 200;

/** `durationThreshold` passed to the 'event' observer. The Event Timing
 * spec clamps anything lower to 16ms anyway (see file header), so this is
 * already the floor. */
export const EVENT_DURATION_THRESHOLD_MS = 16;

/** Property name used on `window` in the page to stash buffered entries. */
const REPORT_KEY = "__dxMainThreadReport";

/**
 * Installs the long-task + event-timing observers before any page script
 * runs. Call once per `page`, before its first `page.goto` -- it
 * reinstalls automatically on every subsequent navigation (that is the
 * point of `addInitScript`), so one call covers a whole test.
 */
export async function installMainThreadObserver(page: Page): Promise<void> {
  await page.addInitScript(
    ({ key, durationThreshold }: { key: string; durationThreshold: number }) => {
      const w = window as any;
      w[key] = { longTasks: [], events: [] };

      try {
        new PerformanceObserver((list) => {
          for (const entry of list.getEntries() as any[]) {
            const attribution = Array.isArray(entry.attribution) ? entry.attribution : [];
            w[key].longTasks.push({
              name: entry.name,
              startTime: entry.startTime,
              duration: entry.duration,
              attribution: attribution.map((a: any) => ({
                containerType: a.containerType,
                containerSrc: a.containerSrc,
                containerId: a.containerId,
                containerName: a.containerName,
              })),
            });
          }
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
        }).observe({ type: "longtask", buffered: true } as any);
      } catch {
        // Long Tasks API unsupported in this engine (Firefox/WebKit as of
        // this writing) -- leave the array empty rather than throwing out
        // of an init script, which would break every page load in the
        // test. main-thread.spec.ts skips non-Chromium projects outright;
        // this try/catch is the second line of defense for any other
        // caller of this helper.
      }

      try {
        new PerformanceObserver((list) => {
          for (const entry of list.getEntries() as any[]) {
            w[key].events.push({
              name: entry.name,
              startTime: entry.startTime,
              duration: entry.duration,
              processingStart: entry.processingStart,
              processingEnd: entry.processingEnd,
              interactionId: entry.interactionId ?? 0,
            });
          }
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
        }).observe({ type: "event", durationThreshold, buffered: true } as any);
      } catch {
        // 'event' entry type / durationThreshold unsupported -- same
        // reasoning as above.
      }
    },
    { key: REPORT_KEY, durationThreshold: EVENT_DURATION_THRESHOLD_MS }
  );
}

/** Raw buffered entries seen so far (since the last reset). Prefer
 * `withGesture` for actually measuring an interaction -- this and
 * `resetMainThreadReport` are the primitives it is built from, exposed
 * for a test that needs to inspect the buffer directly. */
export async function readMainThreadReport(
  page: Page
): Promise<{ longTasks: LongTaskEntry[]; events: EventTimingEntry[] }> {
  return page.evaluate((key) => {
    const w = window as any;
    return w[key] ?? { longTasks: [], events: [] };
  }, REPORT_KEY);
}

/** Clears the buffered entries in the page. Used by `withGesture` right
 * before each gesture so first-load/earlier-test noise never counts
 * toward the gesture being measured; exposed standalone too, per this
 * file's design brief. */
export async function resetMainThreadReport(page: Page): Promise<void> {
  await page.evaluate((key) => {
    const w = window as any;
    if (w[key]) {
      w[key].longTasks = [];
      w[key].events = [];
    }
  }, REPORT_KEY);
}

/** Extra wait after a gesture before reading the buffer. A
 * `PerformanceObserver` callback fires asynchronously once its entries
 * exist -- and a long task's own entry cannot exist until the task
 * itself has finished -- so a gesture that ends right as its own long
 * task ends needs one more tick for that entry to actually land in the
 * buffer before `readMainThreadReport` reads it. */
const SETTLE_MS = 150;

/**
 * Measures one gesture: resets the buffer, marks the start time, runs
 * `gesture`, waits `SETTLE_MS` for trailing observer callbacks to flush,
 * then returns everything recorded at or after the start mark plus the
 * two maxima the spec asserts on. Never call this to measure page load --
 * the reset it performs is exactly what discards first-load tasks (see
 * file header, "Excluding what the page did not cause").
 */
export async function withGesture(page: Page, gesture: () => Promise<void>): Promise<GestureReport> {
  await resetMainThreadReport(page);
  const t0 = await page.evaluate(() => performance.now());
  await gesture();
  await page.waitForTimeout(SETTLE_MS);
  const raw = await readMainThreadReport(page);
  const longTasks = raw.longTasks.filter((t) => t.startTime >= t0);
  const interactions = raw.events.filter((e) => e.startTime >= t0);
  const maxTaskMs = longTasks.reduce((m, t) => Math.max(m, t.duration), 0);
  const maxInteractionMs = interactions.reduce((m, e) => Math.max(m, e.duration), 0);
  return { longTasks, interactions, maxTaskMs, maxInteractionMs };
}

function worstOf(report: GestureReport): number {
  return Math.max(report.maxTaskMs, report.maxInteractionMs);
}

function withinThresholds(report: GestureReport): boolean {
  return report.maxTaskMs <= LONG_TASK_THRESHOLD_MS && report.maxInteractionMs <= INTERACTION_THRESHOLD_MS;
}

export interface AssertResponsiveOptions {
  /** Max attempts (default 3). See main-thread.spec.ts's header,
   * "Robustness under load": a genuine main-thread block reproduces on
   * every attempt, a scheduling hiccup does not, so the first passing
   * attempt is accepted and no further attempts run. */
  attempts?: number;
  /** Run before each RETRY (attempt 2+ only, never before attempt 1) to
   * put the page back into whatever state `gesture` expects -- e.g.
   * re-navigate and reopen an overlay the previous attempt's own gesture
   * closed (a close gesture) or dismissed (a drag-to-dismiss gesture). */
  reset?: () => Promise<void>;
}

/**
 * Runs `gesture` under `withGesture`, retrying per `opts` and keeping the
 * best (lowest combined-max) attempt, stopping as soon as one attempt is
 * within both thresholds. Always records a `test.info().annotations`
 * entry with the best attempt's numbers -- pass or fail -- so a green run
 * still shows what was measured, not just that it passed. Throws (failing
 * the test) if every attempt exceeds a threshold, naming the worst
 * offending long task/interaction. `LONG_TASK_THRESHOLD_MS` and
 * `INTERACTION_THRESHOLD_MS` are read directly from this module and are
 * never parameterized down by a caller.
 *
 * A `gesture` is expected to also assert (per the spec file's own
 * `open`/`close`/`interact` shapes) that it actually engaged the
 * component -- e.g. `expect(thumb).toHaveAttribute("aria-valuenow", "80")`
 * after a drag -- so a broken locator is a normal failure, not a silently
 * vacuous measurement. That means `gesture` (and so `withGesture`) can
 * throw for a reason that has nothing to do with main-thread timing: a
 * real pointer drag occasionally not registering under this sandbox's own
 * CPU contention is exactly the kind of transient failure the retry
 * policy exists to absorb (found live while writing this file: an
 * un-caught throw from attempt 1 used to escape this loop entirely,
 * skipping attempts 2-3 and this function's own summary/annotation).
 * Each attempt is therefore wrapped in its own try/catch: a thrown
 * attempt counts against the attempt budget and is retried like an
 * over-threshold one, and only a `gesture` that fails on literally every
 * attempt re-throws (the last attempt's own error, since a functional
 * failure that never once produced a measurement is a different kind of
 * red than "slow" and deserves its own real stack, not a synthesized one).
 */
export async function assertResponsive(
  page: Page,
  label: string,
  gesture: () => Promise<void>,
  opts: AssertResponsiveOptions = {}
): Promise<GestureReport> {
  const attempts = Math.max(1, opts.attempts ?? 3);
  const all: GestureReport[] = [];
  let best: GestureReport | undefined;
  let lastError: unknown;

  for (let attempt = 1; attempt <= attempts; attempt++) {
    if (attempt > 1 && opts.reset) {
      await opts.reset();
    }
    let report: GestureReport;
    try {
      report = await withGesture(page, gesture);
    } catch (err) {
      lastError = err;
      continue;
    }
    lastError = undefined;
    all.push(report);
    if (!best || worstOf(report) < worstOf(best)) {
      best = report;
    }
    if (withinThresholds(report)) {
      break;
    }
  }

  if (!best) {
    // Every attempt threw before ever producing a measurement -- a
    // functional failure (e.g. a broken locator), not a performance one.
    // Re-throw as-is rather than inventing a main-thread-shaped message
    // for a failure that has nothing to do with timing.
    throw lastError ?? new Error(`${label}: gesture failed on every attempt with no error captured`);
  }

  const finalReport = best;
  const summary =
    `${label}: maxTask=${finalReport.maxTaskMs.toFixed(1)}ms (threshold ${LONG_TASK_THRESHOLD_MS}ms), ` +
    `maxInteraction=${finalReport.maxInteractionMs.toFixed(1)}ms (threshold ${INTERACTION_THRESHOLD_MS}ms) ` +
    `-- best of ${all.length} measured attempt(s)`;

  try {
    test.info().annotations.push({ type: "main-thread", description: summary });
  } catch {
    // test.info() only resolves while a test is actually executing --
    // never load-bearing for the measurement itself.
  }

  if (!withinThresholds(finalReport)) {
    const worstTask = [...finalReport.longTasks].sort((a, b) => b.duration - a.duration)[0];
    const worstInteraction = [...finalReport.interactions].sort((a, b) => b.duration - a.duration)[0];
    throw new Error(
      `${summary}. ` +
        `Worst long task: ${worstTask ? JSON.stringify(worstTask) : "none"}. ` +
        `Worst interaction: ${worstInteraction ? JSON.stringify(worstInteraction) : "none"}.`
    );
  }

  return finalReport;
}
