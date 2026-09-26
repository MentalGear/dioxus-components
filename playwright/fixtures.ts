/**
 * Shared `test`/`expect` for every spec in this suite (dev-docs/backlog.md
 * row 110). Import `test`/`expect`/`devices` from here instead of
 * "@playwright/test" directly; keep importing types (`type Page`,
 * `type Locator`, ...) from "@playwright/test" as before -- only the
 * runtime `test` object needs to come from this file.
 *
 * WHAT THIS FIXES: Playwright's own actionability pre-check before
 * `.hover()`/`.focus()`/`.click()` (`scrollIntoViewIfNeeded`) issues an
 * UNQUALIFIED scroll -- one that does not specify its own `behavior` --
 * whenever the target isn't already fully in view. An unqualified scroll's
 * default `behavior: "auto"` means "defer to the scrolling box's own CSS
 * `scroll-behavior`", and this site sets `html { scroll-behavior: smooth }`
 * globally (`preview/assets/main.css:54`, for legitimate in-page TOC-link
 * navigation) -- so that pre-check animates for a couple hundred ms after
 * `.hover()`/`.click()` has already dispatched its event and returned,
 * dragging the target out from under the now-stationary cursor. Confirmed
 * (dev-docs/backlog.md row 110) as the root cause of two independent
 * flakes found by execution: `carousel.spec.ts`'s "hovering the carousel
 * stops rotation..." test, and `oracle/tier2-html/top-layer.spec.ts`'s
 * `pinNearTop` helper.
 *
 * THE CONSTRUCTION: an auto (no-request-needed) fixture overrides the
 * worker-scoped `browser` fixture so that EVERY `BrowserContext` this
 * worker ever creates -- whether through Playwright's own built-in
 * `context`/`page` test fixtures, or a spec's own `browser.newContext(...)`
 * call (e.g. `combobox.spec.ts`'s mobile-emulation test,
 * `oracle/tier2-html/native-dialog.spec.ts`'s touch-emulation test) --
 * gets `context.addInitScript(...)` forcing
 * `html { scroll-behavior: auto !important }` before any of the page's own
 * scripts run, on every navigation. One override point, no per-call-site
 * patch, and nothing a future spec could opt out of by forgetting to call
 * a helper.
 *
 * Stock Playwright has no `use`/config-level option for this: `addInitScript`
 * is a method on `page`/`context`, not a `BrowserContextOptions` field
 * (checked `PlaywrightTestOptions`/`BrowserContextOptions` in
 * `node_modules/playwright/types/test.d.ts` and
 * `node_modules/playwright-core/types/types.d.ts` -- neither has an
 * init-script or stylesheet option), so a shared fixtures module every
 * spec sources `test`/`expect` from is the actual mechanism, not a setting
 * inside `playwright/*.config.ts`. Because it lives here rather than in
 * any one config, it applies uniformly under every config in this repo
 * (`playwright.config.ts`, `ssg.local.config.ts`, `baseline.local.config.ts`,
 * `oracle.local.config.ts`, `xvfb.local.config.ts`, and CI's use of
 * `playwright.config.ts`/`ssg.local.config.ts`) without any of them
 * needing their own copy of this logic.
 *
 * WHAT THIS DOES NOT TOUCH: a scroll call that names its OWN explicit
 * `behavior` (e.g. `primitives/src/carousel.rs`'s
 * `scroller.scrollBy({ left: delta, behavior })`, where `behavior` is
 * computed to `'smooth'`/`'auto'` in JS) is never affected -- CSS
 * `scroll-behavior` only supplies the default for an scroll that omits
 * `behavior`; an explicit argument always wins regardless of what the
 * ancestor CSS says. `carousel.spec.ts`'s "the first paged transition
 * actually animates" test (which asserts on exactly such an explicit-
 * `behavior: 'smooth'` call) keeps passing under this fixture -- verified
 * by execution, not just by reading (dev-docs/backlog.md row 110's own
 * verification note).
 *
 * This fixture also never touches `reducedMotion`/`prefers-reduced-motion`
 * emulation -- the carousel autoplay "prefers-reduced-motion: reduce never
 * starts rotation at all" test (`carousel.spec.ts`) keeps emulating that
 * itself via `page.emulateMedia({ reducedMotion: "reduce" })`, unaffected
 * by anything here.
 */
import { test as base, expect, devices } from "@playwright/test";

const FORCE_INSTANT_SCROLL_INIT_SCRIPT = () => {
  // `document.documentElement` does not exist yet at the moment an
  // init script runs (confirmed by execution in this sandbox's Chromium:
  // it reads `null` here) -- the CDP hook this is built on
  // (`Page.addScriptToEvaluateOnNewDocument`) fires before the parser has
  // created ANY node, not merely before other page scripts. Setting the
  // property immediately, unconditionally, throws (`Cannot read
  // properties of null`) and is silently swallowed (this does not surface
  // as a Playwright `pageerror`), so the whole line -- and the CSS
  // override -- is a silent no-op. Applying it as soon as `<html>` exists
  // instead (an immediate check, in case it already does by the time this
  // runs, plus a `readystatechange` listener for when it does not yet)
  // reliably lands well before any test interaction, which always waits
  // for at least `networkidle` (real specs) or full hydration
  // (`gotoHydrated`) first.
  const apply = () => {
    if (document.documentElement) {
      document.documentElement.style.setProperty("scroll-behavior", "auto", "important");
    }
  };
  apply();
  document.addEventListener("readystatechange", apply);
};

export const test = base.extend({
  // Overriding the worker-scoped `browser` fixture (not a new, separate
  // fixture) means every consumer of `browser` -- including Playwright's
  // own built-in `context`/`page` fixtures, which call `browser.newContext`
  // internally -- goes through this same wrapped `newContext`. Runs once
  // per worker process, not once per test.
  browser: async ({ browser }, use) => {
    const originalNewContext = browser.newContext.bind(browser);
    browser.newContext = (async (...args: Parameters<typeof originalNewContext>) => {
      const context = await originalNewContext(...args);
      await context.addInitScript(FORCE_INSTANT_SCROLL_INIT_SCRIPT);
      return context;
    }) as typeof browser.newContext;
    await use(browser);
  },
});

export { expect, devices };
