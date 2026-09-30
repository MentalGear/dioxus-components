/**
 * Carousel: smoke + a11y attributes + keyboard paging (LTR and RTL) +
 * end-of-range button state + scroll-snap landing, across every shipped
 * variant (main, sizes, peek, vertical, rtl) that has ordinary
 * Previous/Next + role=group slides -- `indicators` (the APG tablist
 * style) has neither, so it is covered by its own dedicated describe
 * block instead of this generic sweep.
 *
 * SCOPING: every variant of a "Normal"-kind component renders on the same
 * page at once (`ComponentVariantHighlight` in `preview/src/main.rs`), so
 * every locator below is rooted through `demoFrame()` -- the same
 * `#component-preview-frame`/`#component-preview-frame-<variant>`
 * construction `oracle/tier3-radix/rtl.spec.ts` already established for
 * this exact class of collision (see that file's own header doc). Each
 * variant's `Carousel` also carries its own, non-colliding `aria-label`
 * (`preview/src/components/carousel/variants/<name>/mod.rs`) as a second,
 * defense-in-depth layer -- both matter: even with unique region names,
 * every variant's `CarouselPrevious`/`CarouselNext` share the exact same
 * default "Previous slide"/"Next slide" labels by design, so scoping is
 * what actually disambiguates a role+name query across variants.
 */

import { test, expect } from "./fixtures";
import { type Page, type Locator } from "@playwright/test";
import { BASE_URL } from "./base-url";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";
import { gotoHydrated } from "./hydration";

// `waitUntil: "networkidle"` alone (not the default "load") is not enough
// under the SSG lane: a prerendered page's interactive elements already
// exist in the served HTML before the wasm bundle finishes loading and
// hydrating, so an interaction issued right after goto can land on inert
// markup -- Dioxus's event delegation only attaches once hydration walks
// the tree, and a missed DOM event (a click, a mouseenter) is gone for
// good, not queued. Under `dx serve` this race cannot happen (elements are
// inserted into the DOM only once the client renders them, by which point
// listeners are already attached), which is why this file's suite was fully
// green there while two SSG-only interactions (carousel.spec.ts:633's first
// click, :1512's initial hover) silently landed before hydration/mid-tick
// and were never observed as intended. `gotoHydrated` (./hydration.ts)
// waits on networkidle AND a real hydration-ready signal the app now sets,
// closing the gap networkidle alone leaves open.
const GOTO_OPTS = { timeout: 20 * 60 * 1000 };

async function goto(page: Page, variant: string) {
  await gotoHydrated(page, `${BASE_URL}/component/?name=carousel&variant=${variant}&`, GOTO_OPTS);
}

/** See this file's own header ("SCOPING"). */
function demoFrame(
  page: Page,
  variant:
    | "main"
    | "sizes"
    | "spacing"
    | "peek"
    | "align"
    | "api"
    | "indicators"
    | "vertical"
    | "rtl"
    | "rewind"
    | "autoplay"
    | "virtual_loop"
    | "virtual_loop_rtl"
    | "virtual_many",
): Locator {
  const id = variant === "main" ? "component-preview-frame" : `component-preview-frame-${variant}`;
  return page.locator(`#${id}`);
}

/**
 * Scroll `locator`'s element into view instantly, for tests that need a
 * settled, known position before their own subsequent real-coordinate math
 * (`page.mouse`-based drags below) -- NOT as a guard against Playwright's
 * own pre-`.hover()`/`.focus()` actionability scroll any more: that class
 * (dev-docs/backlog.md row 110 -- an unqualified scroll's default
 * `behavior: "auto"` deferring to the site's global
 * `html { scroll-behavior: smooth }`, `preview/assets/main.css`, and
 * animating for a couple hundred ms after `.hover()` already fired,
 * dragging the target out from under the cursor) is now handled once for
 * every test by `./fixtures.ts`'s shared `test`/`expect`, which forces
 * `scroll-behavior: auto` for the whole suite -- see the "hovering the
 * carousel stops rotation..." test below, which used to call this
 * defensively and no longer needs to.
 *
 * `oracle/tier2-html/top-layer.spec.ts`'s own `pinNearTop` helper was the
 * other instance of that same class (2026-09-18, commit daebe40); it now
 * relies on the same shared fixture too, per that helper's own doc.
 */
async function scrollIntoViewInstant(locator: Locator): Promise<void> {
  await locator.evaluate((el) =>
    el.scrollIntoView({ block: "center", inline: "center", behavior: "instant" }),
  );
}

/**
 * Poll until the given slide's inline-axis (horizontal) or block-axis
 * (vertical) start position matches its scroll container's own -- i.e.
 * scroll-snap has actually settled on this slide's boundary, not merely
 * "the click/keypress handler ran." Mirrors the exact technique
 * `primitives/src/carousel.rs`'s own `CAROUSEL_SCROLL_TRACKING_JS` uses
 * (`getBoundingClientRect`, never `scrollLeft`, so this works identically
 * under `dir="rtl"` with no sign-convention special-casing).
 *
 * The poll loop runs INSIDE the page (one `evaluate` call, a `requestAnimationFrame`
 * loop), not as repeated Node-side `expect().toPass()` round trips
 * (dev-docs/backlog.md row 112's construction: "sample in-page ... never
 * via separate round trips"). Root-caused for the vertical variant's
 * "ArrowDown/ArrowUp page the vertical carousel" test (row 112 instance 2,
 * ~1/5 red under `--workers=2`, 5/5 green under `--workers=1`, never
 * reproduced in an isolated repro of that one test): the *previous*
 * version's `toPass({ timeout: 3000 })` re-issued a fresh CDP round trip
 * (`content.boundingBox()` + `item.boundingBox()`) on every poll attempt,
 * so the 3000ms budget had to cover both the scroll-snap settle itself AND
 * however long each Node<->browser round trip took that attempt -- under
 * two workers' worth of concurrent Chromium/CPU contention specifically
 * (never under one), a slower round trip eats into the same fixed budget
 * as a slower settle, coupling a real UI wait to Node-process/IPC
 * scheduling latency that has nothing to do with whether the carousel
 * actually settled in time. Driving the whole wait from a single
 * `requestAnimationFrame` loop already running in the page removes that
 * coupling entirely: the 3000ms deadline is measured against
 * `performance.now()` in the SAME realm as the animation, with zero
 * per-attempt IPC cost, so it reflects only genuine settle time.
 */
async function expectSnappedToBoundary(content: Locator, item: Locator, orientation: "horizontal" | "vertical" = "horizontal") {
  const contentHandle = await content.elementHandle();
  const itemHandle = await item.elementHandle();
  if (!contentHandle || !itemHandle) {
    throw new Error("expectSnappedToBoundary: content or item resolved to no element");
  }
  const result = await content.page().evaluate(
    ([contentEl, itemEl, orientation]) => {
      return new Promise<{
        ok: boolean;
        delta: number;
        containerBox: { x: number; y: number };
        itemBox: { x: number; y: number };
      }>((resolve) => {
        const deadline = performance.now() + 3000;
        const check = () => {
          const containerRect = (contentEl as Element).getBoundingClientRect();
          const itemRect = (itemEl as Element).getBoundingClientRect();
          const delta =
            orientation === "horizontal"
              ? Math.abs(containerRect.x - itemRect.x)
              : Math.abs(containerRect.y - itemRect.y);
          // A couple of CSS pixels of tolerance for sub-pixel rounding
          // during a smooth-scroll settle -- not a loose bound: an
          // unsettled scroll (still mid-animation, or landed on the wrong
          // slide) misses by tens/hundreds of pixels, not 1-2.
          if (delta <= 2) {
            resolve({
              ok: true,
              delta,
              containerBox: { x: containerRect.x, y: containerRect.y },
              itemBox: { x: itemRect.x, y: itemRect.y },
            });
            return;
          }
          if (performance.now() >= deadline) {
            resolve({
              ok: false,
              delta,
              containerBox: { x: containerRect.x, y: containerRect.y },
              itemBox: { x: itemRect.x, y: itemRect.y },
            });
            return;
          }
          requestAnimationFrame(check);
        };
        requestAnimationFrame(check);
      });
    },
    [contentHandle, itemHandle, orientation] as const,
  );
  expect(
    result.ok,
    `expected ${orientation} delta <= 2, got ${result.delta} -- ${JSON.stringify(result)}`,
  ).toBe(true);
}

/**
 * Drives a real pointer drag with `page.mouse` (down, N intermediate
 * moves, up) -- deliberately not a synthetic `dispatchEvent`, since
 * `CAROUSEL_DRAG_JS`'s own threshold/pointer-capture/click-suppression
 * logic all key off genuine `pointerdown`/`pointermove`/`pointerup`
 * events, which a script-fired fake event does not reliably reproduce
 * (`round5`'s own carousel-drag lane doc has the details).
 */
async function dragBy(page: Page, target: Locator, dx: number, dy: number, steps = 12) {
  // Without this, `boundingBox()` can return coordinates outside the
  // current viewport (found by execution: `main`'s own content sat at a
  // large negative y on first paint) -- `page.mouse` targets real
  // viewport coordinates, so a drag computed from an off-screen box
  // silently lands on nothing.
  await target.scrollIntoViewIfNeeded();
  const box = await target.boundingBox();
  if (!box) {
    throw new Error("dragBy: target has no bounding box");
  }
  const startX = box.x + box.width / 2;
  const startY = box.y + box.height / 2;
  await page.mouse.move(startX, startY);
  await page.mouse.down();
  for (let i = 1; i <= steps; i++) {
    await page.mouse.move(startX + (dx * i) / steps, startY + (dy * i) / steps);
    // A short pause between steps, matching a real mouse's ~60 Hz event
    // cadence rather than firing all steps in the same tick.
    await page.waitForTimeout(16);
  }
  await page.mouse.up();
}

/**
 * Like `dragBy`, but leaves the pointer DOWN at the end (no `mouse.up()`) --
 * for the edge rubber-band tests below, which need to read the track's
 * transform WHILE still dragging, before release's own `bounceHome` eases
 * it back to identity. Callers must release with `page.mouse.up()`
 * themselves once they are done inspecting the held state.
 */
async function dragHold(page: Page, target: Locator, dx: number, dy: number, steps = 12) {
  await target.scrollIntoViewIfNeeded();
  const box = await target.boundingBox();
  if (!box) {
    throw new Error("dragHold: target has no bounding box");
  }
  const startX = box.x + box.width / 2;
  const startY = box.y + box.height / 2;
  await page.mouse.move(startX, startY);
  await page.mouse.down();
  for (let i = 1; i <= steps; i++) {
    await page.mouse.move(startX + (dx * i) / steps, startY + (dy * i) / steps);
    await page.waitForTimeout(16);
  }
}

/**
 * The carousel track's own inline `transform` (mode B3's edge rubber-band,
 * `primitives/src/carousel.rs`'s `CAROUSEL_DRAG_JS`/`CAROUSEL_WHEEL_BOUNCE_JS`
 * -- both mutate `element.style.transform` directly via `document::eval`,
 * never through a Dioxus-rendered attribute, so this reads the live inline
 * style, not a rendered one). Empty string means identity (no bounce).
 */
async function readContentTransform(content: Locator): Promise<string> {
  return content.evaluate((el) => (el as HTMLElement).style.transform);
}

/** Parses the single `translateX(...)`/`translateY(...)` px value mode B3
 * writes, or `null` if the transform is empty/identity. */
function parseTranslatePx(transform: string): number | null {
  const match = transform.match(/translate[XY]\(([-\d.]+)px\)/);
  return match ? Number.parseFloat(match[1]) : null;
}

/**
 * The *real* accessible names of every `role="group"`/`"tabpanel"` node the
 * browser's own platform accessibility tree currently exposes (Chrome's
 * CDP `Accessibility` domain, populated from the same computation real
 * assistive tech consumes) -- deliberately not `page.getByRole`/
 * `locator.ariaSnapshot()`. Measured live (this lane): this installed
 * Playwright build's own ARIA-role engine does not treat `inert` as
 * excluding a node at all -- a minimal `<div role="group" inert>` still
 * matched `getByRole("group")` and appeared in `ariaSnapshot()`, while the
 * *same* CDP call this helper uses omitted it entirely from the very first
 * query (not merely "present but ignored"). `inert`'s actual, spec-defined
 * effect (removal from the accessibility tree) is a platform-level
 * property, so the platform's own tree -- not Playwright's separate,
 * DOM-based ARIA approximation -- is the correct oracle for this
 * component's own "only visible slides are exposed" contract. A resolved
 * discrepancy, not a shortcut: `getByRole`/`ariaSnapshot` remain the right
 * tool everywhere else in this file (they already correctly reflect
 * `aria-*`/native semantics for everything that isn't `inert`), and this
 * helper is reached for *only* the a11y-tree-exposure assertions below.
 *
 * Scoped to `rootSelector` via `DOM.querySelector` + `Accessibility.queryAXTree`
 * (a *subtree* query), not `getFullAXTree` for the whole page -- this file's
 * own "SCOPING" header note applies here too: every demo variant's own
 * carousel is mounted on the same page at once, and several share the exact
 * same "k of N" label shape (an `autoplay`/`indicators`/`tabs` variant sitting
 * at its own untouched slide 1 is a perfectly legitimate, *different*
 * `"1 of 5"`/`"1 of 4"` node) -- an unscoped whole-page query cannot tell
 * those apart from a stale/leftover node in the variant actually under test.
 */
async function axTreeGroupNames(page: Page, rootSelector: string): Promise<string[]> {
  const client = await page.context().newCDPSession(page);
  await client.send("DOM.enable");
  await client.send("Accessibility.enable");
  const { root } = await client.send("DOM.getDocument", { depth: -1, pierce: true });
  const { nodeId } = await client.send("DOM.querySelector", { nodeId: root.nodeId, selector: rootSelector });
  const { nodes } = await client.send("Accessibility.queryAXTree", { nodeId });
  const names = (nodes as Array<{ role?: { value?: string }; name?: { value?: string }; ignored?: boolean }>)
    .filter((n) => !n.ignored && (n.role?.value === "group" || n.role?.value === "tabpanel"))
    .map((n) => n.name?.value ?? "");
  await client.detach();
  return names;
}

/**
 * The carousel track's own per-slide pitch (px, along the given axis),
 * measured live from a rendered slide's own box rather than assumed --
 * every distance-based drag test below derives its drag length from this,
 * not a hardcoded pixel constant. A previous hardcoded-constant version of
 * these tests silently broke when a demo wrapper's rendered width changed
 * (176px -> 320px, `93d0ee7`): a distance tuned to clear 50% of the old
 * pitch cleared only ~42% of the new one, so `expectSnappedToBoundary`
 * reported the drag landing exactly one slide short. Measuring live at
 * each test's own run time is what survives the *next* geometry change
 * too, not just this one.
 */
async function slidePitch(item: Locator, orientation: "horizontal" | "vertical" = "horizontal"): Promise<number> {
  const box = await item.boundingBox();
  if (!box) {
    throw new Error("slidePitch: item has no bounding box");
  }
  return orientation === "horizontal" ? box.width : box.height;
}

/**
 * Samples `.dx-carousel-content`'s own scroll position at animation-frame
 * cadence, starting just before `trigger()` runs, until the value has both
 * (a) changed from its own starting point and (b) gone `STABLE_FRAMES_NEEDED`
 * consecutive frames without changing again -- or `HARD_CAP_MS` elapses,
 * whichever comes first -- so a caller can tell an animated transition (the
 * position passes through a value strictly between where it started and
 * where it ended) from an instant jump (it skips straight from start to
 * end). See this file's own "actually animates" describe block below, which
 * is the reason this exists: asserting the code *chose* `behavior: 'smooth'`
 * is not the same claim as the motion actually being animated, and only
 * this sampling proves the latter.
 *
 * A fixed wall-clock sampling window (previously 600ms, opened before
 * `trigger()`) is a live bug, not a hardening measure: against a `dx serve`
 * dev server the click-to-scroll latency happens to fit inside 600ms, so it
 * passes, but against a statically-served release SSG build the wasm
 * `document::eval` round trip alone can exceed it -- every sample then reads
 * the pre-scroll value, reporting a false "instant jump" on a transition
 * that (re-sampled with a longer window) demonstrably animates through
 * dozens of distinct intermediate values. A settle-condition stop is honest
 * regardless of how long the round trip actually takes on a given
 * build/server, or how long the transition itself runs -- unlike a fixed
 * window, it cannot silently start under-sampling a slower environment or
 * a shorter transition.
 */
async function sampleScrollDuring(
  page: Page,
  contentId: string,
  axis: "scrollLeft" | "scrollTop",
  trigger: () => Promise<void>,
): Promise<number[]> {
  const STABLE_FRAMES_NEEDED = 6;
  const HARD_CAP_MS = 3000;
  await page.evaluate(
    ({ id, axis, stableFramesNeeded, hardCapMs }) => {
      const w = window as unknown as {
        __dxCarouselSamples: number[];
        __dxCarouselDone: boolean;
      };
      w.__dxCarouselSamples = [];
      w.__dxCarouselDone = false;
      const el = document.getElementById(id) as unknown as Record<string, number>;
      const start = el[axis];
      const t0 = performance.now();
      let stableCount = 0;
      const tick = () => {
        const samples = w.__dxCarouselSamples;
        const value = el[axis];
        const previous = samples.length > 0 ? samples[samples.length - 1] : value;
        samples.push(value);
        stableCount = value === previous ? stableCount + 1 : 0;
        const hasChanged = value !== start;
        const settled = hasChanged && stableCount >= stableFramesNeeded;
        const timedOut = performance.now() - t0 > hardCapMs;
        if (settled || timedOut) {
          w.__dxCarouselDone = true;
          return;
        }
        requestAnimationFrame(tick);
      };
      requestAnimationFrame(tick);
    },
    { id: contentId, axis, stableFramesNeeded: STABLE_FRAMES_NEEDED, hardCapMs: HARD_CAP_MS },
  );
  await trigger();
  // A comfortable margin above the in-page HARD_CAP_MS -- that hard cap is
  // what's expected to end the wait in the overwhelming common case; this
  // outer one is only a safety net against the flag never being set at all.
  await page.waitForFunction(
    () => (window as unknown as { __dxCarouselDone: boolean }).__dxCarouselDone === true,
    undefined,
    { timeout: HARD_CAP_MS + 5000 },
  );
  return page.evaluate(() => (window as unknown as { __dxCarouselSamples: number[] }).__dxCarouselSamples);
}

/**
 * True if `samples` pass strictly through a value between its own first
 * and last entry. An instant jump never does (it sits at the start value,
 * then the end value, with nothing in between); a genuine smooth-scroll
 * transition always does -- the honest "was this actually animated"
 * check per this file's own "actually animates" describe block.
 */
function passesThroughAnIntermediateValue(samples: number[]): boolean {
  const start = samples[0];
  const end = samples[samples.length - 1];
  const lo = Math.min(start, end);
  const hi = Math.max(start, end);
  if (hi - lo < 2) {
    return false;
  }
  return samples.some((v) => v > lo + 1 && v < hi - 1);
}

for (const variant of ["main", "sizes", "peek", "vertical", "rtl"] as const) {
  test.describe(`Carousel (${variant} variant): smoke + a11y attributes`, () => {
    test(`region has role=region, aria-roledescription=carousel, and an accessible name that doesn't contain the word "carousel"`, async ({ page }) => {
      await goto(page, variant);
      const frame = demoFrame(page, variant);
      const region = frame.getByRole("region");
      await expect(region).toHaveAttribute("aria-roledescription", "carousel");
      const name = await region.evaluate((el) => el.getAttribute("aria-label") ?? "");
      expect(name.length).toBeGreaterThan(0);
      expect(name.toLowerCase()).not.toContain("carousel");
    });

    test('first slide has role=group, aria-roledescription=slide, and a "N of M" accessible name', async ({ page }) => {
      await goto(page, variant);
      const frame = demoFrame(page, variant);
      const firstSlide = frame.locator('[role="group"][aria-roledescription="slide"]').first();
      await expect(firstSlide).toHaveAttribute("aria-label", /^1 of \d+$/);
    });

    test("Previous/Next are native buttons with real accessible names, and activating them never moves focus off itself unexpectedly", async ({ page }) => {
      await goto(page, variant);
      const frame = demoFrame(page, variant);
      const previous = frame.getByRole("button", { name: "Previous slide" });
      const next = frame.getByRole("button", { name: "Next slide" });
      await expect(previous).toHaveJSProperty("tagName", "BUTTON");
      await expect(next).toHaveJSProperty("tagName", "BUTTON");

      await next.click();
      // Native <button> semantics: activating it keeps focus on itself,
      // it is never yanked elsewhere.
      await expect(next).toBeFocused();
    });

    test("Previous is disabled at the first slide", async ({ page }) => {
      await goto(page, variant);
      const frame = demoFrame(page, variant);
      await expect(frame.getByRole("button", { name: "Previous slide" })).toBeDisabled();
    });
  });
}

/**
 * shadcn-parity geometry: 28x28px buttons, no box-shadow, and visually
 * outside the track -- never overlapping a slide -- on every variant and
 * axis. Values are shadcn/ui's own live-measured carousel
 * (`ui.shadcn.com/docs/components/carousel`, JS running, 1280x800 @2x):
 * `getBoundingClientRect()` on both buttons reported exactly 28x28,
 * `getComputedStyle().boxShadow` reported "none", and the near edge of each
 * button sat 20px clear of the track's own edge (`--dx-space-12`, 48px,
 * minus the button's own 28px). Their own vertical ("Orientation") demo
 * confirmed the same 20px-clear rule holds on the block axis too, not just
 * inferred from the horizontal case -- see
 * `preview/src/components/carousel/style.css`'s own comments for the full
 * derivation and the exact numbers this asserts against.
 *
 * Clearance is measured against `.dx-carousel-content` (the track), NOT
 * `role=region` (`.dx-carousel` itself): the carousel-narrow lane moved
 * Previous/Next from a negative `inset-inline-*`/`inset-block-*` outset
 * past `.dx-carousel`'s own edge to a `padding-inline`/`padding-block`
 * RESERVATION inside it, so the buttons now sit fully inside `.dx-carousel`'s
 * own box (that containment is what the next describe block regression-
 * tests) -- `region`'s box no longer marks the track's edge, only the
 * track itself still does. The 20px-clear number is unchanged either way;
 * only which element it is measured from moved.
 */
test.describe("Carousel: shadcn-parity geometry (size, shadow, outside placement)", () => {
  const BUTTON_SIZE_PX = 28;
  const MIN_CLEAR_PX = 20 - 1; // 1px tolerance for sub-pixel layout rounding

  for (const variant of ["main", "sizes", "peek", "vertical", "rtl"] as const) {
    test(`${variant}: Previous/Next are 28x28, shadow-less, 20px clear of the track`, async ({ page }) => {
      await goto(page, variant);
      const frame = demoFrame(page, variant);
      // The clipping viewport wrapper, NOT `.dx-carousel-content` itself:
      // the gap model (backlog row 91's shadcn-parity addendum) gives the
      // scroller its own negative `margin-inline-start`/`-block-start`
      // (`--dx-carousel-gap`'s own compensation), which deliberately
      // widens and shifts `.dx-carousel-content`'s own layout box by the
      // gap's width -- clipped by this wrapper, so nothing about it looks
      // different, but a raw `boundingBox()` of the scroller itself no
      // longer marks the track's own VISUAL edge the way it did before
      // that construction existed. The viewport wrapper's own box is
      // exactly the clipped, visual track edge, unaffected by the gap
      // model either way.
      const content = frame.locator('[data-slot="carousel-viewport"]');
      const previous = frame.getByRole("button", { name: "Previous slide" });
      const next = frame.getByRole("button", { name: "Next slide" });

      await expect(previous).toHaveCSS("box-shadow", "none");
      await expect(next).toHaveCSS("box-shadow", "none");

      const [contentBox, previousBox, nextBox] = await Promise.all([
        content.boundingBox(),
        previous.boundingBox(),
        next.boundingBox(),
      ]);
      expect(contentBox).not.toBeNull();
      expect(previousBox).not.toBeNull();
      expect(nextBox).not.toBeNull();

      for (const box of [previousBox!, nextBox!]) {
        expect(box.width).toBeCloseTo(BUTTON_SIZE_PX, 0);
        expect(box.height).toBeCloseTo(BUTTON_SIZE_PX, 0);
      }

      if (variant === "vertical") {
        // Block axis: Previous above the track, Next below it.
        expect(contentBox!.y - (previousBox!.y + previousBox!.height)).toBeGreaterThanOrEqual(MIN_CLEAR_PX);
        expect(nextBox!.y - (contentBox!.y + contentBox!.height)).toBeGreaterThanOrEqual(MIN_CLEAR_PX);
      } else if (variant === "rtl") {
        // Inline axis, mirrored: Previous (the *start* edge) resolves to
        // the physical right under dir="rtl"; Next (the *end* edge) to the
        // physical left -- the opposite pairing from every other variant.
        expect(previousBox!.x - (contentBox!.x + contentBox!.width)).toBeGreaterThanOrEqual(MIN_CLEAR_PX);
        expect(contentBox!.x - (nextBox!.x + nextBox!.width)).toBeGreaterThanOrEqual(MIN_CLEAR_PX);
      } else {
        expect(contentBox!.x - (previousBox!.x + previousBox!.width)).toBeGreaterThanOrEqual(MIN_CLEAR_PX);
        expect(nextBox!.x - (contentBox!.x + contentBox!.width)).toBeGreaterThanOrEqual(MIN_CLEAR_PX);
      }
    });
  }
});

/**
 * carousel-narrow lane regression: Previous/Next must never extend beyond
 * their own demo frame's box, at any viewport a real consumer's container
 * could actually be, in either theme. This is the direct, black-box
 * assertion for the bug this lane fixed (reported live on the `rtl`
 * variant: both arrows clipped at the frame edge on a narrow viewport) --
 * see `preview/src/components/carousel/style.css`'s own comment on the
 * horizontal/vertical placement rules for the construction and the
 * geometric derivation, and the lane's own report for the full before/
 * after measurement table this reproduces the shape of.
 *
 * Viewports: the six the lane brief itself named as the minimum
 * (1280/900/768/640/480/390) -- this page's own layout is not simply
 * narrower at a narrower viewport (a sidebar collapses around 700-750px,
 * so the rendered frame width is NOT monotonic in viewport width; 640px
 * actually yields a wider frame than 768px does) -- so this sweeps the
 * exact set asked for rather than a hand-picked "worst case" that could
 * miss a non-adjacent regression the way a monotonic assumption would.
 *
 * `toBeGreaterThanOrEqual(-0.5)`/`toBeLessThanOrEqual(...+0.5)`: a half
 * pixel of slack for sub-pixel layout rounding, the same kind of tolerance
 * `MIN_CLEAR_PX` above already applies -- not a loose bound: the pre-fix
 * construction overflowed by 12-15px on the horizontal variants at these
 * same viewports (see the lane report's before/after table), tens of
 * times this tolerance.
 */
test.describe("Carousel: arrows never overflow their frame (carousel-narrow regression)", () => {
  const VIEWPORTS = [1280, 900, 768, 640, 480, 390];
  const OVERFLOW_TOLERANCE_PX = 0.5;

  for (const dark of [false, true]) {
    for (const width of VIEWPORTS) {
      test(`${dark ? "dark" : "light"} mode, ${width}px viewport: no button escapes its frame`, async ({
        page,
      }) => {
        await page.setViewportSize({ width, height: 900 });
        await page.goto(
          `${BASE_URL}/component/?name=carousel&variant=main&${dark ? "dark_mode=true" : ""}`,
          GOTO_OPTS,
        );

        for (const variant of ["main", "sizes", "peek", "vertical", "rtl"] as const) {
          const frame = demoFrame(page, variant);
          const previous = frame.getByRole("button", { name: "Previous slide" });
          const next = frame.getByRole("button", { name: "Next slide" });

          // Polls (via `toPass`, the same retry idiom `expectSnappedToBoundary`
          // above already uses for its own settle) rather than trusting one
          // instantaneous `boundingBox()` triple: this page mounts five
          // independent carousels at once, each running its own mount-time
          // settle effect (`primitives/src/carousel.rs`'s own
          // `CAROUSEL_SCROLL_INTO_VIEW_JS` effect), and under this suite's
          // parallel workers sharing one plain `http.server` (slower to
          // answer four workers' concurrent requests than one), a read
          // racing that settle can catch the frame and a button mid-shift,
          // tens of pixels apart, even at the roomiest viewport -- confirmed
          // by construction: standalone re-measurement (fresh browser
          // context, no other worker contention) of the exact same variant/
          // viewport never reproduced a mismatch, and this repo's own
          // dev-docs already name "grepping/measuring before the page has
          // actually settled" as a proven false-alarm source for this
          // component. A real overflow does not self-correct on retry, so
          // this still fails (after 3s) if the buttons are genuinely
          // outside their frame.
          await expect(async () => {
            const [frameBox, previousBox, nextBox] = await Promise.all([
              frame.boundingBox(),
              previous.boundingBox(),
              next.boundingBox(),
            ]);
            expect(frameBox, `${variant}: frame box`).not.toBeNull();
            expect(previousBox, `${variant}: previous box`).not.toBeNull();
            expect(nextBox, `${variant}: next box`).not.toBeNull();

            for (const [label, box] of [
              ["previous", previousBox!],
              ["next", nextBox!],
            ] as const) {
              expect(box.x, `${variant} ${label}: left edge vs frame`).toBeGreaterThanOrEqual(
                frameBox!.x - OVERFLOW_TOLERANCE_PX,
              );
              expect(box.x + box.width, `${variant} ${label}: right edge vs frame`).toBeLessThanOrEqual(
                frameBox!.x + frameBox!.width + OVERFLOW_TOLERANCE_PX,
              );
              expect(box.y, `${variant} ${label}: top edge vs frame`).toBeGreaterThanOrEqual(
                frameBox!.y - OVERFLOW_TOLERANCE_PX,
              );
              expect(box.y + box.height, `${variant} ${label}: bottom edge vs frame`).toBeLessThanOrEqual(
                frameBox!.y + frameBox!.height + OVERFLOW_TOLERANCE_PX,
              );
            }
          }).toPass({ timeout: 3000 });
        }
      });
    }
  }
});

test.describe("Carousel: paging by button and scroll-snap landing", () => {
  test("Next/Previous page one slide at a time and land exactly on the slide boundary", async ({ page }) => {
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const content = frame.locator(".dx-carousel-content");
    const next = frame.getByRole("button", { name: "Next slide" });
    const previous = frame.getByRole("button", { name: "Previous slide" });
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 5` });

    await expectSnappedToBoundary(content, slide(1));

    await next.click();
    await expect(slide(2)).toHaveAttribute("data-selected", "true");
    await expectSnappedToBoundary(content, slide(2));

    await next.click();
    await expect(slide(3)).toHaveAttribute("data-selected", "true");
    await expectSnappedToBoundary(content, slide(3));

    await previous.click();
    await expect(slide(2)).toHaveAttribute("data-selected", "true");
    await expectSnappedToBoundary(content, slide(2));
  });

  test("Next becomes disabled at the last slide, and re-enables Previous away from the first", async ({ page }) => {
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const next = frame.getByRole("button", { name: "Next slide" });
    const previous = frame.getByRole("button", { name: "Previous slide" });

    await expect(previous).toBeDisabled();
    await expect(next).toBeEnabled();

    for (let i = 0; i < 4; i++) {
      await next.click();
    }

    await expect(next).toBeDisabled();
    await expect(previous).toBeEnabled();

    await previous.click();
    await expect(next).toBeEnabled();
  });
});

test.describe("Carousel: root-level keyboard paging (LTR)", () => {
  test("ArrowRight moves to the next slide, ArrowLeft moves back, focus stays put", async ({ page }) => {
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const next = frame.getByRole("button", { name: "Next slide" });
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 5` });

    // Focus somewhere inside the carousel -- the root-level handler fires
    // for a keydown anywhere inside it, not only on the buttons.
    await next.focus();

    await page.keyboard.press("ArrowRight");
    await expect(slide(2)).toHaveAttribute("data-selected", "true");
    await expect(next).toBeFocused();

    await page.keyboard.press("ArrowRight");
    await expect(slide(3)).toHaveAttribute("data-selected", "true");

    await page.keyboard.press("ArrowLeft");
    await expect(slide(2)).toHaveAttribute("data-selected", "true");
  });
});

test.describe("Carousel: root-level keyboard paging (RTL, swapped per Radix's convention)", () => {
  test("ArrowLeft moves to the next slide and ArrowRight moves back -- the opposite of LTR", async ({ page }) => {
    await goto(page, "rtl");
    const frame = demoFrame(page, "rtl");
    const next = frame.getByRole("button", { name: "Next slide" });
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 4` });

    await next.focus();
    await expect(slide(1)).toHaveAttribute("data-selected", "true");

    await page.keyboard.press("ArrowLeft");
    await expect(slide(2)).toHaveAttribute("data-selected", "true");

    await page.keyboard.press("ArrowLeft");
    await expect(slide(3)).toHaveAttribute("data-selected", "true");

    await page.keyboard.press("ArrowRight");
    await expect(slide(2)).toHaveAttribute("data-selected", "true");
  });

  test("Next/Previous accessible names are unaffected by direction (only the arrow-key mapping swaps)", async ({ page }) => {
    await goto(page, "rtl");
    const frame = demoFrame(page, "rtl");
    await expect(frame.getByRole("button", { name: "Previous slide" })).toBeVisible();
    await expect(frame.getByRole("button", { name: "Next slide" })).toBeVisible();
  });
});

test.describe("Carousel: vertical orientation pages on the block axis", () => {
  test("ArrowDown/ArrowUp page the vertical carousel, and it snaps on the block axis", async ({ page }) => {
    await goto(page, "vertical");
    const frame = demoFrame(page, "vertical");
    const content = frame.locator(".dx-carousel-content");
    const next = frame.getByRole("button", { name: "Next slide" });
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 4` });

    await expectSnappedToBoundary(content, slide(1), "vertical");

    await next.focus();
    await page.keyboard.press("ArrowDown");
    await expect(slide(2)).toHaveAttribute("data-selected", "true");
    await expectSnappedToBoundary(content, slide(2), "vertical");

    await page.keyboard.press("ArrowUp");
    await expect(slide(1)).toHaveAttribute("data-selected", "true");
  });
});

test.describe("Carousel: a custom picker built on use_carousel()'s CarouselApi", () => {
  // Lives on the `api` demo now -- the old `indicators` variant that used
  // to host this custom, non-tablist dot picker was merged into the APG
  // tablist demo (`CarouselIndicators`/`CarouselIndicator`, renamed from
  // `CarouselTabList`/`CarouselTab`); see playwright/carousel.spec.ts's own
  // "tablist (dot-picker) variant" describe block for that one instead.
  test("clicking a dot indicator jumps directly to that slide, and the active dot tracks the selection", async ({ page }) => {
    await goto(page, "api");
    const frame = demoFrame(page, "api");
    const indicators = frame.locator('[aria-label="Slide picker"] button');
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 5` });

    await expect(indicators).toHaveCount(5);
    await expect(indicators.nth(0)).toHaveAttribute("data-active", "true");

    await indicators.nth(2).click();
    await expect(slide(3)).toHaveAttribute("data-selected", "true");
    await expect(indicators.nth(2)).toHaveAttribute("data-active", "true");
    await expect(indicators.nth(0)).toHaveAttribute("data-active", "false");
  });

  test("the visible 'Slide n of m' counter tracks the selection", async ({ page }) => {
    await goto(page, "api");
    const frame = demoFrame(page, "api");
    const counter = frame.locator("p", { hasText: /^Slide \d+ of \d+$/ });

    await expect(counter).toHaveText("Slide 1 of 5");
    await frame.getByRole("button", { name: "Next slide" }).click();
    await expect(counter).toHaveText("Slide 2 of 5");
  });
});

test.describe("Carousel: light and dark mode render without error", () => {
  for (const dark of [false, true]) {
    test(`main variant loads in ${dark ? "dark" : "light"} mode`, async ({ page }) => {
      await page.goto(
        `${BASE_URL}/component/?name=carousel&variant=main&${dark ? "dark_mode=true" : ""}`,
        GOTO_OPTS,
      );
      const frame = demoFrame(page, "main");
      await expect(frame.getByRole("region")).toBeVisible();
    });
  }
});

test.describe("Axe automated scan", () => {
  test("carousel component page (every variant mounted) has no automatically detectable a11y issues", async ({ page }) => {
    await goto(page, "main");
    await expectNoAxeViolations(page, "carousel: all variants", {
      excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT],
    });
  });
});

/**
 * round5 regression: switching from slide 1 to slide 2 used to jump
 * instantly instead of animating (every later transition already
 * animated correctly) -- see `primitives/src/carousel.rs`'s own mount-time
 * effect doc for the root cause and construction. Confirmed live before
 * fixing it (round5, `$S/round5/carousel-drag/item1-repro-{main,rtl}.log`):
 * the first click sent `CAROUSEL_SCROLL_INTO_VIEW_JS` `{instant: true,
 * behavior: 'auto'}` while every later click sent `{instant: false,
 * behavior: 'smooth'}`, on an unmodified tree with the test browser
 * reporting `prefers-reduced-motion: no-preference` throughout -- so this
 * asserts the actual, sampled scroll motion rather than "the code chose
 * smooth", which the pre-fix code could still have passed by coincidence
 * of wording.
 */
test.describe("Carousel: the first paged transition actually animates (round5 regression)", () => {
  test("clicking Next from slide 1 passes through an intermediate scroll position, not an instant jump", async ({ page }) => {
    await goto(page, "main");
    const reducedMotion = await page.evaluate(
      () => window.matchMedia("(prefers-reduced-motion: reduce)").matches,
    );
    test.skip(
      reducedMotion,
      "test browser reports prefers-reduced-motion: reduce -- CAROUSEL_SCROLL_INTO_VIEW_JS deliberately forces an instant scroll in that case (WCAG 2.3.3 territory), so whether the transition animates cannot be asked honestly in this environment",
    );

    const frame = demoFrame(page, "main");
    const content = frame.locator(".dx-carousel-content");
    const contentId = (await content.getAttribute("id"))!;
    const next = frame.getByRole("button", { name: "Next slide" });

    const samples = await sampleScrollDuring(page, contentId, "scrollLeft", () => next.click());

    expect(
      passesThroughAnIntermediateValue(samples),
      `expected an intermediate scrollLeft strictly between the first (${samples[0]}) and last (${samples[samples.length - 1]}) sample; got: ${JSON.stringify(samples)}`,
    ).toBe(true);
  });
});

/**
 * round5 feature: mouse/pen drag-to-scroll on the track, layered on the
 * same scroll-snap construction (see `primitives/src/carousel.rs`'s own
 * "Pointer drag" doc). Touch is deliberately never exercised here -- it
 * already scrolls natively and Playwright's own synthetic touch input
 * would not exercise this code path anyway (`CAROUSEL_DRAG_JS` explicitly
 * ignores `pointerType === 'touch'`).
 */
test.describe("Carousel: pointer drag (mouse/pen)", () => {
  test("a drag past the threshold advances the slide, and selected (buttons, N of M label) stays correct", async ({ page }) => {
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const content = frame.locator(".dx-carousel-content");
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 5` });
    await expect(slide(1)).toHaveAttribute("data-selected", "true");
    await expect(frame.getByRole("button", { name: "Previous slide" })).toBeDisabled();

    const pitch = await slidePitch(slide(1));
    await dragBy(page, content, -pitch * 0.7, 0);

    await expectSnappedToBoundary(content, slide(2));
    await expect(slide(2)).toHaveAttribute("data-selected", "true");
    // The exact same bridge a click/keypress uses (`use_carousel_scroll_tracking`)
    // is what a drag settles through too -- Previous's disabled state is
    // one of the things that would desync if a drag had its own, second
    // source of truth for the index instead.
    await expect(frame.getByRole("button", { name: "Previous slide" })).toBeEnabled();
  });

  test("a click inside a slide still works when the pointer never crosses the drag threshold", async ({ page }) => {
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const button = frame.getByTestId("carousel-slide-button");
    await expect(button).toHaveAttribute("data-clicked", "false");

    await button.click();

    await expect(button).toHaveAttribute("data-clicked", "true");
  });

  test("a real drag starting on a slide's own button pages the carousel and suppresses that button's own click", async ({ page }) => {
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const button = frame.getByTestId("carousel-slide-button");
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 5` });

    // Pitch measured from the slide itself, not `button` -- `button` is a
    // small element *inside* slide 1, and its own box is far narrower than
    // the track's actual per-slide pitch (see `slidePitch`'s own doc). The
    // drag still *originates* on `button`, exercising the same "a drag
    // starting on interactive content still pages, and suppresses that
    // content's own click" behavior this test is for.
    const pitch = await slidePitch(slide(1));
    await dragBy(page, button, -pitch * 0.7, 0);

    await expectSnappedToBoundary(frame.locator(".dx-carousel-content"), slide(2));
    await expect(slide(2)).toHaveAttribute("data-selected", "true");
    await expect(button).toHaveAttribute("data-clicked", "false");
  });

  /**
   * A drag that ends clamped against an edge comes to rest exactly on a slide,
   * so its release needs no settle scroll -- and every `scrollend` during the
   * drag is dropped while `data-dragging` is set. Before the tracking bridge
   * treated the end of a drag as the end of a scroll, nothing reported where
   * it rested: dragging from slide 2 hard past slide 1 left `selected` on
   * slide 2 with slide 1 showing (measured on this build before the fix:
   * "2 of 5" at scrollLeft 0).
   */
  test("a drag from slide 2 hard past the first slide selects slide 1 (the drag's end reports where it rested)", async ({
    page,
  }) => {
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const content = frame.locator(".dx-carousel-content");
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 5` });
    await frame.getByRole("button", { name: "Next slide" }).click();
    await expectSnappedToBoundary(content, slide(2));
    await expect(slide(2)).toHaveAttribute("data-selected", "true");

    await dragBy(page, content, (await slidePitch(slide(2))) * 2.5, 0, 20);
    await expectSnappedToBoundary(content, slide(1));
    await expect(slide(1)).toHaveAttribute("data-selected", "true");
    await expect(frame.getByRole("button", { name: "Previous slide" })).toBeDisabled();
  });

  test("a short movement below the threshold does not page", async ({ page }) => {
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const content = frame.locator(".dx-carousel-content");
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 5` });

    await dragBy(page, content, -2, 0, 2);
    await page.waitForTimeout(300);

    await expect(slide(1)).toHaveAttribute("data-selected", "true");
  });

  test("drag works under dir=rtl -- the physical direction that means 'next' mirrors, matching the keyboard's own RTL swap", async ({ page }) => {
    await goto(page, "rtl");
    const frame = demoFrame(page, "rtl");
    const content = frame.locator(".dx-carousel-content");
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 4` });

    // Rightward, not leftward -- see primitives/src/carousel.rs's own
    // "Pointer drag" doc for why this mirrors LTR (the same swap
    // `ArrowLeft`/`ArrowRight` already gets at the carousel root).
    const pitch = await slidePitch(slide(1));
    await dragBy(page, content, pitch * 0.7, 0);

    await expectSnappedToBoundary(content, slide(2));
    await expect(slide(2)).toHaveAttribute("data-selected", "true");
  });

  test("drag works in the vertical variant", async ({ page }) => {
    await goto(page, "vertical");
    const frame = demoFrame(page, "vertical");
    const content = frame.locator(".dx-carousel-content");
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 4` });

    const pitch = await slidePitch(slide(1), "vertical");
    await dragBy(page, content, 0, -pitch * 0.7);

    await expectSnappedToBoundary(content, slide(2), "vertical");
    await expect(slide(2)).toHaveAttribute("data-selected", "true");
  });
});

/**
 * carousel-feel lane, item 2: a drag release used to settle on the
 * nearest slide by restoring `scroll-snap-type` and trusting Chromium's
 * own re-snap to both choose the destination *and* animate the way there
 * -- measured, it never animated: sampling `scrollLeft` across a release
 * yielded exactly two distinct values (the mid-drag position and the
 * final slide), both landing instantly, for drags of very different
 * lengths. Fixed by routing the release through the same `scrollIntoView`
 * paging path `CarouselPrevious`/`CarouselNext` already use -- see
 * `primitives/src/carousel.rs`'s own `CarouselContent` doc, "Release
 * settle" section, for the construction.
 *
 * Sampling starts only once the drag has already stopped moving and is
 * sitting at a raw, unsnapped position -- `page.mouse.up()` is the only
 * action inside `sampleScrollDuring`'s own `trigger`, not the whole drag
 * (unlike the "pointer drag" describe block above). The drag's own
 * per-move `scrollBy` calls already produce many intermediate values on
 * their own, which would make an assertion over the *whole* gesture pass
 * even if the release itself were still an instant jump -- isolating the
 * release is what actually exercises this fix, the same way the "first
 * paged transition actually animates" describe block isolates a single
 * button click.
 */
test.describe("Carousel: a drag release actually animates to the nearest slide (item 2 regression)", () => {
  test("releasing a drag passes through several intermediate scrollLeft values, not an instant jump", async ({
    page,
  }) => {
    await goto(page, "main");
    const reducedMotion = await page.evaluate(
      () => window.matchMedia("(prefers-reduced-motion: reduce)").matches,
    );
    test.skip(
      reducedMotion,
      "test browser reports prefers-reduced-motion: reduce -- the release deliberately forces an instant scroll in that case, so whether it animates cannot be asked honestly in this environment",
    );

    const frame = demoFrame(page, "main");
    const content = frame.locator(".dx-carousel-content");
    const contentId = (await content.getAttribute("id"))!;
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 5` });

    await content.scrollIntoViewIfNeeded();
    const box = await content.boundingBox();
    if (!box) {
      throw new Error("content has no bounding box");
    }
    const pitch = await slidePitch(slide(1));
    const startX = box.x + box.width / 2;
    const startY = box.y + box.height / 2;
    const dx = -pitch * 0.7;
    const steps = 12;

    // Drag most of the way to slide 2 but stop short of releasing, so
    // sampling begins from wherever the drag currently sits (see this
    // describe block's own header for why the release must be isolated).
    await page.mouse.move(startX, startY);
    await page.mouse.down();
    for (let i = 1; i <= steps; i++) {
      await page.mouse.move(startX + (dx * i) / steps, startY);
      await page.waitForTimeout(16);
    }

    const samples = await sampleScrollDuring(page, contentId, "scrollLeft", () => page.mouse.up());

    expect(
      passesThroughAnIntermediateValue(samples),
      `expected an intermediate scrollLeft strictly between the first (${samples[0]}) and last (${samples[samples.length - 1]}) sample; got: ${JSON.stringify(samples)}`,
    ).toBe(true);
    await expectSnappedToBoundary(content, slide(2));
    await expect(slide(2)).toHaveAttribute("data-selected", "true");
  });
});

/**
 * `scroll-snap-stop: always` (`primitives/src/carousel.rs`, a plain inline
 * `style` declaration on each slide, right alongside `scroll-snap-align`
 * -- see that file's own `CarouselContent` doc, "scroll-snap-stop"
 * section) requires this track to stop at the first snap position a
 * scroll operation would otherwise pass, rather than skipping over
 * several to settle on whichever is numerically nearest. Measured to
 * genuinely cap a browser-animated smooth scroll -- a caller's own
 * `scrollBy(..., {behavior: 'smooth'})`, and by the same CSS Scroll Snap
 * Spec language a native wheel/trackpad fling -- to one slide of travel
 * regardless of the requested distance, which is what this test exercises
 * directly.
 *
 * The distance matters. An earlier version of this test scrolled 2.5
 * slide widths and asserted landing on slide 2 -- a false green, because
 * "settle on whichever snap point is numerically nearest" (the un-capped
 * behaviour, indistinguishable from this property being entirely inert)
 * *also* lands on slide 2 at that distance. That version measured 54/54
 * passing against a build where `scroll-snap-stop` was inert on every
 * slide (a broken `<style>`-tag/`@supports` construction -- see
 * `CarouselContent`'s own doc for that history). Three slide widths is
 * used instead because it discriminates: uncapped, an exact 3.0-pitch
 * scroll has no rounding ambiguity and settles on slide 4; capped, the
 * scroll must stop at the very first snap position it would otherwise
 * pass, landing on slide 2 -- two different, separately checkable
 * predictions.
 *
 * Deliberately **not** exercised here via this crate's own pointer-drag
 * gesture (`dragBy`): measured directly (isolated `page.evaluate` against
 * a release build, outside this test file) that this crate's own drag
 * path does *not* get this guarantee, because it moves the track with
 * many independently-instant, unsnapped `scrollBy` calls while
 * `scroll-snap-type` is suspended for the gesture's own duration, and
 * restoring that property afterward is a fresh, static re-evaluation of
 * an already-stationary position -- not a "scrolling operation" this
 * property constrains on this engine (Chromium). A drag-driven version of
 * this exact test was red on this same build/property (settled 2 slides
 * from the origin, not 1) before this test was rewritten to test the
 * mechanism this property actually delivers, rather than the one this
 * lane originally hoped it would also cover. See `CarouselContent`'s own
 * doc for the full write-up; that gap is being reported, not silently
 * worked around here.
 */
test.describe("Carousel: scroll-snap-stop caps a smooth scroll to one slide", () => {
  test("a smooth scroll covering three slide widths lands exactly one slide away, not three", async ({ page }) => {
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const content = frame.locator(".dx-carousel-content");
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 5` });
    await expect(slide(1)).toHaveAttribute("data-selected", "true");

    const pitch = await slidePitch(slide(1));
    const contentId = await content.getAttribute("id");
    // 3 slides' worth -- see this describe block's own header for why this
    // distance (not the previous 2.5) is the one that actually
    // discriminates a working `scroll-snap-stop` from an inert one.
    await page.evaluate(
      ({ id, distance }) => {
        document.getElementById(id)!.scrollBy({ left: distance, behavior: "smooth" });
      },
      { id: contentId, distance: pitch * 3 },
    );

    await expectSnappedToBoundary(content, slide(2));
    await expect(slide(2)).toHaveAttribute("data-selected", "true");
    await expect(slide(3)).toHaveAttribute("data-selected", "false");
    await expect(slide(4)).toHaveAttribute("data-selected", "false");
  });
});

/**
 * True if `samples` pass strictly through a value between their own first
 * and last entry, on a scale-relative tolerance.
 *
 * Deliberately NOT `passesThroughAnIntermediateValue`: that one guards with
 * `hi - lo < 2` and bands with `lo + 1`/`hi - 1`, which are pixel
 * thresholds. Opacity's whole range is 0..1, so a 1 -> 0.5 fade has a span
 * of 0.5 and would fail that guard unconditionally -- the test would be red
 * no matter how well the fade worked. This one takes a minimum span and
 * pads by a fraction of the observed span instead.
 */
function passesThroughAnIntermediateFraction(samples: number[], minSpan = 0.05): boolean {
  if (samples.length < 3) {
    return false;
  }
  const start = samples[0];
  const end = samples[samples.length - 1];
  const lo = Math.min(start, end);
  const hi = Math.max(start, end);
  if (hi - lo < minSpan) {
    return false;
  }
  const pad = (hi - lo) * 0.1;
  return samples.some((v) => v > lo + pad && v < hi - pad);
}

/** Sample an element's computed opacity every frame across `trigger`. */
async function sampleOpacityDuring(
  page: Page,
  selector: string,
  trigger: () => Promise<void>,
  windowMs = 1500,
): Promise<number[]> {
  await page.evaluate(
    ({ selector, windowMs }) => {
      const w = window as unknown as { __dxOpacity: number[]; __dxOpacityDone: boolean };
      w.__dxOpacity = [];
      w.__dxOpacityDone = false;
      const el = document.querySelector(selector) as HTMLElement | null;
      if (!el) {
        w.__dxOpacityDone = true;
        return;
      }
      const t0 = performance.now();
      const tick = () => {
        w.__dxOpacity.push(parseFloat(getComputedStyle(el).opacity));
        if (performance.now() - t0 > windowMs) {
          w.__dxOpacityDone = true;
          return;
        }
        requestAnimationFrame(tick);
      };
      requestAnimationFrame(tick);
    },
    { selector, windowMs },
  );
  await trigger();
  await page.waitForFunction(
    () => (window as unknown as { __dxOpacityDone: boolean }).__dxOpacityDone === true,
    undefined,
    { timeout: windowMs + 5000 },
  );
  return page.evaluate(() => (window as unknown as { __dxOpacity: number[] }).__dxOpacity);
}

/**
 * The disabled-state fade (round 6, user-reported): Previous/Next are
 * genuinely `disabled` at the first/last slide, and that opacity change used
 * to apply instantly. The transition lives on the BASE rule rather than
 * inside `:disabled` -- scoped to `:disabled` it would animate entering the
 * state but not leaving it, since the non-disabled style would then carry no
 * transition of its own. So both directions are asserted here.
 */
test.describe("Carousel: the disabled-state opacity change actually fades", () => {
  test("Next fades both entering and leaving :disabled", async ({ page }) => {
    await goto(page, "main");
    const reducedMotion = await page.evaluate(
      () => window.matchMedia("(prefers-reduced-motion: reduce)").matches,
    );
    test.skip(
      reducedMotion,
      "test browser reports prefers-reduced-motion: reduce -- the shared theme layer deliberately suppresses this fade, so whether it animates cannot be asked honestly here",
    );

    const frame = demoFrame(page, "main");
    const next = frame.getByRole("button", { name: "Next slide" });
    const previous = frame.getByRole("button", { name: "Previous slide" });

    // Walk to the second-to-last slide so the next click disables Next.
    for (let i = 0; i < 3; i++) {
      await next.click();
      await page.waitForTimeout(700);
    }

    const disabling = await sampleOpacityDuring(page, ".dx-carousel-next", () => next.click());
    await expect(next).toBeDisabled();
    expect(
      passesThroughAnIntermediateFraction(disabling),
      `entering :disabled should pass through an intermediate opacity; got ${JSON.stringify(disabling)}`,
    ).toBe(true);

    await page.waitForTimeout(700);

    const enabling = await sampleOpacityDuring(page, ".dx-carousel-next", () => previous.click());
    await expect(next).toBeEnabled();
    expect(
      passesThroughAnIntermediateFraction(enabling),
      `leaving :disabled should pass through an intermediate opacity; got ${JSON.stringify(enabling)}`,
    ).toBe(true);
  });
});

/**
 * Edge rubber-band, pointer half (mode B3), backlog row 102 -- port of the
 * closed design in `dev-docs/research/carousel-overscroll-2026-09-23.md`.
 * See `primitives/src/carousel.rs`'s own `CarouselContent` doc, "Edge
 * rubber-band" section, for the construction these tests hold to: a
 * `transform` on `.dx-carousel-content` only, never a spacer child, and
 * never a change to `selected`. The wheel/trackpad half is mode D now; its
 * tests are the next describe block.
 *
 * RED-FIRST: written and run against the pre-port `carousel.rs` (mode A,
 * plain clamping) before any of this file's own port landed -- every test
 * below failed (`readContentTransform` always returned `""`, since nothing
 * ever wrote `style.transform`). All pass against the ported code.
 */
test.describe("Carousel: edge rubber-band, pointer (mode B3)", () => {
  test("pointer overdrag past the first slide produces a bounded, nonzero transform that returns to identity on release", async ({
    page,
  }) => {
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const content = frame.locator(".dx-carousel-content");
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 5` });
    await expect(slide(1)).toHaveAttribute("data-selected", "true");
    expect(await readContentTransform(content)).toBe("");

    const pitch = await slidePitch(slide(1));
    // Positive dx at slide 1 (the start) asks for a "previous" slide that
    // does not exist -- see `CAROUSEL_DRAG_JS`'s own "Pointer drag" doc for
    // why a positive drag is the physical-previous direction in this
    // (LTR, horizontal) variant.
    const rawTravel = pitch * 1.5;
    await dragHold(page, content, rawTravel, 0);

    const transform = await readContentTransform(content);
    const depth = parseTranslatePx(transform);
    expect(depth, `expected a nonzero translateX, got "${transform}"`).not.toBeNull();
    expect(depth!).toBeGreaterThan(0);
    // Damped: the visible depth is strictly less than the raw pointer
    // travel that produced it -- the rubber function's whole point.
    expect(depth!).toBeLessThan(rawTravel);

    await page.mouse.up();
    await expect(async () => {
      expect(await readContentTransform(content)).toBe("");
    }).toPass({ timeout: 2000 });

    // Never a second source of truth for the index (research doc's own
    // framing): the bounce never moved `selected`.
    await expect(slide(1)).toHaveAttribute("data-selected", "true");
    await expect(frame.getByRole("button", { name: "Previous slide" })).toBeDisabled();
  });

  test("pointer overdrag past the last slide produces a bounded, nonzero transform that returns to identity on release", async ({
    page,
  }) => {
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const content = frame.locator(".dx-carousel-content");
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 5` });
    const next = frame.getByRole("button", { name: "Next slide" });

    for (let i = 0; i < 4; i++) {
      await next.click();
      await expectSnappedToBoundary(content, slide(i + 2));
    }
    await expect(slide(5)).toHaveAttribute("data-selected", "true");
    await expect(next).toBeDisabled();

    const pitch = await slidePitch(slide(5));
    const rawTravel = pitch * 1.5;
    // Negative dx at the last slide asks for a "next" slide that does not
    // exist.
    await dragHold(page, content, -rawTravel, 0);

    const transform = await readContentTransform(content);
    const depth = parseTranslatePx(transform);
    expect(depth, `expected a nonzero translateX, got "${transform}"`).not.toBeNull();
    expect(depth!).toBeLessThan(0);
    expect(Math.abs(depth!)).toBeLessThan(rawTravel);

    await page.mouse.up();
    await expect(async () => {
      expect(await readContentTransform(content)).toBe("");
    }).toPass({ timeout: 2000 });

    await expect(slide(5)).toHaveAttribute("data-selected", "true");
    await expect(next).toBeDisabled();
  });

  test("pointer overdrag at the start boundary works under dir=rtl", async ({ page }) => {
    await goto(page, "rtl");
    const frame = demoFrame(page, "rtl");
    const content = frame.locator(".dx-carousel-content");
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 4` });
    await expect(slide(1)).toHaveAttribute("data-selected", "true");

    const pitch = await slidePitch(slide(1));
    const rawTravel = pitch * 1.5;
    // Negative dx is the physical-previous direction under RTL (mirrored
    // from LTR -- see the "pointer drag" describe block's own rtl test,
    // which drags positive to advance).
    await dragHold(page, content, -rawTravel, 0);

    const transform = await readContentTransform(content);
    const depth = parseTranslatePx(transform);
    expect(depth, `expected a nonzero translateX, got "${transform}"`).not.toBeNull();
    expect(depth!).toBeLessThan(0);
    expect(Math.abs(depth!)).toBeLessThan(rawTravel);

    await page.mouse.up();
    await expect(async () => {
      expect(await readContentTransform(content)).toBe("");
    }).toPass({ timeout: 2000 });

    await expect(slide(1)).toHaveAttribute("data-selected", "true");
  });

  test("pointer overdrag at the start boundary works in the vertical variant", async ({ page }) => {
    await goto(page, "vertical");
    const frame = demoFrame(page, "vertical");
    const content = frame.locator(".dx-carousel-content");
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 4` });
    await expect(slide(1)).toHaveAttribute("data-selected", "true");

    const pitch = await slidePitch(slide(1), "vertical");
    const rawTravel = pitch * 1.5;
    // Positive dy is the physical-previous direction on the block axis
    // (the "pointer drag" describe block's own vertical test drags
    // negative to advance).
    await dragHold(page, content, 0, rawTravel);

    const transform = await readContentTransform(content);
    const depth = parseTranslatePx(transform);
    expect(depth, `expected a nonzero translateY, got "${transform}"`).not.toBeNull();
    expect(depth!).toBeGreaterThan(0);
    expect(depth!).toBeLessThan(rawTravel);

    await page.mouse.up();
    await expect(async () => {
      expect(await readContentTransform(content)).toBe("");
    }).toPass({ timeout: 2000 });

    await expect(slide(1)).toHaveAttribute("data-selected", "true");
  });

  test("a mid-range pointer drag produces no transform and still pages correctly (regression guard)", async ({
    page,
  }) => {
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const content = frame.locator(".dx-carousel-content");
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 5` });

    const pitch = await slidePitch(slide(1));
    await dragBy(page, content, -pitch * 0.7, 0);

    await expectSnappedToBoundary(content, slide(2));
    await expect(slide(2)).toHaveAttribute("data-selected", "true");
    expect(await readContentTransform(content)).toBe("");
  });
});

/**
 * Wheel/trackpad edge band (mode D, the "platform-aware band") --
 * `primitives/src/carousel.rs`'s `CAROUSEL_WHEEL_BAND_JS`, ported from the
 * owner's bench (`bench-rev40.html`, `mode === "onewriter"`). It replaced
 * B3's wheel path; the pointer half above is still B3.
 *
 * D's contract, which every test below holds it to:
 * - the band is a `transform` on `.dx-carousel-content` (the scroller) and
 *   `""` at rest -- the same element and reader (`readContentTransform`)
 *   the B3 tests used, never a scroll-position write;
 * - ownership is decided once per gesture from where the scroller RESTS
 *   when the gesture starts: only a gesture that starts at rest at an edge
 *   and pushes into it gets a band (bounded by `LIMIT` = 0.5 x the axis);
 * - release: smooth momentum decay, cursor movement, slow-push silence
 *   (250ms), notched-wheel silence (140ms), reversal, or the 1500ms
 *   backstop; then an ease-out spring-back (340ms) to `""`;
 * - only this carousel's own axis counts.
 *
 * SAMPLING (backlog row 112's construction): every sequence is dispatched
 * IN-PAGE with real inter-event timing (`playWheel`), and the transform is
 * sampled in-page on every animation frame and right after every event --
 * an animated value is never read across a CDP round trip. Synthetic
 * `WheelEvent`s are untrusted, so they never scroll the scroller itself;
 * where a test needs the platform's own scroll (a fling arriving at slide 1)
 * it moves the scroller in-page, instantly, standing in for the compositor.
 * The telemetry this reads (`window.__dxCarouselWheel`) is D's own
 * debug-flag output (`localStorage["dx-carousel-debug"] === "1"`), enabled
 * per test by `enableWheelDebug`. Real-hardware feel remains the owner's
 * own check; these pin the logic with recorded-shape sequences.
 */

/** Delta magnitudes (px) shaped like the owner's recordings; each test
 * applies the sign that pushes into the edge under test. */
const WHEEL = {
  // A trackpad push: a ramp, then fingers lift and momentum decays.
  rampAndDecay: [3, 9, 21, 35, 50, 60, 62, 58, 54, 50, 46, 43, 40, 37, 34, 31, 29, 27, 25, 23, 21, 19, 18, 16, 15, 14, 13],
  // Rising, never repeating, peak >= 40px: the band holds (no decay, no
  // silence release shorter than the 1500ms backstop).
  hold: [10, 25, 45, 60, 70, 75, 78, 80, 83],
  // A native fling's momentum tail (~0.93 per event: "coasting").
  momentum: [95, 88, 82, 76, 70, 65, 61, 57, 53, 49],
  // The owner's recorded re-push after coasting into slide 1 (bench rev 40
  // note): a dip, then a rise.
  repush: [13, 28, 67, 112, 133, 160],
  // A slow push: every step under 40px (the owner's were 5-16px).
  slow: [5, 8, 12, 14, 12, 13, 12, 14, 12],
};

type WheelPlay = {
  /** Signed deltas, one event each. */
  deltas: readonly number[];
  axis?: "x" | "y";
  deltaMode?: number;
  shiftKey?: boolean;
  /** Real time between events (ms). */
  dtMs?: number;
  /** Keep sampling this long after the last event (ms). */
  tailMs?: number;
  /** After this many events, move the scroller (instantly) so its first
   * slide rests at the physical start edge -- the platform's own momentum
   * scroll carrying the track into slide 1. */
  arriveAtStartAfter?: number;
};
type WheelRun = {
  /** Band (signed px) sampled every animation frame, `t` in ms from the first event. */
  samples: { t: number; v: number }[];
  /** Band right after each event. */
  afterEvent: number[];
  eventTimes: number[];
  axisSize: number;
  finalTransform: string;
};

async function playWheel(content: Locator, play: WheelPlay): Promise<WheelRun> {
  return content.evaluate(async (node, p) => {
    const el = node as HTMLElement;
    const axis = p.axis ?? "x";
    const parse = (s: string) => {
      const m = s.match(/translate[XY]\(([-\d.]+)px\)/);
      return m ? Number.parseFloat(m[1]) : 0;
    };
    const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
    const samples: { t: number; v: number }[] = [];
    const afterEvent: number[] = [];
    const eventTimes: number[] = [];
    const t0 = performance.now();
    let running = true;
    const tick = () => {
      samples.push({ t: performance.now() - t0, v: parse(el.style.transform) });
      if (running) requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);
    for (let i = 0; i < p.deltas.length; i++) {
      const d = p.deltas[i];
      el.dispatchEvent(
        new WheelEvent("wheel", {
          deltaX: axis === "x" ? d : 0,
          deltaY: axis === "y" ? d : 0,
          deltaMode: p.deltaMode ?? 0,
          shiftKey: p.shiftKey ?? false,
          bubbles: true,
          cancelable: true,
        }),
      );
      const t = performance.now() - t0;
      eventTimes.push(t);
      afterEvent.push(parse(el.style.transform));
      samples.push({ t, v: afterEvent[i] });
      if (p.arriveAtStartAfter === i + 1) {
        const horizontal = el.getAttribute("data-orientation") !== "vertical";
        const c = el.getBoundingClientRect();
        let lo = Infinity;
        for (const child of Array.from(el.children)) {
          const r = child.getBoundingClientRect();
          lo = Math.min(lo, horizontal ? r.left : r.top);
        }
        const delta = lo - (horizontal ? c.left : c.top);
        el.scrollBy({ left: horizontal ? delta : 0, top: horizontal ? 0 : delta, behavior: "instant" });
      }
      if (i < p.deltas.length - 1) await sleep(p.dtMs ?? 16);
    }
    await sleep(p.tailMs ?? 0);
    running = false;
    const track = el.parentElement as HTMLElement;
    const horizontal = el.getAttribute("data-orientation") !== "vertical";
    return {
      samples,
      afterEvent,
      eventTimes,
      axisSize: horizontal ? track.clientWidth : track.clientHeight,
      finalTransform: el.style.transform,
    };
  }, play);
}

/** Turn on D's debug telemetry for the next navigation (read once at mount). */
async function enableWheelDebug(page: Page): Promise<void> {
  await page.addInitScript(() => {
    try {
      window.localStorage.setItem("dx-carousel-debug", "1");
    } catch {
      // A storage-less context simply records nothing; tests that need it fail loudly.
    }
  });
}

type WheelRecord = {
  id: string;
  startedAtEdge: string;
  /** The edge the gesture started at rest against, when `isTrueEnd` refused it. */
  falseEnd: string | null;
  owned: boolean | null;
  notched: boolean;
  deltas: number[];
  releasedBy: string | null;
  peakBand: number;
  lastEventAt: number;
  releasedAt: number | null;
  endedByRepush: boolean;
};

/** D's telemetry records for this scroller only (every variant shares the page). */
async function wheelRecords(content: Locator): Promise<WheelRecord[]> {
  return content.evaluate((el) => {
    const log = (window as unknown as { __dxCarouselWheel?: WheelRecord[] }).__dxCarouselWheel ?? [];
    return JSON.parse(JSON.stringify(log.filter((r) => r.id === el.id)));
  });
}

/** Waits (polling a static end state, not sampling an animation) for the band to be gone. */
async function expectBandGone(content: Locator, timeout = 3000): Promise<void> {
  await expect.poll(() => readContentTransform(content), { timeout }).toBe("");
}

function maxAbsBand(run: WheelRun): number {
  return Math.max(0, ...run.samples.map((s) => Math.abs(s.v)));
}

/** In-page rAF sampler around real (CDP) input, for the tests that must use
 * trusted events -- same "sample in-page" construction as `playWheel`. */
async function sampleBandDuring(content: Locator, action: () => Promise<void>, tailMs = 0): Promise<number[]> {
  await content.evaluate((node) => {
    const el = node as HTMLElement;
    const w = window as unknown as { __dxBandSamples: number[]; __dxBandRun: boolean };
    w.__dxBandSamples = [];
    w.__dxBandRun = true;
    const tick = () => {
      const m = el.style.transform.match(/translate[XY]\(([-\d.]+)px\)/);
      w.__dxBandSamples.push(m ? Number.parseFloat(m[1]) : 0);
      if (w.__dxBandRun) requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);
  });
  await action();
  if (tailMs) await content.page().waitForTimeout(tailMs);
  return content.evaluate(() => {
    const w = window as unknown as { __dxBandSamples: number[]; __dxBandRun: boolean };
    w.__dxBandRun = false;
    return w.__dxBandSamples;
  });
}

const neg = (xs: number[]) => xs.map((x) => -x);

test.describe("Carousel: wheel/trackpad edge band (mode D, platform-aware)", () => {
  test("an owned push at rest at the start edge draws a band bounded by 0.5 x the axis, then springs back to identity", async ({
    page,
  }) => {
    await enableWheelDebug(page);
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const content = frame.locator(".dx-carousel-content");
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 5` });
    await expect(slide(1)).toHaveAttribute("data-selected", "true");
    expect(await readContentTransform(content)).toBe("");

    // Negative deltaX at slide 1 asks for more "previous" than exists.
    const run = await playWheel(content, { deltas: neg(WHEEL.rampAndDecay), tailMs: 900 });
    const peak = maxAbsBand(run);
    expect(peak, "an owned push must draw a band").toBeGreaterThan(5);
    expect(Math.min(...run.afterEvent), "pushing right, the band translates right").toBeGreaterThanOrEqual(0);
    expect(peak).toBeLessThanOrEqual(run.axisSize * 0.5 + 0.5);
    expect(run.finalTransform, "springs back to identity after the release").toBe("");

    const [rec] = (await wheelRecords(content)).slice(-1);
    expect(rec.startedAtEdge).toBe("left");
    expect(rec.owned).toBe(true);
    expect(rec.releasedBy, "the momentum tail is what released it").toBe("smooth-decay");
    // THE RULE: the band never moved `selected`.
    await expect(slide(1)).toHaveAttribute("data-selected", "true");
  });

  test("an owned push at the end edge draws a band in the other direction and springs back", async ({ page }) => {
    await enableWheelDebug(page);
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const content = frame.locator(".dx-carousel-content");
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 5` });
    const next = frame.getByRole("button", { name: "Next slide" });
    for (let i = 0; i < 4; i++) {
      await next.click();
      await expectSnappedToBoundary(content, slide(i + 2));
    }
    await expect(slide(5)).toHaveAttribute("data-selected", "true");

    const run = await playWheel(content, { deltas: WHEEL.rampAndDecay, tailMs: 900 });
    expect(Math.min(...run.samples.map((s) => s.v)), "pushing left past the end, the band translates left").toBeLessThan(-5);
    expect(maxAbsBand(run)).toBeLessThanOrEqual(run.axisSize * 0.5 + 0.5);
    expect(run.finalTransform).toBe("");
    const [rec] = (await wheelRecords(content)).slice(-1);
    expect(rec.startedAtEdge).toBe("right");
    expect(rec.owned).toBe(true);
    await expect(slide(5)).toHaveAttribute("data-selected", "true");
  });

  /**
   * Replaces B3's "a sustained constant wheel stream ... grows sub-linearly"
   * test. A perfectly constant stream is now, correctly, a notched wheel
   * (see the notched test below) and releases after 140ms, so the same two
   * curve properties are checked on a sustained push whose steps vary the
   * way a real hand's do: the band stays under the asymptote (`LIMIT` x the
   * axis -- 0.5 now, tighter than B3's 1.0), and doubling the input less
   * than doubles the depth (WebKit's concave curve). Assertions unchanged
   * in kind; the bound is tighter, not looser.
   */
  test("a sustained hard push stays within 0.5 x the axis and grows sub-linearly", async ({ page }) => {
    await enableWheelDebug(page);
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const content = frame.locator(".dx-carousel-content");
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 5` });
    await expect(slide(1)).toHaveAttribute("data-selected", "true");

    const sustained = Array.from({ length: 20 }, (_, i) => -(150 + (i % 3) * 7));
    // Momentum all the way down, as a real tail runs (~0.93 per event, to a
    // few px): D's decay test only counts once the peak is above 3 x the
    // smallest step this device has sent, and a page that has only ever
    // seen 100px+ steps has not learned its quantum yet.
    const tail = Array.from({ length: 34 }, (_, i) => -Math.max(3, Math.round(140 * 0.93 ** i)));
    const run = await playWheel(content, { deltas: [...sustained, ...tail], tailMs: 900 });
    const depth10 = Math.abs(run.afterEvent[9]);
    const depth20 = Math.abs(run.afterEvent[19]);
    expect(depth10).toBeGreaterThan(0);
    expect(depth20, "still pushing should still grow the depth").toBeGreaterThan(depth10);
    expect(depth20, "doubling the input should not double the depth (sub-linear curve)").toBeLessThan(depth10 * 2);
    expect(maxAbsBand(run)).toBeLessThanOrEqual(run.axisSize * 0.5 + 0.5);
    expect(run.finalTransform).toBe("");
    const [rec] = (await wheelRecords(content)).slice(-1);
    expect(rec.releasedBy).toBe("smooth-decay");
    await expect(slide(1)).toHaveAttribute("data-selected", "true");
  });

  test("a fling that arrives at slide 1 from another slide with momentum draws NO band (the platform's edge)", async ({
    page,
  }) => {
    await enableWheelDebug(page);
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const content = frame.locator(".dx-carousel-content");
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 5` });
    await frame.getByRole("button", { name: "Next slide" }).click();
    await expectSnappedToBoundary(content, slide(2));

    // Starts at slide 2 (not at rest at an edge); the "platform" carries the
    // track into slide 1 after four events and the tail keeps arriving there.
    const run = await playWheel(content, { deltas: neg(WHEEL.momentum), arriveAtStartAfter: 4, tailMs: 300 });
    expect(maxAbsBand(run), "D never draws on a gesture that did not start at rest at the edge").toBe(0);
    expect(run.finalTransform).toBe("");
    const recs = await wheelRecords(content);
    expect(recs.length).toBeGreaterThan(0);
    const [rec] = recs.slice(-1);
    expect(rec.startedAtEdge).toBe("none");
    expect(rec.owned).toBe(false);
  });

  test("after coasting into slide 1, the owner's dip-then-rise re-push starts a new, owned gesture and draws a band", async ({
    page,
  }) => {
    await enableWheelDebug(page);
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const content = frame.locator(".dx-carousel-content");
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 5` });
    await frame.getByRole("button", { name: "Next slide" }).click();
    await expectSnappedToBoundary(content, slide(2));

    // ..., -57, -53, -49 (momentum), then -13, -28, -67, -112, -133, -160
    // (new fingers) -- one continuous stream, never a GAP_MS silence, so only
    // the coast-then-rise rule (bench rev 40) can split it.
    const deltas = neg([...WHEEL.momentum, ...WHEEL.repush]);
    const run = await playWheel(content, { deltas, arriveAtStartAfter: 4, tailMs: 50 });
    const nMomentum = WHEEL.momentum.length;
    const tRepush = run.eventTimes[nMomentum];
    const before = run.samples.filter((s) => s.t < tRepush);
    expect(Math.max(0, ...before.map((s) => Math.abs(s.v))), "no band during the platform's own arrival").toBe(0);
    expect(Math.max(...run.afterEvent.slice(nMomentum)), "the re-push is owned: D draws its band").toBeGreaterThan(5);

    const recs = await wheelRecords(content);
    const owned = recs.filter((r) => r.owned === true);
    expect(owned.length).toBe(1);
    expect(owned[0].startedAtEdge).toBe("left");
    // The record before it is the arrival, ended by the re-push rule (not a gap).
    const i = recs.indexOf(owned[0]);
    expect(recs[i - 1].owned).toBe(false);
    expect(recs[i - 1].endedByRepush).toBe(true);
    await expectBandGone(content);
  });

  test("a slow push (steps under 16px) holds the band, then releases ~250ms after its last event", async ({ page }) => {
    await enableWheelDebug(page);
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const content = frame.locator(".dx-carousel-content");

    const run = await playWheel(content, { deltas: neg(WHEEL.slow), tailMs: 1100 });
    const last = run.eventTimes[run.eventTimes.length - 1];
    const held = run.afterEvent[run.afterEvent.length - 1];
    expect(held).toBeGreaterThan(0);
    // No release inside the first 200ms of silence: the band stays exactly put.
    const early = run.samples.filter((s) => s.t > last && s.t < last + 200);
    expect(early.length).toBeGreaterThan(0);
    for (const s of early) expect(s.v).toBeCloseTo(held, 5);
    expect(run.finalTransform, "released and home well before the 1500ms backstop").toBe("");

    const [rec] = (await wheelRecords(content)).slice(-1);
    expect(rec.releasedBy).toBe("slow-silence");
    const silence = (rec.releasedAt ?? 0) - rec.lastEventAt;
    expect(silence).toBeGreaterThanOrEqual(230);
    expect(silence).toBeLessThan(700);
  });

  for (const [label, play] of [
    ["deltaMode 1 (lines)", { deltas: [-3, -3, -3, -3], deltaMode: 1, dtMs: 40 }],
    ["deltaMode 0, repeated identical 120px steps", { deltas: [-120, -120, -120, -120], dtMs: 40 }],
  ] as const) {
    test(`a notched mouse wheel -- ${label} -- releases fast, not after the 1500ms backstop`, async ({ page }) => {
      await enableWheelDebug(page);
      await goto(page, "main");
      const frame = demoFrame(page, "main");
      const content = frame.locator(".dx-carousel-content");

      const run = await playWheel(content, { ...play, tailMs: 900 });
      expect(maxAbsBand(run)).toBeGreaterThan(0);
      expect(run.finalTransform, "home within ~140ms + the 340ms spring-back").toBe("");
      const [rec] = (await wheelRecords(content)).slice(-1);
      expect(rec.owned).toBe(true);
      expect(rec.notched).toBe(true);
      expect(rec.releasedBy).toBe("notch-silence");
      const silence = (rec.releasedAt ?? 0) - rec.lastEventAt;
      expect(silence).toBeGreaterThanOrEqual(120);
      expect(silence, "faster than even the slow-push release").toBeLessThan(250);
    });
  }

  test("a cursor movement after the push counts as the fingers lifting and releases the band", async ({ page }) => {
    await enableWheelDebug(page);
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const content = frame.locator(".dx-carousel-content");
    const box = await content.boundingBox();
    if (!box) throw new Error("content has no bounding box");
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);

    const run = await playWheel(content, { deltas: neg(WHEEL.hold) });
    expect(run.finalTransform, "the band is held (no silence release this short)").not.toBe("");
    await page.mouse.move(box.x + box.width / 2 + 20, box.y + box.height / 2);
    await page.mouse.move(box.x + box.width / 2 + 40, box.y + box.height / 2);
    await expectBandGone(content, 1200);
    const [rec] = (await wheelRecords(content)).slice(-1);
    expect(rec.releasedBy).toBe("cursor-moved");
  });

  test("vertical wheel input over a horizontal carousel at slide 1 draws no band, and still scrolls the page", async ({
    page,
  }) => {
    await enableWheelDebug(page);
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const content = frame.locator(".dx-carousel-content");

    for (const sign of [-1, 1]) {
      const run = await playWheel(content, { deltas: WHEEL.rampAndDecay.map((x) => sign * x), axis: "y", tailMs: 100 });
      expect(maxAbsBand(run), `deltaY ${sign < 0 ? "up" : "down"} is not this carousel's axis`).toBe(0);
    }
    // Ignored entirely: not even a gesture record.
    expect(await wheelRecords(content)).toEqual([]);

    // Real (trusted) vertical wheel over the carousel scrolls the page.
    await content.hover();
    const before = await page.evaluate(() => window.scrollY);
    const samples = await sampleBandDuring(content, async () => {
      await page.mouse.wheel(0, 200);
      await page.mouse.wheel(0, 200);
    }, 300);
    expect(Math.max(0, ...samples.map(Math.abs))).toBe(0);
    await expect.poll(() => page.evaluate(() => window.scrollY)).toBeGreaterThan(before);
  });

  test("Shift+wheel (reported as deltaY with shiftKey) counts as horizontal intent", async ({ page }) => {
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const content = frame.locator(".dx-carousel-content");
    const run = await playWheel(content, { deltas: neg(WHEEL.rampAndDecay), axis: "y", shiftKey: true, tailMs: 900 });
    expect(maxAbsBand(run)).toBeGreaterThan(5);
    expect(run.finalTransform).toBe("");
  });

  test("vertical orientation: an owned push at the top draws a translateY band and ignores horizontal input", async ({
    page,
  }) => {
    await enableWheelDebug(page);
    await goto(page, "vertical");
    const frame = demoFrame(page, "vertical");
    const content = frame.locator(".dx-carousel-content");
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 4` });
    await expect(slide(1)).toHaveAttribute("data-selected", "true");

    const sideways = await playWheel(content, { deltas: neg(WHEEL.rampAndDecay), axis: "x", tailMs: 100 });
    expect(maxAbsBand(sideways), "deltaX is not a vertical carousel's axis").toBe(0);

    const run = await playWheel(content, { deltas: neg(WHEEL.rampAndDecay), axis: "y", tailMs: 900 });
    expect(Math.min(...run.afterEvent), "pushing down at the top, the band translates down").toBeGreaterThanOrEqual(0);
    expect(maxAbsBand(run)).toBeGreaterThan(5);
    expect(maxAbsBand(run)).toBeLessThanOrEqual(run.axisSize * 0.5 + 0.5);
    expect(run.finalTransform).toBe("");
    const recs = await wheelRecords(content);
    expect(recs.map((r) => r.startedAtEdge)).toContain("top");
    await expect(slide(1)).toHaveAttribute("data-selected", "true");
  });

  test("RTL: an owned push at slide 1 (the physical right edge) draws a leftward band", async ({ page }) => {
    await enableWheelDebug(page);
    await goto(page, "rtl");
    const frame = demoFrame(page, "rtl");
    const content = frame.locator(".dx-carousel-content");
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 4` });
    await expect(slide(1)).toHaveAttribute("data-selected", "true");

    // Under RTL, slide 1 rests at the physical RIGHT; positive deltaX pushes
    // past it. All geometry is physical, so nothing here is mirrored by hand.
    const run = await playWheel(content, { deltas: WHEEL.rampAndDecay, tailMs: 900 });
    expect(Math.max(...run.afterEvent), "the band translates left").toBeLessThanOrEqual(0);
    expect(maxAbsBand(run)).toBeGreaterThan(5);
    expect(maxAbsBand(run)).toBeLessThanOrEqual(run.axisSize * 0.5 + 0.5);
    expect(run.finalTransform).toBe("");
    const [rec] = (await wheelRecords(content)).slice(-1);
    expect(rec.startedAtEdge).toBe("right");
    expect(rec.owned).toBe(true);
    await expect(slide(1)).toHaveAttribute("data-selected", "true");
  });

  /**
   * The owner's approved rule: the band only engages at a TRUE end of the
   * data (`isTrueEnd` in `CAROUSEL_WHEEL_BAND_JS`), and a looping carousel
   * has none -- including `LoopMode::Rewind`, whose ends are physical but
   * wrap. This used to assert the opposite (Rewind bands like any physical
   * edge); the rule changed, so the assertion did.
   */
  test("loop_mode Rewind never bands, at its first, middle or last slide", async ({ page }) => {
    await enableWheelDebug(page);
    await goto(page, "rewind");
    const frame = demoFrame(page, "rewind");
    const region = frame.getByRole("region", { name: "Rewind-loop gallery", exact: true });
    const content = region.locator(".dx-carousel-content");
    const slide = (n: number) => region.getByRole("group", { name: `${n} of 5` });
    const next = region.getByRole("button", { name: "Next slide" });
    await expect(slide(1)).toHaveAttribute("data-selected", "true");
    expect(await content.getAttribute("data-loop")).toBe("true");

    // Slide 1 (rest at the physical start), slide 3 (middle), slide 5 (end).
    let current = 1;
    for (const [n, pushes] of [
      [1, [-1]],
      [3, [-1, 1]],
      [5, [1]],
    ] as const) {
      while (current < n) {
        await next.click();
        current += 1;
        await expectSnappedToBoundary(content, slide(current));
      }
      for (const sign of pushes) {
        const run = await playWheel(content, { deltas: WHEEL.rampAndDecay.map((x) => sign * x), tailMs: 400 });
        expect(maxAbsBand(run), `slide ${n}, push ${sign}`).toBe(0);
      }
    }
    // The pushes at slides 1 and 5 really started at rest against an edge,
    // and were refused as not a true end -- not merely never at an edge.
    const refused = (await wheelRecords(content)).map((r) => r.falseEnd).filter(Boolean);
    expect(refused).toEqual(expect.arrayContaining(["left", "right"]));
  });

  test("a CarouselVirtualContent seamless loop has no true end, so no band even at its window's physical start", async ({
    page,
  }) => {
    await enableWheelDebug(page);
    await goto(page, "virtual_loop");
    const frame = demoFrame(page, "virtual_loop");
    const content = frame.locator(".dx-carousel-content");
    await expect.poll(() => frame.locator('[data-selected="true"]').getAttribute("aria-label")).toBe("1 of 12");

    // Jump to the window's own physical start and push into it in the same
    // task, before any re-anchor can run: the bridge is attached, and it is
    // `isTrueEnd` (a looping carousel has no true end) that refuses it.
    const result = await content.evaluate((node) => {
      const el = node as HTMLElement;
      // Physical min over every rendered slide (the window is keyed by
      // position, so DOM order alone says nothing about which is leftmost).
      const startGap = () => {
        let lo = Infinity;
        for (const child of Array.from(el.children)) lo = Math.min(lo, child.getBoundingClientRect().left);
        return lo - el.getBoundingClientRect().left;
      };
      // Test setup only: snapping (with `scroll-snap-stop: always`) holds
      // this scroller on its current slide against an instant jump, so
      // suspend it just for the placement, and restore it after.
      const snap = el.style.scrollSnapType;
      el.style.scrollSnapType = "none";
      el.scrollBy({ left: startGap(), behavior: "instant" });
      const gap = startGap();
      for (let i = 0; i < 8; i++) {
        el.dispatchEvent(new WheelEvent("wheel", { deltaX: -(20 + i * 5), bubbles: true, cancelable: true }));
      }
      const transform = el.style.transform;
      el.style.scrollSnapType = snap;
      return { gap, transform };
    });
    expect(Math.abs(result.gap), `the scroller really was at its window's start (gap ${result.gap}px)`).toBeLessThan(1);
    expect(result.transform).toBe("");
    const [rec] = (await wheelRecords(content)).slice(-1);
    expect(rec.falseEnd, "at rest against the window's start, refused as not a true end").toBe("left");
    expect(rec.owned).not.toBe(true);
  });

  test("prefers-reduced-motion: reduce draws no wheel band", async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const content = frame.locator(".dx-carousel-content");
    const run = await playWheel(content, { deltas: neg(WHEEL.rampAndDecay), tailMs: 100 });
    expect(maxAbsBand(run)).toBe(0);
    expect(run.finalTransform).toBe("");
  });

  test("telemetry is off unless the debug flag is set: nothing is recorded", async ({ page }) => {
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const content = frame.locator(".dx-carousel-content");
    const run = await playWheel(content, { deltas: neg(WHEEL.rampAndDecay), tailMs: 100 });
    expect(maxAbsBand(run), "the band itself still works").toBeGreaterThan(5);
    expect(await page.evaluate(() => (window as unknown as { __dxCarouselWheel?: unknown }).__dxCarouselWheel)).toBeUndefined();
  });

  test("real (trusted) wheel input at the start and end edges draws a band that springs back, without paging", async ({
    page,
  }) => {
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const content = frame.locator(".dx-carousel-content");
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 5` });
    const next = frame.getByRole("button", { name: "Next slide" });

    // `deltaX`: a plain vertical delta on a horizontal carousel is not its
    // axis (see the vertical-input test above). 120px identical steps are a
    // notched wheel, so each burst releases fast.
    await content.hover();
    const start = await sampleBandDuring(content, async () => {
      for (let i = 0; i < 6; i++) await page.mouse.wheel(-120, 0);
    });
    expect(Math.max(...start), "a band at the start edge").toBeGreaterThan(0);
    await expectBandGone(content);
    await expect(slide(1)).toHaveAttribute("data-selected", "true");

    for (let i = 0; i < 4; i++) {
      await next.click();
      await expectSnappedToBoundary(content, slide(i + 2));
    }
    await content.hover();
    const end = await sampleBandDuring(content, async () => {
      for (let i = 0; i < 6; i++) await page.mouse.wheel(120, 0);
    });
    expect(Math.min(...end), "a band at the end edge").toBeLessThan(0);
    await expectBandGone(content);
    await expect(slide(5)).toHaveAttribute("data-selected", "true");
  });

  test("a mid-range wheel scroll produces no lingering transform and still pages correctly (regression guard)", async ({
    page,
  }) => {
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const content = frame.locator(".dx-carousel-content");
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 5` });

    await content.hover();
    const pitch = await slidePitch(slide(1));
    for (let i = 0; i < 8; i++) {
      await page.mouse.wheel(pitch / 6, 0);
    }

    await expectSnappedToBoundary(content, slide(2));
    await expect(slide(2)).toHaveAttribute("data-selected", "true");
    await expectBandGone(content, 2000);
  });
});

/** The presentational clipping wrapper `CarouselContent` renders around its
 * scroller (`primitives/src/carousel.rs`'s `CarouselContent` doc, "Edge
 * rubber-band (mode B3)" section, "Correction" paragraph). */
function viewportLocator(frame: Locator): Locator {
  return frame.locator('[data-slot="carousel-viewport"]');
}

/**
 * True if the topmost element actually painted at page coordinate `(x, y)`
 * -- `document.elementFromPoint`, not rect math -- is `content` itself or
 * one of its descendants (a slide). Rect math on the transformed element
 * would flag the same translated box as "outside the viewport" whether or
 * not anything actually clips it; asking the browser what it painted there
 * is the only check that distinguishes "clipped" from "not."
 */
async function paintsAt(content: Locator, x: number, y: number): Promise<boolean> {
  return content.evaluate((contentEl, { x, y }) => {
    const el = document.elementFromPoint(x, y);
    return !!el && contentEl.contains(el);
  }, { x, y });
}

/**
 * Clipping viewport (backlog row 102's own dated addendum, 2026-09-25):
 * `CarouselContent` translates its scroller for the edge rubber-band, but
 * an element's own `overflow` clips relative to ITS OWN box, and a
 * `transform` moves that box -- clip region included -- as one rigid unit.
 * So the scroller was never actually clipping its own overdrag; on real
 * hardware a trackpad overdrag pushed the whole visible slide track ~750px
 * past the carousel, unclipped, overlapping the rest of the page
 * (`primitives/src/carousel.rs`'s `CarouselContent` doc has the full
 * "Correction" writeup). The fix is a stationary wrapper
 * (`div[data-slot="carousel-viewport"]`, `overflow: clip`) around the
 * scroller, so translating the scroller moves content inside a clip
 * boundary that never itself moves.
 *
 * RED-FIRST: run against pre-fix `carousel.rs` (the scroller alone, no
 * wrapper) before this lane's own fix landed -- every "does not paint
 * outside the viewport" assertion below failed, since nothing clipped the
 * translated scroller at all; see this lane's own report for the actual
 * failure output. All pass against the fixed code.
 */
test.describe("Carousel: clipping viewport (edge overdrag never paints outside the carousel)", () => {
  test("the viewport wraps the scroller, clips with overflow: clip (not hidden), and the transformed track is its descendant", async ({
    page,
  }) => {
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const viewport = viewportLocator(frame);
    const content = frame.locator(".dx-carousel-content");

    await expect(viewport).toHaveCount(1);
    const overflow = await viewport.evaluate((el) => {
      const cs = getComputedStyle(el);
      return { x: cs.overflowX, y: cs.overflowY };
    });
    // `clip`, never `hidden`: a `hidden` box is still a scroll container,
    // which would make the paging path's `scrollIntoView` on a slide scroll
    // this wrapper too and permanently offset the whole track.
    expect(overflow.x).toBe("clip");
    expect(overflow.y).toBe("clip");

    const isDescendant = await viewport.evaluate((vpEl) => vpEl.querySelector(".dx-carousel-content") !== null);
    expect(isDescendant).toBe(true);
  });

  test("a pointer overdrag past the first slide (horizontal, LTR) never paints a slide past the viewport's edge", async ({
    page,
  }) => {
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const viewport = viewportLocator(frame);
    const content = frame.locator(".dx-carousel-content");
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 5` });

    const pitch = await slidePitch(slide(1));
    const rawTravel = pitch * 1.5;
    // Positive dx at slide 1 -- see the "edge rubber-band" describe block's
    // identical test above for why this is the physical-previous direction.
    await dragHold(page, content, rawTravel, 0);
    await expect(async () => {
      expect(parseTranslatePx(await readContentTransform(content))).not.toBeNull();
    }).toPass({ timeout: 2000 });

    const box = await viewport.boundingBox();
    if (!box) {
      throw new Error("viewport has no bounding box");
    }
    // Just past the viewport's right edge, inside the padding-inline
    // gutter but before Previous/Next's own 20px-clear zone
    // (`preview/src/components/carousel/style.css`'s own derivation) --
    // this is squarely "the page, where the translated track would be" if
    // it were not clipped, and clear of the button so a hit there can only
    // mean the track itself painted through.
    const x = box.x + box.width + 10;
    const y = box.y + box.height * 0.2;
    expect(await paintsAt(content, x, y), "a slide painted past the viewport's own right edge").toBe(false);

    await page.mouse.up();
    await expect(async () => {
      expect(await readContentTransform(content)).toBe("");
    }).toPass({ timeout: 2000 });
  });

  test("a pointer overdrag at the start boundary under dir=rtl never paints a slide past the viewport's edge", async ({
    page,
  }) => {
    await goto(page, "rtl");
    const frame = demoFrame(page, "rtl");
    const viewport = viewportLocator(frame);
    const content = frame.locator(".dx-carousel-content");
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 4` });

    const pitch = await slidePitch(slide(1));
    const rawTravel = pitch * 1.5;
    // Negative dx is the physical-previous direction under RTL -- see the
    // "edge rubber-band" describe block's identical test above.
    await dragHold(page, content, -rawTravel, 0);
    await expect(async () => {
      expect(parseTranslatePx(await readContentTransform(content))).not.toBeNull();
    }).toPass({ timeout: 2000 });

    const box = await viewport.boundingBox();
    if (!box) {
      throw new Error("viewport has no bounding box");
    }
    // The track moved left this time (negative translateX), so it is the
    // LEFT edge that would escape.
    const x = box.x - 10;
    const y = box.y + box.height * 0.2;
    expect(await paintsAt(content, x, y), "a slide painted past the viewport's own left edge").toBe(false);

    await page.mouse.up();
    await expect(async () => {
      expect(await readContentTransform(content)).toBe("");
    }).toPass({ timeout: 2000 });
  });

  test("a pointer overdrag at the start boundary in the vertical variant never paints a slide past the viewport's edge", async ({
    page,
  }) => {
    await goto(page, "vertical");
    const frame = demoFrame(page, "vertical");
    const viewport = viewportLocator(frame);
    const content = frame.locator(".dx-carousel-content");
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 4` });

    const pitch = await slidePitch(slide(1), "vertical");
    const rawTravel = pitch * 1.5;
    // Positive dy -- see the "edge rubber-band" describe block's identical
    // test above.
    await dragHold(page, content, 0, rawTravel);
    await expect(async () => {
      expect(parseTranslatePx(await readContentTransform(content))).not.toBeNull();
    }).toPass({ timeout: 2000 });

    const box = await viewport.boundingBox();
    if (!box) {
      throw new Error("viewport has no bounding box");
    }
    // The track moved down (positive translateY), so it is the BOTTOM edge
    // that would escape. `x` stays away from the horizontal center, where
    // Previous/Next sit (`inset-inline-start: 50%`).
    const x = box.x + box.width * 0.1;
    const y = box.y + box.height + 10;
    expect(await paintsAt(content, x, y), "a slide painted past the viewport's own bottom edge").toBe(false);

    await page.mouse.up();
    await expect(async () => {
      expect(await readContentTransform(content)).toBe("");
    }).toPass({ timeout: 2000 });
  });

  // Both wheel tests drive a push that HOLDS its band (rising, never
  // repeating, peak >= 40px: only the 1500ms backstop or a cursor move
  // releases it -- see the mode D describe block's own header), so the
  // static held band is what `paintsAt` probes; nothing animated is read
  // across a round trip. The page's mouse is never moved during the probe
  // (a cursor move is D's "fingers lifted" signal).
  test("a wheel overdrag at the start boundary never paints a slide past the viewport's edge", async ({ page }) => {
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const viewport = viewportLocator(frame);
    const content = frame.locator(".dx-carousel-content");
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 5` });
    await expect(slide(1)).toHaveAttribute("data-selected", "true");

    const run = await playWheel(content, { deltas: neg(WHEEL.hold) });
    const depth = parseTranslatePx(run.finalTransform);
    expect(depth, "expected a held, nonzero translateX").not.toBeNull();
    expect(depth!).toBeGreaterThan(0);

    const box = await viewport.boundingBox();
    if (!box) {
      throw new Error("viewport has no bounding box");
    }
    const x = box.x + box.width + 10;
    const y = box.y + box.height * 0.2;
    expect(await paintsAt(content, x, y), "a slide painted past the viewport's own right edge").toBe(false);

    await expectBandGone(content, 3000);
  });

  test("a wheel overdrag at the end boundary never paints a slide past the viewport's edge", async ({ page }) => {
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const viewport = viewportLocator(frame);
    const content = frame.locator(".dx-carousel-content");
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 5` });
    const next = frame.getByRole("button", { name: "Next slide" });

    for (let i = 0; i < 4; i++) {
      await next.click();
      await expectSnappedToBoundary(content, slide(i + 2));
    }
    await expect(slide(5)).toHaveAttribute("data-selected", "true");

    const run = await playWheel(content, { deltas: WHEEL.hold });
    const depth = parseTranslatePx(run.finalTransform);
    expect(depth, "expected a held, nonzero translateX").not.toBeNull();
    expect(depth!).toBeLessThan(0);

    const box = await viewport.boundingBox();
    if (!box) {
      throw new Error("viewport has no bounding box");
    }
    const x = box.x - 10;
    const y = box.y + box.height * 0.2;
    expect(await paintsAt(content, x, y), "a slide painted past the viewport's own left edge").toBe(false);

    await expectBandGone(content, 3000);
  });
});

/**
 * `loop_mode: LoopMode::Rewind` -- the explicit opt-in (backlog row 91's
 * shadcn-parity addendum); `r#loop: true` alone (the default
 * `LoopMode::Seamless`) does nothing for the plain children API any more,
 * see the "library-only v1" describe block above and
 * `primitives/src/carousel.rs`'s own `LoopMode` doc. Retires the old
 * `looping`/`looping_rtl` demos (which showed this behavior unconditionally,
 * with no opt-in) in favor of one `rewind` demo, two rows (LTR/RTL) --
 * see `preview/src/components/carousel/variants/rewind/mod.rs`.
 */
test.describe("Carousel: loop_mode Rewind (explicit opt-in wraparound)", () => {
  test("Previous and Next are never disabled, even at the first/last slide", async ({ page }) => {
    await goto(page, "rewind");
    const frame = demoFrame(page, "rewind");
    const region = frame.getByRole("region", { name: "Rewind-loop gallery", exact: true });
    const previous = region.getByRole("button", { name: /previous/i });
    const next = region.getByRole("button", { name: /next/i });

    await expect(previous).toBeEnabled();
    await expect(next).toBeEnabled();

    for (let i = 0; i < 4; i++) {
      await next.click();
    }
    await expect(region.getByRole("group", { name: "5 of 5" })).toHaveAttribute("data-selected", "true");
    // A non-loop (or non-opted-in) carousel would have `next` disabled
    // here -- `LoopMode::Rewind` never does.
    await expect(next).toBeEnabled();
    await expect(previous).toBeEnabled();
  });

  test("Next at the last slide rewinds to the first INSTANTLY; Previous at the first rewinds to the last", async ({ page }) => {
    await goto(page, "rewind");
    const frame = demoFrame(page, "rewind");
    const region = frame.getByRole("region", { name: "Rewind-loop gallery", exact: true });
    const content = region.locator(".dx-carousel-content");
    const previous = region.getByRole("button", { name: /previous/i });
    const next = region.getByRole("button", { name: /next/i });
    const slide = (n: number) => region.getByRole("group", { name: `${n} of 5` });

    // Previous from slide 1 wraps to slide 5.
    await previous.click();
    await expect(slide(5)).toHaveAttribute("data-selected", "true");
    await expectSnappedToBoundary(content, slide(5));

    // Next from slide 5 wraps back to slide 1 -- and, unlike every other
    // (animated) transition (see the "actually animates" describe block's
    // own identically-shaped test), does so as an INSTANT jump: sampling
    // `scrollLeft` across the transition must never pass through an
    // intermediate value, only the start and end positions.
    const contentId = (await content.getAttribute("id"))!;
    const samples = await sampleScrollDuring(page, contentId, "scrollLeft", () => next.click());
    await expect(slide(1)).toHaveAttribute("data-selected", "true");
    await expectSnappedToBoundary(content, slide(1));
    expect(
      passesThroughAnIntermediateValue(samples),
      `expected an instant jump (no intermediate scrollLeft between the first (${samples[0]}) and last (${samples[samples.length - 1]}) sample); got: ${JSON.stringify(samples)}`,
    ).toBe(false);

    // Root-level ArrowRight at the last slide also wraps.
    for (let i = 0; i < 4; i++) {
      await next.click();
    }
    await expect(slide(5)).toHaveAttribute("data-selected", "true");
    await next.focus();
    await page.keyboard.press("ArrowRight");
    await expect(slide(1)).toHaveAttribute("data-selected", "true");
  });

  test("under RTL, loop_mode Rewind still wraps both directions with the swapped arrow keys", async ({ page }) => {
    await goto(page, "rewind");
    const frame = demoFrame(page, "rewind");
    const region = frame.getByRole("region", { name: "Rewind-loop gallery (RTL)", exact: true });
    const previous = region.getByRole("button", { name: /previous/i });
    const next = region.getByRole("button", { name: /next/i });
    const slide = (n: number) => region.getByRole("group", { name: `${n} of 4` });

    await expect(previous).toBeEnabled();
    await expect(next).toBeEnabled();

    // Previous from slide 1 wraps to slide 4.
    await previous.click();
    await expect(slide(4)).toHaveAttribute("data-selected", "true");
    await expect(previous).toBeEnabled();

    // Next from slide 4 wraps back to slide 1.
    await next.click();
    await expect(slide(1)).toHaveAttribute("data-selected", "true");

    // RTL root keyboard: ArrowLeft means next (swapped), so it should wrap
    // from the last slide back to the first too.
    for (let i = 0; i < 3; i++) {
      await next.click();
    }
    await expect(slide(4)).toHaveAttribute("data-selected", "true");
    await next.focus();
    await page.keyboard.press("ArrowLeft");
    await expect(slide(1)).toHaveAttribute("data-selected", "true");
  });

  /**
   * Rewritten for the owner's true-end rule (the drag band now follows the
   * wheel band's shared `dxIsTrueEnd` predicate): this test used to be titled
   * "dragging past a physical edge still rubber-bands under loop" -- though it
   * only ever asserted that the transform ended at identity, never that a
   * band appeared. A looping carousel has no true end, so the drag clamps at
   * the physical edge with no band at all, and still never wraps.
   */
  test("dragging past a looping carousel's physical edge never rubber-bands and never wraps", async ({ page }) => {
    await goto(page, "rewind");
    const frame = demoFrame(page, "rewind");
    const region = frame.getByRole("region", { name: "Rewind-loop gallery", exact: true });
    const content = region.locator(".dx-carousel-content");
    const slide = (n: number) => region.getByRole("group", { name: `${n} of 5` });

    await expect(slide(1)).toHaveAttribute("data-selected", "true");
    // A large rightward drag from slide 1 (nothing before it): no band, no
    // wrap to slide 5 -- `loop`/`loop_mode` only govern Previous/Next/the
    // root keyboard (this crate's own module doc). Sampled in-page every
    // frame for the whole drag (row 112), not read once after it.
    const samples = await sampleBandDuring(content, async () => {
      await dragBy(page, content, 250, 0);
    }, 300);
    expect(Math.max(0, ...samples.map(Math.abs)), "no band at a looping carousel's edge").toBe(0);
    expect(await readContentTransform(content)).toBe("");
    await expect(slide(1)).toHaveAttribute("data-selected", "true");
  });

  test("axe: the rewind variant has no automatically detectable a11y issues", async ({ page }) => {
    await goto(page, "rewind");
    await expectNoAxeViolations(page, "carousel: rewind variant", {
      include: "#component-preview-frame-rewind",
    });
  });
});

/**
 * Autoplay + rotation control (APG "auto-rotating" carousel). The
 * `autoplay` variant's own `delay_ms: 1200` (see its own doc for why) --
 * tests below wait a bit past that, never past a second full delay, to
 * stay fast without being flaky.
 */
test.describe("Carousel: autoplay + rotation control", () => {
  test("the rotation control precedes Previous/Next/the slide content in document order, and toggles its own label", async ({ page }) => {
    await goto(page, "autoplay");
    const frame = demoFrame(page, "autoplay");
    const rotation = frame.getByRole("button", { name: /automatic slide show/i });
    const previous = frame.getByRole("button", { name: "Previous slide" });
    const next = frame.getByRole("button", { name: "Next slide" });
    const firstSlide = frame.getByRole("group", { name: "1 of 5" });

    await expect(rotation).toHaveAttribute("aria-label", "Stop automatic slide show");
    await expect(rotation).not.toHaveAttribute("aria-pressed");

    // Document order, matching the R4 oracle rule's own construction
    // (`playwright/oracle/tier1-apg/carousel.spec.ts`'s `precedes`) --
    // APG's own requirement that the rotation control precede everything
    // else focusable in the carousel.
    const precedes = async (a: Locator, b: Locator) => {
      const bHandle = await b.elementHandle();
      return a.evaluate(
        (elA, elB) => !!(elA.compareDocumentPosition(elB as Node) & Node.DOCUMENT_POSITION_FOLLOWING),
        bHandle,
      );
    };
    expect(await precedes(rotation, previous)).toBe(true);
    expect(await precedes(rotation, next)).toBe(true);
    expect(await precedes(rotation, firstSlide)).toBe(true);

    await rotation.click();
    await expect(rotation).toHaveAttribute("aria-label", "Start automatic slide show");
    await rotation.click();
    await expect(rotation).toHaveAttribute("aria-label", "Stop automatic slide show");
  });

  test("aria-live on the slides container toggles with rotation state", async ({ page }) => {
    await goto(page, "autoplay");
    const frame = demoFrame(page, "autoplay");
    const content = frame.locator(".dx-carousel-content");
    const rotation = frame.getByRole("button", { name: /automatic slide show/i });

    await expect(content).toHaveAttribute("aria-live", "off");
    await rotation.click();
    await expect(content).toHaveAttribute("aria-live", "polite");
    await rotation.click();
    // Clicking the rotation button leaves the pointer hovering the
    // carousel (the button is inside it) -- and hover legitimately pauses
    // rotation regardless of `playing` (this component's own doc
    // deliberately does NOT port the vendored reference's own
    // "ignore hover/focus once explicitly started" quirk, the same class
    // of bug already flagged and left unported for the basic reference's
    // own `hasUserActivatedPlay` latch, `dev-docs/research/carousel-2026-09-19.md`
    // §1.3 point 3) -- so the pointer must move away first to observe the
    // live region reflect `playing` again.
    await page.mouse.move(5, 5);
    await expect(content).toHaveAttribute("aria-live", "off");
  });

  test("autoplay advances the slide on its own", async ({ page }) => {
    await goto(page, "autoplay");
    const frame = demoFrame(page, "autoplay");
    // Relative, not absolute: under parallel load, hydration can take longer
    // than one autoplay interval, so the carousel may already have left
    // slide 1 by the time the page is ready (2 of 10 runs at --workers=4
    // failed an up-front "slide 1 is selected" check). Whatever slide is
    // selected now, a different one must become selected on its own.
    const selectedLabel = () =>
      frame.locator('[role="group"][aria-roledescription="slide"][data-selected="true"]').first().getAttribute("aria-label");
    await expect.poll(selectedLabel).not.toBeNull();
    const before = await selectedLabel();
    await expect.poll(selectedLabel, { timeout: 3000 }).not.toBe(before);
  });

  test("keyboard focus entering the carousel stops rotation, and it does not resume on its own", async ({ page }) => {
    await goto(page, "autoplay");
    const frame = demoFrame(page, "autoplay");
    const content = frame.locator(".dx-carousel-content");
    const rotation = frame.getByRole("button", { name: /automatic slide show/i });

    // No per-call-site guard needed here (or in the hover test below):
    // `./fixtures.ts` (dev-docs/backlog.md row 110) forces
    // `scroll-behavior: auto` on `<html>` for every test in this suite, so
    // Playwright's own pre-`.focus()`/`.hover()` actionability scroll
    // (an unqualified scroll, which defers to that ancestor CSS) is
    // already instant -- it cannot animate out from under the cursor the
    // way it used to before that shared fixture existed.
    await content.focus();
    // The rotation control's own label reflects the user's *intent*
    // (`playing`), unaffected by an ambient, temporary pause -- only the
    // live region (`rotating`) does. It stays "Stop automatic slide show"
    // throughout this whole test; asserted once here as the baseline.
    await expect(rotation).toHaveAttribute("aria-label", "Stop automatic slide show");
    await expect(content).toHaveAttribute("aria-live", "polite");

    const selectedAt = async () =>
      frame.locator('[role="group"][data-selected="true"]').getAttribute("aria-label");
    const before = await selectedAt();
    // Wait past two full ticks -- focus is still inside the content
    // element, so this must never advance regardless of elapsed time.
    await page.waitForTimeout(2600);
    expect(await selectedAt()).toBe(before);

    // Losing focus alone does not resume it either (only the rotation
    // control does) -- move focus elsewhere on the page.
    await page.keyboard.press("Tab");
    await expect(content).toHaveAttribute("aria-live", "polite");
    await page.waitForTimeout(1600);
    expect(await selectedAt()).toBe(before);
  });

  test("hovering the carousel stops rotation, and moving away resumes it", async ({ page }) => {
    await goto(page, "autoplay");
    const frame = demoFrame(page, "autoplay");
    const content = frame.locator(".dx-carousel-content");
    const rotation = frame.getByRole("button", { name: /automatic slide show/i });

    // This test used to call `scrollIntoViewInstant(content)` here as a
    // per-call-site guard: without it, Playwright's own pre-`.hover()`
    // auto-scroll inherited the page's global smooth-scroll CSS and could
    // still be animating when `mouseenter` fired, dragging the carousel
    // out from under the cursor and firing a genuine `mouseleave` a few
    // hundred ms later -- observed in this session as this exact test's
    // flake (rotation legitimately, correctly resuming after that real
    // `mouseleave`, not a component bug). Simplified per
    // dev-docs/backlog.md row 110: `./fixtures.ts`'s shared `test`/`expect`
    // (which this file now imports) forces `scroll-behavior: auto` on
    // `<html>` for every test in this suite, so that pre-`.hover()` scroll
    // is already instant and cannot animate out from under the cursor --
    // no per-call-site guard needed any more.
    await content.hover();
    // The button's own label is unaffected (reflects intent, not the
    // ambient pause) -- only the live region does. See the focus test's
    // own identical note.
    await expect(rotation).toHaveAttribute("aria-label", "Stop automatic slide show");
    await expect(content).toHaveAttribute("aria-live", "polite");

    const selectedAt = async () =>
      frame.locator('[role="group"][data-selected="true"]').getAttribute("aria-label");
    const before = await selectedAt();
    await page.waitForTimeout(1600);
    expect(await selectedAt()).toBe(before);

    // Move the mouse well away from the carousel -- resumes on its own,
    // unlike the focus case above (the vendored tabbed reference's own
    // accessibility prose: "Automatic rotation resumes when the mouse
    // moves away ... unless another condition ... has been triggered").
    await page.mouse.move(5, 5);
    await expect(content).toHaveAttribute("aria-live", "off");
    await expect
      .poll(selectedAt, { timeout: 3000 })
      .not.toBe(before);
  });

  test("Previous/Next stop rotation for good (stopOnInteraction)", async ({ page }) => {
    await goto(page, "autoplay");
    const frame = demoFrame(page, "autoplay");
    const rotation = frame.getByRole("button", { name: /automatic slide show/i });
    const next = frame.getByRole("button", { name: "Next slide" });

    await expect(rotation).toHaveAttribute("aria-label", "Stop automatic slide show");
    await next.click();
    await expect(rotation).toHaveAttribute("aria-label", "Start automatic slide show");
  });

  test("prefers-reduced-motion: reduce never starts rotation at all", async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await goto(page, "autoplay");
    const frame = demoFrame(page, "autoplay");
    const rotation = frame.getByRole("button", { name: /automatic slide show/i });
    const content = frame.locator(".dx-carousel-content");

    await expect(rotation).toHaveAttribute("aria-label", "Start automatic slide show");
    await expect(content).toHaveAttribute("aria-live", "polite");

    const selectedAt = async () =>
      frame.locator('[role="group"][data-selected="true"]').getAttribute("aria-label");
    const before = await selectedAt();
    await page.waitForTimeout(1600);
    expect(await selectedAt()).toBe(before);
  });

  test("axe: the autoplay variant has no automatically detectable a11y issues", async ({ page }) => {
    await goto(page, "autoplay");
    await expectNoAxeViolations(page, "carousel: autoplay variant", {
      include: "#component-preview-frame-autoplay",
    });
  });
});

/**
 * Indicators (tablist/dot-picker) variant -- APG's "tabbed" carousel
 * style, `CarouselIndicators`/`CarouselIndicator` (renamed from
 * `CarouselTabList`/`CarouselTab`). The `indicators` variant has 5 slides,
 * no separate Previous/Next (matching the vendored reference's own
 * structure). Merged with what used to be a separate, non-tablist
 * `indicators` demo (a custom `use_carousel()`-built dot picker) -- that
 * composition now lives on the `api` demo instead.
 */
test.describe("Carousel: indicators (tablist/dot-picker) variant", () => {
  test("roles: tablist/tab/tabpanel, roving tabindex, aria-selected sync with the current slide", async ({ page }) => {
    await goto(page, "indicators");
    const frame = demoFrame(page, "indicators");
    const tablist = frame.getByRole("tablist");
    const tabs = frame.getByRole("tab");
    const slide = (n: number) => frame.locator(`[role="tabpanel"][aria-label="${n} of 5"]`);

    await expect(tablist).toBeVisible();
    await expect(tabs).toHaveCount(5);
    await expect(tabs.nth(0)).toHaveAttribute("aria-selected", "true");
    await expect(tabs.nth(0)).toHaveAttribute("tabindex", "0");
    for (let i = 1; i < 5; i++) {
      await expect(tabs.nth(i)).toHaveAttribute("aria-selected", "false");
      await expect(tabs.nth(i)).toHaveAttribute("tabindex", "-1");
    }
    await expect(slide(1)).toBeVisible();
  });

  test("clicking a tab activates its slide and moves the roving tab stop", async ({ page }) => {
    await goto(page, "indicators");
    const frame = demoFrame(page, "indicators");
    const tabs = frame.getByRole("tab");
    const content = frame.locator(".dx-carousel-content");
    const slide = (n: number) => frame.locator(`[role="tabpanel"][aria-label="${n} of 5"]`);

    await tabs.nth(2).click();
    await expect(tabs.nth(2)).toHaveAttribute("aria-selected", "true");
    await expect(tabs.nth(2)).toHaveAttribute("tabindex", "0");
    await expect(tabs.nth(0)).toHaveAttribute("aria-selected", "false");
    await expect(tabs.nth(0)).toHaveAttribute("tabindex", "-1");
    await expect(slide(3)).toHaveAttribute("data-selected", "true");
    await expectSnappedToBoundary(content, slide(3));
  });

  test("ArrowRight/ArrowLeft move focus and automatically activate the newly focused tab (no Enter needed)", async ({ page }) => {
    await goto(page, "indicators");
    const frame = demoFrame(page, "indicators");
    const tabs = frame.getByRole("tab");
    const slide = (n: number) => frame.locator(`[role="tabpanel"][aria-label="${n} of 5"]`);

    await tabs.nth(0).focus();
    await page.keyboard.press("ArrowRight");
    await expect(tabs.nth(1)).toBeFocused();
    await expect(tabs.nth(1)).toHaveAttribute("aria-selected", "true");
    await expect(slide(2)).toHaveAttribute("data-selected", "true");

    await page.keyboard.press("ArrowLeft");
    await expect(tabs.nth(0)).toBeFocused();
    await expect(slide(1)).toHaveAttribute("data-selected", "true");
  });

  test("ArrowLeft/ArrowRight wrap at both ends of the tablist", async ({ page }) => {
    await goto(page, "indicators");
    const frame = demoFrame(page, "indicators");
    const tabs = frame.getByRole("tab");

    await tabs.nth(0).focus();
    await page.keyboard.press("ArrowLeft");
    await expect(tabs.nth(4)).toBeFocused();
    await expect(tabs.nth(4)).toHaveAttribute("aria-selected", "true");

    await page.keyboard.press("ArrowRight");
    await expect(tabs.nth(0)).toBeFocused();
  });

  test("Home/End move focus to the first/last tab and activate it", async ({ page }) => {
    await goto(page, "indicators");
    const frame = demoFrame(page, "indicators");
    const tabs = frame.getByRole("tab");
    const slide = (n: number) => frame.locator(`[role="tabpanel"][aria-label="${n} of 5"]`);

    await tabs.nth(0).focus();
    await page.keyboard.press("End");
    await expect(tabs.nth(4)).toBeFocused();
    await expect(slide(5)).toHaveAttribute("data-selected", "true");

    await page.keyboard.press("Home");
    await expect(tabs.nth(0)).toBeFocused();
    await expect(slide(1)).toHaveAttribute("data-selected", "true");
  });

  test("each tab's aria-controls points at its matching tabpanel's id", async ({ page }) => {
    await goto(page, "indicators");
    const frame = demoFrame(page, "indicators");
    const firstTab = frame.getByRole("tab").nth(0);
    const firstPanelId = await frame.locator('[role="tabpanel"]').nth(0).getAttribute("id");
    expect(firstPanelId).toBeTruthy();
    await expect(firstTab).toHaveAttribute("aria-controls", firstPanelId!);
  });

  test("axe: the indicators variant has no automatically detectable a11y issues", async ({ page }) => {
    await goto(page, "indicators");
    await expectNoAxeViolations(page, "carousel: indicators variant", {
      include: "#component-preview-frame-indicators",
    });
  });

  test("the active dot's opacity/width transition is present normally, and near-instant under prefers-reduced-motion", async ({
    page,
  }) => {
    await goto(page, "indicators");
    const frame = demoFrame(page, "indicators");
    const firstTab = frame.getByRole("tab").nth(0);

    const transitionDuration = await firstTab.evaluate(
      (el) => getComputedStyle(el).transitionDuration,
    );
    // Two comma-separated durations (opacity, width) -- style.css's own
    // `.dx-carousel-indicator` rule -- neither near-zero under the
    // default (no reduced-motion) preference.
    const durations = transitionDuration.split(",").map((s) => parseFloat(s));
    expect(durations.length).toBeGreaterThanOrEqual(2);
    for (const d of durations) {
      expect(d).toBeGreaterThan(0.05);
    }
  });

  test("the active dot's transition is near-instant under prefers-reduced-motion", async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await goto(page, "indicators");
    const frame = demoFrame(page, "indicators");
    const firstTab = frame.getByRole("tab").nth(0);

    // dx-components-theme.css's own shared reduced-motion layer forces
    // `transition-duration` to `var(--dx-motion-duration-reduced)` (0.01ms)
    // `!important` on any `[role="tab"]` -- this button qualifies without
    // this component needing its own per-rule override (see
    // `.dx-carousel-indicator`'s own style.css comment).
    const transitionDuration = await firstTab.evaluate(
      (el) => getComputedStyle(el).transitionDuration,
    );
    const durations = transitionDuration.split(",").map((s) => parseFloat(s));
    for (const d of durations) {
      expect(d).toBeLessThan(0.01);
    }
  });
});

/**
 * Owner report (live site): "autoplay scrolls the main page." Root cause,
 * found by reading `primitives/src/carousel.rs`: every paging path fired
 * `target.scrollIntoView()` on the newly-selected slide, and
 * `scrollIntoView` walks and scrolls *every* scrollable ancestor between
 * the target and the viewport, including the page itself -- so a click,
 * a keypress, a tab activation, a drag release, or an unattended autoplay
 * tick would all silently drag a reader back to the carousel, wherever
 * they had scrolled to. The fix (this lane) replaces every one of those
 * call sites with `scroller.scrollBy({ left | top: delta })`, called
 * directly on the carousel's own scroller element -- `scrollBy` only
 * ever writes the element it is called on, so there is no ancestor for it
 * to reach. See `CAROUSEL_SCROLL_TO_JS`'s own doc in `carousel.rs` for the
 * full construction.
 *
 * RED-FIRST: every test below was run against the pre-fix `carousel.rs`
 * (`target.scrollIntoView()` at both call sites) before landing this
 * lane's own fix, and every one of them failed -- `window.scrollY` moved
 * by hundreds to thousands of pixels in each case (see this lane's own
 * report for the exact numbers). All pass against the fixed code.
 *
 * Methodology notes specific to this describe block:
 * - The demo page has its own "jump to the requested `variant=`" scroll
 *   on load (an unrelated, expected feature, not this bug), which is
 *   itself `smooth` (`html { scroll-behavior: smooth }`,
 *   `preview/assets/main.css`, backlog row 110) -- so every test below
 *   waits for `window.scrollY` to stop changing (`waitForScrollStable`)
 *   before doing anything else, then repositions the page with an
 *   *instant* `scrollTo` (never the default/`smooth` behavior, and never
 *   Playwright's own `.click()`/`.hover()` actionability pre-scroll,
 *   which also inherits that same global smooth-scroll rule and can still
 *   be mid-flight when the very next assertion reads `scrollY`).
 * - Buttons are paged with `dispatchEvent("click")`, not `.click()`, so
 *   Playwright's own pre-click `scrollIntoViewIfNeeded` never runs and
 *   never contributes a scroll of its own for this test to (correctly)
 *   ignore.
 * - The `tabs` variant's activation test focuses its target tab with
 *   `el.focus({ preventScroll: true })` rather than Playwright's plain
 *   `.focus()`. This is deliberate, not a shortcut: found live in this
 *   session, a *plain* `.focus()` on an off-screen tab still moves the
 *   page even against the FIXED code, because focusing any off-screen
 *   element is the browser's own default, spec-mandated behavior for
 *   `element.focus()` generally -- entirely independent of this
 *   component, present for every focusable element on the web, and not
 *   something a carousel's own code can (or should) suppress from the
 *   outside. `preventScroll: true` is exactly the tool the platform gives
 *   a caller to opt out of that default when it does its own scroll
 *   positioning, which is precisely this test's situation; it isolates
 *   the thing actually under test -- `CarouselIndicator`'s own `onfocus` handler
 *   (`carousel_ctx.set_selected.call(...)`) -- from that unrelated native
 *   behavior. Confirmed live: plain `.focus()` moves the page on *both*
 *   pre-fix and post-fix code (the native behavior, unaffected by this
 *   lane); `el.focus({ preventScroll: true })` moves the page only on the
 *   pre-fix code and never on the fixed code, which is the one signal
 *   that actually distinguishes this bug from that unrelated mechanism.
 */
test.describe("Carousel: paging never scrolls the page (ancestor-scroll regression)", () => {
  /** Poll `window.scrollY` until it stops changing -- see this describe
   * block's own "Methodology notes" above for why (the page's own
   * unrelated "jump to this variant" scroll on load). */
  async function waitForScrollStable(page: Page): Promise<number> {
    let previous: number | null = null;
    for (let i = 0; i < 50; i++) {
      const y = await page.evaluate(() => window.scrollY);
      if (y === previous) {
        return y;
      }
      previous = y;
      await page.waitForTimeout(150);
    }
    throw new Error("waitForScrollStable: window.scrollY never settled");
  }

  test("several autoplay ticks never move window scroll position", async ({ page }) => {
    await goto(page, "autoplay");
    const frame = demoFrame(page, "autoplay");
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 5` });
    const selectedLabel = async () =>
      frame.locator('[role="group"][data-selected="true"]').getAttribute("aria-label");

    await waitForScrollStable(page);
    // Instant, never the page's own default `smooth` behavior (see this
    // describe block's own "Methodology notes").
    await page.evaluate(() => window.scrollTo({ top: 0, left: 0, behavior: "instant" }));
    const scrollXBefore = await page.evaluate(() => window.scrollX);
    const scrollYBefore = await page.evaluate(() => window.scrollY);
    const before = await selectedLabel();

    // The `autoplay` variant's own `delay_ms: 1200` (this file's own
    // header note) -- comfortably past two full ticks.
    await page.waitForTimeout(1200 * 2 + 600);

    // The carousel did actually advance on its own -- otherwise a
    // never-firing timer would trivially pass this test for the wrong
    // reason.
    expect(await selectedLabel()).not.toBe(before);
    expect(await page.evaluate(() => window.scrollY)).toBe(scrollYBefore);
    expect(await page.evaluate(() => window.scrollX)).toBe(scrollXBefore);
    // The slide the ticks landed on is still exactly snapped -- the fix
    // changed *how* the scroller pages, never where it lands.
    const selectedIndex = Number((await selectedLabel())!.split(" ")[0]);
    await expectSnappedToBoundary(frame.locator(".dx-carousel-content"), slide(selectedIndex));
  });

  test("a Next click and a Previous click never move window scroll position, with the page scrolled away", async ({
    page,
  }) => {
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const content = frame.locator(".dx-carousel-content");
    const next = frame.getByRole("button", { name: "Next slide" });
    const previous = frame.getByRole("button", { name: "Previous slide" });
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 5` });

    await waitForScrollStable(page);
    await page.evaluate(() => window.scrollTo({ top: 0, left: 0, behavior: "instant" }));
    const scrollYBefore = await page.evaluate(() => window.scrollY);
    const scrollXBefore = await page.evaluate(() => window.scrollX);

    // `dispatchEvent`, not `.click()` -- see this describe block's own
    // "Methodology notes" for why.
    await next.dispatchEvent("click");
    await expect(slide(2)).toHaveAttribute("data-selected", "true");
    expect(await page.evaluate(() => window.scrollY)).toBe(scrollYBefore);
    expect(await page.evaluate(() => window.scrollX)).toBe(scrollXBefore);
    await expectSnappedToBoundary(content, slide(2));

    await previous.dispatchEvent("click");
    await expect(slide(1)).toHaveAttribute("data-selected", "true");
    expect(await page.evaluate(() => window.scrollY)).toBe(scrollYBefore);
    expect(await page.evaluate(() => window.scrollX)).toBe(scrollXBefore);
    await expectSnappedToBoundary(content, slide(1));
  });

  test("a CarouselIndicator activation never moves window scroll position (isolated from the browser's own focus-scroll)", async ({
    page,
  }) => {
    await goto(page, "indicators");
    const frame = demoFrame(page, "indicators");
    const tab = frame.getByRole("tab").nth(2);
    const content = frame.locator(".dx-carousel-content");
    const slide = (n: number) => frame.locator(`[role="tabpanel"][aria-label="${n} of 5"]`);

    await waitForScrollStable(page);
    await page.evaluate(() => window.scrollTo({ top: 0, left: 0, behavior: "instant" }));
    const scrollYBefore = await page.evaluate(() => window.scrollY);
    const scrollXBefore = await page.evaluate(() => window.scrollX);

    // `{ preventScroll: true }` -- see this describe block's own
    // "Methodology notes" for why this, and not Playwright's plain
    // `.focus()`, is the correct isolation of the thing under test.
    await tab.evaluate((el) => (el as HTMLElement).focus({ preventScroll: true }));
    await expect(tab).toHaveAttribute("aria-selected", "true");
    await expect(slide(3)).toHaveAttribute("data-selected", "true");
    expect(await page.evaluate(() => window.scrollY)).toBe(scrollYBefore);
    expect(await page.evaluate(() => window.scrollX)).toBe(scrollXBefore);
    await expectSnappedToBoundary(content, slide(3));
  });

  test("a pointer-drag release settle never moves window scroll position", async ({ page }) => {
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const content = frame.locator(".dx-carousel-content");
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 5` });
    await expect(slide(1)).toHaveAttribute("data-selected", "true");

    await waitForScrollStable(page);
    // Bring the carousel fully into view ourselves first, instantly --
    // `dragBy` needs real, stable viewport coordinates to drive
    // `page.mouse`, and would otherwise do this same positioning itself,
    // uninstrumented, the moment it is called (see this describe block's
    // own "Methodology notes"; `scrollIntoViewInstant`, this file's own
    // existing helper above, is the same technique). Establishing it here
    // means the baseline below is captured on an already-stable page, so
    // this test isolates the drag+release SETTLE itself
    // (`CAROUSEL_DRAG_JS`'s own `endDrag`) as the only thing that must not
    // move the page any further.
    await scrollIntoViewInstant(content);
    const scrollYBefore = await page.evaluate(() => window.scrollY);
    const scrollXBefore = await page.evaluate(() => window.scrollX);

    const pitch = await slidePitch(slide(1));
    await dragBy(page, content, -pitch * 0.7, 0);

    await expectSnappedToBoundary(content, slide(2));
    await expect(slide(2)).toHaveAttribute("data-selected", "true");
    expect(await page.evaluate(() => window.scrollY)).toBe(scrollYBefore);
    expect(await page.evaluate(() => window.scrollX)).toBe(scrollXBefore);
  });
});

/**
 * The seamless-loop research's own a11y contract
 * (`dev-docs/research/carousel-loop-2026-09-25/loop-a11y-guidance.md` §7,
 * cross-checked against every a11y-layer-having carousel library's own
 * converged construction in that same round's `loop-libraries.md`): only
 * the current (or, for a multi-per-view layout, every slide actually
 * inside the viewport at rest) slide is reachable by Tab or exposed to the
 * accessibility tree -- everything else is `inert`
 * (`primitives/src/carousel.rs`'s own "Visible slides / `inert`" doc on
 * [`CarouselItem`]). `getByRole`/`ariaSnapshot` cannot verify the
 * accessibility-tree half of this (see `axTreeGroupNames`'s own doc,
 * above, for the live discrepancy this session found and worked around).
 */
test.describe("Carousel: only visible slides are reachable (inert)", () => {
  test("non-current slides are inert on the main variant; the accessibility tree exposes only the current one", async ({ page }) => {
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const items = frame.locator(".dx-carousel-item");
    await expect(items).toHaveCount(5);

    await expect(items.nth(0)).not.toHaveAttribute("inert");
    for (let i = 1; i < 5; i++) {
      await expect(items.nth(i)).toHaveAttribute("inert");
    }

    const namesBefore = await axTreeGroupNames(page, "#component-preview-frame [role=region]");
    expect(namesBefore).toContain("1 of 5");
    for (let n = 2; n <= 5; n++) {
      expect(namesBefore).not.toContain(`${n} of 5`);
    }

    await frame.getByRole("button", { name: "Next slide" }).click();
    await expect(items.nth(0)).toHaveAttribute("inert");
    await expect(items.nth(1)).not.toHaveAttribute("inert");

    // Chrome's own accessibility tree recomputes asynchronously (a plain
    // CDP re-query right after the DOM mutation can still observe the
    // pre-mutation tree) -- `expect.poll` retries the whole scoped query
    // until it reflects the settled DOM, the same "poll rather than assume
    // instantaneous" discipline this file already applies to geometry
    // settling (`expectSnappedToBoundary`).
    await expect
      .poll(() => axTreeGroupNames(page, "#component-preview-frame [role=region]"))
      .toContain("2 of 5");
    const namesAfter = await axTreeGroupNames(page, "#component-preview-frame [role=region]");
    expect(namesAfter).not.toContain("1 of 5");
  });

  // `looping`/`looping_rtl` (retired -- see the `LoopMode` describe block
  // above) is not replaced with `rewind` here: `rewind` renders TWO
  // independent `Carousel` regions on one page, so a single flat
  // `.dx-carousel-item` locator across both would see the RTL row's own
  // non-inert current slide land at an index this loop's "everything past
  // 0 is inert" assumption does not hold for -- the single-region
  // assumption below is real, not incidental.
  for (const variant of ["indicators", "autoplay"] as const) {
    test(`non-current slides are inert on the ${variant} variant too`, async ({ page }) => {
      await goto(page, variant);
      const frame = demoFrame(page, variant);
      const items = frame.locator(".dx-carousel-item");
      const count = await items.count();
      expect(count).toBeGreaterThan(1);

      await expect(items.nth(0)).not.toHaveAttribute("inert");
      for (let i = 1; i < count; i++) {
        await expect(items.nth(i)).toHaveAttribute("inert");
      }
    });
  }

  test("a multi-per-view layout widens the visible set to every slide actually in the viewport (sizes variant)", async ({ page }) => {
    await goto(page, "sizes");
    const frame = demoFrame(page, "sizes");
    const items = frame.locator(".dx-carousel-item");
    const count = await items.count();

    // Measured live, not assumed -- exactly how many WHOLE slides are
    // visible at once depends on `--dx-carousel-per-view` against however
    // wide this demo's own responsive wrapper renders (this file's own
    // established "measure, don't hardcode" convention, `slidePitch`'s own
    // doc) -- 2 below the demo's own `lg` breakpoint, 3 above it. The only
    // structural invariant asserted is that the non-inert set is a
    // contiguous prefix starting at slide 0 (this demo never scrolls on
    // its own before this point) and is more than just the one selected
    // slide -- proving the widening actually happened, not merely that the
    // single-slide case still works.
    const inertFlags: boolean[] = [];
    for (let i = 0; i < count; i++) {
      inertFlags.push(await items.nth(i).evaluate((el) => el.hasAttribute("inert")));
    }
    const firstInert = inertFlags.indexOf(true);
    expect(firstInert).toBeGreaterThan(1); // more than just slide 0 is visible
    expect(inertFlags.slice(0, firstInert).every((v) => v === false)).toBe(true);
    expect(inertFlags.slice(firstInert).every((v) => v === true)).toBe(true);

    const names = await axTreeGroupNames(page, "#component-preview-frame-sizes [role=region]");
    for (let i = 0; i < firstInert; i++) {
      expect(names).toContain(`${i + 1} of ${count}`);
    }
    for (let i = firstInert; i < count; i++) {
      expect(names).not.toContain(`${i + 1} of ${count}`);
    }
  });

  test("whole-slide geometry: each visible item's width is the viewport divided by --dx-carousel-per-view, within 1px, and slides land on boundaries (sizes variant)", async ({ page }) => {
    // Root-cause regression test for the "~2.3 slides visible" incident
    // (`dev-docs/research/shadcn-carousel-parity.md`) this construction
    // closes: a real flex `gap` was additive to a percentage `flex-basis`,
    // so N items never summed to exactly the viewport width. Pinned here
    // against the live-rendered geometry, not just the CSS source, so a
    // future regression in the calc (or in the gap model it depends on)
    // is caught the same way the incident itself was found -- by
    // measurement.
    await goto(page, "sizes");
    const frame = demoFrame(page, "sizes");
    const viewport = frame.locator('[data-slot="carousel-viewport"]');
    const content = frame.locator(".dx-carousel-content");
    const items = frame.locator(".dx-carousel-item");

    const viewportBox = await viewport.boundingBox();
    const contentBox = await content.boundingBox();
    expect(viewportBox).not.toBeNull();
    expect(contentBox).not.toBeNull();
    const perView = await frame
      .locator(".dx-carousel-demo-sizes")
      .evaluate((el) => getComputedStyle(el).getPropertyValue("--dx-carousel-per-view").trim());
    const n = Number(perView);
    expect(n).toBeGreaterThanOrEqual(2);

    // Each item's own basis is `calc(100% / n)` OF THE SCROLLER's own
    // (content) width, not the clipped viewport's -- the gap model's own
    // negative-margin compensation deliberately widens the scroller by
    // `--dx-carousel-gap`'s own width (`content_gap_margin`'s own doc),
    // clipped by the viewport wrapper, so `contentBox.width`, not
    // `viewportBox.width`, is the correct denominator here.
    for (let i = 0; i < n; i++) {
      const box = await items.nth(i).boundingBox();
      expect(box).not.toBeNull();
      expect(box!.width).toBeCloseTo(contentBox!.width / n, 0);
    }
    // The N slides still visually fill the clipped viewport exactly, with
    // no overflow: the last of the N items' own right edge lands flush
    // with the viewport's own right edge (both are pinned to the same
    // physical edge by construction -- content's own right edge is never
    // shifted, only its left edge is, so this holds regardless of gap).
    const lastVisibleBox = await items.nth(n - 1).boundingBox();
    expect(lastVisibleBox).not.toBeNull();
    expect(lastVisibleBox!.x + lastVisibleBox!.width).toBeCloseTo(
      viewportBox!.x + viewportBox!.width,
      0,
    );
    // The (n+1)-th slide (0-based index n) must not be even partially
    // inside the viewport -- a whole-slide layout has no partial peek.
    const nextBox = await items.nth(n).boundingBox();
    expect(nextBox).not.toBeNull();
    expect(nextBox!.x).toBeGreaterThanOrEqual(viewportBox!.x + viewportBox!.width - 1);
  });

  test("inert state is frozen while a drag is in progress -- it only updates on release/settle", async ({ page }) => {
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const content = frame.locator(".dx-carousel-content");
    const items = frame.locator(".dx-carousel-item");
    const count = await items.count();

    const inertSnapshot = async () => {
      const flags: boolean[] = [];
      for (let i = 0; i < count; i++) {
        flags.push(await items.nth(i).evaluate((el) => el.hasAttribute("inert")));
      }
      return flags;
    };

    const before = await inertSnapshot();
    const pitch = await slidePitch(items.first());
    await dragHold(page, content, -pitch * 0.7, 0);

    // Sample several times while still holding -- none of these may differ
    // from the pre-drag baseline (freeze rule: `loop-a11y-guidance.md` §7 /
    // `carousel-overscroll-2026-09-23.md` §3/§6, the same "never write mid-
    // gesture" discipline every other scroll-position write in this module
    // already follows).
    for (let i = 0; i < 4; i++) {
      expect(await inertSnapshot()).toEqual(before);
      await page.waitForTimeout(60);
    }

    await page.mouse.up();
    // Now it may (and, since the drag crossed a slide boundary, should)
    // update.
    await expect(async () => {
      const after = await inertSnapshot();
      expect(after).not.toEqual(before);
    }).toPass({ timeout: 3000 });
  });

  test("focus is redirected to the carousel's own content region -- never lost to <body> -- when the focused slide leaves via the root keyboard handler", async ({ page }) => {
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const content = frame.locator(".dx-carousel-content");
    const slideButton = frame.getByTestId("carousel-slide-button");

    await slideButton.focus();
    await expect(slideButton).toBeFocused();

    await page.keyboard.press("ArrowRight");
    await expect(frame.getByRole("group", { name: "2 of 5" })).toHaveAttribute("data-selected", "true");

    const contentId = await content.getAttribute("id");
    await expect
      .poll(async () =>
        page.evaluate(() => ({
          isBody: document.activeElement === document.body,
          id: document.activeElement?.id ?? null,
        })),
      )
      .toEqual({ isBody: false, id: contentId });
  });

  test("focus is redirected -- not lost to <body> -- when the focused slide leaves via clicking Next", async ({ page }) => {
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const content = frame.locator(".dx-carousel-content");
    const slideButton = frame.getByTestId("carousel-slide-button");

    await slideButton.focus();
    await frame.getByRole("button", { name: "Next slide" }).click();
    await expect(frame.getByRole("group", { name: "2 of 5" })).toHaveAttribute("data-selected", "true");

    const isBody = await page.evaluate(() => document.activeElement === document.body);
    expect(isBody).toBe(false);
  });

  test("the focus redirect never scrolls the page (focus() scrolls ancestors unless preventScroll)", async ({ page }) => {
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const content = frame.locator(".dx-carousel-content");
    const slideButton = frame.getByTestId("carousel-slide-button");

    // Park the page so the content region's top edge is just off-screen: a
    // bare `focus()` on it would scroll the page to reveal it.
    let previous: number | null = null;
    for (let i = 0; i < 50; i++) {
      const y = await page.evaluate(() => window.scrollY);
      if (y === previous) break;
      previous = y;
      await page.waitForTimeout(150);
    }
    const contentId = await content.getAttribute("id");
    await page.evaluate((id) => {
      const top = document.getElementById(id!)!.getBoundingClientRect().top + window.scrollY;
      window.scrollTo({ top: top + 40, left: 0, behavior: "instant" });
    }, contentId);
    await slideButton.evaluate((el) => (el as HTMLElement).focus({ preventScroll: true }));
    await expect(slideButton).toBeFocused();
    const scrollYBefore = await page.evaluate(() => window.scrollY);

    await page.keyboard.press("ArrowRight");
    await expect(frame.getByRole("group", { name: "2 of 5" })).toHaveAttribute("data-selected", "true");
    await expect.poll(() => page.evaluate(() => document.activeElement?.id ?? null)).toBe(contentId);
    expect(await page.evaluate(() => window.scrollY)).toBe(scrollYBefore);
  });

  test("Tab from the last focusable in the current slide leaves the carousel -- no keyboard trap", async ({ page }) => {
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const region = frame.locator('[role="region"]');
    const slideButton = frame.getByTestId("carousel-slide-button");

    await slideButton.focus();
    await expect(slideButton).toBeFocused();
    await page.keyboard.press("Tab");

    const regionId = await region.getAttribute("id");
    const stillInside = await page.evaluate(
      ({ regionId }) => {
        const active = document.activeElement;
        const region = regionId
          ? document.getElementById(regionId)
          : document.querySelector('[role="region"][aria-label="Featured photos"]');
        return !!(region && active && region.contains(active));
      },
      { regionId },
    );
    expect(stillInside).toBe(false);
  });

  test("autoplay does not advance while focus is inside a slide (pause-on-focus invariant, still holds with inert)", async ({ page }) => {
    // None of the shipped demo variants happen to put a focusable element
    // inside an autoplay carousel's own slide content, so a plain
    // `tabindex` is added to the CURRENT slide's own element for this test
    // only -- the point under test is the Rust-side `focus_within` gate
    // (`AutoplayContext`'s own doc), not any particular demo markup. It
    // must be the current slide: a non-current one is already `inert` at
    // page load (this describe block's own contract), and an inert
    // element refuses programmatic `.focus()` too, not just pointer/Tab --
    // so focusing anything else here would silently no-op and the test
    // would pass for the wrong reason.
    await goto(page, "autoplay");
    const frame = demoFrame(page, "autoplay");
    const slide1 = frame.getByRole("group", { name: "1 of 5" });
    await slide1.evaluate((el) => el.setAttribute("tabindex", "-1"));
    await slide1.evaluate((el) => (el as HTMLElement).focus());
    await expect(async () => {
      const focused = await page.evaluate(() => document.activeElement?.getAttribute("aria-label"));
      expect(focused).toBe("1 of 5");
    }).toPass({ timeout: 1000 });

    const selectedAt = async () =>
      frame.locator('[data-selected="true"]').getAttribute("aria-label");
    const before = await selectedAt();
    expect(before).toBe("1 of 5");
    await page.waitForTimeout(2600); // past two full 1200ms ticks
    expect(await selectedAt()).toBe(before);
  });

  test("a drag starting on a partially-visible neighbour slide still pages the carousel (inert content is not a hit-test target)", async ({ page }) => {
    // The old `multiple` variant (a bare `flex-basis: 40%` override) is
    // retired -- `peek` is the new, explicitly opt-in home for a genuinely
    // partially-visible neighbour slide (`--dx-carousel-peek: 20%`); the
    // `sizes` variant that replaced `multiple` shows only WHOLE slides by
    // design (that is the entire point of the shadcn-parity fix), so it no
    // longer has a partially-visible neighbour to drag-start on at all.
    await goto(page, "peek");
    const frame = demoFrame(page, "peek");
    const content = frame.locator(".dx-carousel-content");
    const viewport = frame.locator('[data-slot="carousel-viewport"]');
    const items = frame.locator(".dx-carousel-item");
    const count = await items.count();

    let neighbourIndex = -1;
    for (let i = 0; i < count; i++) {
      if (await items.nth(i).evaluate((el) => el.hasAttribute("inert"))) {
        neighbourIndex = i;
        break;
      }
    }
    expect(neighbourIndex, "expected at least one inert (partially visible) neighbour").toBeGreaterThan(-1);

    const neighbour = items.nth(neighbourIndex);
    // Bring the demo into the *page's* own viewport first -- `page.mouse`
    // targets real screen coordinates, and this component sits far down
    // the long component-catalog page (`dragBy`'s own doc, above, hits the
    // identical problem). Scrolling the PAGE is a different scroll
    // container from the carousel's own internal `.dx-carousel-content`
    // track, so this cannot itself reveal the neighbour slide the way
    // `scrollIntoViewIfNeeded` on the *track* would -- it only moves which
    // part of the page is on screen, not which slide is scrolled into the
    // track's own viewport.
    await scrollIntoViewInstant(viewport);
    // A point inside BOTH the neighbour's own layout box and the clipping
    // viewport's -- guaranteed actually painted on screen (not clipped)
    // without ever calling `scrollIntoViewIfNeeded` on the neighbour itself
    // (which would scroll this very slide fully into the *track's* view,
    // defeating "partially visible").
    const [viewportBox, neighbourBox] = await Promise.all([viewport.boundingBox(), neighbour.boundingBox()]);
    expect(viewportBox).not.toBeNull();
    expect(neighbourBox).not.toBeNull();
    const x = Math.max(viewportBox!.x, neighbourBox!.x) + 2;
    const y = neighbourBox!.y + neighbourBox!.height / 2;
    expect(x).toBeLessThan(viewportBox!.x + viewportBox!.width);

    const selectedBefore = await frame.locator('[data-selected="true"]').getAttribute("aria-label");
    const pitch = await slidePitch(items.first());

    await page.mouse.move(x, y);
    await page.mouse.down();
    for (let i = 1; i <= 12; i++) {
      await page.mouse.move(x - (pitch * 0.7 * i) / 12, y);
      await page.waitForTimeout(16);
    }
    await page.mouse.up();

    await expect(async () => {
      const selectedAfter = await frame.locator('[data-selected="true"]').getAttribute("aria-label");
      expect(selectedAfter).not.toBe(selectedBefore);
    }).toPass({ timeout: 3000 });
  });

  test("axe: still clean with non-visible slides inert (main variant)", async ({ page }) => {
    await goto(page, "main");
    await expectNoAxeViolations(page, "carousel: main variant with inert slides", {
      include: "#component-preview-frame",
    });
  });
});

/**
 * `CarouselVirtualContent`: the data-driven, virtualised drop-in for
 * `CarouselContent` + `CarouselItem`s (`primitives/src/carousel.rs`'s own
 * `CarouselVirtualContent` doc has the construction; `dev-docs/backlog.md`
 * row 91's dated addendum has this lane's own report).
 *
 * Every slide here carries `data-index`/`data-position` (see
 * `CarouselItem`'s own doc for why every bridge in this module reads
 * those, never a child's array position) but never the themed
 * `.dx-carousel-item` class -- that class is applied by the themed
 * wrapper around `CarouselItem` specifically
 * (`preview/src/components/carousel/component.rs`), and
 * `CarouselVirtualContent` renders its own slide markup directly (the
 * primitive owns the wrapping div, not a nested themed component) -- so
 * every locator below scopes on `[data-position]` instead.
 *
 * `virtual_loop` (12 items, `radius: 2` -> a 5-slide window, `loop: true`,
 * `CarouselAutoplay { delay_ms: 1200 }`, `CarouselIndicators` dots) and
 * `virtual_many` (200 items, `loop: false`) are the two demo variants
 * (`preview/src/components/carousel/variants/virtual_loop|virtual_many/mod.rs`).
 */
test.describe("CarouselVirtualContent: the DOM never holds more than 2*radius+1 slides", () => {
  test("virtual_loop (N=12) mounts exactly 5 slides", async ({ page }) => {
    await goto(page, "virtual_loop");
    const frame = demoFrame(page, "virtual_loop");
    await expect(frame.locator("[data-position]")).toHaveCount(5);
  });

  test("virtual_many (N=200) mounts exactly 5 slides once away from the (non-looping) start edge", async ({ page }) => {
    await goto(page, "virtual_many");
    const frame = demoFrame(page, "virtual_many");
    // At the very first slide the (non-wrapping) window clamps to
    // `[0, radius]` -- 3 slides, not 5 (`window()`'s own documented
    // clamping behaviour, `primitives/src/virtual/window.rs`). Page away
    // from that edge first so the window is centred and at its full
    // width, the case this test is actually about.
    const next = frame.getByRole("button", { name: "Next slide" });
    await next.dispatchEvent("click");
    await next.dispatchEvent("click");
    await expect.poll(() => frame.locator('[data-selected="true"]').getAttribute("aria-label")).toBe(
      "3 of 200",
    );
    await expect(frame.locator("[data-position]")).toHaveCount(5);
  });
});

test.describe("CarouselVirtualContent: seamless loop (virtual_loop)", () => {
  function selectedLabel(frame: Locator) {
    return () => frame.locator('[data-selected="true"]').getAttribute("aria-label");
  }

  test("Next through all 12 slides wraps 12 -> 1, one slide per click, DOM count never grows", async ({ page }) => {
    await goto(page, "virtual_loop");
    const frame = demoFrame(page, "virtual_loop");
    const next = frame.getByRole("button", { name: "Next slide" });
    const label = selectedLabel(frame);

    await expect.poll(label).toBe("1 of 12");
    for (let i = 2; i <= 12; i++) {
      await next.dispatchEvent("click");
      await expect.poll(label).toBe(`${i} of 12`);
      // Structurally impossible to have "rewound" through every
      // intervening slide the way the CarouselItem-based `looping`
      // variant does: the scroller only ever holds 5 DOM slides.
      await expect(frame.locator("[data-position]")).toHaveCount(5);
    }
    // The 13th Next wraps physically forward (12 -> 1), not a rewind.
    await next.dispatchEvent("click");
    await expect.poll(label).toBe("1 of 12");
    await expect(frame.locator("[data-position]")).toHaveCount(5);
  });

  test("Previous through all 12 slides wraps 1 -> 12, one slide per click", async ({ page }) => {
    await goto(page, "virtual_loop");
    const frame = demoFrame(page, "virtual_loop");
    const previous = frame.getByRole("button", { name: "Previous slide" });
    const label = selectedLabel(frame);

    await expect.poll(label).toBe("1 of 12");
    // Previous is never `disabled` under `loop: true` -- confirms the
    // wrap is reachable via this button at all before relying on it.
    await expect(previous).toBeEnabled();
    // Starting at "1 of 12", the FIRST Previous click is itself the wrap
    // (1 -> 12); the rest count down normally back around to 1.
    for (const i of [12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1]) {
      await previous.dispatchEvent("click");
      await expect.poll(label).toBe(`${i} of 12`);
      await expect(frame.locator("[data-position]")).toHaveCount(5);
    }
  });

  test("ArrowRight/ArrowLeft page and wrap the same way as the buttons", async ({ page }) => {
    await goto(page, "virtual_loop");
    const frame = demoFrame(page, "virtual_loop");
    // The scroll region itself (`tabindex="0"`), not the carousel's own
    // `role="region"` root (which is never focusable) -- both carry
    // `data-orientation`, so a bare `[data-orientation]` locator's
    // `.first()` grabs the (ancestor, non-focusable) root instead.
    const content = frame.locator(".dx-carousel-content");
    const label = selectedLabel(frame);

    await content.focus();
    await expect(content).toBeFocused();
    await expect.poll(label).toBe("1 of 12");
    // A small gap between presses -- see this file's own "seamless loop"
    // describe block's "autoplay" test for why a real settle needs real
    // wall-clock time between steps (`radius: 2`'s own margin is not
    // unlimited): a genuinely human keyboard cadence, not a synthetic
    // zero-delay flood, is what this construction is built for.
    for (let i = 0; i < 13; i++) {
      await page.keyboard.press("ArrowRight");
      await page.waitForTimeout(120);
    }
    await expect.poll(label).toBe("2 of 12"); // 13 steps forward from 1, mod 12
    for (let i = 0; i < 2; i++) {
      await page.keyboard.press("ArrowLeft");
      await page.waitForTimeout(120);
    }
    await expect.poll(label).toBe("12 of 12");
  });

  test("autoplay advances by exactly one slide per tick and wraps 12 -> 1 with no drift", async ({ page }) => {
    await goto(page, "virtual_loop");
    const frame = demoFrame(page, "virtual_loop");
    const label = selectedLabel(frame);
    await expect.poll(label).toBe("1 of 12");

    // Poll densely (well under the 1200ms tick) rather than sampling at a
    // period close to the tick's own -- an earlier version of this test
    // sampled every 1300ms against a 1200ms tick and saw an apparent
    // "skipped" index once per run purely from that beat frequency (a
    // measurement artifact, not a product bug -- see this lane's own
    // report). This instead records every distinct value seen and asserts
    // the exact sequence.
    const seen: string[] = [(await label())!];
    const deadline = Date.now() + 9000;
    while (Date.now() < deadline && seen.length < 8) {
      await page.waitForTimeout(100);
      const cur = await label();
      if (cur !== seen[seen.length - 1]) {
        seen.push(cur!);
      }
    }
    expect(seen).toEqual([
      "1 of 12",
      "2 of 12",
      "3 of 12",
      "4 of 12",
      "5 of 12",
      "6 of 12",
      "7 of 12",
      "8 of 12",
    ]);
  });

  test("the dot picker (CarouselIndicators) jumps directly to a distant slide and re-anchors instantly", async ({ page }) => {
    await goto(page, "virtual_loop");
    const frame = demoFrame(page, "virtual_loop");
    const label = selectedLabel(frame);
    await expect.poll(label).toBe("1 of 12");

    // Jump straight to slide 7 (6 away -- outside the 5-slide window),
    // the "distant dot click" re-anchor path (`CarouselVirtualContent`'s
    // own "Seamless loop" doc, second bullet). `CarouselIndicator`'s own
    // default accessible name is "Slide {n}" (renamed from the old
    // `CarouselTabList`/`CarouselTab` -- see `component.rs`'s own doc),
    // not the retired custom dot-picker's "Go to slide {n}".
    await frame.getByRole("tab", { name: "Slide 7" }).click();
    await expect.poll(label).toBe("7 of 12");
    await expect(frame.locator("[data-position]")).toHaveCount(5);
    // Now a single Next from there still advances by exactly one, proving
    // the anchor is genuinely centred on 7, not merely displaying it.
    await frame.getByRole("button", { name: "Next slide" }).dispatchEvent("click");
    await expect.poll(label).toBe("8 of 12");
  });

  test('"k of 12" is correct exactly at the wrap in both directions', async ({ page }) => {
    await goto(page, "virtual_loop");
    const frame = demoFrame(page, "virtual_loop");
    const next = frame.getByRole("button", { name: "Next slide" });
    const previous = frame.getByRole("button", { name: "Previous slide" });
    const label = selectedLabel(frame);

    for (let i = 2; i <= 12; i++) {
      await next.dispatchEvent("click");
      await expect.poll(label).toBe(`${i} of 12`);
    }
    await next.dispatchEvent("click");
    await expect.poll(label).toBe("1 of 12");
    await previous.dispatchEvent("click");
    await expect.poll(label).toBe("12 of 12");
  });
});

test.describe("CarouselVirtualContent: trackpad-style wheel scrolling wraps seamlessly", () => {
  test("a forward wheel-scroll sequence advances one slide at a time, never a multi-slide jump", async ({ page }) => {
    await goto(page, "virtual_loop");
    const frame = demoFrame(page, "virtual_loop");
    const content = frame.locator("[data-orientation]").first();
    const label = () => frame.locator('[data-selected="true"]').getAttribute("aria-label");

    await expect.poll(label).toBe("1 of 12");
    const pitch = await slidePitch(frame.locator('[data-selected="true"]'));
    await content.hover();

    const seen: string[] = ["1 of 12"];
    // One wheel burst per intended step -- headless Chromium needs
    // `deltaX` (this file's own established wheel-test convention; see
    // the "wheel overdrag" describe block's own note on this), several
    // events per burst to cross one slide pitch reliably, with a settle
    // pause between bursts so each step's own settle completes before the
    // next burst starts (never mid-gesture -- `data-dragging`/the wheel
    // bridge's own idle detection would otherwise coalesce a fast stream
    // into a single multi-slide travel, which is a real behavior of
    // native scrolling, not a bug in this construction, and would defeat
    // this test's own "one slide at a time" premise).
    for (let step = 0; step < 6; step++) {
      for (let i = 0; i < 6; i++) {
        await page.mouse.wheel(pitch / 6, 0);
      }
      await page.waitForTimeout(300);
      const cur = (await label())!;
      if (cur !== seen[seen.length - 1]) {
        seen.push(cur);
      }
      await expect(frame.locator("[data-position]")).toHaveCount(5);
    }
    // Exactly one new slide per step -- no step is ever skipped (a
    // multi-slide jump) and none repeats (a stall).
    expect(seen).toEqual([
      "1 of 12",
      "2 of 12",
      "3 of 12",
      "4 of 12",
      "5 of 12",
      "6 of 12",
      "7 of 12",
    ]);
  });
});

test.describe("CarouselVirtualContent: a11y", () => {
  test("only the current slide is non-inert; the rest are inert; this holds across a wrap", async ({ page }) => {
    await goto(page, "virtual_loop");
    const frame = demoFrame(page, "virtual_loop");
    const slides = frame.locator("[data-position]");

    const inertSnapshot = async () => {
      const count = await slides.count();
      const rows: { index: string | null; inert: boolean }[] = [];
      for (let i = 0; i < count; i++) {
        rows.push({
          index: await slides.nth(i).getAttribute("data-index"),
          inert: await slides.nth(i).evaluate((el) => el.hasAttribute("inert")),
        });
      }
      return rows;
    };

    let rows = await inertSnapshot();
    expect(rows.filter((r) => !r.inert)).toEqual([{ index: "0", inert: false }]);

    // Wrap forward past the last slide (12 -> 1) and re-check: still
    // exactly one non-inert slide, now data-index 0 again but a different
    // DOM node's own `data-position` (seamless, not a rewind).
    const next = frame.getByRole("button", { name: "Next slide" });
    for (let i = 0; i < 12; i++) {
      await next.dispatchEvent("click");
      await expect
        .poll(async () => (await inertSnapshot()).filter((r) => !r.inert).length)
        .toBe(1);
    }
    rows = await inertSnapshot();
    expect(rows.filter((r) => !r.inert)).toEqual([{ index: "0", inert: false }]);
  });

  test("focus is never lost to <body> across a wrap driven by the root keyboard handler", async ({ page }) => {
    await goto(page, "virtual_loop");
    const frame = demoFrame(page, "virtual_loop");
    // The scroll region itself, not the (non-focusable) `role="region"`
    // root -- see the "ArrowRight/ArrowLeft" test's own comment, above,
    // for why a bare `[data-orientation]` locator is ambiguous here.
    const content = frame.locator(".dx-carousel-content");

    await content.focus();
    await expect(content).toBeFocused();
    for (let i = 0; i < 12; i++) {
      await page.keyboard.press("ArrowRight");
      await page.waitForTimeout(120);
    }
    const isBody = await page.evaluate(() => document.activeElement === document.body);
    expect(isBody).toBe(false);
  });

  test("virtual_many: Previous/Next are genuinely disabled at the real, non-looping ends", async ({ page }) => {
    await goto(page, "virtual_many");
    const frame = demoFrame(page, "virtual_many");
    const next = frame.getByRole("button", { name: "Next slide" });
    const previous = frame.getByRole("button", { name: "Previous slide" });
    const label = () => frame.locator('[data-selected="true"]').getAttribute("aria-label");

    await expect.poll(label).toBe("1 of 200");
    await expect(previous).toBeDisabled();
    await expect(next).toBeEnabled();

    // Paced with a small explicit wait *and* `expect.poll` after every
    // click -- see the "ArrowRight/ArrowLeft" test's own comment, above,
    // for why a zero-delay flood of 200 clicks is not this construction's
    // target use case (a settle needs real wall-clock time to land within
    // `radius`'s own margin); this is still far faster than a real user,
    // just not adversarially so. The explicit wait matters more here than
    // in the shorter loop/wrap tests above: at 200 iterations even a rare
    // (~1-in-200) settle-latency race is likely to surface at least once,
    // measured live against the SSG lane specifically (a plain
    // `expect.poll` alone, with no wait, missed exactly one step around
    // i=176 in that environment).
    for (let i = 2; i <= 200; i++) {
      await next.dispatchEvent("click");
      await page.waitForTimeout(40);
      await expect.poll(label).toBe(`${i} of 200`);
    }
    await expect(next).toBeDisabled();
    await expect(previous).toBeEnabled();
    // Clamped at the real end, never wrapping -- one more Next is a no-op.
    await next.dispatchEvent("click");
    await expect.poll(label).toBe("200 of 200");
  });

  test("axe: virtual_loop and virtual_many have no automatically detectable a11y issues", async ({ page }) => {
    // Every variant of a "Normal"-kind component (including these two)
    // renders on the same page at once -- see this file's own header
    // ("SCOPING") -- so the existing "carousel component page (every
    // variant mounted)" scan in the "Axe automated scan" describe block
    // above already covers both; this test targets them individually so a
    // regression names the right variant rather than "carousel: all
    // variants" generically.
    await goto(page, "virtual_loop");
    await expectNoAxeViolations(page, "carousel: virtual_loop variant", {
      include: "#component-preview-frame-virtual_loop",
    });
    await expectNoAxeViolations(page, "carousel: virtual_many variant", {
      include: "#component-preview-frame-virtual_many",
    });
  });

  test("virtual_loop_rtl: seamless loop wraps under RTL with the swapped arrow keys too", async ({ page }) => {
    // Cheap RTL coverage of the identical seamless-loop path (per-lane
    // instruction: "add an RTL variant if cheap") -- not a full duplicate
    // of every `virtual_loop` test above, just the one thing RTL could
    // plausibly break: the key-swap composed with physical wraparound.
    await goto(page, "virtual_loop_rtl");
    const frame = demoFrame(page, "virtual_loop_rtl");
    const next = frame.getByRole("button", { name: "Next slide" });
    const label = () => frame.locator('[data-selected="true"]').getAttribute("aria-label");

    await expect.poll(label).toBe("1 of 12");
    // RTL: ArrowLeft is the swapped "next" key (Direction::resolve_horizontal).
    await next.focus();
    await page.keyboard.press("ArrowLeft");
    await expect.poll(label).toBe("2 of 12");

    await expectNoAxeViolations(page, "carousel: virtual_loop_rtl variant", {
      include: "#component-preview-frame-virtual_loop_rtl",
    });
  });
});

/**
 * The re-centre correction (`CarouselVirtualContent`'s own "Seamless
 * loop" doc): once a paging step settles, the window re-renders around
 * the new anchor and a dedicated effect instantly re-aligns the scroller
 * to compensate for the resulting one-slide-width DOM shift. This must
 * never be visible -- sampled every animation frame across a step that
 * triggers it, the currently-selected slide's own rect must ease
 * smoothly to rest and never move again afterward (no jump back-and-forth,
 * no post-settle correction visible as a second motion).
 */
test.describe("CarouselVirtualContent: the re-centre never paints a visible jump", () => {
  test("the current slide's rect eases to a stable rest position with no jump after settling", async ({ page }) => {
    await goto(page, "virtual_loop");
    const frame = demoFrame(page, "virtual_loop");
    const content = frame.locator("[data-orientation]").first();
    const next = frame.getByRole("button", { name: "Next slide" });

    await content.evaluate((el) => {
      const w = window as unknown as { __samples: number[]; __stopSampling: () => void };
      w.__samples = [];
      let raf: number;
      const sample = () => {
        const sel = el.querySelector('[data-selected="true"]');
        if (sel) {
          w.__samples.push(sel.getBoundingClientRect().left);
        }
        raf = requestAnimationFrame(sample);
      };
      raf = requestAnimationFrame(sample);
      w.__stopSampling = () => cancelAnimationFrame(raf);
    });

    await next.dispatchEvent("click");
    // Comfortably past the paging animation, the settle, and the instant
    // re-centre correction that follows it.
    await page.waitForTimeout(1200);

    const samples: number[] = await content.evaluate(() => {
      const w = window as unknown as { __samples: number[]; __stopSampling: () => void };
      w.__stopSampling();
      return w.__samples;
    });
    expect(samples.length).toBeGreaterThan(10);

    // The final quarter of samples (well after the animation should have
    // finished) must be perfectly flat -- if the re-centre's own instant
    // correction were visible as a second motion after the eased scroll
    // already looked settled, it would show up here as a nonzero diff.
    const tail = samples.slice(Math.floor(samples.length * 0.75));
    for (let i = 1; i < tail.length; i++) {
      expect(Math.abs(tail[i] - tail[i - 1])).toBeLessThan(1);
    }
  });
});

test.describe("CarouselVirtualContent: paging never scrolls the page", () => {
  async function waitForScrollStable(page: Page): Promise<number> {
    let previous: number | null = null;
    for (let i = 0; i < 50; i++) {
      const y = await page.evaluate(() => window.scrollY);
      if (y === previous) {
        return y;
      }
      previous = y;
      await page.waitForTimeout(150);
    }
    throw new Error("waitForScrollStable: window.scrollY never settled");
  }

  test("Next/Previous through a full wrap never moves window scroll position", async ({ page }) => {
    await goto(page, "virtual_loop");
    const frame = demoFrame(page, "virtual_loop");
    const next = frame.getByRole("button", { name: "Next slide" });
    const label = () => frame.locator('[data-selected="true"]').getAttribute("aria-label");

    await waitForScrollStable(page);
    await page.evaluate(() => window.scrollTo({ top: 0, left: 0, behavior: "instant" }));
    const scrollYBefore = await page.evaluate(() => window.scrollY);
    const scrollXBefore = await page.evaluate(() => window.scrollX);

    // Paced with `expect.poll` per click -- see the "ArrowRight/ArrowLeft"
    // test's own comment (`CarouselVirtualContent: seamless loop` describe
    // block, above) for why a zero-delay flood of clicks is not this
    // construction's target use case.
    for (let i = 2; i <= 13; i++) {
      await next.dispatchEvent("click");
      await expect.poll(label).toBe(`${((i - 1) % 12) + 1} of 12`);
    }
    expect(await page.evaluate(() => window.scrollY)).toBe(scrollYBefore);
    expect(await page.evaluate(() => window.scrollX)).toBe(scrollXBefore);
  });
});
