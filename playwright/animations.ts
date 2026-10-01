import type { Locator } from "@playwright/test";

/**
 * Resolves once `locator`'s element has been mounted AND every
 * animation/transition running on it (or its descendants) has finished --
 * the load-independent replacement for `page.waitForTimeout(<animation
 * duration * safety factor>)` before reading an animated value
 * (dev-docs/backlog.md row 115; same class as row 112's "asserting on an
 * animated value").
 *
 * WHY a fixed wait is wrong: a CSS animation's clock starts at the first
 * frame the page renders AFTER the element mounts, not at the click that
 * mounted it. On a starved main thread/compositor (the full suite at
 * `--workers=4` runs 4 Chromiums on 4 cores) that first frame can arrive
 * hundreds of ms late, so "200ms animation + 300ms slack" is not a
 * guarantee, only a probability. `drawer.spec.ts:110` read the panel's
 * bounding box mid-slide-in this way.
 *
 * Two rAFs first: the animation object does not exist until the style
 * recalc of the next frame, and "no running animations" is trivially true
 * before it does -- the very false-settled reading this exists to prevent.
 */
export async function awaitAnimationsSettled(locator: Locator): Promise<void> {
  await locator.waitFor({ state: "attached" });
  await locator.evaluate(async (el) => {
    const frame = () => new Promise<void>((r) => requestAnimationFrame(() => r()));
    await frame();
    await frame();
    const deadline = performance.now() + 10_000;
    for (;;) {
      const unsettled = el
        .getAnimations({ subtree: true })
        .filter((a) => a.playState === "running" || a.playState === "pending");
      if (unsettled.length === 0) return;
      if (performance.now() > deadline) {
        throw new Error(`animations still running after 10s: ${unsettled.map((a) => (a as CSSAnimation).animationName ?? a.id).join(", ")}`);
      }
      await Promise.all(unsettled.map((a) => a.finished.catch(() => undefined)));
      await frame();
    }
  });
}
