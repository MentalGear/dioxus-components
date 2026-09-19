/**
 * ORACLE: tier 2 (HTML) -- main-thread responsiveness (INP / long tasks).
 *
 * Source: dev-docs/backlog.md row 45, drawing on
 * dev-docs/recommended-implementations.md §11 ("Main-thread responsiveness
 * (interaction latency)", 2026-09-03), which is itself built on kciter,
 * "The Browser's Main Thread Is Expensive"
 * (https://kciter.so/posts/the-expensive-main-thread/en/). §11's own
 * words: "Install a PerformanceObserver for longtask (and event timing
 * with durationThreshold) before an interaction and assert no task over
 * 50 ms and no interaction over 200 ms fired during open/close of each
 * overlay ... Until it exists, rules 1-5 are enforced by review only."
 * This file is that oracle -- rules 1-5 (no same-handler layout
 * read+write, passive scroll/touch listeners unless preventDefault,
 * compositor-only animation unless bounded+sampled, no busy-wait, heavy
 * work kept out of render) stay review-enforced; this file is the
 * mechanical backstop that would have caught the two live incidents this
 * *type* of bug has already produced.
 *
 * Normative sources for the two APIs this file drives directly (both
 * verified reachable, HTTP 200, checked 2026-09-18) -- see
 * `../../main-thread.ts`'s header for the exact interfaces/attributes
 * each one contributes to `GestureReport`:
 *   - W3C Long Tasks API -- a "long task" is, verbatim, "any of the
 *     following occurrences whose duration exceeds 50ms": an event-loop
 *     task (plus its microtask checkpoint), an "update the rendering"
 *     step, or a pause between event-loop steps. `PerformanceLongTaskTiming
 *     .attribution` names the culprit frame/container:
 *     https://www.w3.org/TR/longtasks-1/#sec-PerformanceLongTaskTiming
 *     https://www.w3.org/TR/longtasks-1/#sec-TaskAttributionTiming
 *   - W3C Event Timing API -- `durationThreshold` on `PerformanceObserverInit`
 *     ("Should add PerformanceEventTiming" clamps it: "Let minDuration be
 *     the maximum between 16 and options's durationThreshold value" -- 16
 *     is therefore the floor this file passes, not an arbitrary choice),
 *     `interactionId` (nonzero only for a discrete user interaction; 0 for
 *     e.g. a `pointermove`/`mousemove` mid-drag -- which is exactly the
 *     entry type that would have caught row 73, see below), and
 *     `processingStart`/`processingEnd`:
 *     https://www.w3.org/TR/event-timing/#dom-performanceobserverinit-durationthreshold
 *     https://www.w3.org/TR/event-timing/#dom-performanceeventtiming-interactionid
 *
 * What this would have caught (the trigger that fired this row twice in
 * one month):
 *   - Row 73 (drawer drag hang): `DrawerContent`'s drag-continuation
 *     `use_effect` read `raw_offset()` with tracked syntax and then wrote
 *     that same signal in the same run, re-triggering itself forever from
 *     the first `pointerdown` -- the tab's whole main thread wedged, and
 *     the live integration run's own `page.mouse.down()` hung the 5-minute
 *     ceiling with no panic, no console output. This oracle's drawer-drag
 *     test drives the *identical* gesture shape (`page.mouse.down()` then
 *     several `mouse.move` steps) that hung, under a `test.info()` timeout
 *     far shorter than 5 minutes, with a `longtask` observer that was
 *     already running before the first `pointerdown` -- an infinite
 *     synchronous re-trigger loop is, definitionally, one arbitrarily long
 *     task, and would have failed this file's `maxTaskMs <= 50`
 *     assertion (or simply timed out the test, which is its own kind of
 *     red) long before a live device report or a 5-minute CI hang did.
 *   - Row 74 (first-render panic): out of this file's own scope (a wasm
 *     `unreachable` trap is not a slow task, it is a crash), but the same
 *     round surfaced the general hazard this file's subject list has to
 *     respect -- see "A note on row 74's other lesson" below.
 *
 * Subjects (per this row's own scope: every overlay's open/close, every
 * form control's primary interaction), all driven against the shared
 * read-only dev server at :8080 -- see `OVERLAY_SUBJECTS`/
 * `FORM_CONTROL_SUBJECTS` below for the full, current list and exactly
 * which gesture/locator each one uses. Locators are read from each
 * component's own existing spec (dialog.spec.ts, slider.spec.ts, etc.) as
 * a guide -- proven working against this exact app -- but this file
 * declares its own copies rather than importing them, per this row's
 * "keep this file independent" instruction, so a change to another spec's
 * internals cannot silently change what this oracle measures.
 *
 * A note on row 74's other lesson ("one bad demo takes down /", and more
 * generally: unscoped accessible-role/name queries can silently resolve
 * against the site's own persistent chrome instead of the demo). This
 * file hit a live instance of the same *class* while being written,
 * independent of row 74's own two root causes: on `/component/?name=
 * virtual_list&`, the persistent sidebar navigation renders seven of its
 * own `<ul>` elements (implicit `role="list"`) before the actual demo's
 * two `.dx-virtual-list-container` elements in DOM order, so a bare
 * `page.getByRole("list").first()` -- confirmed live, 2026-09-18, against
 * the integration server that already carries the batch-2 Sidebar lane --
 * silently resolves to a 68px-tall sidebar menu, not the scrollable
 * virtualized list; scrolling it is consequently a no-op, provably (a
 * real wheel gesture on the *correct* element moves `scrollTop` from 0 to
 * 800; the same gesture through `.first()` never moves it). This file's
 * own virtual_list subject therefore scopes by the demo's own CSS class
 * (`.dx-virtual-list-container`) instead. `playwright/virtual_list.spec.ts`
 * itself still uses the unscoped `.getByRole("list").first()` form (this
 * lane does not own that file, so it is reported rather than fixed here)
 * -- worth a backlog line: any *sibling* spec using an unscoped
 * `getByRole("list")` against a route that also renders the sidebar
 * should be re-checked now that the sidebar (batch 2) has landed, the
 * same way row 74 already had to rescope `getByRole('link', ...)` once
 * the persistent chrome grew a "Docs" link.
 *
 * Robustness under load (design requirement, not an afterthought -- this
 * sandbox runs several lanes' builds/servers concurrently, so a single
 * scheduling hiccup landing inside a gesture's measurement window is a
 * real, expected occurrence, not a hypothetical): every gesture goes
 * through `assertResponsive` (`../../main-thread.ts`), which runs it up
 * to 3 times and accepts the first attempt that clears both thresholds --
 * a genuine main-thread block (row 73's shape: an infinite synchronous
 * re-trigger loop) reproduces on every single attempt, deterministically,
 * because it is a property of the code, not of scheduling; a scheduling
 * hiccup (another lane's `rustc`/`dx` competing for the same 4 cores for
 * one unlucky frame) does not reproduce on a clean retry. This makes the
 * retry a strictly *more* trustworthy signal than a single run, not a
 * weaker one -- and the thresholds themselves (`LONG_TASK_THRESHOLD_MS`,
 * `INTERACTION_THRESHOLD_MS` in main-thread.ts) are never lowered to
 * compensate; only the number of chances a real pass gets to show up
 * changes. Every test records its measured maxima via
 * `test.info().annotations` regardless of outcome, so a green run's exact
 * numbers stay visible in the HTML report rather than disappearing into
 * "passed". This file also recommends running it with `PW_WORKERS=1` --
 * two Chromium tabs each opening DevTools-protocol connections and firing
 * gestures at the same 4 cores this sandbox's builds are also contending
 * for is exactly the kind of shared-resource noise the retry policy
 * exists to absorb, not a reason to add a second, self-inflicted source
 * of it. Deliberately not `test.describe.configure({ mode: "serial" })`
 * for this -- see the comment just above `test.beforeEach` further down
 * this file for why that mode's other effect
 * (stop-the-file-on-first-failure) is the wrong choice for an inventory
 * file where every subject's result is independently meaningful.
 *
 * Red proof (this row's own requirement): proven without a build, by
 * temporarily injecting a synthetic busy handler via a throwaway script
 * (kept out of this committed file entirely, per this row's own
 * instruction) -- see this lane's report for the exact busy-handler code
 * and both red transcripts. Summary: (a) a `pointerdown` listener that
 * spins 120ms (over the 50ms long-task threshold) turns the drawer-drag
 * and slider-drag tests red on `maxTaskMs`; (b) a `click` handler that
 * spins 250ms (over the 200ms interaction threshold) turns a click-based
 * test red on `maxInteractionMs`. Neither change is present in this file
 * or in `../../main-thread.ts` -- there is no env var or flag anywhere in
 * the committed code path that can re-enable it.
 *
 * Chromium-only, deliberately (row 45's own caveat, repeated here because
 * it changes this file's shape, not just its README entry): the Long
 * Tasks and Event Timing APIs are not implemented the same way in Firefox
 * or WebKit as of this writing, so `test.beforeEach` below skips both
 * non-Chromium projects outright rather than silently recording all-zero
 * (vacuously "passing") reports for them -- a skip is an honest "not
 * measured here"; a silent zero would read as a false "measured and
 * fine". The numbers this file produces are therefore a Chromium
 * regression guard, not an iOS/Safari measurement -- the same caveat every
 * other tier-2 oracle in this suite already carries.
 */
import { test, expect, type Page } from "@playwright/test";
import {
  installMainThreadObserver,
  assertResponsive,
  type GestureReport,
} from "../../main-thread";

const BASE_URL = "http://127.0.0.1:8080";
const NAV_TIMEOUT = 20 * 60 * 1000; // first run compiles the app -- same budget every sibling oracle uses

const url = (name: string, extra = "") => `${BASE_URL}/component/?name=${name}&${extra}`;

async function goto(page: Page, name: string, extra = ""): Promise<void> {
  await page.goto(url(name, extra), { timeout: NAV_TIMEOUT, waitUntil: "networkidle" });
}

// One worker, one gesture at a time is STRONGLY recommended for this file:
// `PW_WORKERS=1` (session.local.config.ts's own env hook). Two Chromium
// tabs each dispatching real pointer/keyboard input and running their own
// PerformanceObserver callbacks would compete for the same handful of
// cores this sandbox's concurrent `dx`/`rustc` builds are already
// contending for -- exactly the kind of noise `assertResponsive`'s retry
// exists to absorb, not a second, self-inflicted source of it.
//
// Deliberately NOT `test.describe.configure({ mode: "serial" })`: that
// mode's well-documented other effect is stopping the whole file on the
// first failure and skipping every remaining test (observed directly
// while writing this file -- one genuine/noise-induced red on subject 21
// of 58 silently skipped the other 37, which is exactly the wrong
// behaviour for a broad regression-guard inventory where every subject's
// own pass/fail is independently meaningful). Every subject here is
// fully independent (its own `goto`), so the only property wanted is "not
// concurrent with each other", which `PW_WORKERS=1` already gives without
// that side effect.

test.beforeEach(async ({ browserName }) => {
  test.skip(
    browserName !== "chromium",
    "Long Tasks API / Event Timing API are Chromium-only in this sandbox (dev-docs/backlog.md row 45; " +
      "dev-docs/recommended-implementations.md §11) -- skipped, not silently reported as a passing zero."
  );
});

// ---------------------------------------------------------------------------
// Overlays: every open AND close is its own gesture/measurement, per this
// row's own wording ("before each overlay's open/close").
// ---------------------------------------------------------------------------

interface OverlaySubject {
  name: string;
  goto: (page: Page) => Promise<void>;
  /** Performs the opening gesture AND waits for the open state -- so a
   * failure to open is a normal test failure, not a false pass/timeout
   * inside the main-thread measurement itself. */
  open: (page: Page) => Promise<void>;
  /** Performs the closing gesture AND waits for the closed state. */
  close: (page: Page) => Promise<void>;
}

const menubarFileMenu = (page: Page) =>
  page.getByRole("menu").filter({ has: page.getByRole("menuitem", { name: "New" }) }).last();

const navbarInputsMenu = (page: Page) =>
  page.getByRole("menu").filter({ has: page.getByRole("menuitem", { name: "Inputs" }) }).first();

const navigationMenuNav = (page: Page) => page.getByRole("navigation", { name: "Component navigation menu" });

const selectTrigger = (page: Page) => page.getByRole("combobox").filter({ hasText: /Select an option|Apple|Banana/ });

const comboboxInput = (page: Page) => page.getByRole("combobox", { name: "Select framework" });
const comboboxListbox = (page: Page) => page.locator("[role='listbox'][data-state='open']");

const toastCloseButton = (page: Page) => page.getByRole("button", { name: "close", exact: true }).first();

const OVERLAY_SUBJECTS: OverlaySubject[] = [
  {
    name: "dialog",
    goto: (page) => goto(page, "dialog"),
    open: async (page) => {
      await page.getByRole("button", { name: "Show Dialog" }).click();
      await expect(page.getByRole("dialog")).toBeVisible();
    },
    close: async (page) => {
      await page.keyboard.press("Escape");
      await expect(page.getByRole("dialog")).toHaveCount(0);
    },
  },
  {
    name: "alert_dialog",
    goto: (page) => goto(page, "alert_dialog"),
    open: async (page) => {
      await page.getByRole("button", { name: "Show Alert Dialog" }).click();
      await expect(page.getByRole("alertdialog")).toBeVisible();
    },
    close: async (page) => {
      await page.keyboard.press("Escape");
      await expect(page.getByRole("alertdialog")).toHaveCount(0);
    },
  },
  {
    name: "sheet",
    goto: (page) => goto(page, "sheet"),
    open: async (page) => {
      await page.getByRole("button", { name: "Right" }).click();
      await expect(page.locator('[data-slot="sheet-root"]')).toHaveAttribute("data-state", "open");
    },
    close: async (page) => {
      await page.keyboard.press("Escape");
      await expect(page.locator('[data-slot="sheet-root"]')).toHaveCount(0);
    },
  },
  {
    name: "drawer",
    goto: (page) => goto(page, "drawer"),
    open: async (page) => {
      await page.getByRole("button", { name: "Move Goal" }).click();
      await expect(page.locator('[data-slot="drawer-root"]')).toHaveAttribute("data-state", "open");
    },
    close: async (page) => {
      await page.keyboard.press("Escape");
      await expect(page.locator('[data-slot="drawer-root"]')).toHaveCount(0);
    },
  },
  {
    name: "popover",
    goto: (page) => goto(page, "popover"),
    open: async (page) => {
      await page.getByRole("button", { name: "Show Popover" }).click();
      await expect(page.getByRole("dialog")).toBeVisible();
    },
    close: async (page) => {
      await page.keyboard.press("Escape");
      await expect(page.getByRole("dialog")).toHaveCount(0);
    },
  },
  {
    // Non-modal arm: native `popover="auto"`, not the `<dialog>` engine the
    // main variant above uses (see popover.spec.ts's own close-fade-animation
    // comments) -- Escape closes a light-dismissible native popover per the
    // HTML popover API regardless, confirmed live (2026-09-18: dialog count
    // 1 -> 0 on Escape) before being relied on here.
    name: "popover_non_modal",
    goto: (page) => goto(page, "popover", "variant=non_modal&"),
    open: async (page) => {
      await page.getByRole("button", { name: "Open popover" }).click();
      await expect(page.getByRole("dialog")).toBeVisible();
    },
    close: async (page) => {
      await page.keyboard.press("Escape");
      await expect(page.getByRole("dialog")).toHaveCount(0);
    },
  },
  {
    name: "dropdown_menu",
    goto: (page) => goto(page, "dropdown_menu"),
    open: async (page) => {
      const trigger = page.getByRole("button", { name: "Open Menu" });
      await trigger.click();
      await expect(trigger).toHaveAttribute("data-state", "open");
    },
    close: async (page) => {
      const trigger = page.getByRole("button", { name: "Open Menu" });
      await page.keyboard.press("Escape");
      await expect(trigger).toHaveAttribute("data-state", "closed");
    },
  },
  {
    name: "context_menu",
    goto: (page) => goto(page, "context_menu"),
    open: async (page) => {
      await page.getByRole("button", { name: "right click here" }).click({ button: "right" });
      await expect(page.getByRole("menu")).toHaveAttribute("data-state", "open");
    },
    close: async (page) => {
      await page.keyboard.press("Escape");
      await expect(page.getByRole("menu")).toHaveCount(0);
    },
  },
  {
    // menubar's own page also carries other (closed) role=menu nodes --
    // confirmed live (baseline count 2, 3 with File open, 2 again after
    // Escape) -- so this scopes by content the way menubar.spec.ts's own
    // `fileMenuContent` does, rather than counting `role=menu` directly.
    name: "menubar",
    goto: (page) => goto(page, "menubar"),
    open: async (page) => {
      await page.getByRole("menuitem", { name: "File" }).click();
      await expect(menubarFileMenu(page)).toHaveAttribute("data-state", "open");
    },
    close: async (page) => {
      await page.keyboard.press("Escape");
      await expect(menubarFileMenu(page)).toHaveCount(0);
    },
  },
  {
    name: "select",
    goto: (page) => goto(page, "select"),
    open: async (page) => {
      await selectTrigger(page).click();
      await expect(page.getByRole("listbox")).toHaveAttribute("data-state", "open");
    },
    close: async (page) => {
      await page.keyboard.press("Escape");
      await expect(page.getByRole("listbox")).toHaveCount(0);
    },
  },
  {
    // Opens on ArrowDown from the focused input, not a click -- matches
    // combobox.spec.ts's own "opens from the focused input with the
    // keyboard" test.
    name: "combobox",
    goto: (page) => goto(page, "combobox"),
    open: async (page) => {
      await comboboxInput(page).focus();
      await page.keyboard.press("ArrowDown");
      await expect(comboboxListbox(page)).toBeVisible();
    },
    close: async (page) => {
      await page.keyboard.press("Escape");
      await expect(comboboxListbox(page)).toHaveCount(0);
    },
  },
  {
    name: "command",
    goto: (page) => goto(page, "command"),
    open: async (page) => {
      await page.getByRole("button", { name: "Open Command Palette" }).click();
      await expect(page.getByRole("dialog")).toBeVisible();
    },
    close: async (page) => {
      await page.keyboard.press("Escape");
      await expect(page.getByRole("dialog")).toHaveCount(0);
    },
  },
  {
    name: "tooltip",
    goto: (page) => goto(page, "tooltip"),
    open: async (page) => {
      await page.locator("#component-preview-frame").first().getByText("Rich content").hover();
      await expect(page.getByRole("tooltip")).toBeVisible();
    },
    close: async (page) => {
      // Same close trigger tooltip.spec.ts's own tests use: move away, not Escape.
      await page.mouse.move(0, 0);
      await expect(page.getByRole("tooltip")).toHaveCount(0);
    },
  },
  {
    name: "hover_card",
    goto: (page) => goto(page, "hover_card"),
    open: async (page) => {
      await page.getByRole("button", { name: "Dioxus" }).hover();
      await expect(page.getByRole("tooltip")).toBeVisible();
    },
    close: async (page) => {
      await page.mouse.move(0, 0);
      await expect(page.getByRole("tooltip")).toHaveCount(0);
    },
  },
  {
    // Hover-driven disclosure, not click -- matches navigation_menu.spec.ts's
    // own "hover opens the panel" / "pointer leaving ... closes the panel".
    name: "navigation_menu",
    goto: (page) => goto(page, "navigation_menu"),
    open: async (page) => {
      const trigger = navigationMenuNav(page).getByRole("button", { name: "Getting started" });
      await trigger.hover();
      await expect(trigger).toHaveAttribute("aria-expanded", "true");
    },
    close: async (page) => {
      const trigger = navigationMenuNav(page).getByRole("button", { name: "Getting started" });
      await page.getByRole("heading", { level: 1 }).hover();
      await expect(trigger).toHaveAttribute("aria-expanded", "false");
    },
  },
  {
    name: "date_picker",
    goto: (page) => goto(page, "date_picker"),
    open: async (page) => {
      await page.getByRole("button", { name: "Show Calendar" }).first().click();
      await expect(page.getByRole("dialog")).toBeVisible();
    },
    close: async (page) => {
      await page.keyboard.press("Escape");
      await expect(page.getByRole("dialog")).toHaveCount(0);
    },
  },
  {
    name: "color_picker",
    goto: (page) => goto(page, "color_picker"),
    open: async (page) => {
      await page.getByRole("button", { name: /Color picker/i }).first().click();
      await expect(page.getByRole("dialog")).toBeVisible();
    },
    close: async (page) => {
      await page.keyboard.press("Escape");
      await expect(page.getByRole("dialog")).toHaveCount(0);
    },
  },
  {
    // Closed via its own close button, not Escape -- toast.spec.ts never
    // tests Escape either.
    name: "toast",
    goto: (page) => goto(page, "toast"),
    open: async (page) => {
      await page.getByRole("button", { name: "Info (60s)" }).click();
      await expect(toastCloseButton(page)).toBeVisible();
    },
    close: async (page) => {
      await toastCloseButton(page).click();
      await expect(page.getByRole("button", { name: "close", exact: true })).toHaveCount(0);
    },
  },
  {
    name: "navbar",
    goto: (page) => goto(page, "navbar"),
    open: async (page) => {
      await navbarInputsMenu(page).hover();
      await expect(navbarInputsMenu(page)).toHaveAttribute("data-state", "open");
    },
    close: async (page) => {
      // Confirmed live (2026-09-18): Escape closes the navbar dropdown the
      // same way it closes dropdown_menu/context_menu/menubar -- not
      // exercised by navbar.spec.ts itself, which only tests navigation
      // away (a real click that leaves the page), not in-place closing.
      await page.keyboard.press("Escape");
      await expect(navbarInputsMenu(page)).toHaveAttribute("data-state", "closed");
    },
  },
];

for (const subject of OVERLAY_SUBJECTS) {
  test(`${subject.name}: open is responsive`, async ({ page }) => {
    await installMainThreadObserver(page);
    await subject.goto(page);
    const report = await assertResponsive(page, `${subject.name} open`, () => subject.open(page), {
      reset: () => subject.goto(page),
    });
    recordReport(subject.name, "open", report);
  });

  test(`${subject.name}: close is responsive`, async ({ page }) => {
    await installMainThreadObserver(page);
    const toOpenState = async () => {
      await subject.goto(page);
      await subject.open(page);
    };
    await toOpenState();
    const report = await assertResponsive(page, `${subject.name} close`, () => subject.close(page), {
      reset: toOpenState,
    });
    recordReport(subject.name, "close", report);
  });
}

// ---------------------------------------------------------------------------
// Form controls: one primary-interaction gesture each, per this row's own
// wording. `slider` and `data_table` each name two gestures explicitly, so
// each gets two entries here.
// ---------------------------------------------------------------------------

interface FormControlSubject {
  name: string;
  goto: (page: Page) => Promise<void>;
  /** Performs the gesture AND a minimal sanity assertion that it actually
   * engaged the component -- so a broken locator is a normal test
   * failure, not a silently-vacuous main-thread measurement. */
  interact: (page: Page) => Promise<void>;
}

/** Mirrors slider.spec.ts's own `sliderGroup`/`sliderTrack`/`sliderTrackPoint`
 * helpers (proven against this exact app) -- copied, not imported, per
 * this row's "keep this file independent" instruction. */
function sliderGroup(page: Page, name: string | RegExp) {
  return page
    .getByRole("slider", { name })
    .first()
    .locator('xpath=ancestor::*[@role="group" and @data-orientation="horizontal"][1]');
}
function sliderTrack(slider: ReturnType<typeof sliderGroup>) {
  return slider.locator('div[data-orientation="horizontal"]:has([role="slider"])').first();
}
async function sliderTrackPoint(track: ReturnType<typeof sliderTrack>, frac: number) {
  const box = await track.boundingBox();
  if (!box) throw new Error("slider track has no bounding box");
  return { x: box.x + box.width * frac, y: box.y + box.height / 2 };
}

const FORM_CONTROL_SUBJECTS: FormControlSubject[] = [
  {
    // Real pointer sequence, several `mouse.move` steps -- one of the two
    // gestures this row names as motivating (drawer drag is the other).
    name: "slider (drag)",
    goto: (page) => goto(page, "slider"),
    interact: async (page) => {
      const thumb = page.getByRole("slider", { name: "Demo Slider" });
      const track = sliderTrack(sliderGroup(page, "Demo Slider"));
      const start = await sliderTrackPoint(track, 0.5);
      const end = await sliderTrackPoint(track, 0.8);
      await page.mouse.move(start.x, start.y);
      await page.mouse.down();
      await page.mouse.move(end.x, end.y, { steps: 10 });
      await page.mouse.up();
      await expect(thumb).toHaveAttribute("aria-valuenow", "80");
    },
  },
  {
    name: "slider (arrow keys)",
    goto: (page) => goto(page, "slider"),
    interact: async (page) => {
      const thumb = page.getByRole("slider", { name: "Demo Slider" });
      await thumb.focus();
      await page.keyboard.press("ArrowRight");
      await page.keyboard.press("ArrowRight");
      await page.keyboard.press("ArrowRight");
      await expect(thumb).toHaveAttribute("aria-valuenow", "53");
    },
  },
  {
    name: "switch",
    goto: (page) => goto(page, "switch"),
    interact: async (page) => {
      const el = page.getByRole("switch", { name: "Switch Demo" });
      await el.click();
      await expect(el).toBeChecked();
    },
  },
  {
    name: "checkbox",
    goto: (page) => goto(page, "checkbox"),
    interact: async (page) => {
      const el = page.getByRole("checkbox", { name: "Demo Checkbox" });
      await el.click();
      await expect(el).toBeChecked();
    },
  },
  {
    name: "radio_group",
    goto: (page) => goto(page, "radio_group"),
    interact: async (page) => {
      const el = page.getByRole("radio", { name: "Blue" });
      await el.click();
      await expect(el).toBeChecked();
    },
  },
  {
    name: "tabs",
    goto: (page) => goto(page, "tabs"),
    interact: async (page) => {
      // Scoped exactly as tabs.spec.ts's own tests are, since this page's
      // "Variants" section renders a second, unrelated Tabs instance.
      const activeTab = page
        .locator('[role="tabpanel"][data-state="active"]:not(#component-preview-frame)')
        .filter({ hasText: /^Tab \d Content$/ });
      await page.getByRole("tab", { name: "Tab 2" }).click();
      await expect(activeTab).toContainText("Tab 2 Content");
    },
  },
  {
    name: "toggle",
    goto: (page) => goto(page, "toggle"),
    interact: async (page) => {
      const el = page.getByRole("button", { name: "B", exact: true });
      await el.click();
      await expect(el).toHaveAttribute("data-state", "on");
    },
  },
  {
    name: "toggle_group",
    goto: (page) => goto(page, "toggle_group"),
    interact: async (page) => {
      const el = page.getByRole("button", { name: "B", exact: true });
      await el.click();
      await expect(el).toHaveAttribute("data-state", "on");
    },
  },
  {
    name: "input_otp (typing)",
    goto: (page) => goto(page, "input_otp"),
    interact: async (page) => {
      const input = page.locator("#otp-main");
      await input.click();
      await page.keyboard.type("123456");
      await expect(input).toHaveValue("123456");
    },
  },
  {
    name: "accordion",
    goto: (page) => goto(page, "accordion"),
    interact: async (page) => {
      const items = page.locator("[data-open]").filter({ has: page.getByRole("button") });
      await expect(items.first()).toHaveAttribute("data-disabled", "false", { timeout: 30000 });
      await items.getByRole("button").first().click();
      await expect(items.first()).toHaveAttribute("data-open", "true");
    },
  },
  {
    name: "collapsible",
    goto: (page) => goto(page, "collapsible"),
    interact: async (page) => {
      await page.getByRole("button", { name: "Recent Activity" }).click();
    },
  },
  {
    name: "scroll_area (wheel)",
    goto: (page) => goto(page, "scroll_area"),
    interact: async (page) => {
      const area = page.locator("[data-scroll-direction]").first();
      const box = await area.boundingBox();
      if (!box) throw new Error("scroll area has no bounding box");
      await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
      await page.mouse.wheel(0, 300);
      await expect.poll(() => area.evaluate((el) => el.scrollTop)).toBeGreaterThan(0);
    },
  },
  {
    // Real pointer sequence, several `mouse.move` steps -- mirrors
    // resizable.spec.ts's own `dragHandleBy`.
    name: "resizable (handle drag)",
    goto: (page) => goto(page, "resizable"),
    interact: async (page) => {
      const handle = page.getByRole("separator", { name: "Sidebar" });
      const box = await handle.boundingBox();
      if (!box) throw new Error("resizable handle has no bounding box");
      const startX = box.x + box.width / 2;
      const startY = box.y + box.height / 2;
      await page.mouse.move(startX, startY);
      await page.mouse.down();
      await page.mouse.move(startX + 60, startY, { steps: 10 });
      await page.mouse.up();
    },
  },
  {
    // Keyboard drag lifecycle (Enter grabs, arrows move, Enter drops), not
    // a pointer drag: drag_and_drop_list.spec.ts's own "mouse drag" tests
    // dispatch *synthetic, untrusted* DragEvents (`new DragEvent(...)`,
    // `isTrusted: false`) via `page.evaluate` -- deliberately, since
    // Chromium's native HTML5 drag-and-drop session cannot be driven by
    // CDP mouse events at all (why that spec built a custom dispatcher
    // instead of just using `page.mouse`). An untrusted, script-dispatched
    // event is not a real interaction and per-spec is not even guaranteed
    // an `interactionId` -- the wrong subject for an oracle measuring real
    // interaction cost. The keyboard lifecycle below is a real, trusted,
    // CDP-dispatched gesture and the exact keyboard alternative
    // drag_and_drop_list.spec.ts's own "focus after successful drop lands
    // on moved item" test already proves works.
    name: "drag_and_drop_list (keyboard drag)",
    goto: (page) => goto(page, "drag_and_drop_list"),
    interact: async (page) => {
      const list = page.getByRole("list", { name: "Sortable list" }).first();
      const items = list.locator('[aria-roledescription="sortable item"]');
      await items.first().click();
      await page.keyboard.press("Enter");
      await page.keyboard.press("ArrowDown");
      await page.keyboard.press("Enter");
      await expect(items.nth(1)).toBeFocused();
    },
  },
  {
    // THE motivating gesture for this whole row (row 73): a long, slow
    // drag past the dismiss threshold, several separately-timed
    // `mouse.move` steps -- byte-for-byte the shape of drawer.spec.ts's own
    // "a long, slow drag past the dismiss threshold closes the drawer",
    // which is the exact test that hung at `page.mouse.down()` before the
    // fix (dev-docs/backlog.md row 73).
    name: "drawer (drag)",
    goto: (page) => goto(page, "drawer"),
    interact: async (page) => {
      await page.getByRole("button", { name: "Move Goal" }).click();
      const root = page.locator('[data-slot="drawer-root"]');
      await expect(root).toHaveAttribute("data-state", "open");
      const content = page.locator('[data-slot="drawer-content"]');
      const contentBox = await content.boundingBox();
      const handleBox = await page.locator('[data-slot="drawer-handle"]').boundingBox();
      if (!contentBox || !handleBox) throw new Error("drawer content/handle has no bounding box");
      const startX = handleBox.x + handleBox.width / 2;
      const startY = handleBox.y + handleBox.height / 2;
      const dragDistance = contentBox.height * 0.4;

      await page.mouse.move(startX, startY);
      await page.mouse.down();
      const steps = 6;
      for (let i = 1; i <= steps; i++) {
        await page.mouse.move(startX, startY + (dragDistance * i) / steps, { steps: 2 });
        await page.waitForTimeout(120);
      }
      await page.waitForTimeout(200);
      await page.mouse.up();

      await expect(root).toHaveCount(0);
    },
  },
  {
    name: "tag_group (remove)",
    goto: (page) => goto(page, "tag_group"),
    interact: async (page) => {
      const removeButtons = page.getByRole("button", { name: /^Remove item / });
      const before = await removeButtons.count();
      await removeButtons.first().click();
      await expect(page.getByRole("button", { name: /^Remove item / })).toHaveCount(before - 1);
    },
  },
  {
    name: "calendar (month navigation)",
    goto: (page) => goto(page, "calendar"),
    interact: async (page) => {
      const calendar = page.locator("#component-preview-frame").first();
      const nextButton = calendar.getByRole("button").nth(1);
      const monthSelect = calendar.locator("select").first();
      const before = await monthSelect.inputValue();
      await nextButton.click();
      await expect.poll(() => monthSelect.inputValue()).not.toBe(before);
    },
  },
  {
    name: "data_table (sort)",
    goto: (page) => goto(page, "data_table"),
    interact: async (page) => {
      const header = page.getByRole("button", { name: "Email" });
      const th = page.locator("th", { has: header });
      const before = await th.getAttribute("aria-sort");
      await header.click();
      await expect(th).not.toHaveAttribute("aria-sort", before ?? "none");
    },
  },
  {
    name: "data_table (filter typing)",
    goto: (page) => goto(page, "data_table"),
    interact: async (page) => {
      const input = page.getByRole("textbox", { name: "Filter by email" });
      await input.click();
      await page.keyboard.type("ken");
      await expect(input).toHaveValue("ken");
    },
  },
  {
    // Scoped by the demo's own CSS class, not `getByRole("list")` -- see
    // this file's header, "A note on row 74's other lesson".
    name: "virtual_list (scroll)",
    goto: (page) => goto(page, "virtual_list"),
    interact: async (page) => {
      const container = page.locator(".dx-virtual-list-container").first();
      const box = await container.boundingBox();
      if (!box) throw new Error("virtual list container has no bounding box");
      await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
      await page.mouse.wheel(0, 800);
      await expect.poll(() => container.evaluate((el) => el.scrollTop)).toBeGreaterThan(0);
    },
  },
];

for (const subject of FORM_CONTROL_SUBJECTS) {
  test(`${subject.name}: primary interaction is responsive`, async ({ page }) => {
    await installMainThreadObserver(page);
    await subject.goto(page);
    const report = await assertResponsive(page, `${subject.name} interaction`, () => subject.interact(page), {
      reset: () => subject.goto(page),
    });
    recordReport(subject.name, "interaction", report);
  });
}

// ---------------------------------------------------------------------------
// `test.info().annotations` (design requirement 3): recorded here, once,
// for every subject above, rather than repeated at each call site.
// ---------------------------------------------------------------------------
function recordReport(name: string, phase: string, report: GestureReport): void {
  test.info().annotations.push({
    type: "main-thread-report",
    description:
      `${name} (${phase}): ${report.longTasks.length} long task(s) (max ${report.maxTaskMs.toFixed(1)}ms), ` +
      `${report.interactions.length} qualifying event(s) (max ${report.maxInteractionMs.toFixed(1)}ms)`,
  });
}
