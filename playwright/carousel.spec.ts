/**
 * Carousel: smoke + a11y attributes + keyboard paging (LTR and RTL) +
 * end-of-range button state + scroll-snap landing, across every shipped
 * variant (main, multiple, indicators, vertical, rtl).
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

import { test, expect, type Page, type Locator } from "@playwright/test";
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
    | "multiple"
    | "indicators"
    | "vertical"
    | "rtl"
    | "looping"
    | "autoplay"
    | "tabs"
    | "virtual_loop"
    | "virtual_many",
): Locator {
  const id = variant === "main" ? "component-preview-frame" : `component-preview-frame-${variant}`;
  return page.locator(`#${id}`);
}

/**
 * Scroll `locator`'s element into view INSTANTLY (bypassing
 * `preview/assets/main.css`'s global `html { scroll-behavior: smooth }`
 * rule) before an action -- `.hover()` in particular -- that would
 * otherwise trigger Playwright's own actionability pre-check
 * (`scrollIntoViewIfNeeded`). An *unqualified* scroll's default `behavior:
 * "auto"` means "respect the scrolling box's own CSS `scroll-behavior`",
 * not "jump instantly" -- so on a page with that global rule, Playwright's
 * own pre-hover scroll animates smoothly instead of jumping, and (this
 * component's demo sits well down the long component-catalog page, so the
 * scroll distance is large) keeps moving the page for a couple hundred ms
 * *after* `.hover()` has already dispatched `mouseenter` and returned --
 * dragging the target out from under the now-stationary cursor and firing
 * a genuine, browser-native `mouseleave` a few hundred ms later. Calling
 * this first makes the element already in view, so that pre-check becomes
 * a no-op and no such scroll ever starts.
 *
 * This is the same class `oracle/tier2-html/top-layer.spec.ts`'s own
 * `pinNearTop` helper exists for (2026-09-18, commit daebe40's global
 * smooth-scroll rule racing an unqualified scroll call) -- a second
 * instance, not a new class; see that helper's own doc for the general
 * shape. Found here by execution (this session): a scratch MutationObserver
 * + `getBoundingClientRect`/`scrollY` probe against the SSG build showed,
 * on every repro of "hovering the carousel stops rotation, and moving away
 * resumes it" flaking, `mouseenter` firing while the page was still
 * mid-scroll, then a genuine `mouseleave` ~250-300ms later as the
 * still-animating scroll dragged the carousel out from under the cursor,
 * then rotation resuming its normal ~1200ms cadence from a fresh cycle --
 * never a stale tick's tail settling late (which would have ruled out this
 * mechanism in favor of "before was read mid-transition"). Forcing
 * `html { scroll-behavior: auto }` for a scratch run took the repro rate
 * from ~3/10 to 0/15; this instant pre-scroll independently also took it
 * to 0/15 -- two different ways of removing the same in-flight scroll,
 * both eliminating the failure, is the confirmation.
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
 */
async function expectSnappedToBoundary(content: Locator, item: Locator, orientation: "horizontal" | "vertical" = "horizontal") {
  await expect(async () => {
    const [containerBox, itemBox] = await Promise.all([content.boundingBox(), item.boundingBox()]);
    expect(containerBox).not.toBeNull();
    expect(itemBox).not.toBeNull();
    const delta =
      orientation === "horizontal"
        ? Math.abs(containerBox!.x - itemBox!.x)
        : Math.abs(containerBox!.y - itemBox!.y);
    // A couple of CSS pixels of tolerance for sub-pixel rounding during a
    // smooth-scroll settle -- not a loose bound: an unsettled scroll (still
    // mid-animation, or landed on the wrong slide) misses by tens/hundreds
    // of pixels, not 1-2.
    expect(delta).toBeLessThanOrEqual(2);
  }).toPass({ timeout: 3000 });
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

for (const variant of ["main", "multiple", "indicators", "vertical", "rtl"] as const) {
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

  for (const variant of ["main", "multiple", "indicators", "vertical", "rtl"] as const) {
    test(`${variant}: Previous/Next are 28x28, shadow-less, 20px clear of the track`, async ({ page }) => {
      await goto(page, variant);
      const frame = demoFrame(page, variant);
      const content = frame.locator(".dx-carousel-content");
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

        for (const variant of ["main", "multiple", "indicators", "vertical", "rtl"] as const) {
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
  test("clicking a dot indicator jumps directly to that slide, and the active dot tracks the selection", async ({ page }) => {
    await goto(page, "indicators");
    const frame = demoFrame(page, "indicators");
    const indicators = frame.locator(".dx-carousel-indicator");
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 4` });

    await expect(indicators).toHaveCount(4);
    await expect(indicators.nth(0)).toHaveAttribute("data-active", "true");

    await indicators.nth(2).click();
    await expect(slide(3)).toHaveAttribute("data-selected", "true");
    await expect(indicators.nth(2)).toHaveAttribute("data-active", "true");
    await expect(indicators.nth(0)).toHaveAttribute("data-active", "false");
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
 * Edge rubber-band (mode B3), backlog row 102 -- port of the closed design
 * in `dev-docs/research/carousel-overscroll-2026-09-23.md`. See
 * `primitives/src/carousel.rs`'s own `CarouselContent` doc, "Edge
 * rubber-band (mode B3)" section, for the construction these tests hold
 * to: a `transform` on `.dx-carousel-content` only, never a scroll-position
 * write on the wheel path, never a spacer child, and never a change to
 * `selected`.
 *
 * RED-FIRST: written and run against the pre-port `carousel.rs` (mode A,
 * plain clamping) before any of this file's own port landed -- every test
 * below failed (`readContentTransform` always returned `""`, since nothing
 * ever wrote `style.transform`). All pass against the ported code.
 *
 * A note on the wheel tests specifically: `page.mouse.wheel` fires
 * synthetic, discrete `wheel` events with no compositor momentum behind
 * them at all -- it can drive the same edge-detection/transform/spring-back
 * code path a real trackpad gesture does, but it cannot reproduce a real
 * trackpad's momentum, deceleration curve, or "feel." Headless Chromium has
 * no way to fake that honestly; the owner still needs to feel-test this on
 * real hardware (a real mouse drag and a real trackpad, both directions,
 * both axes) before calling the feel itself settled.
 */
test.describe("Carousel: edge rubber-band (mode B3)", () => {
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

  test("wheel overdrag at the start boundary produces a transform that settles back to identity, without paging", async ({
    page,
  }) => {
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const content = frame.locator(".dx-carousel-content");
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 5` });
    await expect(slide(1)).toHaveAttribute("data-selected", "true");

    await content.hover();
    // `deltaX`, not `deltaY`: measured live against this exact server that
    // a plain vertical wheel delta on a HORIZONTALLY-scrolling container
    // does not scroll it at all in headless Chromium (no vertical overflow
    // to redirect) -- the event reaches the element (confirmed by a direct
    // listener probe) but `scrollLeft` never moves, so the page scrolls
    // instead and the next synthetic event lands somewhere else entirely.
    // `deltaX` is also the physically correct simulated gesture for a real
    // trackpad's horizontal two-finger swipe. Negative at slide 1 (the
    // start) asks for more "previous" than exists. Several events in quick
    // succession, matching a real burst, so the accumulated overdrag is
    // comfortably measurable.
    for (let i = 0; i < 6; i++) {
      await page.mouse.wheel(-120, 0);
    }

    await expect(async () => {
      const transform = await readContentTransform(content);
      const depth = parseTranslatePx(transform);
      expect(depth, `expected a nonzero translateX, got "${transform}"`).not.toBeNull();
      expect(depth!).toBeGreaterThan(0);
    }).toPass({ timeout: 2000 });

    // Settles back on its own once the burst goes idle (no pointerup for a
    // wheel gesture) -- bounded well above the idle backstop (90ms) plus
    // the spring-back's own duration (340ms).
    await expect(async () => {
      expect(await readContentTransform(content)).toBe("");
    }).toPass({ timeout: 3000 });

    // The wheel path never wrote the scroll position at all (THE RULE) --
    // `selected` is unchanged.
    await expect(slide(1)).toHaveAttribute("data-selected", "true");
  });

  test("wheel overdrag at the end boundary produces a transform that settles back to identity, without paging", async ({
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

    await content.hover();
    // `deltaX` -- see the start-boundary test's own comment above.
    for (let i = 0; i < 6; i++) {
      await page.mouse.wheel(120, 0);
    }

    await expect(async () => {
      const transform = await readContentTransform(content);
      const depth = parseTranslatePx(transform);
      expect(depth, `expected a nonzero translateX, got "${transform}"`).not.toBeNull();
      expect(depth!).toBeLessThan(0);
    }).toPass({ timeout: 2000 });

    await expect(async () => {
      expect(await readContentTransform(content)).toBe("");
    }).toPass({ timeout: 3000 });

    await expect(slide(5)).toHaveAttribute("data-selected", "true");
  });

  /**
   * The wheel path's rubber-band curve, `x*c*d/(d+c*x)` (Apple's own
   * formula, `c = 0.55`, `d = axisSize()` -- `primitives/src/carousel.rs`'s
   * `CAROUSEL_WHEEL_BOUNCE_JS`'s own `rubber(x)`, corrected 2026-09-25; see
   * that constant's own doc and `dev-docs/research/carousel-overscroll-2026-09-23.md`
   * §9's dated entry for the earlier, mis-transcribed curve this replaces).
   * Two properties fall directly out of that formula's own shape and hold
   * for ANY sustained (non-decaying) push, independent of the exact axis
   * size measured live below: the depth never exceeds the asymptote `d`,
   * and the curve is strictly concave, so doubling the total raw input
   * strictly less than doubles the visible depth. This test only exercises
   * a sustained, constant-magnitude stream -- no momentum/decay behavior is
   * asserted here (that is deliberately out of scope for this lane; see
   * this lane's own report).
   */
  test("a sustained constant wheel stream at the start boundary stays within the axis size and grows sub-linearly", async ({
    page,
  }) => {
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const content = frame.locator(".dx-carousel-content");
    const viewport = viewportLocator(frame);
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 5` });
    await expect(slide(1)).toHaveAttribute("data-selected", "true");

    await content.hover();
    const box = await viewport.boundingBox();
    if (!box) {
      throw new Error("viewport has no bounding box");
    }
    const axisSize = box.width;

    // `deltaX` -- see the start-boundary test's own comment above.
    // Negative at slide 1 (the start) asks for more "previous" than
    // exists, same direction as the earlier wheel tests in this block.
    const sendBurst = async (n: number) => {
      for (let i = 0; i < n; i++) {
        await page.mouse.wheel(-80, 0);
      }
    };

    await sendBurst(10);
    const depth1 = Math.abs(parseTranslatePx(await readContentTransform(content)) ?? 0);
    expect(depth1, "expected a nonzero bounce after the first burst").toBeGreaterThan(0);
    expect(depth1).toBeLessThanOrEqual(axisSize + 1);

    // Doubling the total raw input (20 events total, same magnitude each,
    // so this is still one sustained push, never decaying).
    await sendBurst(10);
    const depth2 = Math.abs(parseTranslatePx(await readContentTransform(content)) ?? 0);
    expect(depth2).toBeLessThanOrEqual(axisSize + 1);
    expect(depth2, "still pushing should still grow the depth").toBeGreaterThan(depth1);
    expect(depth2, "doubling the input should not double the depth (sub-linear curve)").toBeLessThan(depth1 * 2);

    // Settles back on its own once the burst goes idle.
    await expect(async () => {
      expect(await readContentTransform(content)).toBe("");
    }).toPass({ timeout: 3000 });

    await expect(slide(1)).toHaveAttribute("data-selected", "true");
  });

  test("a mid-range wheel scroll produces no lingering transform and still pages correctly (regression guard)", async ({
    page,
  }) => {
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const content = frame.locator(".dx-carousel-content");
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 5` });

    await content.hover();
    // `deltaX` -- see the start-boundary test's own comment above.
    const pitch = await slidePitch(slide(1));
    for (let i = 0; i < 8; i++) {
      await page.mouse.wheel(pitch / 6, 0);
    }

    await expectSnappedToBoundary(content, slide(2));
    await expect(slide(2)).toHaveAttribute("data-selected", "true");
    await expect(async () => {
      expect(await readContentTransform(content)).toBe("");
    }).toPass({ timeout: 2000 });
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

  test("a wheel overdrag at the start boundary never paints a slide past the viewport's edge", async ({ page }) => {
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const viewport = viewportLocator(frame);
    const content = frame.locator(".dx-carousel-content");
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 5` });
    await expect(slide(1)).toHaveAttribute("data-selected", "true");

    await content.hover();
    // `deltaX` -- see the "edge rubber-band" describe block's identical
    // wheel test above for why.
    for (let i = 0; i < 6; i++) {
      await page.mouse.wheel(-120, 0);
    }

    await expect(async () => {
      const depth = parseTranslatePx(await readContentTransform(content));
      expect(depth, "expected a nonzero translateX").not.toBeNull();
      expect(depth!).toBeGreaterThan(0);
    }).toPass({ timeout: 2000 });

    const box = await viewport.boundingBox();
    if (!box) {
      throw new Error("viewport has no bounding box");
    }
    const x = box.x + box.width + 10;
    const y = box.y + box.height * 0.2;
    expect(await paintsAt(content, x, y), "a slide painted past the viewport's own right edge").toBe(false);

    await expect(async () => {
      expect(await readContentTransform(content)).toBe("");
    }).toPass({ timeout: 3000 });
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

    await content.hover();
    for (let i = 0; i < 6; i++) {
      await page.mouse.wheel(120, 0);
    }

    await expect(async () => {
      const depth = parseTranslatePx(await readContentTransform(content));
      expect(depth, "expected a nonzero translateX").not.toBeNull();
      expect(depth!).toBeLessThan(0);
    }).toPass({ timeout: 2000 });

    const box = await viewport.boundingBox();
    if (!box) {
      throw new Error("viewport has no bounding box");
    }
    const x = box.x - 10;
    const y = box.y + box.height * 0.2;
    expect(await paintsAt(content, x, y), "a slide painted past the viewport's own left edge").toBe(false);

    await expect(async () => {
      expect(await readContentTransform(content)).toBe("");
    }).toPass({ timeout: 3000 });
  });
});

/**
 * `loop`: rewind-style wraparound (backlog row 91, approved fast-follow).
 * The `looping` variant (5 slides, LTR) and `looping_rtl` variant (4
 * slides, `dir="rtl"`) close both directions and the RTL key-swap
 * together -- see `preview/src/components/carousel/variants/looping{,_rtl}/mod.rs`.
 */
test.describe("Carousel: loop (rewind-style wraparound)", () => {
  test("Previous and Next are never disabled, even at the first/last slide", async ({ page }) => {
    await goto(page, "looping");
    const frame = demoFrame(page, "looping");
    const previous = frame.getByRole("button", { name: /previous/i });
    const next = frame.getByRole("button", { name: /next/i });

    await expect(previous).toBeEnabled();
    await expect(next).toBeEnabled();

    for (let i = 0; i < 4; i++) {
      await next.click();
    }
    await expect(frame.getByRole("group", { name: "5 of 5" })).toHaveAttribute("data-selected", "true");
    // A non-loop carousel would have `next` disabled here (see the
    // library-only v1 describe block above) -- `loop` never does.
    await expect(next).toBeEnabled();
    await expect(previous).toBeEnabled();
  });

  test("Next at the last slide rewinds to the first; Previous at the first rewinds to the last", async ({ page }) => {
    await goto(page, "looping");
    const frame = demoFrame(page, "looping");
    const content = frame.locator(".dx-carousel-content");
    const previous = frame.getByRole("button", { name: /previous/i });
    const next = frame.getByRole("button", { name: /next/i });
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 5` });

    // Previous from slide 1 wraps to slide 5.
    await previous.click();
    await expect(slide(5)).toHaveAttribute("data-selected", "true");
    await expectSnappedToBoundary(content, slide(5));

    // Next from slide 5 wraps back to slide 1.
    await next.click();
    await expect(slide(1)).toHaveAttribute("data-selected", "true");
    await expectSnappedToBoundary(content, slide(1));

    // Root-level ArrowRight at the last slide also wraps.
    for (let i = 0; i < 4; i++) {
      await next.click();
    }
    await expect(slide(5)).toHaveAttribute("data-selected", "true");
    await next.focus();
    await page.keyboard.press("ArrowRight");
    await expect(slide(1)).toHaveAttribute("data-selected", "true");
  });

  test("under RTL, loop still wraps both directions with the swapped arrow keys", async ({ page }) => {
    await goto(page, "looping_rtl");
    const frame = demoFrame(page, "looping_rtl");
    const previous = frame.getByRole("button", { name: /previous/i });
    const next = frame.getByRole("button", { name: /next/i });
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 4` });

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

  test("dragging past a physical edge still rubber-bands under loop -- no wrap on drag", async ({ page }) => {
    await goto(page, "looping");
    const frame = demoFrame(page, "looping");
    const content = frame.locator(".dx-carousel-content");
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 5` });

    await expect(slide(1)).toHaveAttribute("data-selected", "true");
    // A large rightward drag from slide 1 (nothing before it) should
    // rubber-band, not wrap to slide 5 -- `loop` only governs
    // Previous/Next/the root keyboard (this crate's own module doc).
    await dragBy(page, content, 250, 0);
    await expect(async () => {
      expect(await readContentTransform(content)).toBe("");
    }).toPass({ timeout: 2000 });
    await expect(slide(1)).toHaveAttribute("data-selected", "true");
  });

  test("axe: the looping variant has no automatically detectable a11y issues", async ({ page }) => {
    await goto(page, "looping");
    await expectNoAxeViolations(page, "carousel: looping variant", {
      include: "#component-preview-frame-looping",
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
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 5` });

    await expect(slide(1)).toHaveAttribute("data-selected", "true");
    await expect(slide(2)).toHaveAttribute("data-selected", "true", { timeout: 3000 });
  });

  test("keyboard focus entering the carousel stops rotation, and it does not resume on its own", async ({ page }) => {
    await goto(page, "autoplay");
    const frame = demoFrame(page, "autoplay");
    const content = frame.locator(".dx-carousel-content");
    const rotation = frame.getByRole("button", { name: /automatic slide show/i });

    // Defense-in-depth, matching the hover test's own fix below: `.focus()`
    // was NOT observed to reproduce that test's race in this session's
    // execution (`scrollIntoViewInstant`'s own doc) -- Playwright's
    // `.focus()` does not require the element to be visible/stable the way
    // `.hover()`/`.click()` do, so it does not appear to run the same
    // pre-action `scrollIntoViewIfNeeded` -- but this test reads `before`
    // in the exact same "immediately after pausing" shape, so the same
    // cheap guard is applied here too rather than relying on that absence
    // of evidence holding forever.
    await scrollIntoViewInstant(content);
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

    // `scrollIntoViewInstant` (this file's own doc on it, above): without
    // this, Playwright's own pre-`.hover()` auto-scroll inherits the
    // page's global smooth-scroll CSS and can still be animating when
    // `mouseenter` fires, dragging the carousel out from under the cursor
    // and firing a genuine `mouseleave` a few hundred ms later -- observed
    // in this session as this exact test's flake (rotation legitimately,
    // correctly resuming after that real `mouseleave`, not a component
    // bug).
    await scrollIntoViewInstant(content);
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
 * Tablist (dot-picker) variant -- APG's "tabbed" carousel style. The
 * `tabs` variant has 5 slides, no separate Previous/Next (matching the
 * vendored reference's own structure).
 */
test.describe("Carousel: tablist (dot-picker) variant", () => {
  test("roles: tablist/tab/tabpanel, roving tabindex, aria-selected sync with the current slide", async ({ page }) => {
    await goto(page, "tabs");
    const frame = demoFrame(page, "tabs");
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
    await goto(page, "tabs");
    const frame = demoFrame(page, "tabs");
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
    await goto(page, "tabs");
    const frame = demoFrame(page, "tabs");
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
    await goto(page, "tabs");
    const frame = demoFrame(page, "tabs");
    const tabs = frame.getByRole("tab");

    await tabs.nth(0).focus();
    await page.keyboard.press("ArrowLeft");
    await expect(tabs.nth(4)).toBeFocused();
    await expect(tabs.nth(4)).toHaveAttribute("aria-selected", "true");

    await page.keyboard.press("ArrowRight");
    await expect(tabs.nth(0)).toBeFocused();
  });

  test("Home/End move focus to the first/last tab and activate it", async ({ page }) => {
    await goto(page, "tabs");
    const frame = demoFrame(page, "tabs");
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
    await goto(page, "tabs");
    const frame = demoFrame(page, "tabs");
    const firstTab = frame.getByRole("tab").nth(0);
    const firstPanelId = await frame.locator('[role="tabpanel"]').nth(0).getAttribute("id");
    expect(firstPanelId).toBeTruthy();
    await expect(firstTab).toHaveAttribute("aria-controls", firstPanelId!);
  });

  test("axe: the tabs variant has no automatically detectable a11y issues", async ({ page }) => {
    await goto(page, "tabs");
    await expectNoAxeViolations(page, "carousel: tabs variant", {
      include: "#component-preview-frame-tabs",
    });
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
 *   the thing actually under test -- `CarouselTab`'s own `onfocus` handler
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

  test("a CarouselTab activation never moves window scroll position (isolated from the browser's own focus-scroll)", async ({
    page,
  }) => {
    await goto(page, "tabs");
    const frame = demoFrame(page, "tabs");
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

  for (const variant of ["tabs", "autoplay", "looping"] as const) {
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

  test("a multi-per-view layout widens the visible set to every slide actually in the viewport (multiple variant)", async ({ page }) => {
    await goto(page, "multiple");
    const frame = demoFrame(page, "multiple");
    const items = frame.locator(".dx-carousel-item");
    const count = await items.count();

    // Measured live, not assumed -- exactly how many slides are fully (or
    // "at least half") visible at once depends on this demo's own rendered
    // `flex-basis: 40%` against however wide its wrapper renders (this
    // file's own established "measure, don't hardcode" convention,
    // `slidePitch`'s own doc). The only structural invariant asserted is
    // that the non-inert set is a contiguous prefix starting at slide 0
    // (this demo never scrolls on its own before this point) and is more
    // than just the one selected slide -- proving the widening actually
    // happened, not merely that the single-slide case still works.
    const inertFlags: boolean[] = [];
    for (let i = 0; i < count; i++) {
      inertFlags.push(await items.nth(i).evaluate((el) => el.hasAttribute("inert")));
    }
    const firstInert = inertFlags.indexOf(true);
    expect(firstInert).toBeGreaterThan(1); // more than just slide 0 is visible
    expect(inertFlags.slice(0, firstInert).every((v) => v === false)).toBe(true);
    expect(inertFlags.slice(firstInert).every((v) => v === true)).toBe(true);

    const names = await axTreeGroupNames(page, "#component-preview-frame-multiple [role=region]");
    for (let i = 0; i < firstInert; i++) {
      expect(names).toContain(`${i + 1} of ${count}`);
    }
    for (let i = firstInert; i < count; i++) {
      expect(names).not.toContain(`${i + 1} of ${count}`);
    }
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
    await goto(page, "multiple");
    const frame = demoFrame(page, "multiple");
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
    // own "Seamless loop" doc, second bullet).
    await frame.locator('[aria-label="Go to slide 7"]').click();
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
