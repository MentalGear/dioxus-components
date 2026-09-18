/**
 * Shared close-fade sampler/assertions for dev-docs/backlog.md rows 19 and
 * 7: `Tooltip`, `HoverCard` and non-modal `Popover` content is promoted to
 * the top layer with the `popover` attribute
 * (`primitives/src/top_layer.rs::use_popover_shown_while_mounted`, called
 * from the web leaves in `tooltip.rs`/`hover_card.rs`/`popover.rs`) and
 * held mounted by `use_animated_open` (`primitives/src/lib.rs`) for its
 * whole CSS close animation (plus a settle hold) before actually
 * unmounting. Row 19 was that the old `use_popover_sync` called
 * `hidePopover()` the instant `open` went `false`, so the UA rule
 * `[popover]:not(:popover-open) { display: none }` hid the element before
 * the close animation this file's stylesheets define (row 7) ever got a
 * chance to play. This file provides the regression check for both: it
 * samples the closing content every animation frame while it is still in
 * the DOM and asserts it (a) stays displayed with decreasing opacity for a
 * few frames, (b) reaches ~0 opacity before unmount, and (c) unmounts
 * promptly rather than leaking.
 *
 * Modeled on `assert-close-animation.ts` (shared analysis, imported by
 * each consuming spec) and `accordion-animation.spec.ts`'s rAF sampling
 * loop shape -- but sampling `opacity`/`display`/`:popover-open` instead
 * of height, and with thresholds of its own (opacity/time, not height/
 * plateau-frames -- do not reuse `assert-close-animation.ts`'s height
 * thresholds, they measure a different physical quantity). Unlike
 * `accordion-animation.spec.ts` (one spec, two sampling *modes* for the
 * same component), `Tooltip`/`HoverCard`/`Popover` are three different
 * components whose close trigger differs (mouse-leave, mouse-leave,
 * script-driven toggle) but whose sampling loop is otherwise identical --
 * so the sampling loop itself is shared here, not just the analysis, to
 * avoid three copies of the same `page.evaluate` snippet.
 */
import type { Page } from "@playwright/test";

export interface FadeSample {
  /** ms since sampling started (i.e. since just before the close trigger fired). */
  t: number;
  /** `getComputedStyle(el).opacity`, parsed as a number in [0, 1]. `NaN` when unmounted. */
  opacity: number;
  /** `getComputedStyle(el).display`. `""` when unmounted. */
  display: string;
  /** Whether the element is still attached to `document.body`. */
  exists: boolean;
  /**
   * `el.matches(':popover-open')` when the element carries a `popover`
   * attribute (the web arm); `null` when it doesn't (native/Blitz arm, or
   * unmounted) -- so a caller can require this to stay `true` on the web
   * arm specifically without misreading a component that has no popover
   * attribute at all as a failure.
   */
  popoverOpen: boolean | null;
}

export interface FadeOutReport {
  /** Every sample captured while the element was still attached (`exists: true`). */
  mountedSamples: FadeSample[];
  /** How many consecutive mounted frames had opacity strictly lower than the previous one. */
  framesWithDroppingOpacity: number;
  /** The opacity of the last mounted sample -- must be ~0 before unmount. */
  finalMountedOpacity: number;
  /** Whether the element was unmounted (no longer `exists`) before sampling's own cap ran out. */
  unmountedWithinCap: boolean;
  /** ms from the close trigger to unmount, or `null` if it never unmounted within the cap. */
  timeToUnmountMs: number | null;
}

/** Opacity at or below this counts as "~0" for the "reaches ~0 before unmount" check. */
const NEAR_ZERO_OPACITY = 0.05;

/** Minimum number of frames the opacity must be observed strictly decreasing across, while mounted and displayed. */
const MIN_DECREASING_FRAMES = 3;

/** The element must be unmounted within this long of the close trigger (docs/backlog.md row 7's "~1.5s"). */
const MAX_UNMOUNT_MS = 1500;

/** Hard cap on the in-page sampling loop itself -- comfortably past `MAX_UNMOUNT_MS` so a genuine "too slow to unmount" failure is captured as data (`unmountedWithinCap: false`), not silently truncated at exactly the assertion threshold. */
const DEFAULT_SAMPLE_CAP_MS = 3000;

/**
 * Starts an in-page `requestAnimationFrame` sampling loop for `contentId`'s
 * computed `opacity`/`display` (and, once it carries a `popover` attribute,
 * `:popover-open`) every frame, until the element is removed from
 * `document.body` or `capMs` elapses.
 *
 * Call this *before* triggering the close (mirrors
 * `accordion-animation.spec.ts`'s `sampleCloseInApp`: `page.evaluate` starts
 * the loop executing in-page immediately, before the returned promise is
 * ever awaited, so calling the close trigger right after -- without
 * awaiting this function first -- never misses the first frames). Await
 * the returned promise after triggering the close to get every sample.
 */
export function startFadeSampling(
  page: Page,
  contentId: string,
  capMs: number = DEFAULT_SAMPLE_CAP_MS,
): Promise<FadeSample[]> {
  return page.evaluate(
    ({ id, capMs }) => {
      return new Promise<FadeSample[]>((resolve) => {
        const frames: FadeSample[] = [];
        const t0 = performance.now();
        function tick() {
          const el = document.getElementById(id);
          const exists = !!el && document.body.contains(el);
          let opacity = NaN;
          let display = "";
          let popoverOpen: boolean | null = null;
          if (exists && el) {
            const cs = getComputedStyle(el);
            opacity = Number(cs.opacity);
            display = cs.display;
            popoverOpen = el.hasAttribute("popover") ? el.matches(":popover-open") : null;
          }
          frames.push({ t: performance.now() - t0, opacity, display, exists, popoverOpen });
          if (exists && performance.now() - t0 < capMs) {
            requestAnimationFrame(tick);
          } else {
            resolve(frames);
          }
        }
        requestAnimationFrame(tick);
      });
    },
    { id: contentId, capMs },
  );
}

/** Pure analysis over already-captured samples -- no page access, easy to unit-reason about and to reuse across the three consuming specs. */
export function analyzeFadeSamples(samples: FadeSample[]): FadeOutReport {
  if (samples.length === 0) {
    throw new Error("no fade samples captured -- startFadeSampling must be called before the close trigger");
  }

  const mountedSamples = samples.filter((s) => s.exists);
  if (mountedSamples.length === 0) {
    throw new Error(
      "content was already unmounted on the very first sample -- sampling must start (startFadeSampling) " +
        "before the close trigger fires, or the close happened synchronously with no animation at all",
    );
  }

  // Row 19's actual defect, restated as an invariant: while still mounted,
  // the content must never be display:none (the old `hidePopover()`-on-
  // `open`-false bug) nor lose `:popover-open` on the web arm (the same
  // bug, restated in Popover-API terms -- losing `:popover-open` is what
  // *makes* the UA's `display: none` rule apply in the first place).
  for (const s of mountedSamples) {
    if (s.display === "none") {
      throw new Error(
        `content had display: none while still mounted at t=${s.t.toFixed(1)}ms -- the close animation ` +
          `cannot play once the element is display:none (docs/backlog.md row 19). Samples: ${JSON.stringify(mountedSamples)}`,
      );
    }
    if (s.popoverOpen === false) {
      throw new Error(
        `content lost :popover-open while still mounted at t=${s.t.toFixed(1)}ms -- hidePopover() ran ` +
          `before the close animation finished (docs/backlog.md row 19). Samples: ${JSON.stringify(mountedSamples)}`,
      );
    }
  }

  let framesWithDroppingOpacity = 0;
  for (let i = 1; i < mountedSamples.length; i++) {
    if (mountedSamples[i].opacity < mountedSamples[i - 1].opacity - 0.001) {
      framesWithDroppingOpacity++;
    }
  }

  const finalMountedSample = mountedSamples[mountedSamples.length - 1];
  const lastSample = samples[samples.length - 1];
  const unmountedWithinCap = !lastSample.exists;

  return {
    mountedSamples,
    framesWithDroppingOpacity,
    finalMountedOpacity: finalMountedSample.opacity,
    unmountedWithinCap,
    timeToUnmountMs: unmountedWithinCap ? lastSample.t : null,
  };
}

/**
 * The three assertions docs/backlog.md row 7 asks for: (a) stays displayed
 * with decreasing opacity for at least a few frames [+ stays
 * `:popover-open` on the web arm, folded into `analyzeFadeSamples`'s own
 * invariant above rather than repeated here], (b) reaches ~0 opacity
 * before unmount, (c) unmounts within ~1.5s. Throws with the full sample
 * trace on any failure; returns the report on success in case a caller
 * wants to assert anything additional.
 */
export function assertFadesOutThenUnmounts(samples: FadeSample[]): FadeOutReport {
  const report = analyzeFadeSamples(samples);

  if (report.framesWithDroppingOpacity < MIN_DECREASING_FRAMES) {
    throw new Error(
      `expected at least ${MIN_DECREASING_FRAMES} frames of decreasing opacity while mounted and displayed, ` +
        `got ${report.framesWithDroppingOpacity}. Samples: ${JSON.stringify(report.mountedSamples)}`,
    );
  }

  if (report.finalMountedOpacity > NEAR_ZERO_OPACITY) {
    throw new Error(
      `content's opacity never reached ~0 before unmount: last mounted opacity was ` +
        `${report.finalMountedOpacity} (must be < ${NEAR_ZERO_OPACITY}). Samples: ${JSON.stringify(report.mountedSamples)}`,
    );
  }

  if (!report.unmountedWithinCap || (report.timeToUnmountMs ?? Infinity) > MAX_UNMOUNT_MS) {
    throw new Error(
      `content did not unmount within ${MAX_UNMOUNT_MS}ms of the close trigger ` +
        `(timeToUnmountMs=${report.timeToUnmountMs}). Samples: ${JSON.stringify(report.mountedSamples)}`,
    );
  }

  return report;
}
