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

import { test, expect } from "./fixtures";
import { type Locator, type Page } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";
import { BASE_URL } from "./base-url";
import { gotoHydrated } from "./hydration";

const NAV_TIMEOUT = 20 * 60 * 1000; // first run compiles the app
const PAGE_URL = `${BASE_URL}/component/?name=date_picker&`;

// dev-docs/backlog.md row 109: this suite clicks the trigger and types into
// segments right after navigating, on the SSG lane -- `gotoHydrated` so
// those interactions can't land before hydration attaches listeners.
async function gotoDatePicker(page: Page) {
  await gotoHydrated(page, PAGE_URL, { timeout: NAV_TIMEOUT });
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

/**
 * Viewport height for the two geometry tests below. The demo's panel is 278px tall and opens 8px below its
 * trigger, whose bottom edge sits at y ~ 446 on this page, so at Playwright's default 1280x720 only ~274px of
 * room is left below it: `position-try-fallbacks: flip-block` correctly puts the panel ABOVE the trigger (and,
 * after a 150px scroll that frees room below, flips it back -- a ~164px jump that reads as "the offset changed").
 * Neither is an anchoring bug: the panel is doing what the contract says when it does not fit. These tests ask
 * "is the panel anchored to its trigger, below it, and does it track a scroll", which only has a defined answer
 * when it fits, so they run in a viewport tall enough that it does (dev-docs/backlog.md row 21; measured on a
 * release SSG build, 2026-10-05: 720 -> flips above, 800/900/1000 -> below and exact scroll tracking).
 */
const ROOMY_VIEWPORT = { width: 1280, height: 900 };

test("opens the calendar popover on trigger click", async ({ page }) => {
  await gotoDatePicker(page);
  await expect(content(page)).toBeHidden();

  await trigger(page).click();

  await expect(content(page)).toBeVisible();
});

test("anchors the popup next to its trigger, not viewport-centered or offset", async ({ page }) => {
  await page.setViewportSize(ROOMY_VIEWPORT); // see ROOMY_VIEWPORT: at 720px tall the panel correctly flips above
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

  // Precondition, so a too-short viewport fails loudly as itself instead of as a confusing "not below".
  expect(viewport.height - t.bottom, `no room below the trigger for the panel -- ${debug}`).toBeGreaterThan(c.height + 8);

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
  await page.setViewportSize(ROOMY_VIEWPORT); // see ROOMY_VIEWPORT: at 720px tall the panel flips above, then back below
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

  // The date picker's own trigger and panel -- NOT `document.querySelector('[style*="anchor-name"]')`: since the
  // header's theme picker button (`.dx-theme-picker-trigger`, inline `anchor-name`) came first in the DOM, that
  // selector measured the sticky header button, whose offset to the panel drifts by exactly the scroll delta.
  const offsetOf = async () => {
    const t = await rectOf(page, trigger(page));
    const c = await rectOf(page, content(page));
    return { top: c.top - t.bottom, left: c.left - t.left };
  };

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
 * Several other gaps against the cited text were found and were deliberately
 * NOT fixed in that pass (documented in the tests that touched them, and in
 * that lane's report): no `aria-valuetext` on the month segment (a bare
 * 1-12 number, arguably "not user-friendly" per the pattern's own example),
 * and `aria-valuenow` reporting today's date rather than being omitted when
 * no date has been chosen yet. Both (plus two related findings from the
 * same investigation -- `Calendar`'s unavailable days having no
 * `aria-disabled`, and Up/Down wrapping the year segment instead of
 * clamping it) were recorded together as `dev-docs/backlog.md` row 84 and
 * are fixed by the tests below this comment (the "aria-valuenow /
 * aria-valuetext" and "Arrow keys" describe blocks) -- see each test's own
 * comment for the red-before/green-after account.
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

type Scheme = "light" | "dark";

/** The site keys its colour scheme on `html[data-theme]`, falling back to `prefers-color-scheme`; set both (see component-catalog.spec.ts). */
async function setScheme(page: Page, scheme: Scheme) {
  await page.emulateMedia({ colorScheme: scheme });
  await page.evaluate((s) => document.documentElement.setAttribute("data-theme", s), scheme);
}

/** What a design token resolves to as a computed `color`, for comparing against an element's. */
async function tokenColor(page: Page, token: string) {
  return page.evaluate((t) => {
    const probe = document.createElement("span");
    probe.style.color = `var(${t})`;
    document.body.appendChild(probe);
    const color = getComputedStyle(probe).color;
    probe.remove();
    return color;
  }, token);
}

/** Wait until `locator`'s box has stopped moving (the open animation and the anchor positioning have settled). */
async function waitSettled(locator: Locator) {
  let prev = "";
  await expect
    .poll(
      async () => {
        const box = await locator.evaluate((el) => {
          const b = el.getBoundingClientRect();
          return `${Math.round(b.x)},${Math.round(b.y)},${Math.round(b.width)},${Math.round(b.height)}`;
        });
        const stable = box === prev;
        prev = box;
        return stable;
      },
      { timeout: 5000, intervals: [100] },
    )
    .toBe(true);
}

/**
 * Records, in the BROWSER's clock (so Playwright round trips -- which a busy machine stretches to
 * seconds -- cannot blur a 300 ms dwell): when the LAST click lands, when a day first reads
 * `data-selected="true"` (and what state the popover was in right then), and when `dialog` flips to
 * `data-state="closed"`.
 */
async function armDwellProbe(dialog: Locator) {
  await dialog.evaluate((d) => {
    const w = window as unknown as { __dwell: { click: number; selected: number; stateAtSelected: string; closed: number } };
    w.__dwell = { click: 0, selected: 0, stateAtSelected: "", closed: 0 };
    document.addEventListener("click", () => (w.__dwell.click = performance.now()), { capture: true });
    new MutationObserver((records) => {
      for (const r of records) {
        const el = r.target as Element;
        if (r.attributeName === "data-selected" && el.getAttribute("data-selected") === "true" && !w.__dwell.selected) {
          w.__dwell.selected = performance.now();
          w.__dwell.stateAtSelected = d.getAttribute("data-state") ?? "";
        }
        if (r.attributeName === "data-state" && el === d && d.getAttribute("data-state") === "closed" && !w.__dwell.closed) {
          w.__dwell.closed = performance.now();
        }
      }
    }).observe(d, { attributes: true, attributeFilter: ["data-state", "data-selected"], subtree: true });
  });
}
async function readDwell(page: Page) {
  const t = await page.evaluate(
    () => (window as unknown as { __dwell: { click: number; selected: number; stateAtSelected: string; closed: number } }).__dwell,
  );
  return { ...t, held: t.closed - t.click };
}

type Step =
  | { click: number }
  | { pointerdown: number }
  | { key: string; on: number }
  | { clickTrigger: true }
  | { wait: number };

/**
 * Runs `steps` against the open `dialog`'s current-month day cells inside ONE `page.evaluate`, so the
 * gaps between them are what the test says (`wait`, in the page's own clock) rather than however long
 * Playwright round trips take. A 300 ms dwell can only be raced from inside the page.
 */
async function runSteps(dialog: Locator, steps: Step[]) {
  await dialog.evaluate(async (d, steps) => {
    const day = (n: number) => {
      const cell = Array.from(d.querySelectorAll<HTMLElement>('.dx-calendar-grid-cell[data-month="current"]')).find(
        (c) => c.textContent?.trim() === String(n),
      );
      if (!cell) throw new Error(`no day ${n}`);
      return cell;
    };
    const root = d.closest(".dx-date-picker") as HTMLElement;
    for (const step of steps) {
      if ("click" in step) day(step.click).click();
      else if ("pointerdown" in step) day(step.pointerdown).dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, composed: true, pointerType: "mouse" }));
      else if ("key" in step) day(step.on).dispatchEvent(new KeyboardEvent("keydown", { key: step.key, bubbles: true, composed: true }));
      else if ("clickTrigger" in step) (root.querySelector('button[aria-label="Show Calendar"]') as HTMLElement).click();
      else await new Promise((r) => setTimeout(r, step.wait));
    }
  }, steps);
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

    // `valueBefore` is `null` on this fresh page as of backlog row 84
    // finding 3's fix (an untouched segment now omits `aria-valuenow`
    // entirely instead of defaulting it) -- `toHaveAttribute` has no way to
    // assert "still absent" from a `null` expected value, so branch
    // explicitly instead of assuming it's always a string.
    if (valueBefore === null) {
      expect(await month.getAttribute("aria-valuenow")).toBeNull();
    } else {
      await expect(month).toHaveAttribute("aria-valuenow", valueBefore);
    }
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

  test("Up wraps from the maximum to the minimum and Down wraps from the minimum to the maximum on month (a cyclical unit)", async ({ page }) => {
    // Month and day are cyclical -- backlog row 84 finding 4's calibration
    // (`date_segment_key_effects`'s own doc): matches native Chromium
    // `<input type="date">`'s month/day sub-fields. Unchanged by this
    // lane's fix.
    await gotoDatePicker(page);
    const month = segment(page, "month");
    await month.click();
    await page.keyboard.press("End");
    await page.keyboard.press("ArrowUp");
    await expect(month).toHaveAttribute("aria-valuenow", "1");
    await page.keyboard.press("Home");
    await page.keyboard.press("ArrowDown");
    await expect(month).toHaveAttribute("aria-valuenow", "12");
  });

  test("Up clamps at the maximum and Down clamps at the minimum on the year segment (not cyclical) -- backlog row 84 finding 4", async ({ page }) => {
    // RED on the unmodified tree (verified live before this fix): `roll_value`
    // applied the same wrap to every segment type with no year-specific
    // exception, so End -> ArrowUp on the year segment reported
    // aria-valuenow="1925" (wrapped to the minimum) instead of staying at
    // "2050". Fixed by threading an explicit per-segment wrap policy through
    // `date_segment_key_effects` (`SegmentValueBounds.wrap`): month/day keep
    // wrapping (see the sibling test above), the year segment now clamps --
    // matches native Chromium `<input type="date">`'s year sub-field, which
    // does not wrap either. GREEN after the fix: both assertions below hold.
    await gotoDatePicker(page);
    const year = segment(page, "year");
    await year.click();
    await page.keyboard.press("End");
    await expect(year).toHaveAttribute("aria-valuenow", "2050");
    await page.keyboard.press("ArrowUp");
    await expect(year).toHaveAttribute("aria-valuenow", "2050");

    await page.keyboard.press("Home");
    await expect(year).toHaveAttribute("aria-valuenow", "1925");
    await page.keyboard.press("ArrowDown");
    await expect(year).toHaveAttribute("aria-valuenow", "1925");
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
  test("aria-valuenow mirrors the live value on every arrow-key change (already exercised above); before any date is chosen it is entirely absent, not today's date -- backlog row 84 finding 3", async ({ page }) => {
    // RED on the unmodified tree (verified live before this fix): `now_value`
    // in `date_picker.rs` was `value().unwrap_or(default)`, and `default` is
    // today's year/month/day -- so an untouched segment's `aria-valuenow`
    // was never blank (it reported e.g. today's actual year), even though
    // `display_value` correctly showed the YYYY/MM/DD placeholder because
    // `value()` itself was still `None`. That silently claimed a value the
    // user never set. Fixed by making `aria-valuenow` mirror `props.value`'s
    // own `Option` faithfully: present once the segment actually holds a
    // value, omitted while it doesn't (the ARIA-wide convention for "no
    // value yet", matching how other range-like roles represent an
    // indeterminate value) -- see `DateSegment`'s `now_value` doc for the
    // full account, including why this also fixes finding 6 (tested below).
    await gotoDatePicker(page);
    const year = segment(page, "year");
    const month = segment(page, "month");
    const day = segment(page, "day");
    await expect(year).toHaveText("YYYY");
    await expect(month).toHaveText("MM");
    await expect(day).toHaveText("DD");
    expect(await year.getAttribute("aria-valuenow")).toBeNull();
    expect(await month.getAttribute("aria-valuenow")).toBeNull();
    expect(await day.getAttribute("aria-valuenow")).toBeNull();

    // And once a value actually exists, it reports the real value -- typing
    // into the year segment must not disturb the still-untouched month/day.
    await year.click();
    await pressEach(page, ["1", "9", "9", "9"]);
    await expect(year).toHaveAttribute("aria-valuenow", "1999");
    expect(await month.getAttribute("aria-valuenow")).toBeNull();
    expect(await day.getAttribute("aria-valuenow")).toBeNull();
  });

  test("the month segment's aria-valuetext is a locale-aware month name once it has a value, and absent while untouched -- backlog row 84 finding 2", async ({ page }) => {
    // RED on the unmodified tree (verified live before this fix): no
    // `aria-valuetext` was ever rendered on any segment -- `DateSegment` is
    // generic over any `Integer` and had no month-name concept of its own,
    // and nothing threaded the calendar grid's own `on_format_month`
    // callback down to it. `spinbutton-pattern.html`'s Roles/States/
    // Properties section: "If the value of aria-valuenow is not
    // user-friendly, e.g., the day of the week is represented by a number,
    // the aria-valuetext property is set ... to a string that makes the
    // spinbutton value understandable" -- a bare month number (1-12) is
    // exactly that case. Fixed by threading a new `on_format_month`
    // callback (defaulting to `Month::to_string()`, i18n-overridable
    // exactly like the calendar grid's own) down to `DatePickerMonthSegment`
    // and gating it on the same "has a value" check as aria-valuenow, so
    // the two stay in lockstep.
    await gotoDatePicker(page);
    const month = segment(page, "month");
    expect(await month.getAttribute("aria-valuetext")).toBeNull();

    await month.click();
    await page.keyboard.press("Home");
    await expect(month).toHaveAttribute("aria-valuenow", "1");
    await expect(month).toHaveAttribute("aria-valuetext", "January");

    await page.keyboard.press("End");
    await expect(month).toHaveAttribute("aria-valuenow", "12");
    await expect(month).toHaveAttribute("aria-valuetext", "December");

    // Year and day are plain numbers -- no aria-valuetext on either, even
    // once they have values (this finding is scoped to the month segment).
    const year = segment(page, "year");
    const day = segment(page, "day");
    await year.click();
    await page.keyboard.press("Home");
    await day.click();
    await page.keyboard.press("Home");
    expect(await year.getAttribute("aria-valuetext")).toBeNull();
    expect(await day.getAttribute("aria-valuetext")).toBeNull();
  });

  test("the first Up/Down keypress on a never-touched segment is a real, detectable transition, not a silent no-op -- backlog row 84 finding 6", async ({ page }) => {
    // RED on the unmodified tree (verified live before this fix): finding 3
    // meant an untouched segment's `aria-valuenow` already equaled
    // `default` (e.g. today's month) before any key was pressed;
    // `date_segment_key_effects`'s `None => default` branch then committed
    // exactly that same `default` on the first Up/Down press -- so
    // `aria-valuenow` read identically before and after (both "today's
    // month"), which is indistinguishable from a dropped keystroke. Fixed
    // as a direct consequence of finding 3's fix: once "no value" stops
    // being reported as "= default", the untouched state has NO
    // aria-valuenow at all, so the first press's `default` commit is a
    // genuine attribute-absent -> attribute-present transition. This test
    // deliberately does NOT press Home first (unlike the general Up/Down
    // test above), specifically to exercise the never-touched, first-ever
    // keypress case findings 3 and 6 are both about.
    await gotoDatePicker(page);
    const month = segment(page, "month");
    const today = await page.evaluate(() => new Date().getMonth() + 1);
    expect(await month.getAttribute("aria-valuenow")).toBeNull();

    await month.click();
    await page.keyboard.press("ArrowDown");

    const after = await month.getAttribute("aria-valuenow");
    expect(after, "the first keypress must produce a real aria-valuenow, not leave it absent").not.toBeNull();
    // `date_segment_key_effects`'s `None => default` branch is unchanged by
    // this fix (only the *reporting* of "no value yet" changed) -- the
    // committed value is still exactly `default` (today's month), just now
    // genuinely new information rather than a repeat of what was already
    // being announced.
    expect(Number(after)).toBe(today);
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

  test("typing into the start and end year/month segments updates each independently (stops short of completing either date -- see the regression guard below)", async ({ page }) => {
    // `DateRangePickerInputValue`'s own commit effect (`date_picker.rs`)
    // only calls `on_date_change`/touches the calendar once a side's
    // year+month+day all resolve to `Some` -- this exercises everything up
    // to but not including that point, which used to be where the hang
    // documented in the regression guard below kicked in (confirmed: this
    // test passes green on its own, both before and after that fix).
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

  test("completing a full date on the start side via the segments (year+month, then Home on day) resolves promptly and commits only the start side", async ({ page }) => {
    // Regression guard for a confirmed page hang (dev-docs/backlog.md's
    // range-hang account). Root cause: the moment a `DateRangePicker`
    // side's year/month/day segments first all resolved, `DateElement`'s
    // own two effects (`date_picker.rs`) and `DateRangeInputContext`'s
    // `start_date`/`end_date` signals formed an unconditional two-way
    // sync -- `DateElement`'s "sync down from selected_date" effect wrote
    // the segment signals, whose "sync up via on_date_change" effect wrote
    // `start_date` right back, and neither hop checked whether the value
    // it was about to write actually differed from what was already
    // there. `Signal::set` marks a signal dirty (rescheduling every
    // subscriber) regardless of whether the new value equals the old one,
    // so the round trip re-triggered itself forever: confirmed live, the
    // render thread pinned near 100% CPU and its RSS grew ~4MB/s
    // (unthrottled allocation on every iteration) until the tab crashed --
    // with no calendar open and the end side never touched, ruling out any
    // popover/focus-advance involvement. Fixed by construction: every
    // write to `start_date`/`end_date` now goes through one guarded choke
    // point (`DateRangeInputContext::set_start_date`/`set_end_date`,
    // mirroring the equality guard `DatePickerContext::set_date`/
    // `DateRangePickerContext::set_range` already use one level up), and
    // `DateElement`'s own sync-down effect only writes a segment signal
    // when its decomposed value actually changes.
    await gotoDatePicker(page);
    const startYear = rangeSegment(page, "start", "year", "range");
    const startMonth = rangeSegment(page, "start", "month", "range");
    const startDay = rangeSegment(page, "start", "day", "range");
    const endYear = rangeSegment(page, "end", "year", "range");

    await startYear.click();
    await pressEach(page, ["2", "0", "2", "6"]);
    await startMonth.click();
    await page.keyboard.press("Home");
    await startDay.click();
    // `page.keyboard.press()` has no `timeout` option of its own (unlike a
    // locator action), so a regression here would hang the whole test file
    // for the 5-minute *test* timeout instead of failing fast -- race it
    // against a manual timeout so a reintroduction of the bug still fails
    // in 5s.
    await Promise.race([
      page.keyboard.press("Home"),
      new Promise((_, reject) =>
        setTimeout(() => reject(new Error("Home on the day segment did not resolve within 5s (regression -- see this test's own comment)")), 5000)
      ),
    ]);

    await expect(startYear).toHaveText("2026");
    await expect(startMonth).toHaveText("01");
    await expect(startDay).toHaveText("01");
    // The end side was never touched.
    await expect(endYear).toHaveText("YYYY");
    // The page as a whole is still responsive, not just this one action.
    expect(await page.evaluate(() => 1 + 1)).toBe(2);
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

  test("the internationalized variant's placeholders match the default variant's on first load (both start at en-US)", async ({ page }) => {
    // `en-US.ftl`'s `D_Abbr`/`M_Abbr`/`Y_Abbr` are "D"/"M"/"Y" -- byte-
    // identical to `main`'s own hardcoded placeholder callbacks
    // (`preview/src/components/date_picker/component.rs`). True both
    // before and after backlog row 84 finding 5's fix (the app's initial
    // locale was always, and still is, `en-US` -- see
    // `use_init_i18n(|| I18nConfig::new(langid!("en-US")) ...)` in
    // `main.rs`); the *switcher actually working* is the next test.
    await gotoDatePicker(page);
    const placeholders = async (variant?: string) => ({
      year: await segment(page, "year", variant).textContent(),
      month: await segment(page, "month", variant).textContent(),
      day: await segment(page, "day", variant).textContent(),
    });
    expect(await placeholders("internationalized")).toEqual(await placeholders(undefined));
    expect(await placeholders(undefined)).toEqual({ year: "YYYY", month: "MM", day: "DD" });
  });

  test("the language switcher actually changes the active locale -- backlog row 84 finding 5", async ({ page }) => {
    // RED on the unmodified tree (verified live before this fix):
    // `preview/src/main.rs`'s `LanguageSelect` `onchange` handler had
    // `i18n().set_language(id)` commented out, so `use_init_i18n` kept the
    // app fixed at `en-US` for the life of the page no matter what the
    // dropdown showed selected -- the control was dead. Fixed by
    // uncommenting the call (wiring it up for real was a contained,
    // two-line change: import `i18n` and make the call) rather than
    // removing the control, since `dioxus_i18n::prelude::i18n()` reads the
    // exact `I18n` context `use_init_i18n` already provides in `App`, and
    // `set_language` writes its reactively-read `active_bundle` signal --
    // no larger plumbing was needed. The internationalized date_picker
    // variant's `on_format_day_placeholder: || tid!("D_Abbr")` etc. (and,
    // as of finding 2's fix, `on_format_month`) read that same bundle, so
    // switching to French should change "D_Abbr" from "D" (en-US) to "J"
    // (fr-FR, per `fr-FR.ftl`) live, with no reload. The plain "main"
    // variant's placeholders are hardcoded in `component.rs` (not
    // `tid!`-driven at all -- see `variants/main/mod.rs`), so they must NOT
    // change -- this isolates "the switcher recompiled translated text" from
    // "the whole page reloaded" or some other unrelated effect.
    await gotoDatePicker(page);
    const languageSelect = page.getByRole("combobox", { name: "Language" });
    await expect(languageSelect).toBeVisible();

    // Placeholder text repeats the formatter's letter `max_length` times
    // (`DateSegment::display_value`) -- "DD" for the 2-character day
    // segment, matching this file's own established convention (e.g. the
    // "renders the same fixed year -> month -> day segment order" test's
    // `{ year: "YYYY", month: "MM", day: "DD" }`).
    const mainDayBefore = await segment(page, "day", undefined).textContent();
    expect(mainDayBefore).toBe("DD");
    const intlDayBefore = await segment(page, "day", "internationalized").textContent();
    expect(intlDayBefore).toBe("DD");

    await languageSelect.selectOption("French");

    // Day ("D" -> "J") and year ("Y" -> "A") both change between en-US and
    // fr-FR, so either alone would prove the switch took effect; month is
    // deliberately not asserted here since fr-FR's own `M_Abbr` is ALSO
    // "M" (`fr-FR.ftl`) -- it would pass whether or not the switch worked,
    // so it isn't discriminating evidence either way.
    await expect(segment(page, "day", "internationalized")).toHaveText("JJ");
    await expect(segment(page, "year", "internationalized")).toHaveText("AAAA");
    // The non-internationalized variant is untouched by the locale change.
    await expect(segment(page, "day", undefined)).toHaveText("DD");
  });
});

test.describe("Unavailable dates variant (variant=unavailable_dates)", () => {
  test("a day inside a disabled range renders data-unavailable and clicking it does not select or commit anything", async ({ page }) => {
    // The demo computes its disabled ranges from today and keeps them inside the
    // month the picker opens on (`unavailable_dates/mod.rs`), so no month/year
    // navigation is needed -- and none is done here, which is the point: the
    // first version of this test jumped to a hard-coded May 2026 through the
    // selects, and the demo it exercised showed nothing to anyone opening it on
    // any other month. `RangeCalendarDay::handle_day_select` (calendar.rs)
    // early-returns for an unavailable date before ever calling
    // `set_selected_date`/committing anything, and never sets a native
    // `disabled` attribute (confirmed live: `.disabled` is `false`) -- only
    // `data-disabled`/`data-unavailable` plus styling communicate it, so the
    // real assertion has to be behavioural (nothing changes), not "the click fails."
    await gotoDatePicker(page);
    await pickerTrigger(page, "unavailable_dates").click();
    const dialog = pickerContent(page, "unavailable_dates");
    await expect(dialog).toBeVisible();

    const unavailable = dialog.locator('.dx-calendar-grid-cell[data-unavailable="true"]');
    const day1 = unavailable.first();
    await expect(day1).toBeVisible();
    await expect(day1).toHaveJSProperty("disabled", false);
    await expect(day1).toHaveAttribute("aria-disabled", "true");

    const startDayBefore = await rangeSegment(page, "start", "day", "unavailable_dates").textContent();
    await day1.click({ force: true });
    await page.waitForTimeout(150);

    await expect(dialog).toBeVisible(); // no commit means no close either
    await expect(rangeSegment(page, "start", "day", "unavailable_dates")).toHaveText(startDayBefore!);
  });

  test("the month the picker opens on has unavailable days, all inside that month (the demo can't go stale)", async ({ page }) => {
    await gotoDatePicker(page);
    await pickerTrigger(page, "unavailable_dates").click();
    const dialog = pickerContent(page, "unavailable_dates");
    await expect(dialog).toBeVisible();

    // 3 ranges of 3 + 3 + 2 days, by construction (see the demo's `unavailable_ranges`).
    const unavailable = dialog.locator('.dx-calendar-grid-cell[data-unavailable="true"]');
    await expect(unavailable).toHaveCount(8);
    const labels = await unavailable.evaluateAll((els) => els.map((el) => el.getAttribute("aria-label")));
    const now = await page.evaluate(() => {
      const n = new Date();
      return { year: n.getFullYear(), month: n.toLocaleString("en-US", { month: "long" }) };
    });
    for (const label of labels) {
      // "Friday, May 15, 2026"
      expect(label, "an unavailable day outside the month in view").toMatch(new RegExp(`, ${now.month} \\d+, ${now.year}$`));
    }
    // ...and every one of them is a real cell of the CURRENT month (not a spill-over day).
    await expect(dialog.locator('.dx-calendar-grid-cell[data-unavailable="true"]:not([data-month="current"])')).toHaveCount(0);
  });

  for (const scheme of ["light", "dark"] as const) {
    test(`unavailable days are muted AND struck through, visibly distinct from available days (${scheme})`, async ({ page }) => {
      await gotoDatePicker(page);
      await setScheme(page, scheme);
      await pickerTrigger(page, "unavailable_dates").click();
      const dialog = pickerContent(page, "unavailable_dates");
      await expect(dialog).toBeVisible();

      const unavailable = dialog.locator('.dx-calendar-grid-cell[data-month="current"][data-unavailable="true"]').first();
      const available = dialog
        .locator('.dx-calendar-grid-cell[data-month="current"]:not([data-unavailable="true"])')
        .first();
      await expect(unavailable).toBeVisible();

      const read = (l: Locator) =>
        l.evaluate((el) => {
          const cs = getComputedStyle(el);
          return { color: cs.color, line: cs.textDecorationLine, cursor: cs.cursor, opacity: cs.opacity };
        });
      const u = await read(unavailable);
      const a = await read(available);
      const muted = await tokenColor(page, "--dx-muted-foreground");

      expect(u.line, "line-through, as react-day-picker's unavailable modifier").toContain("line-through");
      expect(a.line).not.toContain("line-through");
      expect(u.color, "the muted-foreground token").toBe(muted);
      expect(u.color, "distinct from an available day").not.toBe(a.color);
      expect(u.cursor).toBe("not-allowed");
      expect(u.opacity, "not faded a second time by the picker's own [data-disabled] rule").toBe("1");
    });
  }
});

test.describe("Completing a range selection via the calendar", () => {
  // This used to hang the tab -- same root cause as, and fixed by the same
  // construction as, the segments-only guard above (dev-docs/backlog.md's
  // range-hang account), not the popover-closing/top_layer mechanism
  // originally suspected. What actually happens once a calendar click
  // completes a range: `DateRangePickerCalendar`'s `on_range_change` calls
  // the already-guarded `DateRangePickerContext::set_range`, which flows
  // back down through `props.selected_range` into
  // `DateRangePickerInputValue`'s own sync-down effect -- which
  // (unconditionally, before this fix) wrote the *same* `start_date`/
  // `end_date` signals `DateElement`'s segments for the start AND end side
  // both read, tripping the identical unbounded two-effect cycle the
  // segments-only guard above documents, for both sides at once. The
  // popover-closing call (`base_ctx.open.set(false)`) sits one statement
  // after the range-commit call in the same handler, so from the outside
  // it looked correlated with "closing a popover"; it never actually got
  // a chance to run before this fix, since the interpreter never returned
  // from the effect cascade the *preceding* statement (`ctx.set_range`)
  // touched off. Confirmed live before this fix: the demo's
  // `on_range_change` callback logs the fully correct committed range
  // within ~300ms, then the click action itself never resolves and the
  // whole page stops responding to a trivial `page.evaluate(() => 1 + 1)`
  // -- a real hang, not a slow response, with the render thread pinned
  // near 100% CPU and RSS growing until the tab crashed (same signature as
  // the segments-only case). Fixed by the same construction: every write
  // to `start_date`/`end_date` (including this sync-down effect's) now
  // goes through the guarded `DateRangeInputContext::set_start_date`/
  // `set_end_date` choke point. No change was needed in `calendar.rs`,
  // `popover.rs`, or `top_layer.rs`.
  test("clicking a second, different calendar day to complete a range selection resolves, closes the popover, and syncs the segments", async ({ page }) => {
    await gotoDatePicker(page);
    await pickerTrigger(page, "range").click();
    const dialog = pickerContent(page, "range");
    await expect(dialog).toBeVisible();

    await dayCell(dialog, 5).click({ timeout: 5000 });
    await dayCell(dialog, 10).click({ timeout: 5000 });

    await expect(dialog).toBeHidden();
    // The calendar defaults to the current month/year (days 5 and 10 of it
    // are always in view, spillover-free, regardless of which day of the
    // month "today" actually is) -- computed rather than hardcoded, per
    // this file's own established convention (see the aria-valuenow test
    // above and calendar.spec.ts).
    const today = await page.evaluate(() => {
      const now = new Date();
      return { year: now.getFullYear(), month: now.getMonth() + 1 };
    });
    const yyyy = String(today.year);
    const mm = String(today.month).padStart(2, "0");
    const start = { year: rangeSegment(page, "start", "year", "range"), month: rangeSegment(page, "start", "month", "range"), day: rangeSegment(page, "start", "day", "range") };
    const end = { year: rangeSegment(page, "end", "year", "range"), month: rangeSegment(page, "end", "month", "range"), day: rangeSegment(page, "end", "day", "range") };
    await expect(start.year).toHaveText(yyyy);
    await expect(start.month).toHaveText(mm);
    await expect(start.day).toHaveText("05");
    await expect(end.year).toHaveText(yyyy);
    await expect(end.month).toHaveText(mm);
    await expect(end.day).toHaveText("10");
    expect(await page.evaluate(() => 1 + 1)).toBe(2);
  });
});

/**
 * -----------------------------------------------------------------------
 * Completing SEVERAL ranges in a row (dev-docs/backlog.md row 83's follow-up, 2026-10-05).
 *
 * The test above completes ONE range, and a second range hung the tab for a different reason than
 * the first: `DateRangePickerContext::set_range` compared against the controlled value with a
 * TRACKED read, and `DateRangePickerInputValue`'s sync-up effect calls it -- so every change the
 * parent made to the range (the one a calendar click had just reported upward) woke that effect
 * too. When it ran before the sync-down effect had copied the new range into the segments it read
 * the PREVIOUS range from them, reported it, the parent applied it, and the two ranges alternated
 * forever: a wasm loop that never yields to the browser, so even `page.evaluate(() => 1 + 1)`
 * never returned. The first range could not trigger it (nothing was in the segments yet, so there
 * was nothing stale to report) and which effect wakes first is hash-ordered -- a debug dev-server
 * build hung on the 2nd range, a release SSG build on the 3rd, a build with two extra log lines
 * on the 4th -- so no single count of ranges proves it gone: this completes six, and ALSO counts
 * what the demo's `on_range_change` was told, which a ping-pong inflates long before it freezes.
 * (Guarded natively too: `a_range_the_parent_sets_is_never_reported_back`, primitives/src/date_picker.rs.)
 * -----------------------------------------------------------------------
 */
test.describe("Completing several ranges in a row (variant=range)", () => {
  /** `page.evaluate` that FAILS after 2 s instead of hanging the test: a wasm loop never answers it. */
  async function expectResponsive(page: Page, label: string) {
    const answer = await Promise.race([
      page.evaluate(() => 1 + 1),
      new Promise<string>((resolve) => setTimeout(() => resolve("hung"), 2000)),
    ]);
    expect(answer, `the page stopped responding ${label} (a wasm loop -- see this describe's comment)`).toBe(2);
  }

  test("six ranges: each is reported exactly once, shown in the segments, and the page stays responsive after every one", async ({ page }) => {
    const reported: string[] = [];
    page.on("console", (message) => {
      if (message.text().includes("Selected range:")) reported.push(message.text());
    });
    await gotoDatePicker(page);
    const trigger = pickerTrigger(page, "range");
    const dialog = pickerContent(page, "range");
    const start = { day: rangeSegment(page, "start", "day", "range") };
    const end = { day: rangeSegment(page, "end", "day", "range") };
    const pad = (day: number) => String(day).padStart(2, "0");

    // Start before end, end before start, adjacent, a long span: the stale pair a ping-pong reports
    // is always the PREVIOUS range, so each step has to differ from the one before it.
    const picks: Array<[number, number]> = [[6, 13], [15, 20], [3, 9], [10, 12], [22, 25], [1, 2]];
    for (const [index, [first, second]] of picks.entries()) {
      const label = `after range ${index + 1} (${first} -> ${second})`;
      await trigger.click({ timeout: 5000 });
      await expect(dialog).toBeVisible();
      await dayCell(dialog, first).click({ timeout: 5000 });
      await dayCell(dialog, second).click({ timeout: 5000 });
      await expect(dialog).toBeHidden({ timeout: 5000 });
      await expectResponsive(page, label);
      await expect(start.day, label).toHaveText(pad(Math.min(first, second)));
      await expect(end.day, label).toHaveText(pad(Math.max(first, second)));
      // One completion, one report -- and nothing more once the segments have caught up.
      await expect.poll(() => reported.length, { message: `the demo's on_range_change was told ${label}`, timeout: 5000 }).toBe(index + 1);
      await page.waitForTimeout(150);
      expect(reported.length, `a range the parent set was reported back ${label}: ${reported.join(" | ")}`).toBe(index + 1);
    }
    await expectResponsive(page, "after all six");
  });
});

/**
 * -----------------------------------------------------------------------
 * The calendar's stylesheet is on the page BEFORE the popover is ever opened. The calendar sits
 * inside the popover, whose content mounts on first open, so the `<link>` that `CalendarRoot`
 * carries used to be inserted then -- late, and not render-blocking: the first open painted the
 * calendar unstyled for a frame or two. The styled pickers now render it at the picker root. (The
 * docs site's SSG bundle carries it either way; a `dx components add date_picker` project, and
 * `dx serve`, do not, which is what this checks.)
 * -----------------------------------------------------------------------
 */
test.describe("Calendar stylesheet before the first open", () => {
  test("a calendar rule is already loaded while every picker on the page is still closed", async ({ page }) => {
    await gotoDatePicker(page);
    await expect(page.getByRole("dialog")).toHaveCount(0);
    await expect
      .poll(
        () =>
          page.evaluate(() =>
            Array.from(document.styleSheets).some((sheet) => {
              try {
                return Array.from(sheet.cssRules).some((rule) => rule.cssText.includes(".dx-calendar-grid-cell"));
              } catch {
                return false; // a cross-origin sheet (fonts) -- not ours
              }
            }),
          ),
        { message: "no loaded stylesheet carries the calendar's rules before the first open", timeout: 10000 },
      )
      .toBe(true);
  });
});

/**
 * -----------------------------------------------------------------------
 * Multi-month panel (owner report 1): the popover panel used to be sized for ONE month
 * (`.dx-popover-content { width: 18rem }`), so with `month_count: 2` the second month rendered
 * outside it. The panel is now `width: max-content` (capped to the viewport) and the months wrap
 * inside it -- side by side when there is room, stacked when there is not.
 * -----------------------------------------------------------------------
 */
async function monthGeometry(dialog: Locator) {
  return dialog.evaluate((d) => {
    const rect = (e: Element) => {
      const b = e.getBoundingClientRect();
      return { left: b.left, right: b.right, top: b.top, bottom: b.bottom, width: b.width, height: b.height };
    };
    const cal = d.querySelector(".dx-calendar") as HTMLElement;
    const views = Array.from(d.querySelectorAll(".dx-calendar-view"));
    return {
      viewport: { w: document.documentElement.clientWidth, h: window.innerHeight },
      dialog: rect(d),
      calendar: { ...rect(cal), scrollW: cal.scrollWidth, clientW: cal.clientWidth },
      views: views.map(rect),
      prevPerView: views.map((v) => v.querySelectorAll(".dx-calendar-nav-prev").length),
      nextPerView: views.map((v) => v.querySelectorAll(".dx-calendar-nav-next").length),
    };
  });
}

async function openMultiMonth(page: Page) {
  await gotoDatePicker(page);
  await pickerTrigger(page, "multi_month").click();
  const dialog = pickerContent(page, "multi_month");
  await expect(dialog).toBeVisible();
  await expect(dialog.getByRole("grid")).toHaveCount(2);
  await waitSettled(dialog);
  return dialog;
}

test.describe("Multi-month popover panel (variant=multi_month)", () => {
  for (const scheme of ["light", "dark"] as const) {
    test(`the panel grows to hold both months side by side, with a gap; prev is on the first month, next on the last (${scheme})`, async ({ page }) => {
      await page.setViewportSize({ width: 1280, height: 900 });
      await gotoDatePicker(page);
      await setScheme(page, scheme);
      await pickerTrigger(page, "multi_month").click();
      const dialog = pickerContent(page, "multi_month");
      await expect(dialog).toBeVisible();
      await expect(dialog.getByRole("grid")).toHaveCount(2);
      await waitSettled(dialog);

      const g = await monthGeometry(dialog);
      const debug = JSON.stringify(g);
      expect(g.views, debug).toHaveLength(2);
      // Both months are INSIDE the panel (this was the bug: the 2nd one sat outside it).
      for (const v of g.views) {
        expect(v.left, debug).toBeGreaterThanOrEqual(g.dialog.left - 0.5);
        expect(v.right, debug).toBeLessThanOrEqual(g.dialog.right + 0.5);
      }
      expect(g.calendar.scrollW, `the calendar overflows its own box: ${debug}`).toBeLessThanOrEqual(g.calendar.clientW + 1);
      // Side by side on one row, with a gap between them.
      expect(Math.abs(g.views[0].top - g.views[1].top), debug).toBeLessThanOrEqual(1);
      expect(g.views[1].left - g.views[0].right, debug).toBeGreaterThanOrEqual(8);
      // Nav: prev on the first month only, next on the last month only.
      expect(g.prevPerView, debug).toEqual([1, 0]);
      expect(g.nextPerView, debug).toEqual([0, 1]);
      // The panel is inside the viewport.
      expect(g.dialog.left, debug).toBeGreaterThanOrEqual(-0.5);
      expect(g.dialog.right, debug).toBeLessThanOrEqual(g.viewport.w + 0.5);
      // And it is a real surface in this scheme (not transparent), so the months are readable on it.
      const surface = await dialog.evaluate((d) => getComputedStyle(d).backgroundColor);
      expect(surface).not.toBe("rgba(0, 0, 0, 0)");
    });
  }

  test.describe("at phone width", () => {
    test.use({ viewport: { width: 390, height: 844 } });

    test("the months stack vertically and the panel stays inside the viewport", async ({ page }) => {
      const dialog = await openMultiMonth(page);
      const g = await monthGeometry(dialog);
      const debug = JSON.stringify(g);

      expect(g.views, debug).toHaveLength(2);
      // Stacked: the second month starts below the first, in the same column.
      expect(g.views[1].top, debug).toBeGreaterThanOrEqual(g.views[0].bottom - 0.5);
      expect(Math.abs(g.views[1].left - g.views[0].left), debug).toBeLessThanOrEqual(1);
      // Both inside the panel, which is inside the viewport; nothing scrolls sideways.
      for (const v of g.views) {
        expect(v.left, debug).toBeGreaterThanOrEqual(g.dialog.left - 0.5);
        expect(v.right, debug).toBeLessThanOrEqual(g.dialog.right + 0.5);
      }
      expect(g.calendar.scrollW, debug).toBeLessThanOrEqual(g.calendar.clientW + 1);
      expect(g.dialog.left, debug).toBeGreaterThanOrEqual(-0.5);
      expect(g.dialog.right, debug).toBeLessThanOrEqual(g.viewport.w + 0.5);
      expect(g.dialog.width, debug).toBeLessThanOrEqual(g.viewport.w);
      expect(g.prevPerView, debug).toEqual([1, 0]);
      expect(g.nextPerView, debug).toEqual([0, 1]);
    });
  });
});

/**
 * -----------------------------------------------------------------------
 * Closing after a selection (owner report 3): the popover used to close on the same frame as the
 * click, so nobody saw the selected state. It now holds open for `close_delay` (default 300 ms),
 * then plays its normal exit animation and returns focus to the trigger. `close_on_select: false`
 * keeps it open. The timer is owned by the picker's scope (cancelled on unmount -- covered by the
 * `date_picker` unit tests) and by any press/key inside the picker, month navigation, or a reopen.
 * -----------------------------------------------------------------------
 */
const DWELL_MIN_MS = 280; // 300 ms default, less a little clock slack
const CLOSE_MAX_MS = 10_000; // "closes eventually, not stuck" -- loose on purpose, a loaded CI box stretches timers

test.describe("Closing after a selection: dwell, then close", () => {
  test("a day click keeps the popover open with the day shown selected, then closes it after the dwell and returns focus to the trigger", async ({ page }) => {
    await gotoDatePicker(page);
    await pickerTrigger(page).click();
    const dialog = pickerContent(page);
    await expect(dialog).toBeVisible();
    await armDwellProbe(dialog);

    await dayCell(dialog, 15).click();

    await expect(dialog).toBeHidden({ timeout: 15000 });
    const t = await readDwell(page);
    const debug = JSON.stringify(t);
    expect(t.selected, `the day never showed as selected: ${debug}`).toBeGreaterThan(0);
    expect(t.stateAtSelected, `the popover was already closing when the day showed as selected: ${debug}`).toBe("open");
    expect(t.held, `held open ${t.held} ms: ${debug}`).toBeGreaterThanOrEqual(DWELL_MIN_MS);
    expect(t.held, debug).toBeLessThanOrEqual(CLOSE_MAX_MS);
    // The value was committed on the click, not on close, and focus is back on the trigger.
    await expect(segment(page, "day")).toHaveText("15");
    await expect(pickerTrigger(page)).toBeFocused();
  });

  test("keyboard selection (Enter on a day) dwells the same way", async ({ page }) => {
    await gotoDatePicker(page);
    await pickerTrigger(page).click();
    const dialog = pickerContent(page);
    await expect(dialog).toBeVisible();
    await armDwellProbe(dialog);

    await dayCell(dialog, 20).focus();
    await page.keyboard.press("Enter");

    await expect(dialog).toBeHidden({ timeout: 15000 });
    const t = await readDwell(page);
    const debug = JSON.stringify(t);
    expect(t.stateAtSelected, debug).toBe("open");
    expect(t.held, debug).toBeGreaterThanOrEqual(DWELL_MIN_MS);
    expect(t.held, debug).toBeLessThanOrEqual(CLOSE_MAX_MS);
    await expect(segment(page, "day")).toHaveText("20");
    await expect(pickerTrigger(page)).toBeFocused();
  });

  test("Space on a day dwells the same way", async ({ page }) => {
    await gotoDatePicker(page);
    await pickerTrigger(page).click();
    const dialog = pickerContent(page);
    await expect(dialog).toBeVisible();
    await armDwellProbe(dialog);

    await dayCell(dialog, 21).focus();
    await page.keyboard.press("Space");

    await expect(dialog).toBeHidden({ timeout: 15000 });
    const t = await readDwell(page);
    const debug = JSON.stringify(t);
    expect(t.stateAtSelected, debug).toBe("open");
    expect(t.held, debug).toBeGreaterThanOrEqual(DWELL_MIN_MS);
    expect(t.held, debug).toBeLessThanOrEqual(CLOSE_MAX_MS);
  });

  test("range picker: the first click (the start) never closes it; the end click completes the range, then it dwells and closes", async ({ page }) => {
    await gotoDatePicker(page);
    await pickerTrigger(page, "range").click();
    const dialog = pickerContent(page, "range");
    await expect(dialog).toBeVisible();
    await armDwellProbe(dialog);

    await dayCell(dialog, 5).click();
    // Well past the dwell: an incomplete range must not close the picker.
    await page.waitForTimeout(DWELL_MIN_MS * 3);
    await expect(dialog).toHaveAttribute("data-state", "open");

    await dayCell(dialog, 10).click();
    await expect(dialog).toBeHidden({ timeout: 15000 });
    const t = await readDwell(page);
    const debug = JSON.stringify(t);
    expect(t.held, `measured from the END click: ${debug}`).toBeGreaterThanOrEqual(DWELL_MIN_MS);
    expect(t.held, debug).toBeLessThanOrEqual(CLOSE_MAX_MS);
    await expect(rangeSegment(page, "start", "day", "range")).toHaveText("05");
    await expect(rangeSegment(page, "end", "day", "range")).toHaveText("10");
    await expect(pickerTrigger(page, "range")).toBeFocused();
  });

  test("a key pressed inside the picker during the dwell cancels the pending close (it does not close under the user)", async ({ page }) => {
    await gotoDatePicker(page);
    await pickerTrigger(page).click();
    const dialog = pickerContent(page);
    await expect(dialog).toBeVisible();

    // Select, then the user takes over 20 ms later (one page.evaluate: the gap is exactly that).
    await runSteps(dialog, [{ click: 12 }, { wait: 20 }, { key: "ArrowRight", on: 12 }]);
    await page.waitForTimeout(DWELL_MIN_MS * 3);
    await expect(dialog).toHaveAttribute("data-state", "open");
    await expect(dayCell(dialog, 12)).toHaveAttribute("data-selected", "true");

    // Escape still closes it.
    await page.keyboard.press("Escape");
    await expect(dialog).toBeHidden({ timeout: 15000 });
  });

  test("clicking the day that was just selected again (a double-click) keeps the selection instead of toggling it off", async ({ page }) => {
    await gotoDatePicker(page);
    await pickerTrigger(page).click();
    const dialog = pickerContent(page);
    await expect(dialog).toBeVisible();

    await runSteps(dialog, [{ click: 17 }, { wait: 80 }, { click: 17 }]); // inside the dwell
    await expect(dayCell(dialog, 17)).toHaveAttribute("data-selected", "true");
    await expect(segment(page, "day")).toHaveText("17");
    await expect(dialog).toBeHidden({ timeout: 15000 });
    await expect(segment(page, "day")).toHaveText("17"); // and still set once it has closed
  });

  test("range picker: starting a NEW range during the dwell of the previous one cancels the close", async ({ page }) => {
    await gotoDatePicker(page);
    await pickerTrigger(page, "range").click();
    const dialog = pickerContent(page, "range");
    await expect(dialog).toBeVisible();

    // 5..10 completes (a close is now pending); 40 ms later the user presses on 15, which only anchors a new range.
    await runSteps(dialog, [{ click: 5 }, { wait: 20 }, { click: 10 }, { wait: 40 }, { pointerdown: 15 }, { click: 15 }]);
    await page.waitForTimeout(DWELL_MIN_MS * 3);
    await expect(dialog).toHaveAttribute("data-state", "open"); // not closed under a half-picked range
    await expect(dayCell(dialog, 15)).toHaveAttribute("data-selected", "true");

    // Escape abandons the half-picked range (RangeCalendar), and a second Escape closes the popover.
    await page.keyboard.press("Escape");
    await page.keyboard.press("Escape");
    await expect(dialog).toBeHidden({ timeout: 15000 });
  });

  test("closing and reopening during the dwell: the pending close does not land on the reopened popover", async ({ page }) => {
    await gotoDatePicker(page);
    await pickerTrigger(page).click();
    const dialog = pickerContent(page);
    await expect(dialog).toBeVisible();

    // Select, then close through the trigger 20 ms later, inside the dwell.
    await runSteps(dialog, [{ click: 9 }, { wait: 20 }, { clickTrigger: true }]);
    await expect(dialog).toBeHidden({ timeout: 15000 });
    await pickerTrigger(page).click(); // reopen: the original deadline is still to come, or has just passed
    await expect(dialog).toBeVisible();
    await page.waitForTimeout(DWELL_MIN_MS * 3);
    await expect(dialog).toHaveAttribute("data-state", "open");
  });

  test("selecting another day during the dwell restarts it, and the popover closes once, measured from the last click", async ({ page }) => {
    await gotoDatePicker(page);
    await pickerTrigger(page).click();
    const dialog = pickerContent(page);
    await expect(dialog).toBeVisible();
    await armDwellProbe(dialog);

    await runSteps(dialog, [{ click: 8 }, { wait: 150 }, { click: 11 }]);
    await expect(dialog).toBeHidden({ timeout: 15000 });
    const t = await readDwell(page);
    expect(t.held, `measured from the LAST click: ${JSON.stringify(t)}`).toBeGreaterThanOrEqual(DWELL_MIN_MS);
    await expect(segment(page, "day")).toHaveText("11");
  });
});

test.describe("close_on_select: false (variant=keep_open)", () => {
  test("a date pick keeps the popover open; Escape, the trigger and an outside click still close it", async ({ page }) => {
    await gotoDatePicker(page);
    const first = pickerTrigger(page, "keep_open");
    await first.click();
    const dialog = pickerContent(page, "keep_open");
    await expect(dialog).toBeVisible();

    await dayCell(dialog, 14).click();
    await expect(dayCell(dialog, 14)).toHaveAttribute("data-selected", "true");
    await page.waitForTimeout(DWELL_MIN_MS * 3);
    await expect(dialog).toHaveAttribute("data-state", "open"); // well past the default dwell

    // Another date can be tried without reopening.
    await dayCell(dialog, 16).click();
    await expect(dayCell(dialog, 16)).toHaveAttribute("data-selected", "true");
    await page.waitForTimeout(DWELL_MIN_MS * 2);
    await expect(dialog).toHaveAttribute("data-state", "open");

    await page.keyboard.press("Escape");
    await expect(dialog).toBeHidden({ timeout: 15000 });

    // The trigger closes it...
    await first.click();
    await expect(dialog).toBeVisible();
    await first.click();
    await expect(dialog).toBeHidden({ timeout: 15000 });

    // ...and so does a click outside.
    await first.click();
    await expect(dialog).toBeVisible();
    await page.getByRole("heading", { level: 1 }).first().click();
    await expect(dialog).toBeHidden({ timeout: 15000 });
  });
});

/**
 * -----------------------------------------------------------------------
 * Open/close animation (owner report 5). The popover centres itself on its trigger with
 * `transform: translateX(-50%)` (top_layer.rs's engine stylesheet). The date picker's fade used to
 * animate `transform: translateY(...)` too, which REPLACES that for the length of the animation: the
 * panel played its fade half a panel-width to the right of where it settles (144 px for one month)
 * and snapped back on the last frame -- on every open and every close. The fade now animates
 * `translate`, which composes with `transform`.
 * -----------------------------------------------------------------------
 */
test.describe("Open/close animation", () => {
  test("the panel's horizontal centre does not move at all while it fades in and out (the animation must not replace the anchor centring)", async ({ page }) => {
    await gotoDatePicker(page);
    const dialog = content(page);

    // Warm-up open: the calendar's own stylesheet is delivered lazily on the first open (a separate,
    // first-open-only matter), which changes the panel's size mid-open. Measure a settled open.
    await trigger(page).click();
    await expect(dialog).toBeVisible();
    await waitSettled(dialog);
    await page.keyboard.press("Escape");
    await expect(dialog).toBeHidden();

    await page.evaluate(() => {
      const w = window as unknown as { __cx: number[]; __stop: boolean };
      w.__cx = [];
      w.__stop = false;
      const tick = () => {
        const d = document.querySelector(".dx-date-picker-popover-content");
        if (d) {
          const b = d.getBoundingClientRect();
          if (b.width) w.__cx.push(b.x + b.width / 2);
        }
        if (!w.__stop) requestAnimationFrame(tick);
      };
      requestAnimationFrame(tick);
    });
    await trigger(page).click();
    await expect(dialog).toBeVisible();
    await waitSettled(dialog);
    await page.keyboard.press("Escape");
    await expect(dialog).toBeHidden();
    const cx = await page.evaluate(() => {
      const w = window as unknown as { __cx: number[]; __stop: boolean };
      w.__stop = true;
      return w.__cx;
    });

    expect(cx.length, "sampled frames across the open and the close").toBeGreaterThan(6);
    const spread = Math.max(...cx) - Math.min(...cx);
    expect(spread, `centre x moved by ${spread}px across the fade: ${JSON.stringify(cx.map(Math.round))}`).toBeLessThanOrEqual(1.5);
  });
});
