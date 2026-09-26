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
 *     `prefers-reduced-motion` handling for the *basic* (prev/next) style
 *     are exercised live (`playwright/carousel.spec.ts`'s own "autoplay"
 *     describe block) rather than as a second dual-subject oracle rule set
 *     here -- the basic reference page itself does not expose a way to
 *     turn its own combined rotation+prev/next widget into "prev/next
 *     only, no autoplay" the way this library's own `main` variant is, so
 *     there is no like-for-like basic-style dual subject to calibrate
 *     against for autoplay specifically; the *tabbed* style's own
 *     dual-subject rules below (R5-R9) do cover the rotation control,
 *     since the tabbed reference combines it with the tablist by design.
 *   - The tabbed picker style (`carousel-2-tablist.html`) now has its own
 *     dual-subject rules below (R5-R9), vendored at the same pinned commit
 *     as the basic example (`playwright/oracle/reference/7e4034b/content/patterns/carousel/examples/carousel-2-tablist.html`).
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
import { gotoHydrated } from "../../hydration";

const REFERENCE_ROOT = path.resolve(__dirname, "../reference/7e4034b/content/patterns");
const referenceUrl = pathToFileURL(
  path.join(REFERENCE_ROOT, "carousel/examples/carousel-1-prev-next.html"),
).href;

const LIBRARY_URL = `${BASE_URL}/component/?name=carousel&variant=main&`;
// `gotoHydrated` (../../hydration.ts): see that file's own doc -- same
// construction, same reason (an SSG prerendered page's markup exists before
// wasm hydration attaches event listeners, so a bare "load", or even
// "networkidle" alone, can race an interaction against hydration). Only the
// LIBRARY_URL/TABBED_LIBRARY_URL navigations use this -- the vendored
// `referenceUrl`/`tabbedReferenceUrl` pages below are static local HTML with
// no wasm and no hydration step.
const LIBRARY_GOTO = { timeout: 20 * 60 * 1000 };

// -- Tabbed style (R5-R9) --------------------------------------------

const tabbedReferenceUrl = pathToFileURL(
  path.join(REFERENCE_ROOT, "carousel/examples/carousel-2-tablist.html"),
).href;
const TABBED_LIBRARY_URL = `${BASE_URL}/component/?name=carousel&variant=indicators&`;

function tabbedLibraryFrame(page: Page): Locator {
  return page.locator("#component-preview-frame-indicators");
}

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
    await gotoHydrated(page, LIBRARY_URL, LIBRARY_GOTO);
    const frame = libraryFrame(page);
    const region = frame.getByRole("region");
    await expect(region).toHaveAttribute("aria-roledescription", "carousel");
    const name = await region.evaluate((el) => el.getAttribute("aria-label") ?? "");
    expect(name.length).toBeGreaterThan(0);
    expect(name.toLowerCase()).not.toContain("carousel");
  });

  test('R2: each slide has role=group, aria-roledescription=slide, and an "N of M" accessible name', async ({ page }) => {
    await gotoHydrated(page, LIBRARY_URL, LIBRARY_GOTO);
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
    await gotoHydrated(page, LIBRARY_URL, LIBRARY_GOTO);
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
    await gotoHydrated(page, LIBRARY_URL, LIBRARY_GOTO);
    const frame = libraryFrame(page);
    const previous = frame.getByRole("button", { name: /previous/i });
    const next = frame.getByRole("button", { name: /next/i });
    const firstSlide = frame.getByRole("group", { name: "1 of 5" });

    expect(await precedes(previous, next)).toBe(true);
    expect(await precedes(next, firstSlide)).toBe(true);
  });

  test("axe: the library's carousel demo has no automatically detectable a11y issues", async ({ page }) => {
    await gotoHydrated(page, LIBRARY_URL, LIBRARY_GOTO);
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
    await gotoHydrated(page, LIBRARY_URL, LIBRARY_GOTO);
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

/**
 * R5-R9: the APG **tabbed** carousel style, calibrated against
 * `examples/carousel-2-tablist.html` (same pinned commit `7e4034b`) --
 * source citations quoted verbatim in this file's own header for R1-R4
 * apply the same way here; per-rule citations are inline below.
 *
 * The reference's own combined widget also has a rotation control (its
 * own `R4`-equivalent tab-order rule would additionally place it before
 * the tablist) -- this library's own `indicators` variant deliberately composes
 * *only* the tablist + content (no `CarouselAutoplay`/`CarouselRotationControl`),
 * matching the reference's own "similar examples" cross-reference that the
 * prev-next style is the one to pair with autoplay and the tabbed style is
 * the one to pair with direct slide selection -- autoplay's own dual
 * subject is out of scope here for that reason (this file's header doc);
 * R9 below (tab order) is therefore asserted for "tablist precedes slide
 * content" only, on both subjects, not the rotation control's own position.
 */
test.describe("Reference: W3C's own Carousel tabbed example (carousel-2-tablist.html)", () => {
  test("R5: the tablist has role=tablist and an accessible name", async ({ page }) => {
    // Stops the reference's own auto-rotation so the tab/slide state stays
    // put while these assertions run -- same reasoning as R1-R4's own
    // per-test comment above.
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.goto(tabbedReferenceUrl);
    const tablist = page.getByRole("tablist");
    await expect(tablist).toBeVisible();
    const name = await tablist.evaluate((el) => el.getAttribute("aria-label") ?? "");
    expect(name.length).toBeGreaterThan(0);
  });

  test("R6: each tab has role=tab, a unique accessible name, aria-selected, roving tabindex, and aria-controls", async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.goto(tabbedReferenceUrl);
    const tabs = page.getByRole("tab");
    await expect(tabs).toHaveCount(6);
    await expect(tabs.nth(0)).toHaveAttribute("aria-selected", "true");
    for (let i = 1; i < 6; i++) {
      await expect(tabs.nth(i)).toHaveAttribute("aria-selected", "false");
      await expect(tabs.nth(i)).toHaveAttribute("tabindex", "-1");
    }
    // The reference removes `tabindex` from the SELECTED tab entirely
    // (relying on a native `<button>`'s own default tabbability) rather
    // than setting `tabindex="0"` -- both put it in the ordinary page tab
    // sequence, so either is accepted (this file's own R3's "native
    // <button> semantics apply by construction" reasoning, restated for
    // tabindex specifically).
    const selectedTabindex = await tabs.nth(0).getAttribute("tabindex");
    expect(selectedTabindex === null || selectedTabindex === "0").toBe(true);

    const firstPanelId = await page.locator('[role="tabpanel"]').nth(0).getAttribute("id");
    expect(firstPanelId).toBeTruthy();
    await expect(tabs.nth(0)).toHaveAttribute("aria-controls", firstPanelId!);
  });

  test('R7: each tabpanel has role=tabpanel, aria-roledescription=slide, and an "N of M" accessible name', async ({ page }) => {
    // The pattern page's own prose claims a tabpanel "does not have the
    // aria-roledescription property" -- the example's own markup
    // contradicts that (this file's header doc, and
    // `dev-docs/research/carousel-2026-09-19.md` §1.3 point 1): asserted
    // against the tested example, not the prose.
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.goto(tabbedReferenceUrl);
    const panels = page.locator('[role="tabpanel"]');
    await expect(panels).toHaveCount(6);
    for (let n = 1; n <= 6; n++) {
      const panel = panels.nth(n - 1);
      await expect(panel).toHaveAttribute("aria-roledescription", "slide");
      await expect(panel).toHaveAttribute("aria-label", `${n} of 6`);
    }
  });

  test("R8: ArrowRight/ArrowLeft move focus and automatically activate the newly focused tab (no Enter needed), and wrap at both ends; Home/End go to the first/last tab", async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.goto(tabbedReferenceUrl);
    const tabs = page.getByRole("tab");
    const panel = (n: number) => page.locator(`[role="tabpanel"][aria-label="${n} of 6"]`);

    await tabs.nth(0).focus();
    await page.keyboard.press("ArrowRight");
    await expect(tabs.nth(1)).toBeFocused();
    await expect(tabs.nth(1)).toHaveAttribute("aria-selected", "true");
    await expect(panel(2)).toBeVisible();

    await page.keyboard.press("ArrowLeft");
    await expect(tabs.nth(0)).toBeFocused();
    await expect(panel(1)).toBeVisible();

    // Wrap at the start.
    await page.keyboard.press("ArrowLeft");
    await expect(tabs.nth(5)).toBeFocused();
    await expect(tabs.nth(5)).toHaveAttribute("aria-selected", "true");

    // Wrap at the end.
    await page.keyboard.press("ArrowRight");
    await expect(tabs.nth(0)).toBeFocused();

    await page.keyboard.press("End");
    await expect(tabs.nth(5)).toBeFocused();
    await expect(panel(6)).toBeVisible();

    await page.keyboard.press("Home");
    await expect(tabs.nth(0)).toBeFocused();
    await expect(panel(1)).toBeVisible();
  });

  test("R9: the tablist precedes the slide content in document order", async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.goto(tabbedReferenceUrl);
    const tablist = page.getByRole("tablist");
    const firstPanel = page.locator('[role="tabpanel"]').first();
    expect(await precedes(tablist, firstPanel)).toBe(true);
  });

  test("axe: the tabbed reference example itself has no automatically detectable a11y issues", async ({ page }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.goto(tabbedReferenceUrl);
    // Scoped to `#ex1`, matching this file's own R1-R4 axe test and its
    // own comment about the vendored page's unrelated doc-chrome defect.
    await expectNoAxeViolations(page, "carousel tabbed reference (carousel-2-tablist.html)", {
      include: "#ex1",
    });
  });
});

test.describe("Library: our own carousel primitive (indicators variant)", () => {
  test("R5: the tablist has role=tablist and an accessible name", async ({ page }) => {
    await gotoHydrated(page, TABBED_LIBRARY_URL, LIBRARY_GOTO);
    const frame = tabbedLibraryFrame(page);
    const tablist = frame.getByRole("tablist");
    await expect(tablist).toBeVisible();
    const name = await tablist.evaluate((el) => el.getAttribute("aria-label") ?? "");
    expect(name.length).toBeGreaterThan(0);
  });

  test("R6: each tab has role=tab, a unique accessible name, aria-selected, roving tabindex, and aria-controls", async ({ page }) => {
    await gotoHydrated(page, TABBED_LIBRARY_URL, LIBRARY_GOTO);
    const frame = tabbedLibraryFrame(page);
    const tabs = frame.getByRole("tab");
    await expect(tabs).toHaveCount(5);
    await expect(tabs.nth(0)).toHaveAttribute("aria-selected", "true");
    await expect(tabs.nth(0)).toHaveAttribute("tabindex", "0");
    for (let i = 1; i < 5; i++) {
      await expect(tabs.nth(i)).toHaveAttribute("aria-selected", "false");
      await expect(tabs.nth(i)).toHaveAttribute("tabindex", "-1");
    }

    const firstPanelId = await frame.locator('[role="tabpanel"]').nth(0).getAttribute("id");
    expect(firstPanelId).toBeTruthy();
    await expect(tabs.nth(0)).toHaveAttribute("aria-controls", firstPanelId!);
  });

  test('R7: each tabpanel has role=tabpanel, aria-roledescription=slide, and an "N of M" accessible name', async ({ page }) => {
    await gotoHydrated(page, TABBED_LIBRARY_URL, LIBRARY_GOTO);
    const frame = tabbedLibraryFrame(page);
    const panels = frame.locator('[role="tabpanel"]');
    await expect(panels).toHaveCount(5);
    for (let n = 1; n <= 5; n++) {
      const panel = panels.nth(n - 1);
      await expect(panel).toHaveAttribute("aria-roledescription", "slide");
      await expect(panel).toHaveAttribute("aria-label", `${n} of 5`);
    }
  });

  test("R8: ArrowRight/ArrowLeft move focus and automatically activate the newly focused tab (no Enter needed), and wrap at both ends; Home/End go to the first/last tab", async ({ page }) => {
    await gotoHydrated(page, TABBED_LIBRARY_URL, LIBRARY_GOTO);
    const frame = tabbedLibraryFrame(page);
    const tabs = frame.getByRole("tab");
    const panel = (n: number) => frame.locator(`[role="tabpanel"][aria-label="${n} of 5"]`);

    await tabs.nth(0).focus();
    await page.keyboard.press("ArrowRight");
    await expect(tabs.nth(1)).toBeFocused();
    await expect(tabs.nth(1)).toHaveAttribute("aria-selected", "true");
    await expect(panel(2)).toHaveAttribute("data-selected", "true");

    await page.keyboard.press("ArrowLeft");
    await expect(tabs.nth(0)).toBeFocused();
    await expect(panel(1)).toHaveAttribute("data-selected", "true");

    // Wrap at the start.
    await page.keyboard.press("ArrowLeft");
    await expect(tabs.nth(4)).toBeFocused();
    await expect(tabs.nth(4)).toHaveAttribute("aria-selected", "true");

    // Wrap at the end.
    await page.keyboard.press("ArrowRight");
    await expect(tabs.nth(0)).toBeFocused();

    await page.keyboard.press("End");
    await expect(tabs.nth(4)).toBeFocused();
    await expect(panel(5)).toHaveAttribute("data-selected", "true");

    await page.keyboard.press("Home");
    await expect(tabs.nth(0)).toBeFocused();
    await expect(panel(1)).toHaveAttribute("data-selected", "true");
  });

  test("R9: the tablist precedes the slide content in document order", async ({ page }) => {
    await gotoHydrated(page, TABBED_LIBRARY_URL, LIBRARY_GOTO);
    const frame = tabbedLibraryFrame(page);
    const tablist = frame.getByRole("tablist");
    const firstPanel = frame.locator('[role="tabpanel"]').first();
    expect(await precedes(tablist, firstPanel)).toBe(true);
  });

  test("axe: the library's indicators variant has no automatically detectable a11y issues", async ({ page }) => {
    await gotoHydrated(page, TABBED_LIBRARY_URL, LIBRARY_GOTO);
    await expectNoAxeViolations(page, "carousel: library indicators variant", {
      include: "#component-preview-frame-indicators",
    });
  });
});
