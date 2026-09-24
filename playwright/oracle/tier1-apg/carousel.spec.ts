/**
 * ORACLE: tier 1 (APG) -- Carousel pattern rules, calibrated against
 * W3C's own reference implementation.
 *
 * Source pattern sections, quoted verbatim from the pinned checkout of
 * `w3c/aria-practices` (commit `7e4034b262bc0d25332e330d8a582aaf34113829`,
 * vendored under `playwright/oracle/reference/7e4034b/` -- see that
 * directory's own README.md for provenance):
 *
 *   (R1) "A carousel container element that encompasses all components of
 *   the carousel, including both carousel controls and slides, has either
 *   role region or role group... The carousel container has the
 *   aria-roledescription property set to carousel... Note that since the
 *   aria-roledescription is set to 'carousel', the label does not contain
 *   the word 'carousel'."
 *   -- content/patterns/carousel/carousel-pattern.html, "WAI-ARIA Roles,
 *      States, and Properties", and the worked example's own "Role,
 *      Property, State, and Tabindex Attributes" table
 *      (`carousel-region-role`/`carousel-region-aria-roledescription`/
 *      `carousel-region-aria-label` rows).
 *
 *   (R2) "Each slide container has role group with the property
 *   aria-roledescription set to slide... If unique names that identify
 *   the slide content are not available, a number and set size can serve
 *   as a meaningful alternative, e.g., '3 of 10'... because group
 *   elements do not support aria-setsize or aria-posinset."
 *   -- same page, and the worked example's own table
 *      (`carousel-group-role`/`carousel-group-aria-roledescription`/
 *      `carousel-group-aria-label` rows, literally `aria-label="N of 6"`
 *      in `carousel-1-prev-next.html`'s own markup).
 *
 *   (R3) "Enter / Space: Display next or previous slide in the carousel."
 *   -- examples/carousel-1-prev-next.html, "Keyboard Support" ->
 *      "Next and Previous Slide Buttons"
 *      (`data-test-id="carousel-key-enter-or-space-move"`). Native
 *      `<button>` semantics (Tab/Shift+Tab reach it in the ordinary tab
 *      order; activating it never moves focus) apply to both subjects by
 *      construction, not by a pattern-specific citation.
 *
 *   (R4) "Rotation control, previous slide, and next slide buttons
 *   precede the slide content in the Tab sequence."
 *   -- same page, "Keyboard Support" -> "Rotation Control Button"
 *      (`data-test-id="carousel-key-tab"`).
 *
 * WHAT THIS FILE DELIBERATELY DOES NOT COVER, and why (v1 scope, per the
 * user's 2026-09-19 approved decisions -- see
 * `primitives/src/carousel.rs`'s own module doc, "Scope (v1)"):
 *
 *   - The rotation control button, `aria-live` region, and
 *     `prefers-reduced-motion` handling (pattern-page sections on
 *     "Controlling Automatic Slide Rotation" / "Screen Reader
 *     Announcement of Slide Changes") are all part of the *autoplay*
 *     feature, deferred to a fast-follow -- this library's own basic
 *     variant has no rotation control at all, matching the pattern's own
 *     framing that a manually-paged carousel is fully conformant without
 *     one. A dual-subject rule set for these is future work once autoplay
 *     lands.
 *   - The tabbed picker style (`carousel-2-tablist.html`) is also
 *     deferred, so its own reference page was never vendored (see
 *     `playwright/oracle/reference/README.md`'s Carousel row) -- there is
 *     nothing to calibrate a rule against yet.
 *   - `CarouselPrevious`/`CarouselNext` becoming genuinely `disabled` at
 *     the first/last slide is **not** asserted as a dual-subject rule:
 *     the reference implementation *loops* (its own rotation continues
 *     past the last slide back to the first, and manual Previous/Next
 *     activation loops the same way -- confirmed by reading
 *     `examples/js/carousel-prev-next.js`'s `previousCarouselItem`/
 *     `nextCarouselItem`, which each wrap the index with an explicit
 *     boundary check -- `nextIndex < 0` resets to `length - 1`,
 *     `nextIndex >= length` resets to `0` -- and never disable either
 *     button), so it never reaches the boundary state at all. Since v1
 *     deliberately
 *     has no `loop` (a real, documented scope decision, not an APG
 *     requirement either way -- the pattern is silent on looping), this
 *     is asserted as a LIBRARY-ONLY fact in its own describe block below,
 *     not compared against a reference that doesn't exhibit it. Per
 *     `docs/conformance-harness.md`'s own Calibration policy ("if the
 *     REFERENCE subject ever goes red, the rule itself is wrong"), a
 *     dual-subject rule that the reference cannot pass would be exactly
 *     that kind of wrong rule.
 *
 * Calibration shape (docs/conformance-harness.md, "Calibration"; mirrors
 * `disclosure-navigation.spec.ts`'s own two-subject layout): every R1-R4
 * rule below runs once against the pattern's own vendored example page
 * (loaded over `file://`, no network, cannot drift) and once against this
 * library's own `carousel` primitive
 * (`http://127.0.0.1:8080/component/?name=carousel&`, `main` variant). If
 * the REFERENCE subject ever goes red, the rule itself is wrong -- not
 * this library.
 *
 * A note on exact wording: the reference's own accessible names are
 * Title Case ("Previous Slide", "Next Slide"); this library's (matching
 * shadcn's own convention, `dev-docs/research/carousel-2026-09-19.md`
 * §5.1) are sentence case ("Previous slide", "Next slide"). APG requires
 * *a* meaningful accessible name, never this exact string, so every rule
 * below matches case-insensitively rather than pinning one subject's
 * literal casing onto the other.
 */

import { test, expect, type Page, type Locator } from "@playwright/test";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { BASE_URL } from "../../base-url";
import { expectNoAxeViolations } from "../../axe";

const REFERENCE_ROOT = path.resolve(__dirname, "../reference/7e4034b/content/patterns");
const referenceUrl = pathToFileURL(
  path.join(REFERENCE_ROOT, "carousel/examples/carousel-1-prev-next.html"),
).href;

const LIBRARY_URL = `${BASE_URL}/component/?name=carousel&variant=main&`;
const LIBRARY_GOTO = { timeout: 20 * 60 * 1000 };

/**
 * The Library subject's `main` variant, scoped the same way
 * `oracle/tier3-radix/rtl.spec.ts` scopes every "Normal"-kind component's
 * variants (see that file's own header doc): every variant renders on
 * the same page at once, under `#component-preview-frame` for `main`.
 */
function libraryFrame(page: Page): Locator {
  return page.locator("#component-preview-frame");
}

/** True if `a` precedes `b` in document order (Tab order, absent any
 * positive `tabindex` -- neither subject uses one). */
async function precedes(a: Locator, b: Locator): Promise<boolean> {
  const bHandle = await b.elementHandle();
  return a.evaluate(
    (elA, elB) => !!(elA.compareDocumentPosition(elB as Node) & Node.DOCUMENT_POSITION_FOLLOWING),
    bHandle,
  );
}

test.describe("Reference: W3C's own Carousel example (carousel-1-prev-next.html)", () => {
  test("R1: the carousel region has an accessible name, aria-roledescription=carousel, and the name does not contain the word \"carousel\"", async ({ page }) => {
    // Forces the reference's own autoplay off at init (`carousel-prev-next.js`'s
    // own `hasReducedMotion` check, dev-docs/research/carousel-2026-09-19.md
    // §1.3 point 4) -- without this, the reference silently rotates away
    // from slide 1 while these assertions run, since it autoplays by
    // default. Confirmed by execution: every test in this describe block
    // that asserts "the current slide is exactly N" was flaky/red without
    // this, stable with it.
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.goto(referenceUrl);
    const region = page.getByRole("region", { name: "Highlighted television shows" });
    await expect(region).toHaveAttribute("aria-roledescription", "carousel");
    const name = await region.evaluate((el) => el.getAttribute("aria-label") ?? "");
    expect(name.length).toBeGreaterThan(0);
    expect(name.toLowerCase()).not.toContain("carousel");
  });

  test('R2: each slide has role=group, aria-roledescription=slide, and an "N of M" accessible name', async ({ page }) => {
    // Forces the reference's own autoplay off at init (`carousel-prev-next.js`'s
    // own `hasReducedMotion` check, dev-docs/research/carousel-2026-09-19.md
    // §1.3 point 4) -- without this, the reference silently rotates away
    // from slide 1 while these assertions run, since it autoplays by
    // default. Confirmed by execution: every test in this describe block
    // that asserts "the current slide is exactly N" was flaky/red without
    // this, stable with it.
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.goto(referenceUrl);
    // Six slides in this fixture -- 1 of 6 .. 6 of 6. The reference shows
    // one slide at a time (a show/hide `.active` toggle, not a scrolling
    // strip -- dev-docs/research/carousel-2026-09-19.md §2.1's own
    // finding), so the five non-active slides are not visible/ARIA-tree
    // -reachable via `getByRole` even with reduced motion holding the
    // *active* one at slide 1 -- confirmed by execution (R2 timed out
    // past n=1 before this fix). Asserted directly against each element
    // (by raw role+roledescription selector, not `getByRole`'s own
    // visibility-aware matching) instead, which holds regardless of which
    // one is currently active.
    const slides = page.locator('[role="group"][aria-roledescription="slide"]');
    await expect(slides).toHaveCount(6);
    for (let n = 1; n <= 6; n++) {
      await expect(slides.nth(n - 1)).toHaveAttribute("aria-label", `${n} of 6`);
    }
  });

  test("R3: Previous/Next are native buttons with a real accessible name, Enter/Space page the carousel, and activating them never moves focus", async ({ page }) => {
    // Forces the reference's own autoplay off at init (`carousel-prev-next.js`'s
    // own `hasReducedMotion` check, dev-docs/research/carousel-2026-09-19.md
    // §1.3 point 4) -- without this, the reference silently rotates away
    // from slide 1 while these assertions run, since it autoplays by
    // default. Confirmed by execution: every test in this describe block
    // that asserts "the current slide is exactly N" was flaky/red without
    // this, stable with it.
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.goto(referenceUrl);
    const previous = page.getByRole("button", { name: /previous/i });
    const next = page.getByRole("button", { name: /next/i });
    await expect(previous).toHaveJSProperty("tagName", "BUTTON");
    await expect(next).toHaveJSProperty("tagName", "BUTTON");

    await expect(page.getByRole("group", { name: "1 of 6" })).toBeVisible();

    await next.focus();
    await page.keyboard.press("Enter");
    await expect(page.getByRole("group", { name: "2 of 6" })).toBeVisible();
    await expect(next).toBeFocused();

    await page.keyboard.press("Space");
    await expect(page.getByRole("group", { name: "3 of 6" })).toBeVisible();
    await expect(next).toBeFocused();
  });

  test("R4: Previous and Next both precede the slide content in the Tab (document) order", async ({ page }) => {
    // Forces the reference's own autoplay off at init (`carousel-prev-next.js`'s
    // own `hasReducedMotion` check, dev-docs/research/carousel-2026-09-19.md
    // §1.3 point 4) -- without this, the reference silently rotates away
    // from slide 1 while these assertions run, since it autoplays by
    // default. Confirmed by execution: every test in this describe block
    // that asserts "the current slide is exactly N" was flaky/red without
    // this, stable with it.
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.goto(referenceUrl);
    const previous = page.getByRole("button", { name: /previous/i });
    const next = page.getByRole("button", { name: /next/i });
    const firstSlide = page.getByRole("group", { name: "1 of 6" });

    expect(await precedes(previous, next)).toBe(true);
    expect(await precedes(next, firstSlide)).toBe(true);
  });

  test("axe: the reference example itself has no automatically detectable a11y issues", async ({ page }) => {
    // Forces the reference's own autoplay off at init (`carousel-prev-next.js`'s
    // own `hasReducedMotion` check, dev-docs/research/carousel-2026-09-19.md
    // §1.3 point 4) -- without this, the reference silently rotates away
    // from slide 1 while these assertions run, since it autoplays by
    // default. Confirmed by execution: every test in this describe block
    // that asserts "the current slide is exactly N" was flaky/red without
    // this, stable with it.
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.goto(referenceUrl);
    // Scoped to `#ex1` (the carousel widget itself, per this file's own
    // `libraryFrame`-style convention of scanning only the subject under
    // test): the unscoped page also contains this reference page's own
    // documentation, including a `<pre><code id="sc1">` source-code sample
    // further down that trips axe's `scrollable-region-focusable` rule --
    // a pre-existing defect in the vendored W3C reference's own doc
    // chrome, not in the carousel pattern this file calibrates against,
    // and not something this repo edits (dev-docs/backlog.md row 101).
    await expectNoAxeViolations(page, "carousel reference (carousel-1-prev-next.html)", {
      include: "#ex1",
    });
  });
});

test.describe("Library: our own carousel primitive (main variant)", () => {
  test("R1: the carousel region has an accessible name, aria-roledescription=carousel, and the name does not contain the word \"carousel\"", async ({ page }) => {
    await page.goto(LIBRARY_URL, LIBRARY_GOTO);
    const frame = libraryFrame(page);
    const region = frame.getByRole("region");
    await expect(region).toHaveAttribute("aria-roledescription", "carousel");
    const name = await region.evaluate((el) => el.getAttribute("aria-label") ?? "");
    expect(name.length).toBeGreaterThan(0);
    expect(name.toLowerCase()).not.toContain("carousel");
  });

  test('R2: each slide has role=group, aria-roledescription=slide, and an "N of M" accessible name', async ({ page }) => {
    await page.goto(LIBRARY_URL, LIBRARY_GOTO);
    const frame = libraryFrame(page);
    // The `main` variant's own demo (preview/src/components/carousel/
    // variants/main/mod.rs) renders 5 slides.
    await expect(frame.locator('[role="group"][aria-roledescription="slide"]')).toHaveCount(5);
    for (let n = 1; n <= 5; n++) {
      await expect(frame.getByRole("group", { name: `${n} of 5` })).toHaveAttribute(
        "aria-roledescription",
        "slide",
      );
    }
  });

  test("R3: Previous/Next are native buttons with a real accessible name, Enter/Space page the carousel, and activating them never moves focus", async ({ page }) => {
    await page.goto(LIBRARY_URL, LIBRARY_GOTO);
    const frame = libraryFrame(page);
    const previous = frame.getByRole("button", { name: /previous/i });
    const next = frame.getByRole("button", { name: /next/i });
    await expect(previous).toHaveJSProperty("tagName", "BUTTON");
    await expect(next).toHaveJSProperty("tagName", "BUTTON");

    await expect(frame.getByRole("group", { name: "1 of 5" })).toBeVisible();

    await next.focus();
    await page.keyboard.press("Enter");
    await expect(frame.getByRole("group", { name: "2 of 5" })).toHaveAttribute("data-selected", "true");
    await expect(next).toBeFocused();

    await page.keyboard.press("Space");
    await expect(frame.getByRole("group", { name: "3 of 5" })).toHaveAttribute("data-selected", "true");
    await expect(next).toBeFocused();
  });

  test("R4: Previous and Next both precede the slide content in the Tab (document) order", async ({ page }) => {
    await page.goto(LIBRARY_URL, LIBRARY_GOTO);
    const frame = libraryFrame(page);
    const previous = frame.getByRole("button", { name: /previous/i });
    const next = frame.getByRole("button", { name: /next/i });
    const firstSlide = frame.getByRole("group", { name: "1 of 5" });

    expect(await precedes(previous, next)).toBe(true);
    expect(await precedes(next, firstSlide)).toBe(true);
  });

  test("axe: the library's carousel demo has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto(LIBRARY_URL, LIBRARY_GOTO);
    await expectNoAxeViolations(page, "carousel: library main variant", {
      include: "#component-preview-frame",
    });
  });
});

test.describe("Library-only: v1 has no loop, so Previous/Next reach a real, disabled boundary", () => {
  // Not compared against the reference (see this file's header doc,
  // "WHAT THIS FILE DELIBERATELY DOES NOT COVER") -- the reference loops
  // and never disables either button, so there is nothing to calibrate
  // this specific behavior against.
  test("Previous is disabled at the first slide; Next becomes disabled at the last and Previous re-enables away from the first", async ({ page }) => {
    await page.goto(LIBRARY_URL, LIBRARY_GOTO);
    const frame = libraryFrame(page);
    const previous = frame.getByRole("button", { name: /previous/i });
    const next = frame.getByRole("button", { name: /next/i });

    await expect(previous).toBeDisabled();
    await expect(next).toBeEnabled();

    for (let i = 0; i < 4; i++) {
      await next.click();
    }
    await expect(next).toBeDisabled();
    await expect(previous).toBeEnabled();
  });
});
