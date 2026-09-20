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
 * shadcn-parity geometry: 28x28px buttons, no box-shadow, and fully outside
 * the region -- never overlapping a slide -- on every variant and axis.
 * Values are shadcn/ui's own live-measured carousel
 * (`ui.shadcn.com/docs/components/carousel`, JS running, 1280x800 @2x):
 * `getBoundingClientRect()` on both buttons reported exactly 28x28,
 * `getComputedStyle().boxShadow` reported "none", and the near edge of each
 * button sat 20px clear of the region's own edge (`--dx-space-12`, 48px,
 * minus the button's own 28px). Their own vertical ("Orientation") demo
 * confirmed the same 20px-clear rule holds on the block axis too, not just
 * inferred from the horizontal case -- see
 * `preview/src/components/carousel/style.css`'s own comments for the full
 * derivation and the exact numbers this asserts against.
 */
test.describe("Carousel: shadcn-parity geometry (size, shadow, outside placement)", () => {
  const BUTTON_SIZE_PX = 28;
  const MIN_CLEAR_PX = 20 - 1; // 1px tolerance for sub-pixel layout rounding

  for (const variant of ["main", "multiple", "indicators", "vertical", "rtl"] as const) {
    test(`${variant}: Previous/Next are 28x28, shadow-less, 20px clear of the region`, async ({ page }) => {
      await goto(page, variant);
      const frame = demoFrame(page, variant);
      const region = frame.getByRole("region");
      const previous = frame.getByRole("button", { name: "Previous slide" });
      const next = frame.getByRole("button", { name: "Next slide" });

      await expect(previous).toHaveCSS("box-shadow", "none");
      await expect(next).toHaveCSS("box-shadow", "none");

      const [regionBox, previousBox, nextBox] = await Promise.all([
        region.boundingBox(),
        previous.boundingBox(),
        next.boundingBox(),
      ]);
      expect(regionBox).not.toBeNull();
      expect(previousBox).not.toBeNull();
      expect(nextBox).not.toBeNull();

      for (const box of [previousBox!, nextBox!]) {
        expect(box.width).toBeCloseTo(BUTTON_SIZE_PX, 0);
        expect(box.height).toBeCloseTo(BUTTON_SIZE_PX, 0);
      }

      if (variant === "vertical") {
        // Block axis: Previous above the region, Next below it.
        expect(regionBox!.y - (previousBox!.y + previousBox!.height)).toBeGreaterThanOrEqual(MIN_CLEAR_PX);
        expect(nextBox!.y - (regionBox!.y + regionBox!.height)).toBeGreaterThanOrEqual(MIN_CLEAR_PX);
      } else if (variant === "rtl") {
        // Inline axis, mirrored: Previous (the *start* edge) resolves to
        // the physical right under dir="rtl"; Next (the *end* edge) to the
        // physical left -- the opposite pairing from every other variant.
        expect(previousBox!.x - (regionBox!.x + regionBox!.width)).toBeGreaterThanOrEqual(MIN_CLEAR_PX);
        expect(regionBox!.x - (nextBox!.x + nextBox!.width)).toBeGreaterThanOrEqual(MIN_CLEAR_PX);
      } else {
        expect(regionBox!.x - (previousBox!.x + previousBox!.width)).toBeGreaterThanOrEqual(MIN_CLEAR_PX);
        expect(nextBox!.x - (regionBox!.x + regionBox!.width)).toBeGreaterThanOrEqual(MIN_CLEAR_PX);
      }
    });
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
 * `scroll-snap-stop: always` (`primitives/src/carousel.rs`, a scoped
 * `<style>` tag rendered alongside the track -- see that file's own
 * `no_skip_supports_css` doc, including its "Known limitation" section)
 * requires this track to stop at the first snap position a scroll
 * operation would otherwise pass, rather than skipping over several to
 * settle on whichever is numerically nearest. Measured to genuinely cap a
 * browser-animated smooth scroll -- a caller's own `scrollBy(...,
 * {behavior: 'smooth'})`, and by the same CSS Scroll Snap Spec language a
 * native wheel/trackpad fling -- to one slide of travel regardless of the
 * requested distance, which is what this test exercises directly.
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
 * lane originally hoped it would also cover. See `no_skip_supports_css`'s
 * own doc for the full write-up; that gap is being reported, not silently
 * worked around here.
 */
test.describe("Carousel: scroll-snap-stop caps a smooth scroll to one slide", () => {
  test("a smooth scroll covering multiple slide widths lands exactly one slide away, never more", async ({ page }) => {
    await goto(page, "main");
    const frame = demoFrame(page, "main");
    const content = frame.locator(".dx-carousel-content");
    const slide = (n: number) => frame.getByRole("group", { name: `${n} of 5` });
    await expect(slide(1)).toHaveAttribute("data-selected", "true");

    const pitch = await slidePitch(slide(1));
    const contentId = await content.getAttribute("id");
    // 2.5 slides' worth -- comfortably enough that "settle on whichever is
    // numerically nearest" (the pre-existing, still-current behaviour for
    // this crate's own drag path -- see this describe block's own header)
    // would land on slide 3, not slide 2.
    await page.evaluate(
      ({ id, distance }) => {
        document.getElementById(id)!.scrollBy({ left: distance, behavior: "smooth" });
      },
      { id: contentId, distance: pitch * 2.5 },
    );

    await expectSnappedToBoundary(content, slide(2));
    await expect(slide(2)).toHaveAttribute("data-selected", "true");
    await expect(slide(3)).toHaveAttribute("data-selected", "false");
  });
});
