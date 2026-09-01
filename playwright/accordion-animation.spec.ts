/**
 * Regression check for the accordion close animation's "residual height that
 * snaps to 0 on unmount" jank, fixed in
 * preview/src/components/accordion/{component.rs, style.css}: the content
 * must animate all the way to ~0px height (not a padding-sized floor) before
 * `use_animated_open` unmounts it, with no multi-frame plateau on the way
 * there. `accordion.spec.ts`'s smoothness test only checks frame-to-frame
 * deltas while mounted, which a flat plateau satisfies -- this spec asserts
 * the final height.
 *
 * Two modes, selected by env vars, so the SAME assertion logic
 * (`assertCloseAnimationReachesZero`) is what's being claimed to hold in
 * both places:
 *   - ACCORDION_MODE unset or "app" (default): drives the real preview app
 *     route, exactly like the existing `accordion.spec.ts`.
 *   - ACCORDION_MODE=repro: drives a standalone reproduction page (a copy of
 *     the rendered DOM + CSS with a script that toggles `data-open` and
 *     unmounts on animation end, exposing `closeAccordion()`, `closeDone`
 *     and `samples` on `window`) via ACCORDION_URL -- how the fix was first
 *     measured, 138px -> 16px plateau -> snap before, monotonic to 0 after.
 */
import { test, type Page } from "@playwright/test";
import { assertCloseAnimationReachesZero, Sample } from "./assert-close-animation";

const APP_URL =
  process.env.ACCORDION_URL ?? "http://127.0.0.1:8080/component/?name=accordion&";
const MODE = process.env.ACCORDION_MODE ?? "app";
const LOAD_TIMEOUT = 20 * 60 * 1000;

/** App-lane: drive the real accordion component like accordion.spec.ts does. */
async function sampleCloseInApp(page: Page): Promise<Sample[]> {
  await page.goto(APP_URL, { timeout: LOAD_TIMEOUT, waitUntil: "networkidle" });
  const accordionItems = page.locator("[data-open]").filter({ has: page.getByRole("button") });
  const firstItem = accordionItems.first();
  const button = accordionItems.getByRole("button").first();

  await button.click();
  await page.waitForFunction(
    (el) => el?.getAttribute("data-open") === "true",
    await firstItem.elementHandle()
  );
  await page.waitForTimeout(500); // let the open animation fully settle

  const contentId = await button.getAttribute("aria-controls");
  if (!contentId) throw new Error("accordion trigger has no aria-controls");

  const framesPromise = page.evaluate((id) => {
    return new Promise<Sample[]>((resolve) => {
      const frames: Sample[] = [];
      const t0 = performance.now();
      function tick() {
        const el = document.getElementById(id);
        const exists = !!el && document.body.contains(el);
        frames.push({ t: performance.now() - t0, h: exists ? el!.getBoundingClientRect().height : 0, exists });
        // Stop once unmounted (or after a generous cap so this can't hang).
        if (exists && performance.now() - t0 < 3000) {
          requestAnimationFrame(tick);
        } else {
          resolve(frames);
        }
      }
      requestAnimationFrame(tick);
    });
  }, contentId);

  await button.click(); // close
  return framesPromise;
}

/** Repro-lane: drive the standalone buggy.html/fixed.html harness. */
async function sampleCloseInRepro(page: Page): Promise<Sample[]> {
  await page.goto(APP_URL);
  await page.waitForFunction(() => typeof (window as any).closeAccordion === "function");
  await page.evaluate(() => (window as any).closeAccordion());
  await page.waitForFunction(() => (window as any).closeDone === true, undefined, { timeout: 5000 });
  return page.evaluate(() => (window as any).samples);
}

test("close animation reaches ~0 height with no plateau before unmount", async ({ page }) => {
  const samples = MODE === "repro" ? await sampleCloseInRepro(page) : await sampleCloseInApp(page);
  assertCloseAnimationReachesZero(samples);
});
