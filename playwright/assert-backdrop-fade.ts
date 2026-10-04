/**
 * Shared scrim sampler/assertions for every modal overlay built on a native
 * `<dialog>` + `showModal()`: `Dialog`, `AlertDialog`, `Sheet`, `Drawer`,
 * `CommandDialog` and the modal `Popover`. One helper, six specs, so "all six paint
 * the same scrim and fade it the same way in and out" is one claim, not six copies
 * of a measurement.
 *
 * WHAT WENT WRONG BEFORE (measured, not guessed): each component painted its own
 * scrim -- a hand-rolled wrapper `div` with its own colour and its own 100-200ms
 * keyframes -- on top of the UA's default `dialog::backdrop`
 * (`rgba(0 0 0 / 10%)`, no transition). The two stacked (peak 19% on Dialog, 37%
 * on AlertDialog, 55% on Drawer), and where there was no wrapper (modal Popover,
 * Sheet) the UA layer was the whole scrim and it snapped in and out in one frame.
 * Underneath that sits one platform fact: `dialog.close()` removes the dialog from
 * the top layer synchronously and the `::backdrop` goes with it, so no exit
 * transition could play however long it was declared.
 *
 * THE CONSTRUCTION these assertions pin down (see the six `style.css` files): the
 * dialog's own `::backdrop` is the ONLY painter (the wrapper is transparent), it
 * fades in via `@starting-style`, and both directions are the one `transition`
 * declaration so they cannot differ. What lets the exit play differs by component:
 * Dialog, AlertDialog, Sheet, Drawer and CommandDialog put `transition: overlay ... allow-discrete`
 * on the dialog, which holds it in the top layer after `close()`; the modal Popover
 * (whose anchored position is keyed on `:modal`, so it must stay modal) has
 * `popover.rs` defer `close()` until its exit animations settle. The numbers come
 * from one token set (`--dx-overlay-duration`, `-ease`, `-scrim`, `-blur`) defined
 * once in `dx-components-theme.css`, with no per-stylesheet fallback.
 *
 * Like `assert-fade-out.ts`, the duration/direction claims are read from the
 * browser's own `transitionrun` events (queued, never dropped) rather than from
 * frame counts, which depend on how starved the page's main thread is
 * (dev-docs/backlog.md row 115). The per-frame samples only back invariants that
 * hold at any sampling rate: a single painter, the right peak, and "faded back to
 * nothing before the dialog unmounted".
 */
import type { Page } from "@playwright/test";

/** The scrim every modal overlay paints: Nova's `bg-black/10`. */
export const SCRIM_ALPHA = 0.1;
/** Nova's `backdrop-blur-xs`. */
export const SCRIM_BLUR_PX = 4;
/** `--dx-motion-duration-slow`: what `--dx-overlay-duration` resolves to. The one number to retune. */
export const DEFAULT_OVERLAY_MS = 200;

/** How long a dialog may be open without any `::backdrop` transition starting before the capture stops waiting for one. */
const NO_TRANSITION_GRACE_MS = 600;

export interface BackdropTransition {
  /** `"close"` once the dialog was closing as the transition started (`[open]` gone, or its `data-state` already `"closed"`), else `"open"`. */
  phase: "open" | "close";
  property: string;
  /** The transition's own resolved duration in ms (`getComputedTiming().duration`). */
  durationMs: number;
}

/** `[t ms, wrapper contribution, ::backdrop contribution, dialog.open]` per animation frame. */
export type BackdropSample = [number, number, number, boolean];

export interface BackdropCapture {
  samples: BackdropSample[];
  transitions: BackdropTransition[];
  /** `getComputedStyle(dialog, "::backdrop")` once fully open. */
  settled: { backgroundColor: string; backdropFilter: string };
  /** Whether the dialog was still in the DOM when sampling ended. */
  unmounted: boolean;
}

/**
 * Open the overlay, let its scrim finish fading in, close it, and record the
 * scrim until the dialog is gone. `open`/`close` perform the user's action
 * (a click, Escape, ...); they run in the Playwright process, the sampler in the
 * page, so no frame is missed between the trigger and the first sample.
 */
export async function captureBackdrop(
  page: Page,
  open: () => Promise<unknown>,
  close: () => Promise<unknown>,
): Promise<BackdropCapture> {
  await page.evaluate(() => {
    const w = window as unknown as { __bd: any };
    const alpha = (c: string) => {
      if (!c || c === "transparent") return 0;
      const m = /rgba?\(([^)]+)\)/.exec(c);
      if (m) {
        const p = m[1].split(/[ ,/]+/).filter(Boolean);
        return p.length > 3 ? Number(p[3]) : 1;
      }
      const c4 = /color\(.*\/\s*([\d.]+)\)/.exec(c);
      return c4 ? Number(c4[1]) : c.startsWith("color(") ? 1 : 0;
    };
    // The scrim a component used to paint itself, on the wrapper around the dialog.
    // On the web arm it must now contribute nothing.
    const WRAPPERS = [
      ".dx-dialog-backdrop",
      ".dx-alert-dialog-backdrop",
      ".dx-sheet-root",
      ".dx-drawer-root",
      ".dx-command-dialog-backdrop",
    ];
    const st: any = { samples: [], transitions: [], dlg: null, stop: false, settled: null };
    w.__bd = st;
    const t0 = performance.now();

    document.addEventListener(
      "transitionrun",
      (e) => {
        const ev = e as TransitionEvent;
        if (ev.pseudoElement !== "::backdrop") return;
        const dlg = ev.target as HTMLDialogElement;
        const anim = document
          .getAnimations()
          .find(
            (a) =>
              (a.effect as KeyframeEffect | null)?.target === dlg &&
              (a.effect as KeyframeEffect | null)?.pseudoElement === "::backdrop" &&
              (a as CSSTransition).transitionProperty === ev.propertyName,
          );
        const d = Number(anim?.effect?.getComputedTiming().duration ?? 0);
        // Closing is either the native `[open]` already gone (Escape/`close()` on
        // Dialog, AlertDialog, Sheet, Drawer) or the component's own `data-state`
        // already "closed" while the dialog is still held open (the modal Popover,
        // which waits out its exit before calling `close()`).
        const closing = !dlg.open || (dlg.closest("[data-state]") as HTMLElement | null)?.dataset.state === "closed";
        st.transitions.push({
          phase: closing ? "close" : "open",
          property: ev.propertyName,
          durationMs: Number.isFinite(d) ? d : 0,
        });
      },
      true,
    );

    const tick = () => {
      if (!st.dlg) {
        st.dlg = [...document.querySelectorAll("dialog")].find((d) => d.matches(":modal")) ?? null;
      }
      let wrapper = 0;
      for (const sel of WRAPPERS) {
        const el = document.querySelector(sel);
        if (!el) continue;
        const cs = getComputedStyle(el);
        if (cs.display !== "none") wrapper = Math.max(wrapper, alpha(cs.backgroundColor) * Number(cs.opacity));
      }
      let native = 0;
      let isOpen = false;
      if (st.dlg && st.dlg.isConnected) {
        const cs = getComputedStyle(st.dlg, "::backdrop");
        native = alpha(cs.backgroundColor) * Number(cs.opacity);
        isOpen = st.dlg.open;
      }
      st.samples.push([performance.now() - t0, +wrapper.toFixed(4), +native.toFixed(4), isOpen]);
      if (!st.stop) requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);
  });

  await open();
  // Fully open: the dialog is `[open]`, the open fade has STARTED, and every `::backdrop`
  // animation has finished. "Has started" is a `transitionrun` we recorded, not merely the
  // absence of a running animation: the poll runs in a `requestAnimationFrame` callback, which
  // fires BEFORE the style recalc that creates the transition for the frame `[open]` landed in,
  // so on a loaded machine "open and nothing animating" is true for one frame before the fade
  // begins, and `settled` would be read at its first, near-transparent step (alpha 0.008, blur
  // 0.28px). If the transition never runs at all (the snap this file exists to catch), give it
  // `NO_TRANSITION_GRACE_MS` and carry on, so the "no transition ran" assertion still reports it.
  await page.waitForFunction(
    (graceMs) => {
      const st = (window as unknown as { __bd: any }).__bd;
      const d: HTMLDialogElement | null = st.dlg;
      if (!d || !d.open) return false;
      st.openSeenAt ??= performance.now();
      const animating = document
        .getAnimations()
        .some(
          (a) =>
            (a.effect as KeyframeEffect | null)?.target === d &&
            (a.effect as KeyframeEffect | null)?.pseudoElement === "::backdrop" &&
            a.playState !== "finished",
        );
      if (animating) return false;
      const started = st.transitions.some((t: { phase: string }) => t.phase === "open");
      return started || performance.now() - st.openSeenAt > graceMs;
    },
    NO_TRANSITION_GRACE_MS,
    { timeout: 10_000 },
  );
  await page.evaluate(() => {
    const st = (window as unknown as { __bd: any }).__bd;
    const cs = getComputedStyle(st.dlg, "::backdrop");
    st.settled = { backgroundColor: cs.backgroundColor, backdropFilter: cs.backdropFilter };
  });

  await close();
  // Closed: the dialog is gone (unmounted) or has no scrim left. The cap only
  // matters if the scrim never fades back to nothing, which is the failure.
  await page
    .waitForFunction(
      () => {
        const st = (window as unknown as { __bd: any }).__bd;
        const last = st.samples[st.samples.length - 1];
        return !st.dlg.isConnected || (!st.dlg.open && last && last[2] < 0.001);
      },
      undefined,
      { timeout: 5_000 },
    )
    .catch(() => undefined);

  return page.evaluate(() => {
    const st = (window as unknown as { __bd: any }).__bd;
    st.stop = true;
    return {
      samples: st.samples,
      transitions: st.transitions,
      settled: st.settled,
      unmounted: !st.dlg.isConnected,
    } as BackdropCapture;
  });
}

/**
 * The overlay duration the page actually resolves: `--dx-overlay-duration`, defined by the theme. The
 * `--dx-motion-duration-slow` read is only a guard for a theme copy that predates the token.
 */
export async function resolveOverlayMs(page: Page): Promise<number> {
  return page.evaluate((fallbackMs) => {
    const cs = getComputedStyle(document.documentElement);
    const read = (name: string) => cs.getPropertyValue(name).trim();
    const v = read("--dx-overlay-duration") || read("--dx-motion-duration-slow");
    if (!v) return fallbackMs;
    return v.endsWith("ms") ? parseFloat(v) : parseFloat(v) * 1000;
  }, DEFAULT_OVERLAY_MS);
}

/**
 * The shared claim. `overlayMs` is `resolveOverlayMs(page)`; `minExitMs` is the
 * duration the dialog itself must be kept in the top layer for (the scrim's own,
 * unless the panel's exit is longer, as Sheet's slide is).
 */
export function assertSharedBackdrop(capture: BackdropCapture, overlayMs: number, label: string): void {
  const { samples, transitions, settled } = capture;
  const fail = (why: string): never => {
    throw new Error(
      `${label}: ${why}\n  transitions: ${JSON.stringify(transitions)}\n  settled: ${JSON.stringify(settled)}\n` +
        `  samples (t, wrapper, backdrop, open): ${JSON.stringify(samples.filter((_, i) => i % 3 === 0))}`,
    );
  };

  const wrapperPeak = Math.max(...samples.map((s) => s[1]));
  const backdropPeak = Math.max(...samples.map((s) => s[2]));

  // One painter. A wrapper that paints too stacks a second scrim under the first
  // (Dialog 19%, AlertDialog 37%, Drawer 55%, CommandDialog 40% before this).
  if (wrapperPeak > 0.001) fail(`the wrapper painted ${wrapperPeak} of black on top of the dialog's ::backdrop (a second scrim)`);

  // The same scrim everywhere: Nova's black/10 with a 4px blur.
  if (Math.abs(backdropPeak - SCRIM_ALPHA) > 0.005) fail(`::backdrop peaked at ${backdropPeak} black, expected the shared ${SCRIM_ALPHA}`);
  const blur = /blur\(([\d.]+)px\)/.exec(settled.backdropFilter);
  if (!blur || Math.abs(Number(blur[1]) - SCRIM_BLUR_PX) > 0.01) fail(`::backdrop blur was "${settled.backdropFilter}", expected blur(${SCRIM_BLUR_PX}px)`);

  // It fades rather than snaps, in AND out, for the same length. `transitionrun`
  // events are the proof: before this the UA ::backdrop never ran one at all.
  const bg = (phase: "open" | "close") => transitions.filter((t) => t.phase === phase && t.property === "background-color");
  for (const phase of ["open", "close"] as const) {
    const run = bg(phase);
    if (run.length === 0) fail(`no background-color transition ran on the ::backdrop while ${phase === "open" ? "opening" : "closing"} -- the scrim snaps`);
    const ms = run[0].durationMs;
    if (Math.abs(ms - overlayMs) > 1) fail(`the ${phase} fade is ${ms}ms, expected the shared ${overlayMs}ms`);
  }

  // And it really went away: not stranded at the scrim, not left behind unmounted.
  const last = samples[samples.length - 1];
  if (last[2] > 0.001 && !capture.unmounted) fail(`the scrim never faded back to nothing (last ::backdrop alpha ${last[2]})`);
}
