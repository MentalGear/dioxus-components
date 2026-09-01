/**
 * ORACLE: tier 1 (APG) — keyboard matrix.
 *
 * One row = (component, key, APG-specified expectation, observed behaviour).
 * Every row's expectation is quoted from a W3C ARIA Authoring Practices Guide
 * (APG) pattern's own "Keyboard Interaction" section — never from memory.
 * The quotes below were read from a fresh, read-only clone of
 * `w3c/aria-practices` checked out at the SAME pinned commit this repo
 * already vendors executable reference pages from
 * (`playwright/oracle/reference/7e4034b/`, see that directory's README):
 *
 *   commit 7e4034b262bc0d25332e330d8a582aaf34113829
 *
 * That vendored directory holds only three patterns' *executable example*
 * pages (menu-button, combobox-select-only, radio) -- the calibration
 * subjects tier-1 rules run a live behavioural comparison against. This file
 * needs many more patterns' own prose ("Keyboard Interaction" section of each
 * pattern's narrative page, e.g. `content/patterns/menubar/
 * menu-and-menubar-pattern.html#keyboard_interaction`) for citation, which
 * were never vendored because nothing before this file needed to *run*
 * against them. Rather than invent quotes from memory, each was read
 * straight from that pinned commit's tree; the exact path is named in each
 * row's comment, and https://www.w3.org/WAI/ARIA/apg/patterns/<pattern>/ is
 * the same content's live (unpinned) URL for convenience. Where a pattern's
 * own vendored *example* page also exists under `../reference/7e4034b/`, the
 * row cross-references it, but this file does not need `file://` navigation
 * the way `focus-restore-reference.spec.ts` does -- it exists to grade THIS
 * library's demos against the prose, not to re-run the calibration that file
 * already owns.
 *
 * Design (task brief, "keyboard matrix oracle"): assert behaviour (open
 * state via data-state/aria-expanded/visibility, focus via
 * document.activeElement comparisons, aria-activedescendant), not
 * implementation details. Every test is titled
 * "<Component> — <Key>: <APG expectation>". A row the demo pages cannot
 * exercise is `test.skip`ped with a reason ("no fixture") rather than
 * inventing markup. This file does NOT fix anything it finds red -- reds are
 * the deliverable; see docs/conformance-harness.md's tier-1 policy.
 */

import { test, expect, type Page } from "@playwright/test";

const BASE = "http://127.0.0.1:8080/component/?name=";
const goto = (page: Page, name: string) =>
  page.goto(`${BASE}${name}&`, { waitUntil: "networkidle", timeout: 20 * 60 * 1000 });

/** `document.activeElement`'s role/text, for assertions Playwright's own
 * locator-based `toBeFocused()` can't phrase (e.g. "focus is still on X, not
 * on any option"). */
async function activeElement(page: Page) {
  return page.evaluate(() => {
    const el = document.activeElement as HTMLElement | null;
    return {
      role: el?.getAttribute("role") ?? null,
      tag: el?.tagName ?? null,
      text: el?.textContent?.trim().slice(0, 40) ?? null,
    };
  });
}

// ============================================================================
// Menu button (DropdownMenu)
// Source: content/patterns/menu-button/menu-button-pattern.html#keyboard_interaction
// and content/patterns/menubar/menu-and-menubar-pattern.html#keyboard_interaction
// (the menu-button pattern defers post-open behaviour to the Menu/Menubar
// pattern: "The keyboard behaviors needed after the menu is open are
// described in the Menu and Menubar Pattern.")
// Calibration subject for the trigger rows: the vendored example page,
// ../reference/7e4034b/.../menu-button-actions.html (also used by
// focus-restore-reference.spec.ts).
// ============================================================================
test.describe("APG Menu Button pattern — DropdownMenu trigger", () => {
  test("DropdownMenu trigger — Enter: \"opens the menu and places focus on the first menu item\"", async ({ page }) => {
    await goto(page, "dropdown_menu");
    const trigger = page.getByRole("button", { name: "Open Menu" });
    await trigger.focus();

    await page.keyboard.press("Enter");

    // Split assertion on purpose -- the two halves of the APG requirement
    // diverge here (see file header + report): the menu DOES open...
    await expect(trigger, "Enter must open the menu").toHaveAttribute("data-state", "open");
    // ...but focus does not move to the first item as APG requires. Asserting
    // the APG requirement directly (not the observed behaviour) so the gap
    // shows up as a red, per this file's brief ("reds are the deliverable").
    // Source: primitives/src/dropdown_menu.rs DropdownMenu::handle_keydown's
    // Key::Enter arm only toggles `open`; it never calls
    // `ctx.focus.focus_first()` the way the ArrowDown arm does.
    const active = await activeElement(page);
    expect(
      active.role,
      "APG requires focus to move to the first menu item on Enter. " +
        `OBSERVED: focus stays on the trigger button (role=${active.role}, ` +
        `text="${active.text}") instead.`,
    ).toBe("option");
  });

  test("DropdownMenu trigger — Space: \"Opens the menu and places focus on the first menu item\"", async ({ page }) => {
    await goto(page, "dropdown_menu");
    const trigger = page.getByRole("button", { name: "Open Menu" });
    await trigger.focus();

    await page.keyboard.press("Space");

    await expect(
      trigger,
      "Space opens the menu here via the native <button> click -- the root " +
        "keydown handler never Key::Character(\" \")-matches or prevents it, " +
        "and DropdownMenuTrigger's onclick just flips `open`.",
    ).toHaveAttribute("data-state", "open");
    const active = await activeElement(page);
    expect(
      active.role,
      "APG requires focus to move to the first menu item on Space, same as " +
        `Enter. OBSERVED: focus stays on the trigger (role=${active.role}) -- ` +
        "DropdownMenuTrigger's onclick explicitly refocuses the TRIGGER " +
        "itself (`data.set_focus(true)`), not the first item.",
    ).toBe("option");
  });

  test("DropdownMenu trigger — Down Arrow (Optional): \"opens the menu and moves focus to the first menu item\"", async ({ page }) => {
    await goto(page, "dropdown_menu");
    const trigger = page.getByRole("button", { name: "Open Menu" });
    await trigger.focus();

    await page.keyboard.press("ArrowDown");
    await page.waitForTimeout(150);

    // APG (Optional): Down Arrow should open the menu and move focus to the
    // first item. OBSERVED: from a cold (never-opened) trigger this does
    // nothing at all -- the menu does not even open. Source:
    // DropdownMenu::handle_keydown's Key::ArrowDown arm only calls
    // `ctx.focus.focus_next()`; with the menu closed, DropdownMenuContent
    // (and its items) are unmounted (`use_animated_open`), so there is
    // nothing in the roving-focus collection to move to, and the
    // open<->focused sync effect never fires.
    expect(
      await trigger.getAttribute("data-state"),
      "APG (Optional): Down Arrow should open the menu. OBSERVED: a cold " +
        "ArrowDown press does not open the menu at all, unlike a cold " +
        "Enter/Space press, which do (see those rows above).",
    ).toBe("open");
  });

  test("DropdownMenu trigger — Up Arrow (Optional): \"opens the menu and moves focus to the last menu item\"", async ({ page }) => {
    await goto(page, "dropdown_menu");
    const trigger = page.getByRole("button", { name: "Open Menu" });
    await trigger.focus();

    await page.keyboard.press("ArrowUp");
    await page.waitForTimeout(150);

    // APG (Optional): Up Arrow should open the menu and move focus to the
    // last item. OBSERVED: DropdownMenu::handle_keydown's Key::ArrowUp arm
    // is guarded `if open()` -- from closed, it is a complete no-op.
    expect(
      await trigger.getAttribute("data-state"),
      "APG (Optional): Up Arrow should open the menu. OBSERVED: cold " +
        "ArrowUp does not open the menu at all (guarded `if open()` in " +
        "dropdown_menu.rs) -- this optional row is entirely unimplemented.",
    ).toBe("open");
  });
});

test.describe("APG Menu (opened from a menu button) pattern — DropdownMenu content", () => {
  test.beforeEach(async ({ page }) => {
    await goto(page, "dropdown_menu");
    await page.getByRole("button", { name: "Open Menu" }).click();
  });

  test("DropdownMenu item — Escape: \"Close the menu ... and return focus to the element ... from which the menu was opened\"", async ({ page }) => {
    const trigger = page.getByRole("button", { name: "Open Menu" });
    await page.keyboard.press("ArrowDown");
    await expect(page.getByRole("option", { name: "Edit" })).toBeFocused();

    await page.keyboard.press("Escape");

    await expect(trigger).toHaveAttribute("data-state", "closed");
    await expect(trigger, "Escape must return focus to the trigger").toBeFocused();
  });

  test("DropdownMenu item — Home: \"moves focus to the first item in the current menu\"", async ({ page }) => {
    await page.keyboard.press("ArrowDown"); // focus Edit
    await page.keyboard.press("ArrowDown"); // Undo is disabled, skipped -> Duplicate
    await page.keyboard.press("Home");
    await expect(page.getByRole("option", { name: "Edit" })).toBeFocused();
  });

  test("DropdownMenu item — End: \"moves focus to the last item in the current menu\"", async ({ page }) => {
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("End");
    await expect(page.getByRole("option", { name: "Delete" })).toBeFocused();
  });

  test("DropdownMenu item — Down Arrow: \"moves focus to the next item, optionally wrapping from the last to the first\"", async ({ page }) => {
    await page.keyboard.press("ArrowDown"); // Edit
    await page.keyboard.press("End"); // Delete (last)
    await page.keyboard.press("ArrowDown"); // wrap?
    // Wrapping is optional either way per APG -- asserting the observed
    // behaviour (this component's default `roving_loop: true` does wrap) so
    // the row is a meaningful check rather than a vacuous one.
    await expect(
      page.getByRole("option", { name: "Edit" }),
      "roving_loop defaults to true, so ArrowDown from the last item wraps " +
        "to the first (an APG-permitted choice, not a requirement).",
    ).toBeFocused();
  });

  test("DropdownMenu item — Enter: \"activates the item and closes the menu\"", async ({ page }) => {
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown"); // Duplicate
    await page.keyboard.press("Enter");
    await expect(page.getByRole("button", { name: "Open Menu" })).toHaveAttribute(
      "data-state",
      "closed",
    );
    await expect(page.getByText("Selected: Duplicate")).toBeVisible();
  });

  test("DropdownMenu item — Space (Optional): \"activates the menuitem and closes the menu\"", async ({ page }) => {
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown"); // Duplicate
    await page.keyboard.press("Space");
    await expect(page.getByRole("button", { name: "Open Menu" })).toHaveAttribute(
      "data-state",
      "closed",
    );
    await expect(page.getByText("Selected: Duplicate")).toBeVisible();
  });
});

// ============================================================================
// Menubar
// Source: content/patterns/menubar/menu-and-menubar-pattern.html#keyboard_interaction
// No vendored example page for this pattern (only menu-button/combobox/radio
// were vendored, per playwright/oracle/reference/README.md) -- these rows
// grade the demo against the quoted prose directly, with no live reference
// subject.
// ============================================================================
test.describe("APG Menu and Menubar pattern — Menubar", () => {
  test.beforeEach(async ({ page }) => {
    await goto(page, "menubar");
  });

  test("Menubar trigger — Enter: \"When focus is on a menuitem that has a submenu, opens the submenu and places focus on its first item\"", async ({ page }) => {
    const fileTrigger = page.getByRole("menuitem", { name: "File" });
    await fileTrigger.focus();

    await page.keyboard.press("Enter");

    const fileMenu = page
      .getByRole("menu")
      .filter({ has: page.getByRole("menuitem", { name: "New" }) })
      .last();
    await expect(fileMenu, "Enter must open the File submenu").toHaveAttribute(
      "data-state",
      "open",
    );
    // APG requires focus to land on the submenu's first item. Asserting that
    // requirement directly. Source: menubar.rs MenubarMenu's onkeydown
    // Key::Enter arm only calls `ctx.set_open_menu.call(...)`; it never sets
    // `menu_ctx.initial_focus` the way the ArrowDown arm two lines below it
    // does, so OBSERVED focus stays on the File trigger instead.
    await expect(
      fileMenu.getByRole("menuitem", { name: "New" }),
      "APG requires Enter to place focus on the submenu's first item. " +
        "OBSERVED: focus stays on the File trigger itself, unlike ArrowDown " +
        "on the same trigger (next row), which does move focus correctly.",
    ).toBeFocused();
  });

  test("Menubar trigger — Space (Optional): \"opens the submenu and places focus on its first item\"", async ({ page }) => {
    const fileTrigger = page.getByRole("menuitem", { name: "File" });
    await fileTrigger.focus();

    await page.keyboard.press("Space");
    await page.waitForTimeout(150);

    const fileMenu = page
      .getByRole("menu")
      .filter({ has: page.getByRole("menuitem", { name: "New" }) })
      .last();
    // APG (Optional): Space should open the submenu (and place focus on its
    // first item). Source of the observed gap: MenubarMenu's onkeydown match
    // has no Key::Character(" ") arm (falls to `_ => return`), and
    // MenubarTrigger wires only `onpointerup`, never `onclick` -- so the
    // native <button> click a focused-button Space key synthesizes has no
    // listener to act on it either.
    expect(
      await fileMenu.count(),
      "APG (Optional): Space should open the submenu. OBSERVED: Space does " +
        "nothing at all on a menubar trigger -- neither opens nor moves " +
        "focus -- unlike Enter (opens, wrong focus) and ArrowDown (opens, " +
        "correct focus) on the exact same trigger.",
    ).toBe(1);
  });

  test("Menubar trigger — Down Arrow: \"opens the submenu and places focus on the first item in the submenu\"", async ({ page }) => {
    const fileTrigger = page.getByRole("menuitem", { name: "File" });
    await fileTrigger.focus();

    await page.keyboard.press("ArrowDown");

    const fileMenu = page
      .getByRole("menu")
      .filter({ has: page.getByRole("menuitem", { name: "New" }) })
      .last();
    await expect(fileMenu).toHaveAttribute("data-state", "open");
    await expect(
      fileMenu.getByRole("menuitem", { name: "New" }),
      "ArrowDown is the ONLY trigger key that fully satisfies this row " +
        "(opens AND focuses first item) -- see the Enter/Space rows above " +
        "for the same trigger failing one or both halves.",
    ).toBeFocused();
  });

  test("Menubar — Right Arrow: \"When focus is in a menubar, moves focus to the next item, optionally wrapping from the last to the first\"", async ({ page }) => {
    await page.getByRole("menubar").focus();
    const fileButton = page.getByRole("menuitem", { name: "File" });
    const editButton = page.getByRole("menuitem", { name: "Edit" });
    await page.keyboard.press("ArrowRight");
    await expect(editButton).toBeFocused();
    await page.keyboard.press("ArrowRight");
    await expect(
      fileButton,
      "OBSERVED: wraps from the last menubar item back to the first " +
        "(APG marks wrapping optional either way).",
    ).toBeFocused();
  });

  test("Menubar — Left Arrow: \"moves focus to the previous item, optionally wrapping from the first to the last\"", async ({ page }) => {
    await page.getByRole("menubar").focus();
    const editButton = page.getByRole("menuitem", { name: "Edit" });
    // Focus starts on "File" (the first/default item) -- ArrowLeft from the
    // first item should wrap to the LAST item ("Edit" in this two-item demo).
    await page.keyboard.press("ArrowLeft");
    await expect(editButton, "wraps from the first item to the last").toBeFocused();
  });

  test("MenubarMenu — Escape: \"Close the menu ... and return focus to the element ... from which the menu was opened\"", async ({ page }) => {
    const fileTrigger = page.getByRole("menuitem", { name: "File" });
    await fileTrigger.focus();
    await page.keyboard.press("ArrowDown");
    const fileMenu = page
      .getByRole("menu")
      .filter({ has: page.getByRole("menuitem", { name: "New" }) })
      .last();
    await expect(fileMenu).toHaveAttribute("data-state", "open");

    await page.keyboard.press("Escape");

    await expect(fileMenu).toHaveCount(0);
    await expect(fileTrigger, "Escape must return focus to this menu's own trigger").toBeFocused();
  });

  test("MenubarItem — Enter: \"activates the item and closes the menu\"", async ({ page }) => {
    const fileTrigger = page.getByRole("menuitem", { name: "File" });
    await fileTrigger.focus();
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown"); // Open is disabled, skip to Save
    await expect(page.getByRole("menuitem", { name: "Save" })).toBeFocused();

    await page.keyboard.press("Enter");

    const fileMenu = page
      .getByRole("menu")
      .filter({ has: page.getByRole("menuitem", { name: "New" }) })
      .last();
    await expect(fileMenu).toHaveCount(0);
  });
});

// ============================================================================
// Select-only combobox (Select trigger)
// Source (general keyboard interaction, applies to select-only since it is
// non-editable): content/patterns/combobox/combobox-pattern.html
// #keyboard_interaction ("Combobox Keyboard Interaction" + "Listbox Popup
// Keyboard Interaction" sections). Calibration subject:
// ../reference/7e4034b/.../combobox-select-only.html (also used by
// focus-restore-reference.spec.ts, which documents that page's DOM-focus-
// stays-on-combobox/aria-activedescendant technique).
//
// IMPORTANT ROLE CAVEAT (see written report): this library's SelectTrigger
// is an implicit-role <button> with aria-haspopup="listbox" -- NOT
// role="combobox" -- and SelectList uses real roving DOM focus on
// role="option" children, not aria-activedescendant. So "Select" does not
// actually carry the combobox pattern's roles at all; it is graded here only
// because the task and the live bug report both name it as this library's
// stand-in for that pattern (closest existing component to a select-only
// combobox). Kept as a named divergence, not silently normalized away.
// ============================================================================
test.describe("APG Combobox pattern (select-only) — Select trigger", () => {
  const trigger = (page: Page) =>
    page.getByRole("button").filter({ hasText: /Select an option|Apple|Banana/ }).first();

  test("Select trigger — Enter: opens the popup (Combobox Keyboard Interaction)", async ({ page }) => {
    await goto(page, "select");
    const t = trigger(page);
    await t.focus();

    await page.keyboard.press("Enter");

    const listbox = page.getByRole("listbox");
    await expect(listbox).toHaveAttribute("data-state", "open");
  });

  test("Select trigger — Enter: focus lands on the listbox container, not the active option (contrast with APG's own aria-activedescendant technique)", async ({ page }) => {
    await goto(page, "select");
    const t = trigger(page);
    await t.focus();

    await page.keyboard.press("Enter");

    const active = await activeElement(page);
    // This component's own ArrowDown/ArrowUp rows (below) show it CAN put
    // real DOM focus on an option -- so that is the bar Enter/Space are held
    // to here, not APG's aria-activedescendant technique (which this
    // component doesn't use at all; see the describe-block header).
    // OBSERVED root cause (see written report): SelectListRendered's
    // `focused = open() && !ctx.selectable.collection.any_focused()` decides
    // who gets real DOM focus. Enter/Space open via SelectTrigger's plain
    // `onclick`, which never sets `ctx.selectable.initial_focus` (only the
    // ArrowUp/ArrowDown keydown arms do) -- so no option is ever
    // programmatically focused, `any_focused()` stays false forever, and the
    // listbox <div role="listbox"> itself keeps DOM focus for the whole time
    // the popup is open. This is the exact behaviour the live bug report
    // names: "Select opened by keyboard puts focus on the listbox container
    // rather than the active option."
    expect(
      active.role,
      `OBSERVED: focus after Enter lands on the listbox container (role=${active.role}) ` +
        "instead of the first/active option, unlike this same component's " +
        "ArrowDown/ArrowUp (below), which do focus an option.",
    ).toBe("option");
  });

  test("Select trigger — Space: opens the popup (Combobox Keyboard Interaction)", async ({ page }) => {
    await goto(page, "select");
    const t = trigger(page);
    await t.focus();
    await page.keyboard.press("Space");
    await expect(page.getByRole("listbox")).toHaveAttribute("data-state", "open");
  });

  test("Select trigger — Down Arrow: \"moves focus into the popup ... places focus on the first focusable element\"", async ({ page }) => {
    await goto(page, "select");
    const t = trigger(page);
    await t.focus();

    await page.keyboard.press("ArrowDown");

    // Unlike Enter/Space, ArrowDown DOES land focus on the first option --
    // SelectTrigger's onkeydown explicitly sets
    // `initial_focus = collection.first_available_index()` before opening.
    await expect(
      page.getByRole("listbox").getByRole("option").first(),
      "ArrowDown correctly focuses the first option -- Enter/Space (same " +
        "trigger, same widget) do not; this is the intra-component " +
        "inconsistency the live bug report describes.",
    ).toBeFocused();
  });

  test("Select trigger — Up Arrow (Optional): \"places focus on the last focusable element in the popup\"", async ({ page }) => {
    await goto(page, "select");
    const t = trigger(page);
    await t.focus();

    await page.keyboard.press("ArrowUp");

    await expect(page.getByRole("listbox").getByRole("option").last()).toBeFocused();
  });

  test("Select trigger — Alt+Down Arrow (Optional): \"displays the popup without moving focus\"", async ({ page }) => {
    await goto(page, "select");
    const t = trigger(page);
    await t.focus();

    await page.keyboard.press("Alt+ArrowDown");

    // Source of the observed gap: SelectTrigger's onkeydown matches on
    // `event.key()` alone (Key::ArrowDown), which does not distinguish a
    // held Alt modifier from plain ArrowDown.
    await expect(page.getByRole("listbox")).toHaveAttribute("data-state", "open");
    await expect(
      t,
      "APG (Optional): Alt+ArrowDown should open the popup WITHOUT moving " +
        "focus off the trigger. OBSERVED: focus moves to the first option " +
        "instead (same as plain ArrowDown) -- Alt is not distinguished from " +
        "plain ArrowDown at all.",
    ).toBeFocused();
  });

  test("Select listbox — Home (Optional): \"moves focus to and selects the first option\"", async ({ page }) => {
    await goto(page, "select");
    await trigger(page).click();
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown"); // Banana
    await page.keyboard.press("Home");
    await expect(page.getByRole("listbox").getByRole("option", { name: "Apple" })).toBeFocused();
  });

  test("Select listbox — End (Optional): \"moves focus to the last option\"", async ({ page }) => {
    await goto(page, "select");
    await trigger(page).click();
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("End");
    await expect(page.getByRole("listbox").getByRole("option", { name: "Other" })).toBeFocused();
  });

  test("Select listbox — Escape: \"Closes the popup and returns focus to the combobox\"", async ({ page }) => {
    await goto(page, "select");
    const t = trigger(page);
    await t.click();
    await page.keyboard.press("ArrowDown");

    await page.keyboard.press("Escape");

    await expect(page.getByRole("listbox")).toHaveCount(0);
    await expect(t, "Escape must return focus to the trigger").toBeFocused();
  });

  test("Select listbox — printable characters: \"moves focus to the next item with a name that starts with the characters typed\"", async ({ page }) => {
    await goto(page, "select");
    await trigger(page).click();
    await page.keyboard.type("Ban");
    await expect(page.getByRole("listbox").getByRole("option", { name: "Banana" })).toBeFocused();
  });
});

// ============================================================================
// Editable combobox (Combobox)
// Source: content/patterns/combobox/combobox-pattern.html#keyboard_interaction
// This component IS built with role="combobox" + aria-activedescendant
// (input.rs) -- the correct APG technique -- unlike Select above. Rows below
// are lighter-weight than Select's because combobox.spec.ts already covers
// this component in depth; this file adds only the matrix rows the task
// asks for, titled per this file's convention.
// ============================================================================
test.describe("APG Combobox pattern (editable) — Combobox", () => {
  const input = (page: Page) => page.getByRole("combobox", { name: "Select framework" });
  const listbox = (page: Page) => page.locator("[role='listbox'][data-state='open']");

  test("Combobox input — Down Arrow: \"moves focus into the popup\" via aria-activedescendant (DOM focus stays on the combobox)", async ({ page }) => {
    await goto(page, "combobox");
    const i = input(page);
    await i.focus();

    await page.keyboard.press("ArrowDown");

    await expect(listbox(page)).toBeVisible();
    await expect(i, "APG's combobox technique: DOM focus never leaves the input").toBeFocused();
    await expect(listbox(page).getByRole("option", { name: "Next.js" })).toHaveAttribute(
      "data-highlighted",
      "true",
    );
  });

  test("Combobox input — Enter: \"Accepts the focused option ... closing the popup, placing the accepted value in the combobox\"", async ({ page }) => {
    await goto(page, "combobox");
    const i = input(page);
    await i.click();
    await page.keyboard.type("sve");
    await page.keyboard.press("ArrowDown");

    await page.keyboard.press("Enter");

    await expect(listbox(page)).toHaveCount(0);
    await expect(i).toHaveValue("SvelteKit");
  });

  test("Combobox input — Escape: \"Closes the popup and returns focus to the combobox\"", async ({ page }) => {
    await goto(page, "combobox");
    const i = input(page);
    await i.click();
    await page.keyboard.type("sve");
    await page.keyboard.press("ArrowDown");

    await page.keyboard.press("Escape");

    await expect(listbox(page)).toHaveCount(0);
    await expect(i).toBeFocused();
  });
});

// ============================================================================
// Disclosure (Collapsible) and Accordion
// ============================================================================
test.describe("APG Disclosure pattern — Collapsible", () => {
  // Source: content/patterns/disclosure/disclosure-pattern.html#keyboard_interaction
  // "Enter: activates the disclosure control and toggles the visibility of
  // the disclosure content." / "Space: activates the disclosure control and
  // toggles the visibility of the disclosure content." -- Collapsible's
  // trigger is a real <button> (collapsible.rs), so both keys are free
  // native button activation; no custom onkeydown at all.
  test("CollapsibleTrigger — Enter: toggles the disclosure content", async ({ page }) => {
    await goto(page, "collapsible");
    const trigger = page.getByRole("button", { name: "Recent Activity" });
    await trigger.focus();
    await page.keyboard.press("Enter");
    await expect(page.getByText("Fixed a bug in the collapsible component")).toBeVisible();
  });

  test("CollapsibleTrigger — Space: toggles the disclosure content", async ({ page }) => {
    await goto(page, "collapsible");
    const trigger = page.getByRole("button", { name: "Recent Activity" });
    await trigger.focus();
    await page.keyboard.press("Space");
    await expect(page.getByText("Fixed a bug in the collapsible component")).toBeVisible();
  });
});

test.describe("APG Accordion pattern — Accordion", () => {
  // Source: content/patterns/accordion/accordion-pattern.html#keyboard_interaction.
  // Quoted in full because the finding below turns on exactly what this
  // section does and does not say:
  //   "Enter or Space: When focus is on the accordion header for a collapsed
  //   panel, expands the associated panel. ... Tab: Moves focus to the next
  //   focusable element ... Shift+Tab: Moves focus to the previous focusable
  //   element ..."
  // That is the ENTIRE section at the pinned commit. It does not mention
  // ArrowUp/ArrowDown/ArrowLeft/ArrowRight/Home/End at all -- unlike Tabs,
  // Radio Group, Menu(bar), etc., which all specify roving-tabindex arrow
  // navigation, Accordion's current APG prose has no such row to test
  // against. (This differs from some older/other accessible-widget
  // catalogues that do give accordions arrow-key navigation -- but per this
  // file's "do not rely on memory" rule, only the pinned commit's own text
  // counts.)
  const items = (page: Page) =>
    page.locator("[data-open]").filter({ has: page.getByRole("button") });

  test("AccordionTrigger — Enter: \"expands the associated panel\" / collapses an expanded one", async ({ page }) => {
    await goto(page, "accordion");
    const buttons = items(page).getByRole("button");
    const first = items(page).first();
    await buttons.first().focus();
    await page.keyboard.press("Enter");
    await expect(first).toHaveAttribute("data-open", "true");
    await page.keyboard.press("Enter");
    await expect(first).toHaveAttribute("data-open", "false");
  });

  test("AccordionTrigger — Space: \"expands the associated panel\" / collapses an expanded one", async ({ page }) => {
    await goto(page, "accordion");
    const buttons = items(page).getByRole("button");
    const first = items(page).first();
    await buttons.first().focus();
    await page.keyboard.press("Space");
    await expect(first).toHaveAttribute("data-open", "true");
    await page.keyboard.press("Space");
    await expect(first).toHaveAttribute("data-open", "false");
  });

  test.skip(
    "AccordionTrigger — ArrowDown/Home/End: no row exists in accordion-pattern.html's Keyboard Interaction section at the pinned commit to test against",
    () => {},
  );
});

// ============================================================================
// Dialog / Alert Dialog
// Source (trigger open): content/patterns/button/button-pattern.html
// #keyboard_interaction ("Space: Activates the button." / "Enter: Activates
// the button." -- opening a dialog is plain button activation, not a Dialog-
// pattern-specific rule). Source (Escape/Tab):
// content/patterns/dialog-modal/dialog-modal-pattern.html#keyboard_interaction
// ("Escape: Closes the dialog." / Tab-cycle text). AlertDialog's own section
// (alertdialog-pattern.html#keyboard_interaction) just says "See the keyboard
// interaction section for the modal dialog pattern."
// ============================================================================
test.describe("APG Dialog (Modal) pattern — Dialog", () => {
  test("Dialog trigger — Enter: opens the dialog (Button pattern activation)", async ({ page }) => {
    await goto(page, "dialog");
    const trigger = page.getByRole("button", { name: "Show Dialog" });
    await trigger.focus();
    await page.keyboard.press("Enter");
    await expect(page.getByRole("dialog")).toBeVisible();
  });

  test("Dialog trigger — Space: opens the dialog (Button pattern activation)", async ({ page }) => {
    await goto(page, "dialog");
    const trigger = page.getByRole("button", { name: "Show Dialog" });
    await trigger.focus();
    await page.keyboard.press("Space");
    await expect(page.getByRole("dialog")).toBeVisible();
  });

  test("Dialog — Escape: \"Closes the dialog\"", async ({ page }) => {
    await goto(page, "dialog");
    await page.getByRole("button", { name: "Show Dialog" }).click();
    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(dialog).toHaveCount(0);
  });

  test("Dialog — Tab: \"Moves focus to the next tabbable element inside the dialog\" and wraps", async ({ page }) => {
    await goto(page, "dialog");
    await page.getByRole("button", { name: "Show Dialog" }).click();
    const dialog = page.getByRole("dialog");
    const closeButton = dialog.getByRole("button", { name: "Close" });
    await expect(closeButton, "focus moves inside the dialog on open").toBeFocused();
    await page.keyboard.press("Tab");
    await expect(dialog.getByRole("button", { name: "Open Nested Dialog" })).toBeFocused();
  });
});

test.describe("APG Alert Dialog pattern — AlertDialog", () => {
  test("AlertDialog trigger — Enter: opens the dialog (Button pattern activation)", async ({ page }) => {
    await goto(page, "alert_dialog");
    const trigger = page.getByRole("button", { name: "Show Alert Dialog" });
    await trigger.focus();
    await page.keyboard.press("Enter");
    await expect(page.getByRole("alertdialog")).toBeVisible();
  });

  test("AlertDialog — Escape: \"Closes the dialog\" (defers to modal dialog pattern)", async ({ page }) => {
    await goto(page, "alert_dialog");
    await page.getByRole("button", { name: "Show Alert Dialog" }).click();
    const dialog = page.getByRole("alertdialog");
    await expect(dialog).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(dialog).toHaveCount(0);
  });
});

// ============================================================================
// Tabs
// Source: content/patterns/tabs/tabs-pattern.html#keyboard_interaction.
// The pattern permits either automatic activation (arrow moves = activates)
// or manual ("Space or Enter: Activates the tab if it was not activated
// automatically on focus"). Which mode a component implements must be
// recorded, per the task brief.
// ============================================================================
test.describe("APG Tabs pattern — Tabs", () => {
  const activeTabPanel = (page: Page) =>
    page
      .locator('[role="tabpanel"][data-state="active"]:not(#component-preview-frame)')
      .filter({ hasText: /^Tab \d Content$/ });

  test("TabTrigger — Right Arrow: \"Moves focus to the next tab\" WITHOUT activating it (this component implements manual activation)", async ({ page }) => {
    await goto(page, "tabs");
    const tab1 = page.getByRole("tab", { name: "Tab 1" });
    const tab2 = page.getByRole("tab", { name: "Tab 2" });
    await tab1.click();
    await page.keyboard.press("ArrowRight");
    await expect(tab2).toBeFocused();
    await expect(
      activeTabPanel(page),
      "manual-activation mode: moving focus must not itself change which " +
        "tab panel is shown",
    ).toContainText("Tab 1 Content");
  });

  test("TabTrigger — Enter: \"Activates the tab if it was not activated automatically on focus\"", async ({ page }) => {
    await goto(page, "tabs");
    await page.getByRole("tab", { name: "Tab 1" }).click();
    await page.keyboard.press("ArrowRight");
    await page.keyboard.press("Enter");
    await expect(activeTabPanel(page)).toContainText("Tab 2 Content");
  });

  test("TabTrigger — Space: \"Activates the tab if it was not activated automatically on focus\"", async ({ page }) => {
    await goto(page, "tabs");
    await page.getByRole("tab", { name: "Tab 1" }).click();
    await page.keyboard.press("ArrowRight");
    await page.keyboard.press("Space");
    await expect(activeTabPanel(page)).toContainText("Tab 2 Content");
  });

  test("TabTrigger — Home (Optional): \"Moves focus to the first tab\"", async ({ page }) => {
    await goto(page, "tabs");
    await page.getByRole("tab", { name: "Tab 3" }).click();
    await page.keyboard.press("Home");
    await expect(page.getByRole("tab", { name: "Tab 1" })).toBeFocused();
  });

  test("TabTrigger — End (Optional): \"Moves focus to the last tab\"", async ({ page }) => {
    await goto(page, "tabs");
    await page.getByRole("tab", { name: "Tab 1" }).click();
    await page.keyboard.press("End");
    await expect(page.getByRole("tab", { name: "Tab 3" })).toBeFocused();
  });
});

// ============================================================================
// Radio group
// Source: content/patterns/radio/radio-group-pattern.html#keyboard_interaction
// ("For Radio Groups Not Contained in a Toolbar"). Calibration subject:
// ../reference/7e4034b/.../radio.html.
// ============================================================================
test.describe("APG Radio Group pattern — RadioGroup", () => {
  test("RadioGroupItem — Right/Down Arrow: \"move focus to the next radio button ... uncheck the previously focused button, and check the newly focused button\"", async ({ page }) => {
    await goto(page, "radio_group");
    await page.getByRole("radio", { name: "Blue" }).click();
    await page.keyboard.press("ArrowDown");
    const red = page.getByRole("radio", { name: "Red" });
    await expect(red).toBeFocused();
    await expect(red, "focus must also carry the checked state").toHaveAttribute(
      "aria-checked",
      "true",
    );
  });

  test("RadioGroupItem — Space: \"checks the focused radio button if it is not already checked\"", async ({ page }) => {
    await goto(page, "radio_group");
    const blue = page.getByRole("radio", { name: "Blue" });
    const red = page.getByRole("radio", { name: "Red" });
    await blue.click();
    await blue.focus();
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowUp"); // back to Blue via wrap-free move
    await expect(blue).toBeFocused();
    await red.focus();
    await page.keyboard.press("Space");
    await expect(red).toHaveAttribute("aria-checked", "true");
  });
});

// ============================================================================
// Switch / Checkbox / Toggle
// Source: content/patterns/switch/switch-pattern.html#keyboard_interaction
// ("Space: ... changes the state of the switch." / "Enter (Optional): ...
// changes the state of the switch."), content/patterns/checkbox/
// checkbox-pattern.html#keyboard_interaction ("pressing the Space key
// changes the state of the checkbox" -- Enter is not mentioned at all, not
// even as optional), and content/patterns/button/button-pattern.html
// #keyboard_interaction (Toggle has no dedicated APG pattern -- APG
// documents toggle buttons as plain buttons with aria-pressed, so both
// Space and Enter apply).
// ============================================================================
test.describe("APG Switch pattern — Switch", () => {
  test("Switch — Space: changes the state", async ({ page }) => {
    await goto(page, "switch");
    const el = page.getByRole("switch", { name: "Switch Demo" });
    await expect(el).toHaveAttribute("data-state", "unchecked");
    await el.press("Space");
    await expect(el).toHaveAttribute("data-state", "checked");
  });

  test("Switch — Enter (Optional): changes the state", async ({ page }) => {
    await goto(page, "switch");
    const el = page.getByRole("switch", { name: "Switch Demo" });
    await el.focus();
    await page.keyboard.press("Enter");
    // OBSERVED (deliberate, per switch.rs's own comment: "Switches should
    // only toggle on Space, not Enter" -- e.prevent_default() on Enter):
    // this implementation exercises the non-Enter half of APG's optional
    // wording. Not a violation (Enter is explicitly optional), but the
    // opposite choice from Toggle (below), which DOES respond to Enter --
    // recorded as an intra-library inconsistency across otherwise-identical
    // "press a button-like control to flip a boolean" components.
    await expect(
      el,
      "OBSERVED: this component deliberately suppresses Enter (switch.rs " +
        "onkeydown calls prevent_default() on Key::Enter) -- a same-class " +
        "control (Toggle) responds to Enter for the identical action.",
    ).toHaveAttribute("data-state", "unchecked");
  });
});

test.describe("APG Checkbox pattern — Checkbox", () => {
  test("Checkbox — Space: \"changes the state of the checkbox\"", async ({ page }) => {
    await goto(page, "checkbox");
    const el = page.getByRole("checkbox", { name: "Demo Checkbox" });
    await expect(el).toHaveAttribute("data-state", "unchecked");
    await el.focus();
    await page.keyboard.press("Space");
    await expect(el).toHaveAttribute("data-state", "checked");
  });

  test("Checkbox — Enter: not specified by APG at all for checkbox (contrast with Switch's optional allowance)", async ({ page }) => {
    await goto(page, "checkbox");
    const el = page.getByRole("checkbox", { name: "Demo Checkbox" });
    await el.focus();
    await page.keyboard.press("Enter");
    // checkbox.rs suppresses Enter the same way switch.rs does -- correctly
    // conservative here, since checkbox-pattern.html never even lists Enter
    // as optional the way switch-pattern.html does.
    await expect(el).toHaveAttribute("data-state", "unchecked");
  });
});

test.describe("Button pattern (toggle button) — Toggle", () => {
  test("Toggle — Space: activates the button, flipping aria-pressed", async ({ page }) => {
    await goto(page, "toggle");
    const el = page.getByRole("button", { name: "B", exact: true });
    await expect(el).toHaveAttribute("data-state", "off");
    await el.focus();
    await page.keyboard.press("Space");
    await expect(el).toHaveAttribute("data-state", "on");
  });

  test("Toggle — Enter: activates the button, flipping aria-pressed (unlike Switch/Checkbox, this component does NOT suppress Enter)", async ({ page }) => {
    await goto(page, "toggle");
    const el = page.getByRole("button", { name: "B", exact: true });
    await el.focus();
    await page.keyboard.press("Enter");
    await expect(
      el,
      "Toggle has no custom onkeydown at all (toggle.rs just forwards " +
        "props.onkeydown, unset in this demo) -- native <button> Enter " +
        "activation runs unmodified, unlike Switch and Checkbox which both " +
        "explicitly prevent_default() on Key::Enter.",
    ).toHaveAttribute("data-state", "on");
  });
});

test.describe("APG Tabs-adjacent roving widget — ToggleGroup", () => {
  // ToggleGroup has no single dedicated APG pattern; it is a roving-tabindex
  // group of toggle buttons, graded here the same way toggle_group.spec.ts
  // already does, for the Arrow-moves/Enter-activates split this file's
  // matrix cares about.
  test("ToggleGroupItem — Right Arrow: moves focus without toggling", async ({ page }) => {
    await goto(page, "toggle_group");
    const b = page.getByRole("button", { name: "B", exact: true });
    const i = page.getByRole("button", { name: "I", exact: true });
    await b.click();
    await page.keyboard.press("ArrowRight");
    await expect(i).toBeFocused();
    await expect(i).toHaveAttribute("data-state", "off");
  });

  test("ToggleGroupItem — Enter: activates (toggles on) the focused item", async ({ page }) => {
    await goto(page, "toggle_group");
    const b = page.getByRole("button", { name: "B", exact: true });
    const i = page.getByRole("button", { name: "I", exact: true });
    await b.click();
    await page.keyboard.press("ArrowRight");
    await page.keyboard.press("Enter");
    await expect(i).toHaveAttribute("data-state", "on");
  });
});

// ============================================================================
// Slider
// Source: content/patterns/slider/slider-pattern.html#keyboard_interaction.
// ============================================================================
test.describe("APG Slider pattern — Slider", () => {
  const thumb = (page: Page) => page.getByRole("slider", { name: "Demo Slider" });

  test("Slider thumb — Right/Up Arrow: \"Increase the value of the slider by one step\"", async ({ page }) => {
    await goto(page, "slider");
    const t = thumb(page);
    await expect(t).toHaveAttribute("aria-valuenow", "50");
    await t.focus();
    await page.keyboard.press("ArrowRight");
    await expect(t).toHaveAttribute("aria-valuenow", "51");
  });

  test("Slider thumb — Home: \"Set the slider to the first allowed value in its range\"", async ({ page }) => {
    await goto(page, "slider");
    const t = thumb(page);
    await t.focus();

    await page.keyboard.press("Home");
    await page.waitForTimeout(150);

    // Home is REQUIRED by this pattern's Keyboard Interaction section (no
    // "(Optional)" qualifier), unlike Page Up/Page Down below it, which the
    // same section does mark optional. Source of the observed gap:
    // MoveEvent::from_keyboard (move_interaction.rs) matches only
    // Key::ArrowUp/Down/Left/Right; Key::Home falls through its `_ => return
    // None`, and slider.rs's onkeydown returns immediately when
    // `from_keyboard` yields `None`.
    expect(
      await t.getAttribute("aria-valuenow"),
      "APG requires Home to set the slider to its minimum value (0, per " +
        "this demo's min=0.0). OBSERVED: the value is unchanged at 50 -- " +
        "Home is not implemented at all.",
    ).toBe("0");
  });

  test("Slider thumb — End: \"Set the slider to the last allowed value in its range\"", async ({ page }) => {
    await goto(page, "slider");
    const t = thumb(page);
    await t.focus();
    await page.keyboard.press("End");
    await page.waitForTimeout(150);
    expect(
      await t.getAttribute("aria-valuenow"),
      "APG requires End to set the slider to its maximum value (100, per " +
        "this demo's max=100.0). OBSERVED: the value is unchanged at 50 -- " +
        "End is unimplemented, for the same reason as Home above.",
    ).toBe("100");
  });

  test("Slider thumb — Page Up (Optional): \"Increase the slider value by an amount larger than ... Up Arrow\"", async ({ page }) => {
    await goto(page, "slider");
    const t = thumb(page);
    await t.focus();
    await page.keyboard.press("PageUp");
    await page.waitForTimeout(150);
    // Optional row -- graded here for completeness, not as a hard failure.
    expect(
      await t.getAttribute("aria-valuenow"),
      "APG (Optional): Page Up should increase the value by more than one " +
        "step. OBSERVED: unimplemented (same root cause as Home/End above) " +
        "-- the value does not move at all.",
    ).not.toBe("50");
  });
});

// ============================================================================
// Tooltip / HoverCard
// Source: content/patterns/tooltip/tooltip-pattern.html#keyboard_interaction
// ("Escape: Dismisses the Tooltip." / "Focus stays on the triggering element
// while the tooltip is displayed."). HoverCard has no dedicated APG pattern;
// this library models it as a hover/focus-triggered popup sharing role
// "tooltip" with Tooltip and an almost identical show-on-focus/hide-on-blur
// mechanism (hover_card.rs), so it is graded against the same pattern class
// the task brief names it under.
// ============================================================================
test.describe("APG Tooltip pattern — Tooltip", () => {
  test("Tooltip trigger — focus: tooltip becomes visible", async ({ page }) => {
    await goto(page, "tooltip");
    await page.locator("#component-preview-frame").focus();
    await page.keyboard.press("Tab");
    await expect(page.getByRole("tooltip")).toBeVisible();
  });

  test("Tooltip — Escape: \"Dismisses the Tooltip\"", async ({ page }) => {
    await goto(page, "tooltip");
    await page.locator("#component-preview-frame").focus();
    await page.keyboard.press("Tab");
    const tooltip = page.getByRole("tooltip");
    await expect(tooltip).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(tooltip).toHaveCount(0);
  });
});

test.describe("Tooltip-class pattern — HoverCard (no dedicated APG pattern; graded against Tooltip's Escape row)", () => {
  test("HoverCard trigger — focus: card becomes visible (matches Tooltip)", async ({ page }) => {
    await goto(page, "hover_card");
    await page.locator("#component-preview-frame").focus();
    await page.keyboard.press("Tab");
    await expect(page.getByRole("tooltip")).toBeVisible();
  });

  test("HoverCard — Escape: does NOT dismiss the card (inconsistent with Tooltip, the same-class sibling that does)", async ({ page }) => {
    await goto(page, "hover_card");
    await page.locator("#component-preview-frame").focus();
    await page.keyboard.press("Tab");
    const card = page.getByRole("tooltip");
    await expect(card).toBeVisible();

    await page.keyboard.press("Escape");
    await page.waitForTimeout(200);

    // OBSERVED: hover_card.rs registers onmouseenter/onmouseleave/onfocus/
    // onblur but no onkeydown at all -- unlike tooltip.rs's `handle_keydown`,
    // which explicitly closes on Key::Escape. HoverCard is otherwise built
    // to the same show-on-focus contract Tooltip's own APG citation
    // requires ("Focus stays on the triggering element while the tooltip is
    // displayed"), making the missing Escape handling a same-class,
    // component-specific gap rather than a difference in what each control
    // is trying to be.
    await expect(
      card,
      "Graded against Tooltip's own APG citation (\"Escape: Dismisses the " +
        "Tooltip\"), which this same-class control should match. OBSERVED: " +
        "Escape does not dismiss HoverCard at all -- hover_card.rs has no " +
        "keydown handler, unlike tooltip.rs's explicit Key::Escape arm.",
    ).toHaveCount(0);
  });
});

// ============================================================================
// ContextMenu
// Source: content/patterns/menubar/menu-and-menubar-pattern.html, "About
// This Pattern" section -- APG has no dedicated "Context Menu" pattern; it
// treats a context menu as a menu opened "by invoking a command, such as
// Shift + F10 in Windows, that opens a context specific menu," under the
// general Menu and Menubar pattern.
// ============================================================================
test.describe("APG Menu pattern (context-menu invocation) — ContextMenu", () => {
  test("ContextMenu trigger — Shift+F10: opens a \"context specific menu\"", async ({ page }) => {
    await goto(page, "context_menu");
    const trigger = page.getByRole("button", { name: "right click here" });
    await trigger.focus();

    await page.keyboard.press("Shift+F10");

    // OBSERVED (pass, but not via this component's own code): context_menu.rs
    // wires only `oncontextmenu` (context_menu.rs ContextMenuTrigger); there
    // is no Shift+F10/keydown handling anywhere in the primitive. This
    // passes only because Chromium itself synthesizes a native
    // `contextmenu` DOM event for Shift+F10 (and the keyboard "Menu" key),
    // which `oncontextmenu` then picks up for free -- confirmed empirically,
    // not inferred from the source, since the source alone would predict a
    // fail here.
    await expect(page.getByRole("menu")).toBeVisible();
  });
});
