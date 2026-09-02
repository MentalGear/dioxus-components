/**
 * Global stylesheet application — tier 2 (WHATWG HTML / CSSOM).
 *
 * Rule source: HTML Living Standard, "the link element", 'rel="stylesheet"'
 * and CSSOM's "document.styleSheets" — a style sheet participates in the
 * cascade only once it is *loaded*, and a sheet with a pending `@import`
 * (CSS Cascading and Inheritance / css-cascade "@import") is not loaded
 * until the imported sheet is. Browsers therefore keep such a sheet out of
 * `document.styleSheets` and out of the cascade even though `link.sheet`
 * already exposes its parsed rules.
 *
 * Found by execution, 2026-09-02: `preview/assets/main.css` began with an
 * `@import` of the Geist fonts from fonts.googleapis.com; in an environment
 * where that request hangs, the ENTIRE app stylesheet was inert on every
 * route (body computed to the UA default serif, every navbar/layout rule
 * dead) while every other stylesheet applied — and nothing in the harness
 * noticed, because component behaviour lives in per-component css_module
 * files. This rule makes that class of silent failure visible: the app's
 * global stylesheet must be in the applied set on every route, judged by a
 * property only it sets.
 */
import { test, expect } from "@playwright/test";

const ROUTES = [
  "http://127.0.0.1:8080/",
  "http://127.0.0.1:8080/component/?name=select&",
  "http://127.0.0.1:8080/dashboard/email-client",
];

for (const url of ROUTES) {
  test(`global stylesheet (main.css) is applied on ${new URL(url).pathname}${new URL(url).search}`, async ({ page }) => {
    await page.goto(url, { waitUntil: "domcontentloaded", timeout: 60_000 });
    await page.waitForTimeout(3000);
    const state = await page.evaluate(() => {
      const link = document.querySelector('link[rel="stylesheet"][href*="main-"]') as HTMLLinkElement | null;
      return {
        linked: !!link,
        applied: !!link && [...document.styleSheets].some((s) => s.ownerNode === link),
        bodyFont: getComputedStyle(document.body).fontFamily,
      };
    });
    expect(state.linked, "main.css must be linked").toBe(true);
    expect(state.applied, "main.css must be in document.styleSheets (loaded and applied)").toBe(true);
    // `body { font-family: Geist, ... }` is set only by main.css.
    expect(state.bodyFont.startsWith("Geist"), `body font-family should come from main.css, got: ${state.bodyFont}`).toBe(true);
  });
}
