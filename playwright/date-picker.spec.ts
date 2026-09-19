/**
 * DatePicker — calendar popover anchoring (live-site report, item 3,
 * 2026-09-01) and a minimal smoke suite. No date-picker spec existed
 * before this file (docs/backlog.md row 20) -- this closes part of that
 * backlog row (an e2e spec exists now); the rest of row 20's scope
 * (segment typing/arrows/backspace, range picker) is not covered here and
 * remains open, see this session's report.
 *
 * Root cause of the live-site report ("the calendar doesn't anchor to its
 * trigger"), confirmed by execution against this repo's dev server:
 *   1. `date_picker/style.css` never carried its own copy of the
 *      `@supports (anchor-name: --a)` block every other non-modal-arm
 *      consumer in this workspace has (grep turned up zero matches) --
 *      the same shape of gap fixed for `color_picker/style.css`.
 *   2. That alone was not sufficient: `date_picker::DatePickerPopover`
 *      (`primitives/src/date_picker.rs`) rendered its calendar on the
 *      *modal* popover arm unconditionally -- its own `is_modal` prop was
 *      declared but never forwarded to the `PopoverRoot` it renders. The
 *      modal arm's DOM-relative "centering trick" positions the popup
 *      under its *positioned ancestor* (the narrow date-input group), not
 *      its trigger, with no edge/collision avoidance -- confirmed by
 *      execution, a 276px-wide calendar centered under a ~178px-wide
 *      ancestor rendered ~138px off the left edge of the viewport on this
 *      repo's dev server, which is the actual shape of "doesn't anchor to
 *      its trigger."
 * Fixed by forwarding `is_modal` in the primitive, setting
 * `is_modal: false` on `preview/src/components/date_picker/component.rs`'s
 * `DatePickerPopover` calls (matching `ColorPickerPopover`'s own
 * `is_modal: false`), and adding the `@supports` block to this page's
 * `style.css` -- see each fix's own comment for the full account.
 */

import { test, expect, type Page } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";
import { BASE_URL } from "./base-url";

const NAV_TIMEOUT = 20 * 60 * 1000; // first run compiles the app
const PAGE_URL = `${BASE_URL}/component/?name=date_picker&`;

async function gotoDatePicker(page: Page) {
  await page.goto(PAGE_URL, { timeout: NAV_TIMEOUT, waitUntil: "networkidle" });
}

function trigger(page: Page) {
  return page.getByRole("button", { name: "Show Calendar" }).first();
}

function content(page: Page) {
  return page.getByRole("dialog");
}

async function rectOf(page: Page, locator: ReturnType<typeof trigger>) {
  return locator.evaluate((el) => {
    const r = (el as HTMLElement).getBoundingClientRect();
    return { top: r.top, left: r.left, right: r.right, bottom: r.bottom, width: r.width, height: r.height };
  });
}

test("opens the calendar popover on trigger click", async ({ page }) => {
  await gotoDatePicker(page);
  await expect(content(page)).toBeHidden();

  await trigger(page).click();

  await expect(content(page)).toBeVisible();
});

test("anchors the popup next to its trigger, not viewport-centered or offset", async ({ page }) => {
  await gotoDatePicker(page);
  await trigger(page).click();
  await expect(content(page)).toBeVisible();
  // Let the open-state fade-in settle (`dx-date-picker-popover-fade-in`,
  // `.15s ease-out`) before reading geometry -- same reasoning as
  // top-layer.spec.ts Rule 8's ColorPicker case: mid-animation reads can
  // catch a not-yet-settled box, unrelated to anchoring but noisy for it.
  await page.waitForTimeout(200);

  const t = await rectOf(page, trigger(page));
  const c = await rectOf(page, content(page));
  const viewport = await page.evaluate(() => ({ width: window.innerWidth, height: window.innerHeight }));
  const debug = JSON.stringify({ trigger: t, content: c, viewport });

  // Below the trigger (this fixture's default side is "bottom"), not
  // centered in the viewport -- the pre-fix modal arm rendered far enough
  // left of the trigger to spill off the viewport edge entirely, so
  // "roughly centered on the trigger's own horizontal position" is the
  // discriminating assertion, not merely "somewhere on screen."
  expect(c.top, debug).toBeGreaterThan(t.bottom);
  const triggerCenterX = (t.left + t.right) / 2;
  const contentCenterX = (c.left + c.right) / 2;
  expect(Math.abs(contentCenterX - triggerCenterX), debug).toBeLessThan(2);
  // And not the viewport-centered placement the broken modal arm fell
  // back to for content wider than its positioning ancestor: the
  // viewport's own horizontal center would coincide with the trigger's
  // only by accident on this fixture's layout, so assert against it
  // directly using a page-specific fixed offset would be fragile --
  // instead this is covered by the anchor-to-trigger assertion above,
  // which a viewport-centered box could only satisfy if the trigger
  // itself happened to sit at viewport center (it does not, on this
  // fixture's layout, confirmed by execution: trigger center vs. viewport
  // center differ by well over the 2px tolerance used above).
  expect(Math.abs(triggerCenterX - viewport.width / 2)).toBeGreaterThan(20);
});

test("uses the CSS-anchor path, not the JS-measured fallback (no inline top/left)", async ({ page }) => {
  await gotoDatePicker(page);
  // A raw DOM click (bypassing Playwright's own actionability auto-scroll)
  // deliberately keeps `window.scrollY` at 0 for this open: confirmed by
  // execution (this session's diagnosis) that this repo's Chromium build
  // computes a `[popover]` + `position: fixed` element's *very first*
  // `anchor()`-resolved position incorrectly whenever the document is
  // already scrolled at the moment the popover is shown -- off by exactly
  // the scroll offset, as if measured document-relative instead of
  // viewport-relative -- and that wrong value never self-corrects for the
  // life of that popover instance, even once real further scrolling
  // happens. `use_anchor_position_fallback`'s own fallback correctly
  // detects and compensates for this (this repo's actual, existing
  // protection against exactly this failure mode -- see its doc), so
  // end-user positioning is never wrong; this test's whole point, though,
  // is to isolate the *CSS-only* path specifically, which this Chromium
  // quirk can only be kept out of by not scrolling before the open it is
  // asserting about. `trigger(page).click()` here would implicitly
  // scroll the (below-the-fold) trigger into view first and trip this
  // every time.
  await trigger(page).evaluate((el) => (el as HTMLElement).click());
  await expect(content(page)).toBeVisible();

  // `use_anchor_position_fallback` (primitives/src/top_layer.rs) is the
  // only thing that ever writes an inline `top` on this element -- an
  // empty style here confirms the CSS-native `anchor()` path engaged
  // first, the same signal top-layer.spec.ts Rule 8 checks for its
  // ColorPicker case.
  const inlineTop = await content(page).evaluate((el) => (el as HTMLElement).style.top);
  expect(inlineTop).toBe("");
  const marker = await content(page).evaluate((el) => el.className.includes("dx-anchor-popover"));
  expect(marker).toBe(true);
});

test("offset to trigger is unchanged after scrolling (CSS anchor tracks scroll natively)", async ({ page }) => {
  await gotoDatePicker(page);
  // Raw DOM click, scrollY kept at 0 for the open -- see the identical
  // note on the "uses the CSS-anchor path" test above for why: this
  // repo's Chromium miscomputes the very first `anchor()`-resolved
  // position whenever the page is already scrolled at open time, which
  // would put this test on the (already-covered, and already correct)
  // fallback path instead of the CSS-native one this test exists to
  // isolate. The test's own scroll -- what is actually under test here --
  // still happens for real, afterward.
  await trigger(page).evaluate((el) => (el as HTMLElement).click());
  await expect(content(page)).toBeVisible();
  await page.waitForTimeout(200);

  const offsetOf = async () =>
    page.evaluate(() => {
      const c = document.querySelector('[class*="dx-anchor-popover"]')!.getBoundingClientRect();
      const t = document.querySelector('[style*="anchor-name"]')!.getBoundingClientRect();
      return { top: c.top - t.bottom, left: c.left - t.left };
    });

  const before = await offsetOf();
  await page.evaluate(() => window.scrollBy(0, 150));
  // No JS fallback is active on this path (CSS anchor positioning
  // recomputes natively on scroll), but a short wait keeps this robust
  // against any incidental reflow/paint delay, same tolerance top-layer.
  // spec.ts's Rule 8 uses for its own scroll-tracking cases.
  await page.waitForTimeout(150);
  const after = await offsetOf();

  const debug = JSON.stringify({ before, after });
  expect(after.top, debug).toBeCloseTo(before.top, 0);
  expect(after.left, debug).toBeCloseTo(before.left, 0);
});

test("Escape closes the popup", async ({ page }) => {
  await gotoDatePicker(page);
  await trigger(page).click();
  await expect(content(page)).toBeVisible();

  await page.keyboard.press("Escape");

  await expect(content(page)).toBeHidden();
});

test.describe("Axe automated scan", () => {
  test("loaded (popover closed) has no automatically detectable a11y issues", async ({ page }) => {
    await gotoDatePicker(page);
    await expectNoAxeViolations(page, "date-picker: loaded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });

  test("calendar popover open has no automatically detectable a11y issues", async ({ page }) => {
    await gotoDatePicker(page);
    await trigger(page).click();
    await expect(content(page)).toBeVisible();
    await expectNoAxeViolations(page, "date-picker: calendar popover open", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});

/**
 * -----------------------------------------------------------------------
 * Everything below closes the rest of `dev-docs/backlog.md` row 20: segment
 * typing, arrow keys, backspace, and the range picker were "still only
 * verified ad hoc" -- this is the deliberate, source- and APG-derived
 * coverage for that. Row 21 (the two anchoring tests above failing in this
 * sandbox -- a Chromium anchor-position engine freeze after scroll,
 * environment-only) is untouched: nothing below weakens, skips, or
 * resembles those two tests.
 *
 * Reference basis (derive-then-cite, not "assert whatever the crate does"):
 * `primitives/src/date_picker.rs`'s `DateSegment` renders `role="spinbutton"`
 * with `aria-valuemin`/`aria-valuemax`/`aria-valuenow` and a keydown handler
 * (`date_segment_key_effects`) -- the closest APG pattern is
 * `content/patterns/spinbutton/spinbutton-pattern.html` (the *current*,
 * non-deprecated pattern doc -- not its own linked example,
 * `examples/datepicker-spinbuttons.html`, which is itself marked
 * "(Deprecated)" by APG and predates that pattern doc's Home/End rule), in
 * the pinned APG checkout `$S/aria-practices` (commit 7e4034b, same commit
 * `playwright/oracle/reference/README.md` already cites -- not vendored
 * under `playwright/oracle/reference/` by this lane, since nothing here
 * needed the page rendered, only its text). Moving focus *among* several
 * spinbuttons with Left/Right (this crate's `ArrowLeft`/`ArrowRight`) is
 * outside the lone `spinbutton` role's own contract; the closest named APG
 * convention is `content/patterns/toolbar/toolbar-pattern.html`'s own text
 * ("Up Arrow and Down Arrow ... can be reserved for operating controls,
 * such as spin buttons that require vertical arrow keys to operate" while
 * "Left Arrow and Right Arrow navigate among controls") -- cited per-test
 * below, not asserted as if `spinbutton-pattern.html` itself required it.
 *
 * Two real, minimal defects were found deriving this and are fixed in their
 * own commits (see this lane's report for the red-before/green-after
 * evidence): (1) `Home`/`End` were unhandled on every segment, contradicting
 * `spinbutton-pattern.html`'s Keyboard Interaction list (Home/End are not
 * marked "(Optional)" there, unlike Page Up/Down); (2) every segment
 * rendered a dangling `aria-labelledby` referencing an id nothing ever
 * rendered, contradicting that same pattern's own either/or labelling rule.
 * Several other gaps against the cited text were found and are deliberately
 * NOT fixed here (documented in the tests that touch them, and in the
 * report): no `aria-valuetext` on the month segment (a bare 1-12 number,
 * arguably "not user-friendly" per the pattern's own example), and
 * `aria-valuenow` reporting today's date rather than being omitted when no
 * date has been chosen yet.
 */

/**
 * Every variant of this ("Normal"-type) component renders together on the
 * SAME page -- confirmed by reading `preview/src/main.rs`'s
 * `ComponentVariantHighlight` (the "Variants" section loops over every
 * variant unconditionally) and `preview/src/components/mod.rs`'s
 * `component_page!` macro (`name: stringify!($variant)`) -- there is no
 * `?variant=` query-string switch for this component's route (unlike, e.g.,
 * `select`/`combobox`/`popover`, confirmed live: `variant=range` and
 * `variant=main` render byte-identical pages here). Each demo is scoped by
 * `#component-preview-frame` (the "main" demo) or
 * `#component-preview-frame-<variant>` (`frame_id` in `main.rs`, keyed by
 * the variant's own Rust identifier -- `range`, `internationalized`,
 * `multi_month`, `unavailable_dates`), so every test below scopes into the
 * right one explicitly instead of relying on `.first()`/page-wide role
 * queries the way the smoke tests above do (those only ever touch the
 * "main" instance via one trigger, so `.first()` was sufficient there).
 */
function frame(page: Page, variant?: string) {
  return page.locator(variant ? `#component-preview-frame-${variant}` : "#component-preview-frame");
}

type SegmentName = "year" | "month" | "day";

/** A single date field's segment, scoped to one variant's demo. */
function segment(page: Page, name: SegmentName, variant?: string) {
  return frame(page, variant).getByRole("spinbutton", { name });
}

/** The Nth (0 = start, 1 = end) occurrence of a segment name in a range picker's demo (start/end share no distinguishing wrapper -- see `DateElement`'s own doc -- so DOM order is the only handle). */
function rangeSegment(page: Page, side: "start" | "end", name: SegmentName, variant: string) {
  return frame(page, variant).getByRole("spinbutton", { name }).nth(side === "start" ? 0 : 1);
}

function pickerTrigger(page: Page, variant?: string) {
  return frame(page, variant).getByRole("button", { name: "Show Calendar" });
}

function pickerContent(page: Page, variant?: string) {
  return frame(page, variant).getByRole("dialog");
}

/** A calendar grid cell (real `<button>`) by its visible day number, within the *current* month only -- avoids matching a same-numbered day spilling over from the previous/next month. */
function dayCell(dialogLocator: ReturnType<typeof pickerContent>, day: number) {
  return dialogLocator.locator('.dx-calendar-grid-cell[data-month="current"]').filter({ hasText: new RegExp(`^${day}$`) });
}

/** `aria-label` of whatever currently has real DOM focus, e.g. "year"/"month"/"day"/"Show Calendar". */
async function activeLabel(page: Page) {
  return page.evaluate(() => document.activeElement?.getAttribute("aria-label") ?? null);
}

async function pressEach(page: Page, keys: string[]) {
  for (const key of keys) {
    await page.keyboard.press(key);
  }
}

test.describe("Segment typing (digit entry, spinbutton-pattern.html #keyboard_interaction's 'textbox' technique)", () => {
  test("a single digit that would overflow with one more digit commits immediately and auto-advances focus", async ({ page }) => {
    // Month: min=1, max=12, max_length=2. Typing "9" alone: value=9 is in
    // range, but a probed third-digit overflow check ("90" > 12) already
    // fails after this one keystroke -- `push_commit_text_effects` in
    // `date_picker.rs` advances focus immediately rather than waiting for a
    // second digit. Verified red/green isn't applicable here (this is
    // pre-existing, unchanged behavior) but was verified live before
    // writing this assertion (flipping the expected focus target to "month"
    // fails the test, confirming it actually exercises the advance).
    await gotoDatePicker(page);
    const month = segment(page, "month");
    await month.click();
    await page.keyboard.press("9");

    await expect(month).toHaveText("09");
    await expect(month).toHaveAttribute("aria-valuenow", "9");
    expect(await activeLabel(page)).toBe("day");
  });

  test("filling a 4-digit year advances focus only once the value is both in range and one more digit would overflow", async ({ page }) => {
    // Year: min=1925, max=2050, max_length=4. "1"/"19"/"199" are all below
    // min (in_range is false), so the overflow probe never fires for them;
    // only the 4th digit ("1999", now in range, with "19990" overflowing
    // max) triggers the advance. This is the multi-keystroke counterpart of
    // the single-keystroke month case above -- same function, different
    // segment shape.
    await gotoDatePicker(page);
    const year = segment(page, "year");
    await year.click();
    await pressEach(page, ["1", "9", "9", "9"]);

    await expect(year).toHaveText("1999");
    await expect(year).toHaveAttribute("aria-valuenow", "1999");
    expect(await activeLabel(page)).toBe("month");
  });

  test("a non-digit keystroke is blocked and leaves the segment's value and text unchanged", async ({ page }) => {
    await gotoDatePicker(page);
    const month = segment(page, "month");
    const valueBefore = await month.getAttribute("aria-valuenow");
    const textBefore = await month.textContent();
    await month.click();
    await page.keyboard.press("x");

    await expect(month).toHaveAttribute("aria-valuenow", valueBefore!);
    await expect(month).toHaveText(textBefore!);
    // `Key::Character` still calls `prevent_default`/`stop_propagation` for
    // a non-digit (matches `key_effects_non_digit_character_still_blocks_
    // default_but_does_not_edit` in date_picker.rs's own unit tests) but
    // pushes no focus-moving effect, so focus stays put.
    expect(await activeLabel(page)).toBe("month");
  });

  test("digits that would exceed the maximum are clamped to it; the day segment (no next segment registered) keeps focus rather than losing it", async ({ page }) => {
    // Fresh page, day segment: year/month haven't been touched yet, so
    // `day_bounds_for_year_month(None, None, ...)` falls through to the
    // generic `(1, 31)` bounds (verified live) regardless of the sandbox's
    // real date -- read `aria-valuemax` rather than hardcoding 31, so this
    // stays correct even if that fallback ever changes.
    await gotoDatePicker(page);
    const day = segment(page, "day");
    const max = await day.getAttribute("aria-valuemax");
    await day.click();
    await pressEach(page, ["9", "9"]);

    await expect(day).toHaveText(max!.padStart(2, "0"));
    await expect(day).toHaveAttribute("aria-valuenow", max!);
    // Verified live: `FocusNext` from the day segment (this input's last
    // registered collection item, `roving_loop: false`) resolves to no
    // available index, so `CollectionState::set_focus` clears the internal
    // pointer but nothing ever calls `.blur()` -- real DOM focus is
    // untouched. Not a bug: there is no further item for a real "next" to
    // mean anything, on either side of this composite (see the Tab-order
    // test below for what actually comes next in the page).
    expect(await activeLabel(page)).toBe("day");
  });
});

test.describe("Arrow keys: Up/Down step the value; Home/End jump to the bounds", () => {
  test("ArrowUp/ArrowDown increments/decrements the focused segment by one and updates aria-valuenow live", async ({ page }) => {
    // `date_segment_key_effects`'s `Key::ArrowUp`/`ArrowDown` branch on
    // `current_value` (the segment's own `props.value`, `None` until the
    // user has actually committed something), not on the *displayed*
    // `aria-valuenow` (which already falls back to `default` for display
    // purposes via `now_value`). On a never-touched segment `current_value`
    // is still `None`, so the very first Up/Down press takes the `None =>
    // default` branch -- it commits `default` as the real value rather
    // than stepping from it, which is invisible here specifically because
    // `default` already equals what was displayed. Verified live (this is
    // what this test's own first draft got wrong, caught by the assertion
    // failing red: pressing ArrowDown once on a fresh segment left
    // aria-valuenow unchanged instead of decrementing). `Home` first makes
    // this deterministic and gives `current_value` a real `Some(...)`, so
    // Up/Down genuinely step from there -- matching how a user who has
    // already interacted with the segment experiences it.
    await gotoDatePicker(page);
    const month = segment(page, "month");
    const min = Number(await month.getAttribute("aria-valuemin"));
    const max = Number(await month.getAttribute("aria-valuemax"));
    await month.click();
    await page.keyboard.press("Home");
    const before = min;
    await expect(month).toHaveAttribute("aria-valuenow", String(before));

    await page.keyboard.press("ArrowDown");
    const expectedDown = before > min ? before - 1 : max; // roll_value wrap, see below
    await expect(month).toHaveAttribute("aria-valuenow", String(expectedDown));

    await page.keyboard.press("ArrowUp");
    await expect(month).toHaveAttribute("aria-valuenow", String(before));

    await page.keyboard.press("ArrowUp");
    const expectedUp = before < max ? before + 1 : min;
    await expect(month).toHaveAttribute("aria-valuenow", String(expectedUp));
  });

  test("Home sets the focused segment to its minimum and End sets it to its maximum", async ({ page }) => {
    // `spinbutton-pattern.html#keyboard_interaction`: "Home: If the
    // spinbutton has a minimum value, sets the value to its minimum." /
    // "End: ... sets the value to its maximum." -- neither is marked
    // "(Optional)" there (unlike Page Up/Down, which this crate does not
    // implement and this suite does not require). `aria-valuemin`/
    // `aria-valuemax` are unconditionally present on every segment here, so
    // the rule always applies. FIX REGRESSION GUARD: before this lane's
    // `fix(date-picker)` commit, Home/End fell through `date_segment_key_
    // effects`'s wildcard arm and did nothing at all -- confirmed by
    // execution against the unfixed build (month's aria-valuenow was
    // provably unchanged by either key).
    await gotoDatePicker(page);
    const month = segment(page, "month");
    await month.click();
    await page.keyboard.press("Home");
    await expect(month).toHaveAttribute("aria-valuenow", "1");
    await expect(month).toHaveText("01");
    await page.keyboard.press("End");
    await expect(month).toHaveAttribute("aria-valuenow", "12");
    await expect(month).toHaveText("12");

    const year = segment(page, "year");
    await year.click();
    await page.keyboard.press("Home");
    await expect(year).toHaveAttribute("aria-valuenow", "1925");
    await page.keyboard.press("End");
    await expect(year).toHaveAttribute("aria-valuenow", "2050");
  });

  test("Up wraps from the maximum to the minimum and Down wraps from the minimum to the maximum, including on the year segment", async ({ page }) => {
    // `roll_value` (date_picker.rs) applies the same wrap to every segment
    // type with no year-specific exception -- APG's base spinbutton pattern
    // is silent on wrap-vs-clamp (its own deprecated date example wraps day
    // and month, the only two it discusses, and never mentions year at
    // all), so wrapping a *year* value (2050 -> 1925) is a genuine, cited
    // finding, not a violation -- recorded here as current, deliberate(-ish)
    // behavior rather than asserted as if the crate had no choice.
    await gotoDatePicker(page);
    const month = segment(page, "month");
    await month.click();
    await page.keyboard.press("End");
    await page.keyboard.press("ArrowUp");
    await expect(month).toHaveAttribute("aria-valuenow", "1");
    await page.keyboard.press("Home");
    await page.keyboard.press("ArrowDown");
    await expect(month).toHaveAttribute("aria-valuenow", "12");

    const year = segment(page, "year");
    await year.click();
    await page.keyboard.press("End");
    await page.keyboard.press("ArrowUp");
    await expect(year).toHaveAttribute("aria-valuenow", "1925");
    await page.keyboard.press("Home");
    await page.keyboard.press("ArrowDown");
    await expect(year).toHaveAttribute("aria-valuenow", "2050");
  });
});

test.describe("Left/Right moves focus between segments; Backspace clears and moves back", () => {
  test("ArrowRight moves focus to the next segment and ArrowLeft moves back", async ({ page }) => {
    // Not part of the lone `spinbutton` role's own contract (spinbutton-
    // pattern.html's Keyboard Interaction list has no Left/Right entry) --
    // the closest named APG convention for moving focus *among* several
    // spinbuttons is `toolbar-pattern.html`'s own text: "Up Arrow and Down
    // Arrow ... can be reserved for operating controls, such as spin
    // buttons that require vertical arrow keys to operate" while "Left
    // Arrow ... [m]oves focus to the previous control" / "Right Arrow ...
    // to the next control" -- exactly this crate's split (Up/Down operate
    // the segment, Left/Right move the composite's focus), even though the
    // container here is `role="group"`, not `role="toolbar"`.
    await gotoDatePicker(page);
    await segment(page, "year").click();
    await page.keyboard.press("ArrowRight");
    expect(await activeLabel(page)).toBe("month");
    await page.keyboard.press("ArrowRight");
    expect(await activeLabel(page)).toBe("day");
    await page.keyboard.press("ArrowLeft");
    expect(await activeLabel(page)).toBe("month");
    await page.keyboard.press("ArrowLeft");
    expect(await activeLabel(page)).toBe("year");
  });

  test("Backspace on a one-character segment clears it to the placeholder and moves focus to the previous segment", async ({ page }) => {
    await gotoDatePicker(page);
    const day = segment(page, "day");
    await day.click();
    await page.keyboard.press("1");
    await expect(day).toHaveText("01");
    await expect(day).toHaveAttribute("no-date", "false");

    await page.keyboard.press("Backspace");
    await expect(day).toHaveText("DD");
    await expect(day).toHaveAttribute("no-date", "true");
    expect(await activeLabel(page)).toBe("month");
  });

  test("Ctrl+Backspace clears the whole segment's buffer in one keystroke", async ({ page }) => {
    // Mirrors `key_effects_backspace_with_ctrl_or_meta_clears_all`'s
    // asserted effects (`EmitValue(None)`, `FocusPrevious`) -- both the
    // single- and whole-segment-clear paths route through the same
    // empty-buffer branch in `push_commit_text_effects`, so both always
    // request `FocusPrevious` too. On the YEAR segment specifically (this
    // input's first registered item) that request resolves to no available
    // index -- the same "no further item, DOM focus stays put" case the
    // day segment hit above, just mirrored at the other end -- so focus is
    // expected to stay on year, not move nowhere visibly.
    await gotoDatePicker(page);
    const year = segment(page, "year");
    await year.click();
    await pressEach(page, ["1", "9"]); // stays below min (1925), never auto-advances
    await expect(year).toHaveText("0019");

    await page.keyboard.press("Control+Backspace");
    await expect(year).toHaveText("YYYY");
    await expect(year).toHaveAttribute("no-date", "true");
    expect(await activeLabel(page)).toBe("year");
  });
});

test.describe("Tab order", () => {
  test("Tab moves year -> month -> day -> the calendar trigger; Shift+Tab reverses it", async ({ page }) => {
    await gotoDatePicker(page);
    await segment(page, "year").click();
    await page.keyboard.press("Tab");
    expect(await activeLabel(page)).toBe("month");
    await page.keyboard.press("Tab");
    expect(await activeLabel(page)).toBe("day");
    await page.keyboard.press("Tab");
    const onTrigger = await page.evaluate(() => {
      const el = document.activeElement;
      return el?.tagName === "BUTTON" && el.getAttribute("aria-label") === "Show Calendar";
    });
    expect(onTrigger).toBe(true);

    await page.keyboard.press("Shift+Tab");
    expect(await activeLabel(page)).toBe("day");
  });
});

test.describe("aria-valuenow / aria-valuetext, and the fixed dangling aria-labelledby", () => {
  test("aria-valuenow mirrors the live value on every arrow-key change (already exercised above); before any date is chosen it reports today's date, not empty, while the visible text still shows the placeholder", async ({ page }) => {
    // `now_value` in `date_picker.rs` is `value().unwrap_or(default)`,
    // and `default` is today's year/month/day -- so an untouched segment's
    // `aria-valuenow` is never blank, even though `display_value` shows the
    // YYYY/MM/DD placeholder because `value()` itself is still `None`.
    // Documented finding, not fixed here: the base spinbutton pattern's
    // properties list only ever describes `aria-valuenow` as holding "the
    // current value of the spinbutton" -- it has no stated convention for
    // "no value yet" (unlike, e.g., `aria-valuenow`'s own omission being
    // the ARIA-wide way other range-like roles represent an indeterminate
    // value) -- so this isn't a cited violation, but it is a real, user-
    // observable mismatch between what's spoken (a concrete date) and what
    // both sighted users and screen-reader users could reasonably interpret
    // as "nothing chosen yet" that whoever picks this up next should know
    // about before assuming the two always agree.
    await gotoDatePicker(page);
    const today = await page.evaluate(() => {
      const now = new Date();
      return { year: now.getFullYear(), month: now.getMonth() + 1, day: now.getDate() };
    });

    const year = segment(page, "year");
    const month = segment(page, "month");
    const day = segment(page, "day");
    await expect(year).toHaveText("YYYY");
    await expect(month).toHaveText("MM");
    await expect(day).toHaveText("DD");
    await expect(year).toHaveAttribute("aria-valuenow", String(today.year));
    await expect(month).toHaveAttribute("aria-valuenow", String(today.month));
    await expect(day).toHaveAttribute("aria-valuenow", String(today.day));
  });

  test("no aria-valuetext is set on the month segment, even though its bare 1-12 number is arguably not self-explanatory", async ({ page }) => {
    // `spinbutton-pattern.html`'s Roles/States/Properties section: "If the
    // value of aria-valuenow is not user-friendly, e.g., the day of the
    // week is represented by a number, the aria-valuetext property is set
    // ... to a string that makes the spinbutton value understandable."
    // A bare month number (1-12) is arguably the same class of "not
    // user-friendly" case as that example. Documented finding, not fixed
    // here: doing it properly needs a locale-aware month-name string (this
    // component already threads `on_format_month`/`weekday_abbreviation`
    // callbacks for the *calendar* header, but `DateSegment` is generic
    // over any `Integer` and has no month-name concept of its own to reuse)
    // -- a real behavior addition, not the kind of one-line, self-contained
    // fix this lane's brief scopes it to fixing directly.
    await gotoDatePicker(page);
    const month = segment(page, "month");
    expect(await month.getAttribute("aria-valuetext")).toBeNull();
  });

  test("every spinbutton's aria-labelledby, if present, references an element that exists (fix regression guard)", async ({ page }) => {
    // `spinbutton-pattern.html`'s Roles/States/Properties section: "If the
    // spinbutton has a visible label, it is referenced by aria-labelledby
    // ... . Otherwise, the spinbutton element has a label provided by
    // aria-label." No `DateSegment` has a separate *visible* label element
    // (every caller -- `DatePickerYearSegment` etc. -- passes only a plain
    // `aria_label`), so aria-labelledby should never appear at all here.
    // Before this lane's `fix(date-picker)` commit, every segment on the
    // page carried `aria-labelledby="span-<id>-label"` referencing an id
    // nothing ever rendered -- confirmed by execution (24/24 spinbuttons on
    // this gallery page had a dangling reference). Chromium's own
    // accessible-name computation happened to fall back to `aria-label`
    // regardless (confirmed via `locator.ariaSnapshot()`) and axe-core does
    // not flag a dangling `aria-labelledby` either, which is exactly why
    // this needed deriving from the cited rule rather than from either
    // tool's silence. Phrased as "if present, resolves" rather than
    // "is absent" so this stays meaningful if a real visible label is ever
    // added later, per the same rule's other branch.
    await gotoDatePicker(page);
    const danglingCount = await page.evaluate(() => {
      const spins = Array.from(document.querySelectorAll('[role="spinbutton"]'));
      return spins.filter((s) => {
        const labelledBy = s.getAttribute("aria-labelledby");
        return !!labelledBy && !document.getElementById(labelledBy);
      }).length;
    });
    expect(danglingCount).toBe(0);

    // And the accessible name is still exactly "year"/"month"/"day" via
    // `aria-label` alone -- every `segment()` locator throughout this file
    // already depends on this resolving correctly, but this asserts it
    // directly for the main demo's three segments.
    await expect(segment(page, "year")).toHaveCount(1);
    await expect(segment(page, "month")).toHaveCount(1);
    await expect(segment(page, "day")).toHaveCount(1);
  });
});

test.describe("Range picker (DateRangePicker, variant=range)", () => {
  test("renders one 'Date Range' group with a start and an end year/month/day segment set", async ({ page }) => {
    await gotoDatePicker(page);
    const range = frame(page, "range");
    await expect(range.getByRole("group", { name: "Date Range" })).toHaveCount(1);
    await expect(range.getByRole("spinbutton")).toHaveCount(6);
  });

  test("ArrowRight from the start's day segment moves focus into the end's year segment, crossing the '—' separator (the separator is aria-hidden/tabindex=-1 and not part of the roving sequence)", async ({ page }) => {
    // Both sides share one `BaseDatePickerContext.focus` collection
    // (`DateRangePicker` calls `use_collection_provider` once for the whole
    // input, and `DateRangePickerEndValue` registers its segments at
    // `start_index: 3` onward in the same collection) -- confirmed live: a
    // fresh page's start-day (index 2) -> ArrowRight lands on end-year
    // (index 3), a real DOM focus move, not just an internal pointer.
    await gotoDatePicker(page);
    const startDay = rangeSegment(page, "start", "day", "range");
    const endYear = rangeSegment(page, "end", "year", "range");
    await startDay.click();
    await page.keyboard.press("ArrowRight");

    expect(await activeLabel(page)).toBe("year");
    const [activeId, endYearId] = await Promise.all([
      page.evaluate(() => document.activeElement?.id ?? null),
      endYear.evaluate((el) => el.id),
    ]);
    expect(activeId).toBe(endYearId);
  });

  test("typing into the start and end year/month segments updates each independently (stops short of completing either date -- see the known-hang guard below)", async ({ page }) => {
    // `DateRangePickerInputValue`'s own commit effect (`date_picker.rs`)
    // only calls `on_date_change`/touches the calendar once a side's
    // year+month+day all resolve to `Some` -- this exercises everything up
    // to but not including that point, which is unaffected by the hang
    // documented below (confirmed: this test passes green on its own).
    await gotoDatePicker(page);
    const start = { year: rangeSegment(page, "start", "year", "range"), month: rangeSegment(page, "start", "month", "range") };
    const end = { year: rangeSegment(page, "end", "year", "range"), month: rangeSegment(page, "end", "month", "range") };

    await start.year.click();
    await pressEach(page, ["2", "0", "2", "6"]);
    await start.month.click();
    await pressEach(page, ["0", "6"]);
    await end.year.click();
    await pressEach(page, ["2", "0", "2", "7"]);
    await end.month.click();
    await pressEach(page, ["0", "9"]);

    await expect(start.year).toHaveText("2026");
    await expect(start.month).toHaveText("06");
    await expect(end.year).toHaveText("2027");
    await expect(end.month).toHaveText("09");
  });

  test.fail("completing a full date on the start side via the segments (year+month already filled, then Home on day) hangs the tab, independent of any calendar interaction", async ({ page }) => {
    // CONFIRMED DEFECT, root cause not found, NOT fixed by this lane --
    // reported. Discovered while writing the test above: filling
    // start.year + start.month, then giving start.day *any* real value --
    // by typing a 2nd digit (which also auto-advances focus into the end
    // side, so that was this lane's first hypothesis) **or by pressing
    // `Home`, which never advances focus at all** -- hangs the page
    // indefinitely the moment a full start date is first constructed
    // (`DateElement`'s own effect, `date_picker.rs`), with NO calendar
    // open and the end side never touched. Ruling out the focus-advance
    // hypothesis: `Home` only ever calls `EmitValue`, never `FocusNext`
    // (see the Home/End fix above), yet it reproduces identically -- so
    // this is about *completing a date on one side of a range picker* by
    // itself, not about crossing the start/end boundary.
    //
    // This lane's one attempt at a fix -- guarding `DateRangeInputContext`'s
    // `start_date`/`end_date` writes against re-emitting an unchanged value
    // (mirroring `DatePickerContext::set_date`/`DateRangePickerContext::
    // set_range`, which already guard this way and are the reason the
    // *single*-date `DatePicker` and the calendar-driven commit path don't
    // exhibit this) -- was verified NOT to fix it (rebuilt, re-ran the
    // exact repro, still hung) and was reverted rather than left in as a
    // fix that doesn't fix anything. The true mechanism is still open.
    //
    // Confirmed by execution the same way as the calendar-completion hang
    // below: the page stops responding to a trivial `page.evaluate(() => 1
    // + 1)` issued right after, not just to the triggering action itself --
    // a real hang, not a slow response. Possibly the same root cause as
    // that other hang (both involve a `DateRangePicker` side's date being
    // completed for the first time), possibly not; not confirmed either
    // way given this lane's time budget. Kept fast and bounded (short
    // timeouts) rather than left to hit the full test timeout on every run.
    await gotoDatePicker(page);
    const startYear = rangeSegment(page, "start", "year", "range");
    const startMonth = rangeSegment(page, "start", "month", "range");
    const startDay = rangeSegment(page, "start", "day", "range");

    await startYear.click();
    await pressEach(page, ["2", "0", "2", "6"]);
    await startMonth.click();
    await page.keyboard.press("Home");
    await startDay.click();
    // `page.keyboard.press()` has no `timeout` option of its own (unlike a
    // locator action) -- an earlier version of this test passed one and it
    // was silently ignored, so the confirmed hang ran out the full 5-minute
    // *test* timeout instead of failing fast, which `test.fail()` does not
    // treat as an "expected" failure (a timeout aborts the test rather than
    // rejecting the assertion `test.fail()` is watching for). Race it
    // against a manual timeout instead, so this fails in 5s the same way
    // the calendar-completion guard above does.
    await Promise.race([
      page.keyboard.press("Home"),
      new Promise((_, reject) => setTimeout(() => reject(new Error("Home on the day segment did not resolve within 5s (confirmed hang)")), 5000)),
    ]);
  });
});

test.describe("Internationalized variant (variant=internationalized)", () => {
  test("renders the same fixed year -> month -> day segment order as the default variant (locale-based reordering is not implemented)", async ({ page }) => {
    // `DateElement`'s default children (`date_picker.rs`) are hardcoded
    // `DatePickerYearSegment, Separator, DatePickerMonthSegment, Separator,
    // DatePickerDaySegment` regardless of which `on_format_*_placeholder`
    // callbacks a caller supplies -- the internationalized variant
    // (`preview/src/components/date_picker/variants/internationalized/
    // mod.rs`) only swaps in `tid!(...)`-sourced placeholder text, never a
    // different child order. A French or Japanese date field would
    // conventionally reorder these; this documents that this crate does
    // not do so today, rather than silently assuming order is locale-
    // sensitive because the variant is named "internationalized".
    await gotoDatePicker(page);
    const labels = await frame(page, "internationalized")
      .getByRole("spinbutton")
      .evaluateAll((els) => els.map((el) => el.getAttribute("aria-label")));
    expect(labels).toEqual(["year", "month", "day"]);
  });

  test("the internationalized variant's placeholders match the default variant's, since the site's language switcher does not change the active locale yet", async ({ page }) => {
    // `preview/src/main.rs`'s `LanguageSelect` `onchange` handler has
    // `i18n().set_language(id)` commented out -- `use_init_i18n` fixes the
    // app at `en-US` for the life of the page regardless of what the
    // dropdown shows selected. `en-US.ftl`'s `D_Abbr`/`M_Abbr`/`Y_Abbr` are
    // "D"/"M"/"Y" -- byte-identical to `main`'s own hardcoded placeholder
    // callbacks (`preview/src/components/date_picker/component.rs`). This
    // locks in *today's* behavior; it will need updating (not by this
    // lane -- `main.rs` is out of this lane's owned files) once that
    // switcher is wired up for real.
    await gotoDatePicker(page);
    const placeholders = async (variant?: string) => ({
      year: await segment(page, "year", variant).textContent(),
      month: await segment(page, "month", variant).textContent(),
      day: await segment(page, "day", variant).textContent(),
    });
    expect(await placeholders("internationalized")).toEqual(await placeholders(undefined));
    expect(await placeholders(undefined)).toEqual({ year: "YYYY", month: "MM", day: "DD" });
  });
});

test.describe("Unavailable dates variant (variant=unavailable_dates)", () => {
  test("a day inside a disabled range renders data-unavailable and clicking it does not select or commit anything", async ({ page }) => {
    // `preview/.../unavailable_dates/mod.rs` disables three ranges starting
    // 2026-05-15; jump the calendar there via its month/year <select>s
    // (same controls `calendar.spec.ts` already drives) rather than
    // clicking "previous month" repeatedly. `RangeCalendarDay::
    // handle_day_select` (calendar.rs) early-returns for an unavailable
    // date before ever calling `set_selected_date`/committing anything, and
    // never sets a native `disabled` attribute (confirmed live: `.disabled`
    // is `false`) -- only `data-disabled`/`data-unavailable` plus
    // `cursor: not-allowed` styling communicate it, so the real assertion
    // has to be behavioural (nothing changes), not "the click fails."
    await gotoDatePicker(page);
    await pickerTrigger(page, "unavailable_dates").click();
    const dialog = pickerContent(page, "unavailable_dates");
    await expect(dialog).toBeVisible();

    await dialog.locator("select").nth(1).selectOption("2026"); // year
    await dialog.locator("select").first().selectOption({ index: 4 }); // May
    const may15 = dialog.locator('[data-unavailable="true"]').first();
    await expect(may15).toHaveAttribute("aria-label", "Friday, May 15, 2026");
    await expect(may15).toHaveJSProperty("disabled", false);

    const startDayBefore = await rangeSegment(page, "start", "day", "unavailable_dates").textContent();
    await may15.click({ force: true });
    await page.waitForTimeout(150);

    await expect(dialog).toBeVisible(); // no commit means no close either
    await expect(rangeSegment(page, "start", "day", "unavailable_dates")).toHaveText(startDayBefore!);
  });
});

test.describe("Known defect: completing a range selection via the calendar hangs the tab", () => {
  // NOT fixed by this lane -- reported, per this lane's brief, rather than
  // guessed at: `date_picker.rs` (this lane's own file) is confirmed clean
  // up to and including the moment of commit (see below); the mechanism
  // lives somewhere in the interaction between closing an `is_modal: false`
  // popover (`popover.rs`/`top_layer.rs`, not owned by any lane this batch)
  // and `RangeCalendar` unmounting (`calendar.rs`, owned by the parallel
  // rtl-rust lane this batch -- reported to that lane's queue rather than
  // edited here).
  //
  // Repro, confirmed by direct execution against this repo's dev server,
  // TWICE, under two different load conditions (this sandbox's own load
  // average was ~7.8 on 4 cores for the first repro, ~idle for the second
  // -- ruling out CPU contention as the cause):
  //   1. Open the "range" variant's popover, click day 5 (sets the anchor;
  //      the popover stays open -- confirmed fine, fast).
  //   2. Click day 10 (a *different* day -- completes the range). The
  //      demo's `on_range_change` callback fires and logs the fully correct
  //      committed range (`Some(DateRange { start: 2026-09-05, end:
  //      2026-09-10 })`) within ~300ms of the click starting -- the
  //      application-level state update is correct and fast.
  //   3. The click action itself then never resolves. Confirmed with a
  //      70-second explicit timeout (twice): still pending. Confirmed
  //      afterward that the whole page, not just this one action, stops
  //      responding: a trivial `page.evaluate(() => 1 + 1)` issued right
  //      after also never returned (measured for 5+ minutes before this
  //      probe was killed) -- this rules out "just a Playwright click-
  //      tracking quirk" and points at the page's own JS thread genuinely
  //      stuck, not merely slow.
  //   4. Isolated the *single*-date `DatePicker`'s own "click a day, select
  //      and close" flow (same popover machinery, same `is_modal: false`
  //      wiring, same `base_ctx.open.set(false)` call in this lane's own
  //      `date_picker.rs`): completes cleanly and fast. Isolated the bare
  //      `RangeCalendar` primitive's own two-click range completion with NO
  //      popover involved at all (`/component/?name=calendar&variant=
  //      range&`, inline, never promoted to the top layer): also completes
  //      cleanly and fast. Both rule out `date_picker.rs`'s own callback
  //      shape (`ctx.set_range(range); base_ctx.open.set(false);` -- the
  //      exact same shape as the single-date arm that works) and rule out
  //      `RangeCalendar`'s own selection logic in isolation -- the defect
  //      only reproduces in the combination of the two: a range completion
  //      *while inside a closing popover*.
  //   5. A read-only look at `top_layer.rs` (not owned by this lane) found
  //      a multi-frame `requestAnimationFrame`-driven "settle" loop
  //      (`reposition()`, used on every engine per `dev-docs/backlog.md`
  //      row 10's `--dx-anchor-width` account, not just the non-native-
  //      anchor fallback path) that measures the popover content's own
  //      size across frames -- a plausible mechanism if its exit condition
  //      never stabilizes once the measured element's subtree unmounts
  //      mid-settle, but this lane did not instrument it far enough to
  //      confirm that is the actual mechanism, only that it exists and is
  //      the most plausible candidate found without a CPU profiler.
  //
  // Kept as a fast, bounded `test.fail()` regression guard (a short 5s
  // click timeout, not the 70s used to confirm the hang above) rather than
  // a silently-red test: this documents a real, confirmed defect without
  // slowing down every future run of this file by a minute, and will flip
  // to an unexpected pass (Playwright fails the run and says so) the day
  // whichever lane owns the real fix lands it.
  test.fail("clicking a second, different calendar day to complete a range selection resolves", async ({ page }) => {
    await gotoDatePicker(page);
    await pickerTrigger(page, "range").click();
    const dialog = pickerContent(page, "range");
    await expect(dialog).toBeVisible();

    await dayCell(dialog, 5).click({ timeout: 5000 });
    await dayCell(dialog, 10).click({ timeout: 5000 });
  });
});
