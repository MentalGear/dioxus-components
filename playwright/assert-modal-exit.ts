/**
 * Shared per-frame sampler/assertions for the exit of every modal overlay built on a
 * native `<dialog>` + `showModal()`: `Dialog`, `AlertDialog`, `Sheet`, `Drawer` and
 * `CommandDialog` (the modal `Popover` has its own spec). One helper, five specs, so
 * "a closing modal stays a modal, at the same size and place, until its exit has
 * played" is one claim, not five copies of a measurement.
 *
 * WHAT WENT WRONG (owner report 2026-10-05: Dialog close "glitches", the right/left
 * Sheet "changes size" while closing, the Drawer "glitches" on close). A native
 * `dialog.close()` drops the dialog out of the top layer in one frame, and the
 * driver used to call it the moment `open` went false. Out of the top layer a
 * `position: fixed` box is no longer resolved against the viewport but against the
 * nearest ancestor with layout containment -- on the home page that is the demo's
 * `.dx-component-card` (`content-visibility: auto`) -- and is clipped by its paint
 * containment: a Sheet went from 384x800 to 384x256, a Drawer from 1265x346 to
 * 1105x205, mid-slide. `transition: overlay allow-discrete` kept the dialog in the
 * top layer in Chromium only, and only until that transition ended (not in step with
 * the panel's own exit); Firefox and Safari snapped. The component pages have no such
 * ancestor, which is how the earlier verification missed it: THESE CAPTURES RUN ON THE
 * HOME PAGE, inside the card.
 *
 * THE CONSTRUCTION these assertions pin down: `use_dialog_open_driver`
 * (`primitives/src/lib.rs`) defers `close()` until every animation on the dialog (and
 * its `::backdrop`) has settled, and the stylesheets key the exit on
 * `data-state="closed"` and hide the dialog with `display: none` while it is not
 * `[open]`. So, on every engine and with no `overlay` transition:
 *   1. a dialog is never RENDERED outside the top layer -- not in the frames between
 *      mount and `showModal()`, not after `close()` begins its exit (`display !== none`
 *      implies `:modal`);
 *   2. its layout box (width, height, left, top) is constant for every rendered frame:
 *      the exit is motion by `transform`/`opacity`/`translate` only;
 *   3. `close()` really waited for the exit: the `close` event arrives at least as long
 *      after `data-state` flipped to "closed" as the longest animation that was running.
 * 2 and 3 are measured in the browser's own events (a `MutationObserver` and the `close`
 * event, both timestamped on arrival), not by counting frames, which depend on how
 * starved the page's main thread is (dev-docs/backlog.md row 115); 1 and 2 are per-frame
 * invariants that hold at any sampling rate.
 */
import type { Locator, Page } from "@playwright/test";

/** Sub-pixel layout noise, not motion: the layout box must hold within this. */
export const LAYOUT_TOLERANCE_PX = 1;

/** Slack on "close() waited for the exit": timer and event-dispatch jitter. */
export const EXIT_WAIT_SLACK_MS = 40;

/** One animation frame of the dialog while it existed. */
export interface ExitFrame {
  /** ms since the sampler started. */
  t: number;
  /** `getComputedStyle(dialog).display`. */
  display: string;
  /** The `open` attribute. */
  open: boolean;
  /** `dialog.matches(":modal")`: in the top layer as a modal. */
  modal: boolean;
  /** The dialog's own `data-state`. */
  state: string | null;
  /** Layout box, independent of transforms. */
  layout: { w: number; h: number; left: number; top: number };
  /** `getBoundingClientRect()`: layout box plus transform. */
  rect: { x: number; y: number; w: number; h: number };
  opacity: number;
}

export interface ExitCapture {
  frames: ExitFrame[];
  /** `performance.now()` offsets (same clock as `frames[].t`); null if it never happened. */
  closedAt: number | null;
  closeEventAt: number | null;
  /** First sampled frame at which the dialog element was no longer in the DOM. */
  unmountedAt: number | null;
  /** The longest `endTime` (ms) among the animations running on the dialog one frame after `data-state` flipped. */
  expectedExitMs: number;
  /** Whether the dialog element was gone from the DOM at the end. */
  unmounted: boolean;
}

/**
 * Open the overlay, let it settle, close it, and record the dialog (matched by
 * `dialogSelector`) from before it opens until it is unmounted. `open`/`close` perform
 * the user's action; they run in the Playwright process, the sampler in the page, so no
 * frame is missed between the trigger and the first sample.
 */
export async function captureModalExit(
  page: Page,
  dialogSelector: string,
  open: () => Promise<unknown>,
  close: () => Promise<unknown>,
): Promise<ExitCapture> {
  await page.evaluate((selector) => {
    const w = window as unknown as { __mx: any };
    const st: any = {
      frames: [],
      closedAt: null,
      closeEventAt: null,
      unmountedAt: null,
      expectedExitMs: 0,
      stop: false,
      dlg: null,
      t0: performance.now(),
    };
    w.__mx = st;
    const px = (v: string) => (v.endsWith("px") ? parseFloat(v) : NaN);
    const attach = (d: HTMLDialogElement) => {
      st.dlg = d;
      new MutationObserver(() => {
        if (d.dataset.state === "closed" && st.closedAt === null) {
          st.closedAt = performance.now() - st.t0;
          // One frame later every transition/animation this flip started exists; the
          // longest of them is how long a correct driver has to keep the dialog open.
          requestAnimationFrame(() => {
            st.expectedExitMs = Math.max(
              0,
              ...document
                .getAnimations()
                .filter((a) => (a.effect as KeyframeEffect | null)?.target === d)
                .map((a) => Number(a.effect?.getComputedTiming().endTime ?? 0))
                .filter((n) => Number.isFinite(n)),
            );
          });
        }
      }).observe(d, { attributes: true, attributeFilter: ["data-state"] });
      d.addEventListener("close", () => {
        st.closeEventAt = performance.now() - st.t0;
      });
    };
    const tick = () => {
      const d = (st.dlg ?? document.querySelector(selector)) as HTMLDialogElement | null;
      if (d && !st.dlg) attach(d);
      if (st.dlg) {
        const el = st.dlg as HTMLDialogElement;
        if (el.isConnected) {
          const cs = getComputedStyle(el);
          const r = el.getBoundingClientRect();
          st.frames.push({
            t: performance.now() - st.t0,
            display: cs.display,
            open: el.hasAttribute("open"),
            modal: el.matches(":modal"),
            state: el.getAttribute("data-state"),
            layout: {
              w: el.offsetWidth,
              h: el.offsetHeight,
              left: px(cs.left),
              top: px(cs.top),
            },
            rect: { x: r.x, y: r.y, w: r.width, h: r.height },
            opacity: Number(cs.opacity),
          });
        } else if (st.unmountedAt === null) {
          st.unmountedAt = performance.now() - st.t0;
        }
      }
      if (!st.stop) requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);
  }, dialogSelector);

  await open();
  // The home page hydrates ~70 demos; a click on one card's trigger can land before that card's
  // handlers are live and be dropped (no dialog ever appears -- not a product failure). Click once more.
  const appeared = await page
    .waitForFunction(() => !!(window as unknown as { __mx: any }).__mx.dlg, undefined, { timeout: 4_000 })
    .then(() => true, () => false);
  if (!appeared) await open();
  // Fully open: the dialog is modal and nothing is animating on it any more.
  try {
    await page.waitForFunction(
      () => {
        const st = (window as unknown as { __mx: any }).__mx;
        const d: HTMLDialogElement | null = st.dlg;
        if (!d || !d.matches(":modal")) return false;
        return !document.getAnimations().some((a) => (a.effect as KeyframeEffect | null)?.target === d && a.playState !== "finished");
      },
      undefined,
      { timeout: 20_000 },
    );
  } catch (e) {
    const why = await page.evaluate(() => {
      const st = (window as unknown as { __mx: any }).__mx;
      const d: HTMLDialogElement | null = st.dlg;
      return {
        sawDialog: !!d,
        connected: d?.isConnected,
        modal: d?.matches(":modal"),
        open: d?.hasAttribute("open"),
        state: d?.getAttribute("data-state"),
        animations: document
          .getAnimations()
          .filter((a) => (a.effect as KeyframeEffect | null)?.target === d)
          .map((a) => `${(a as CSSAnimation).animationName ?? (a as CSSTransition).transitionProperty}:${a.playState}`),
        dialogs: [...document.querySelectorAll("dialog")].map((x) => `${x.className}|${x.hasAttribute("open")}`),
      };
    });
    throw new Error(`never reached fully open: ${JSON.stringify(why)}\n${(e as Error).message}`);
  }
  await page.evaluate(() => {
    const st = (window as unknown as { __mx: any }).__mx;
    st.openSettledAt = performance.now() - st.t0;
  });

  await close();
  // Closed: the dialog is unmounted, or has been closed and hidden. Unmounting follows
  // the exit by a short hold, so give it room.
  await page.waitForFunction(
    () => {
      const st = (window as unknown as { __mx: any }).__mx;
      return !st.dlg || !st.dlg.isConnected;
    },
    undefined,
    { timeout: 20_000 },
  );

  return page.evaluate(() => {
    const st = (window as unknown as { __mx: any }).__mx;
    st.stop = true;
    return {
      frames: st.frames,
      closedAt: st.closedAt,
      closeEventAt: st.closeEventAt,
      unmountedAt: st.unmountedAt,
      expectedExitMs: st.expectedExitMs,
      unmounted: !st.dlg || !st.dlg.isConnected,
    } as ExitCapture;
  });
}

/** Scroll a home-page demo card into view (cards off screen are `content-visibility`-skipped) and return its locator. */
export async function homeCard(page: Page, component: string): Promise<Locator> {
  const card = page.locator("article.dx-component-card").filter({ has: page.locator(`h3 a[href^="/component/${component}/"]`) }).first();
  await card.scrollIntoViewIfNeeded();
  // `content-visibility: auto` renders a card once it is near the viewport; wait for its demo.
  await page.waitForTimeout(300);
  return card;
}

/**
 * The shared claim: see this file's header. `label` names the component in failures.
 */
export function assertModalExit(capture: ExitCapture, label: string): void {
  const { frames } = capture;
  const fail = (why: string): never => {
    const brief = frames.map(
      (f) =>
        `${Math.round(f.t)}ms ${f.display}${f.open ? " open" : ""}${f.modal ? " modal" : ""} ${f.state ?? "-"} ` +
        `${f.layout.w}x${f.layout.h}@${f.layout.left},${f.layout.top} op${f.opacity.toFixed(2)}`,
    );
    throw new Error(
      `${label}: ${why}\n  closedAt ${capture.closedAt}, closeEventAt ${capture.closeEventAt}, unmountedAt ${capture.unmountedAt}, expectedExitMs ${capture.expectedExitMs}\n  frames:\n    ${brief.join("\n    ")}`,
    );
  };

  if (frames.length === 0) fail("the sampler never saw the dialog");
  const rendered = frames.filter((f) => f.display !== "none");
  if (rendered.length === 0) fail("the dialog was never rendered");

  // 1. Never rendered outside the top layer.
  const loose = rendered.filter((f) => !f.modal);
  if (loose.length > 0) {
    fail(
      `the dialog was rendered ${loose.length} time(s) outside the top layer (display ${loose[0].display}, not :modal, ` +
        `data-state ${loose[0].state}) -- out of it a fixed panel is sized and clipped by a layout-contained ancestor`,
    );
  }

  // 2. A constant layout box for every rendered frame (the exit is transform/opacity only).
  const ref = rendered.find((f) => f.state === "open" && f.layout.w > 0) ?? rendered[0];
  for (const key of ["w", "h", "left", "top"] as const) {
    const bad = rendered.find((f) => !(Math.abs(f.layout[key] - ref.layout[key]) <= LAYOUT_TOLERANCE_PX));
    if (bad) {
      fail(
        `the layout ${key} changed from ${ref.layout[key]} to ${bad.layout[key]} at ${Math.round(bad.t)}ms ` +
          `(data-state ${bad.state}, ${bad.modal ? "modal" : "NOT modal"}) -- a closing panel must move by transform only`,
      );
    }
  }

  // 3. The dialog was not taken out of the top layer (close()d, or unmounted by the wrapper's own
  //    hold) before its exit had played. Normally `close()` comes first and the wrapper unmounts
  //    the element a little later; on a starved main thread the wrapper's unmount can win the race,
  //    which is harmless -- either way the exit had to run to its end first.
  if (capture.closedAt === null) fail("data-state never flipped to closed");
  const endedAt = capture.closeEventAt ?? capture.unmountedAt;
  if (endedAt === null) fail("the dialog was neither close()d nor unmounted");
  const waited = (endedAt as number) - (capture.closedAt as number);
  if (waited < capture.expectedExitMs - EXIT_WAIT_SLACK_MS) {
    fail(`the dialog left the top layer ${Math.round(waited)}ms after data-state=closed, but the exit runs ${Math.round(capture.expectedExitMs)}ms -- it cut the exit short`);
  }
  if (!capture.unmounted) fail("the dialog was never unmounted");
}
