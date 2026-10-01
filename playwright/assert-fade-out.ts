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

/**
 * One CSS animation/transition that started on the closing content, recorded
 * from its `animationstart`/`transitionrun` EVENT -- not from a frame
 * sample -- so it is captured however starved the page is (events are
 * queued, never dropped, unlike `requestAnimationFrame` ticks).
 */
export interface FadeAnimationRecord {
  /** `animationName` / `propertyName`. */
  name: string;
  /** The animation's own resolved duration in ms (`getComputedTiming().duration`), 0 if it could not be read. */
  durationMs: number;
  /** Whether its keyframes (or, for a transition, its property) include `opacity`. */
  animatesOpacity: boolean;
}

/** Everything one close-fade capture records: per-frame samples plus the animations seen start. */
export interface FadeCapture {
  samples: FadeSample[];
  animations: FadeAnimationRecord[];
}


export interface FadeOutReport {
  /** Every sample captured while the element was still attached (`exists: true`). */
  mountedSamples: FadeSample[];
  /** How many sampled frames had opacity strictly lower than the previous one -- informational only; load-dependent, never asserted on (see the note below `NEAR_ZERO_OPACITY`). */
  framesWithDroppingOpacity: number;
  /** The opacity of the last mounted sample -- must be ~0 before unmount. */
  finalMountedOpacity: number;
  /** Whether a real opacity animation/transition (duration > 0) started on the content while closing. */
  sawOpacityAnimation: boolean;
  /** Whether the element was unmounted (no longer `exists`) before sampling's own cap ran out. */
  unmountedWithinCap: boolean;
  /** ms from the close trigger to unmount, or `null` if it never unmounted within the cap. */
  timeToUnmountMs: number | null;
}

/** Opacity at or below this counts as "~0" for the "reaches ~0 before unmount" check. */
const NEAR_ZERO_OPACITY = 0.05;

/*
 * There is deliberately NO "at least N frames of decreasing opacity"
 * threshold (the original asserted N=3). How many `requestAnimationFrame`
 * ticks land inside a ~100-150ms fade depends on how starved the page's main
 * thread is, so any frame-count threshold is a load-dependent assertion: it
 * went red 3 times in 20 under `--workers=4`, with three different sample
 * shapes (`[1, 0.04, 0]`, `[0.15, 0.04, 0]`, `[0, 0, ...]`), and under 25x
 * CPU throttling the first frame can land after the whole fade is over
 * (dev-docs/backlog.md row 115). "It fades rather than snaps" is asserted
 * from the animation's own `animationstart`/`transitionrun` event instead
 * (`FadeCapture.animations`), which fires however few frames are rendered;
 * the per-frame samples keep only the invariants that hold at ANY sampling
 * rate (never display:none while mounted, never loses :popover-open, opacity
 * never increases, ends ~0, unmounts in time).
 */

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
): Promise<FadeCapture> {
  return page.evaluate(
    ({ id, capMs }) => {
      return new Promise<FadeCapture>((resolve) => {
        const frames: FadeSample[] = [];
        const animations: FadeAnimationRecord[] = [];
        const t0 = performance.now();

        // Listeners go on first, synchronously, before the close trigger can
        // fire -- see this function's doc. `animationstart`/`transitionrun`
        // are dispatched for every started animation even if no frame is
        // ever sampled in between.
        const target = document.getElementById(id);
        const record = (name: string, isTransition: boolean) => {
          const el = document.getElementById(id);
          const anim = el
            ?.getAnimations()
            .find((a) => (isTransition ? (a as CSSTransition).transitionProperty === name : (a as CSSAnimation).animationName === name));
          const effect = anim?.effect as KeyframeEffect | null | undefined;
          const duration = Number(effect?.getComputedTiming().duration ?? 0);
          animations.push({
            name,
            durationMs: Number.isFinite(duration) ? duration : 0,
            animatesOpacity: isTransition ? name === "opacity" : !!effect?.getKeyframes().some((k) => "opacity" in k),
          });
        };
        target?.addEventListener("animationstart", (e) => record((e as AnimationEvent).animationName, false));
        target?.addEventListener("transitionrun", (e) => record((e as TransitionEvent).propertyName, true));

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
            resolve({ samples: frames, animations });
          }
        }
        requestAnimationFrame(tick);
      });
    },
    { id: contentId, capMs },
  );
}

/**
 * Waits, in-page, until `contentId`'s OPEN transition has finished: every
 * animation/transition attached to it has settled and its computed opacity
 * is 1. Playwright's `toBeVisible()` does not wait for this -- it counts an
 * `opacity: 0` element as visible -- so under load a close trigger fired
 * right after it lands mid-fade-IN: the close then starts from a low opacity
 * (samples `[0, 0, ...]` or `[0.15, 0.04, 0]`) and there is nothing left to
 * fade out (dev-docs/backlog.md row 115).
 */
export function awaitOpenSettled(page: Page, contentId: string): Promise<void> {
  return page.evaluate(async (id) => {
    const deadline = performance.now() + 10_000;
    for (;;) {
      const el = document.getElementById(id);
      if (!el) throw new Error(`content ${id} is not in the DOM, so it cannot have finished opening`);
      await Promise.all(el.getAnimations().map((a) => a.finished.catch(() => undefined)));
      const unsettled = el.getAnimations().some((a) => a.playState === "running" || a.playState === "pending");
      if (!unsettled && Number(getComputedStyle(el).opacity) >= 0.999) return;
      if (performance.now() > deadline) {
        throw new Error(`content ${id} never finished opening (opacity ${getComputedStyle(el).opacity})`);
      }
      await new Promise((r) => requestAnimationFrame(r));
    }
  }, contentId);
}

/**
 * The one entry point the consuming specs use: settle the open transition
 * (`awaitOpenSettled`), start the in-page sampler (`startFadeSampling`),
 * fire `closeTrigger`, and resolve to the `FadeCapture`. One call, so a spec cannot
 * forget the settle step or start sampling after the trigger.
 */
export async function sampleCloseFade(
  page: Page,
  contentId: string,
  closeTrigger: () => Promise<unknown>,
  capMs: number = DEFAULT_SAMPLE_CAP_MS,
): Promise<FadeCapture> {
  await awaitOpenSettled(page, contentId);
  const framesPromise = startFadeSampling(page, contentId, capMs);
  await closeTrigger();
  return framesPromise;
}

/** Pure analysis over an already-captured `FadeCapture` -- no page access, easy to unit-reason about and to reuse across the three consuming specs. */
export function analyzeFadeSamples(capture: FadeCapture): FadeOutReport {
  const { samples, animations } = capture;
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
    } else if (mountedSamples[i].opacity > mountedSamples[i - 1].opacity + 0.001) {
      throw new Error(
        `opacity INCREASED from ${mountedSamples[i - 1].opacity} to ${mountedSamples[i].opacity} at ` +
          `t=${mountedSamples[i].t.toFixed(1)}ms while closing -- the content re-opened or was still ` +
          `fading in. Samples: ${JSON.stringify(mountedSamples)}`,
      );
    }
  }

  const finalMountedSample = mountedSamples[mountedSamples.length - 1];
  const lastSample = samples[samples.length - 1];
  const unmountedWithinCap = !lastSample.exists;

  return {
    mountedSamples,
    framesWithDroppingOpacity,
    finalMountedOpacity: finalMountedSample.opacity,
    sawOpacityAnimation: animations.some((a) => a.animatesOpacity && a.durationMs > 0),
    unmountedWithinCap,
    timeToUnmountMs: unmountedWithinCap ? lastSample.t : null,
  };
}

/**
 * The three assertions docs/backlog.md row 7 asks for: (a) it FADES rather
 * than snaps -- a real opacity animation/transition (duration > 0) started on
 * the closing content, stays displayed and (web arm) `:popover-open` the
 * whole time, and its opacity never increases [the per-frame invariants are
 * folded into `analyzeFadeSamples` above rather than repeated here],
 * (b) reaches ~0 opacity before unmount, (c) unmounts within ~1.5s. Throws
 * with the full capture on any failure; returns the report on success in
 * case a caller wants to assert anything additional.
 */
export function assertFadesOutThenUnmounts(capture: FadeCapture): FadeOutReport {
  const report = analyzeFadeSamples(capture);

  if (!report.sawOpacityAnimation) {
    throw new Error(
      `no opacity animation/transition (duration > 0) ever started on the closing content -- it snapped ` +
        `rather than faded (docs/backlog.md row 7). Animations seen: ${JSON.stringify(capture.animations)}. ` +
        `Samples: ${JSON.stringify(report.mountedSamples)}`,
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
