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

const GOTO_OPTS = { timeout: 20 * 60 * 1000 };

async function goto(page: Page, variant: string) {
  await page.goto(`${BASE_URL}/component/?name=carousel&variant=${variant}&`, GOTO_OPTS);
}

/** See this file's own header ("SCOPING"). */
function demoFrame(page: Page, variant: "main" | "multiple" | "indicators" | "vertical" | "rtl"): Locator {
  const id = variant === "main" ? "component-preview-frame" : `component-preview-frame-${variant}`;
  return page.locator(`#${id}`);
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
