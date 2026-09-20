/**
 * ORACLE: tier 1 (APG) — nested submenu contract
 * (`DropdownMenu.Sub`/`ContextMenu.Sub`, docs/component-backlog.md row 68).
 *
 * Rule source: APG Menu and Menubar pattern's "Keyboard Interaction" (h2)
 * and "WAI-ARIA Roles, States, and Properties" (h2) sections
 * (`content/patterns/menubar/menu-and-menubar-pattern.html`, pinned commit
 * `7e4034b262bc0d25332e330d8a582aaf34113829` of `w3c/aria-practices` --
 * see `playwright/oracle/reference/README.md`). Quoted below verbatim, read
 * from that pinned commit's own file (fetched fresh for this task, not
 * from memory -- `keyboard-matrix.spec.ts`'s file header documents the
 * same technique and cites the same commit for its own prose quotes).
 * There is neither a heading nor a separate page titled "submenu keyboard
 * interaction" -- the quotes below are bullet items nested under the plain
 * "Keyboard Interaction"/"WAI-ARIA Roles, States, and Properties" headings,
 * for a `menuitem` the page calls a "parent menuitem" once it opens a
 * submenu ("If activating a menuitem opens a submenu, the menuitem is
 * known as a parent menuitem").
 *
 *   ARIA (WAI-ARIA Roles, States, and Properties):
 *   "A parent menuitem has aria-haspopup set to either menu or true."
 *   "A parent menuitem has aria-expanded set to false when its child menu
 *   is not visible and set to true when the child menu is visible."
 *   (This section never mentions aria-controls at all -- checked against
 *   the same pinned commit's file, no match. This crate's sub-trigger sets
 *   aria-controls anyway, as unambiguous ownership of the popup it opens,
 *   but that is this crate's own choice, not an APG-cited requirement, and
 *   nothing below asserts it as a rule.)
 *
 *   Keyboard (Keyboard Interaction):
 *   Enter: "When focus is on a menuitem that has a submenu, opens the
 *   submenu and places focus on its first item."
 *   Right Arrow: "When focus is in a menu and on a menuitem that has a
 *   submenu, opens the submenu and places focus on its first item."
 *   Left Arrow: "When focus is in a submenu of an item in a menu, closes
 *   the submenu and returns focus to the parent menuitem."
 *   Escape (the pattern's one general rule -- there is no separate
 *   submenu-only Escape row): "Close the menu that contains focus and
 *   return focus to the element or context, e.g., menu button or parent
 *   menuitem, from which the menu was opened."
 *
 * Every "role=menu has an accessible name" assertion below reuses the same
 * general-menu rule `menu-roles.spec.ts`'s file header already quotes and
 * cites ("An element with role menu either has: aria-labelledby set to a
 * value that refers to the menuitem or button that controls its display.
 * A label provided by aria-label.") -- not re-derived here, just applied to
 * a submenu's own popup instead of a root menu's. `docs/backlog.md` row 25
 * is the earlier fix for exactly this gap on `ContextMenu`/`Menubar`'s
 * *existing* submenu-shaped popups; this file is the same rule applied to
 * the genuinely new `*SubContent` components row 68 adds, so the gap row
 * 25 closed cannot reopen here silently.
 *
 * Calibration: neither `../reference/7e4034b/` (menu-button/combobox/radio
 * only, per that directory's README) nor this repo vendors an executable
 * example of a menu-with-a-submenu, so -- exactly as `menu-roles.spec.ts`'s
 * ContextMenu/Menubar/Navbar describe blocks and `keyboard-matrix.spec.ts`'s
 * own Menubar rows already do for the identical gap -- this file grades
 * directly against the prose quoted above, with no live calibration
 * subject to run the same assertions against first.
 *
 * Hover-intent open/close-on-leave (`crate::menu_sub::SUBMENU_OPEN_INTENT_
 * DELAY`/`SUBMENU_CLOSE_GRACE_DELAY`, `primitives/src/menu_sub.rs`) is this
 * crate's own UX choice, not cited to APG (that module's own doc says so,
 * and a fresh check of the pinned commit's page found no hover/mouse
 * wording of any kind) -- asserted below as an implementation contract
 * this crate committed to, not as an APG requirement.
 *
 * Subject components -- both nested-submenu hosts this task adds:
 *   - DropdownMenu.Sub  (`preview/…/dropdown_menu`'s demo: root items
 *     Edit/Undo(disabled)/Duplicate, sub-trigger "More tools", submenu
 *     items Rename/Archive, then Delete)
 *   - ContextMenu.Sub   (`preview/…/context_menu`'s demo: root items
 *     Edit/Undo(disabled)/Duplicate/Delete, then sub-trigger "More tools",
 *     submenu items Rename/Archive)
 * Both demos keep their pre-existing Edit/Undo/Duplicate(/Delete) items and
 * indices untouched (see each demo file's own top-of-file comment) --
 * `playwright/dropdown-menu.spec.ts`, `playwright/context-menu.spec.ts`,
 * and this file's own sibling `keyboard-matrix.spec.ts` already depend on
 * that exact shape, so the submenu was added on rather than interleaved
 * with it.
 *
 * RED BEFORE THE FIX: every test in this file exercises
 * `DropdownMenuSub`/`ContextMenuSub`/their `*SubTrigger`/`*SubContent`/
 * `*SubItem`, none of which existed before this task -- the demo pages had
 * no "More tools" trigger and no second popover to open at all, so every
 * locator below (`getByRole("menuitem", { name: "More tools" })` first and
 * foremost) resolved to zero elements and every assertion failed. This is
 * the fresh-design-work row 68's own recommendation names, budgeted
 * against a stale, shape-only patch (`docs/component-backlog.md` row 68,
 * finding 2) -- there was no prior implementation to regress test against.
 *
 * CORRECTED LOCATOR (not an APG rule change -- a query-precision fix, once
 * the feature actually opened for the first time): every "find the
 * submenu" locator originally read
 * `page.getByRole("menu").filter({ has: page.getByRole("menuitem", { name:
 * "Rename" }) })`. A nested submenu's `role="menu"` popup is, by design (see
 * `crate::menu_sub`'s module doc and each `*SubContentRendered`'s own doc,
 * `primitives/src/{dropdown_menu,context_menu}.rs`), a genuine DOM
 * descendant of the *root* menu's own `role="menu"` content -- the same
 * "nested `popover=auto` stacking" shape Radix and native OS menus use, and
 * a normal, ARIA-legal way to structure a submenu (nothing in the pinned
 * `menu-and-menubar-pattern.html` forbids or even discusses DOM nesting of
 * a submenu inside its parent menu). That means the *root* menu also has a
 * "Rename" menuitem as a descendant, transitively through the submenu --
 * `.filter({ has: ... })` matches on descendant containment, so it matched
 * *both* menus once the submenu genuinely opened (a Playwright strict-mode
 * violation: "resolved to 2 elements"), never just one as the original
 * author assumed. Confirmed live against this exact build: after the real
 * fix (`primitives/src/{dropdown_menu,context_menu}.rs`), the failure
 * changed from "0 elements" (the feature genuinely not opening) to "2
 * elements, strict mode violation" (the feature opening correctly, but this
 * locator unable to tell "the submenu" from "the submenu's own ancestor,
 * which -- through it -- also contains a Rename item"). The fix used below,
 * `page.getByRole("menu", { name: "More tools" })`, locates the exact same
 * element by its own accessible name instead (`aria-labelledby` resolving
 * to the sub-trigger's own id and text, per this file's already-asserted
 * general-menu rule) -- unambiguous, since only the one submenu is named
 * "More tools", and it does not weaken a single assertion below: every
 * `data-state`/`aria-labelledby`/item-count/visibility check downstream is
 * unchanged, only how "the submenu" is *found* changes.
 */

import { test, expect, type Page } from "@playwright/test";
import { BASE_URL } from "../../base-url";

const BASE = `${BASE_URL}/component/?name=`;
const goto = (page: Page, name: string) =>
  page.goto(`${BASE}${name}&`, { waitUntil: "networkidle", timeout: 20 * 60 * 1000 });

// `crate::menu_sub::SUBMENU_OPEN_INTENT_DELAY`/`SUBMENU_CLOSE_GRACE_DELAY`
// are both 200ms (`primitives/src/menu_sub.rs`) -- generous margins below
// so this file's own timing never races the implementation's.
const BEFORE_INTENT_DELAY_MS = 80;
const PAST_INTENT_DELAY_MS = 500;

test.describe("APG Menu and Menubar pattern — DropdownMenu.Sub", () => {
  test.beforeEach(async ({ page }) => {
    await goto(page, "dropdown_menu");
    await page.getByRole("button", { name: "Open Menu" }).click();
  });

  test('sub-trigger: role="menuitem", aria-haspopup="menu" or "true", aria-expanded reflects open state', async ({
    page,
  }) => {
    const subTrigger = page.getByRole("menuitem", { name: "More tools" });
    await expect(subTrigger, "the sub-trigger must exist as a menuitem").toBeVisible();

    const haspopup = await subTrigger.getAttribute("aria-haspopup");
    expect(
      haspopup,
      `APG: "A parent menuitem has aria-haspopup set to either menu or true." ` +
        `OBSERVED: aria-haspopup="${haspopup}".`,
    ).toMatch(/^(menu|true)$/);
    await expect(
      subTrigger,
      'APG: "A parent menuitem has aria-expanded set to false when its ' +
        'child menu is not visible" -- starts closed',
    ).toHaveAttribute("aria-expanded", "false");

    await subTrigger.click();

    await expect(
      subTrigger,
      'APG: "... and set to true when the child menu is visible"',
    ).toHaveAttribute("aria-expanded", "true");
  });

  test('sub-trigger owns its submenu via aria-controls, and the submenu popup is role="menu" with an accessible name', async ({
    page,
  }) => {
    const subTrigger = page.getByRole("menuitem", { name: "More tools" });
    await subTrigger.click();

    const controlsId = await subTrigger.getAttribute("aria-controls");
    expect(
      controlsId,
      "this crate's own choice (not an APG-cited requirement -- see this " +
        "file's header): the sub-trigger names the submenu it owns",
    ).toBeTruthy();

    const submenu = page.getByRole("menu", { name: "More tools" });
    await expect(submenu, "the opened submenu is role=\"menu\"").toHaveAttribute(
      "data-state",
      "open",
    );
    expect(await submenu.getAttribute("id"), "aria-controls resolves to the submenu's id").toBe(
      controlsId,
    );

    const labelledby = await submenu.getAttribute("aria-labelledby");
    expect(
      labelledby,
      'general APG menu rule (menu-roles.spec.ts\'s own citation): "An ' +
        "element with role menu either has: aria-labelledby set to a value " +
        'that refers to the menuitem or button that controls its display." ' +
        "-- docs/backlog.md row 25 fixed this for ContextMenu/Menubar's " +
        "existing submenus; must not reopen for this new component.",
    ).toBe(await subTrigger.getAttribute("id"));
  });

  test('every item inside the submenu is role="menuitem", none carries aria-selected', async ({
    page,
  }) => {
    await page.getByRole("menuitem", { name: "More tools" }).click();

    const submenu = page.getByRole("menu", { name: "More tools" });
    const items = submenu.getByRole("menuitem");
    expect(await items.count(), "this submenu has 2 items (Rename/Archive)").toBe(2);
    for (const item of await items.all()) {
      expect(
        await item.getAttribute("aria-selected"),
        "submenu items are activated, not selected -- same menu-pattern " +
          "contract as the root menu's own items",
      ).toBeNull();
    }
  });

  test('ArrowRight: "opens the submenu and places focus on its first item"', async ({ page }) => {
    // Reach the sub-trigger by roving focus, the way a keyboard user would
    // -- not `.focus()`, so this also exercises that Undo (disabled) is
    // correctly skipped on the way there.
    await page.keyboard.press("ArrowDown"); // Edit
    await page.keyboard.press("ArrowDown"); // Undo is disabled, skipped -> Duplicate
    await page.keyboard.press("ArrowDown"); // More tools
    await expect(page.getByRole("menuitem", { name: "More tools" })).toBeFocused();

    await page.keyboard.press("ArrowRight");

    await expect(
      page.getByRole("menuitem", { name: "More tools" }),
      "aria-expanded flips true",
    ).toHaveAttribute("aria-expanded", "true");
    await expect(
      page.getByRole("menuitem", { name: "Rename" }),
      "focus moved to the submenu's first item",
    ).toBeFocused();
  });

  test('Enter: "opens the submenu and places focus on its first item" (same contract as Right Arrow)', async ({
    page,
  }) => {
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown");
    await expect(page.getByRole("menuitem", { name: "More tools" })).toBeFocused();

    await page.keyboard.press("Enter");

    await expect(page.getByRole("menuitem", { name: "Rename" })).toBeFocused();
  });

  test('Left Arrow: "closes the submenu and returns focus to the parent menuitem" — and the parent menu stays open', async ({
    page,
  }) => {
    const trigger = page.getByRole("button", { name: "Open Menu" });
    const subTrigger = page.getByRole("menuitem", { name: "More tools" });

    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowRight");
    await expect(page.getByRole("menuitem", { name: "Rename" })).toBeFocused();

    await page.keyboard.press("ArrowLeft");

    await expect(subTrigger, "closes the submenu").toHaveAttribute("aria-expanded", "false");
    await expect(subTrigger, "returns focus to the parent menuitem").toBeFocused();
    await expect(
      trigger,
      "the parent (root) menu stays open when a submenu closes",
    ).toHaveAttribute("data-state", "open");
  });

  test('Escape: "closes the menu that contains focus and returns focus to ... the parent menuitem ... from which the menu was opened" — the parent menu stays open', async ({
    page,
  }) => {
    const trigger = page.getByRole("button", { name: "Open Menu" });
    const subTrigger = page.getByRole("menuitem", { name: "More tools" });

    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowRight");
    await expect(page.getByRole("menuitem", { name: "Rename" })).toBeFocused();

    await page.keyboard.press("Escape");

    await expect(
      subTrigger,
      '"the menu that contains focus" (the submenu) closes',
    ).toHaveAttribute("aria-expanded", "false");
    await expect(
      subTrigger,
      '"return focus to ... parent menuitem ... from which the menu was opened"',
    ).toBeFocused();
    await expect(
      trigger,
      "only the submenu closed -- the root menu that contains it is untouched",
    ).toHaveAttribute("data-state", "open");
  });

  test("hover: opens after a short intent delay, not instantly; leaving to a sibling item closes it", async ({
    page,
  }) => {
    const subTrigger = page.getByRole("menuitem", { name: "More tools" });
    const submenu = page.getByRole("menu", { name: "More tools" });

    await subTrigger.hover();
    await page.waitForTimeout(BEFORE_INTENT_DELAY_MS);
    await expect(
      subTrigger,
      "a pointer merely passing over the trigger must not flash the " +
        "submenu open instantly (crate::menu_sub::SUBMENU_OPEN_INTENT_DELAY)",
    ).toHaveAttribute("aria-expanded", "false");

    await page.waitForTimeout(PAST_INTENT_DELAY_MS - BEFORE_INTENT_DELAY_MS);
    await expect(subTrigger, "opens once the intent delay elapses").toHaveAttribute(
      "aria-expanded",
      "true",
    );
    await expect(submenu).toBeVisible();

    // Moving to a sibling item (not into the submenu itself) closes it.
    await page.getByRole("menuitem", { name: "Edit" }).hover();
    await expect(
      subTrigger,
      "moving to a sibling item closes the submenu (after the close-grace " +
        "delay, crate::menu_sub::SUBMENU_CLOSE_GRACE_DELAY)",
    ).toHaveAttribute("aria-expanded", "false");
  });

  // REGRESSION (bug report: "on sub-menu mouse hover, the sub-menu
  // disappears / submenus not selectable/usable"). Root cause: the
  // close-grace timer `*SubTrigger`'s own `onmouseleave` schedules used to
  // be private state (`use_delayed_action()` called directly inside
  // `*SubTrigger`), so only the trigger's own `onmouseenter` could cancel
  // it -- `*SubContent`/`*SubContentRendered` had no `onmouseenter`/
  // `onmouseleave` handlers at all. A pointer that left the trigger and
  // moved into the submenu's own content (the expected way to reach a
  // submenu item) never touched the trigger again, so the close scheduled
  // on the way out fired unconditionally ~200ms later and closed the
  // submenu out from under the pointer, regardless of it still hovering
  // the content. Fixed by hoisting the hover timers onto the shared
  // `SubMenuState` (`primitives/src/menu_sub.rs`) so `*SubContentRendered`
  // can cancel/reschedule the same timer from its own `onmouseenter`/
  // `onmouseleave`. This test drives real intermediate `mousemove` events
  // (via `page.mouse.move(..., { steps })`) from the trigger, across the
  // trigger/content boundary, and onto a submenu item -- `.hover()` alone
  // can synthesize a single jump that skips the boundary this bug lives at.
  test("hover: moving the pointer off the trigger and into the submenu content keeps it open and its item clickable", async ({
    page,
  }) => {
    const subTrigger = page.getByRole("menuitem", { name: "More tools" });
    const submenu = page.getByRole("menu", { name: "More tools" });
    const renameItem = page.getByRole("menuitem", { name: "Rename" });

    const triggerBox = await subTrigger.boundingBox();
    if (!triggerBox) throw new Error("sub-trigger has no bounding box");

    // Move the pointer onto the trigger with real intermediate mousemove
    // events, then wait past the hover-intent delay for the submenu to
    // open.
    await page.mouse.move(triggerBox.x + 5, triggerBox.y + triggerBox.height / 2);
    await page.mouse.move(
      triggerBox.x + triggerBox.width / 2,
      triggerBox.y + triggerBox.height / 2,
      { steps: 10 },
    );
    await page.waitForTimeout(PAST_INTENT_DELAY_MS);
    await expect(subTrigger, "submenu opens on hover-intent").toHaveAttribute(
      "aria-expanded",
      "true",
    );
    await expect(submenu).toBeVisible();

    const renameBox = await renameItem.boundingBox();
    if (!renameBox) throw new Error("Rename item has no bounding box");

    // Cross from the trigger into the submenu content with several
    // intermediate steps -- the exact "diagonal move toward the submenu"
    // path a real user's pointer takes, landing on the submenu's own first
    // item rather than back on the trigger.
    await page.mouse.move(
      renameBox.x + renameBox.width / 2,
      renameBox.y + renameBox.height / 2,
      { steps: 15 },
    );

    // Wait past the close-grace delay. Before the fix, this is exactly
    // when the trigger's stale close timer fired and closed the submenu
    // out from under the pointer even though it is now resting on the
    // submenu's own item.
    await page.waitForTimeout(PAST_INTENT_DELAY_MS);
    await expect(
      subTrigger,
      "hovering the submenu's own content must keep it open past the " +
        "close-grace delay, not just the trigger",
    ).toHaveAttribute("aria-expanded", "true");
    await expect(submenu, "the submenu itself must still be visible").toBeVisible();
    await expect(
      renameItem,
      "and its item must still be in the DOM/visible, i.e. actually usable",
    ).toBeVisible();

    // The submenu must still be genuinely usable: clicking the item it's
    // resting on should select it.
    await renameItem.click();
    await expect(
      page.getByText("Selected: Rename"),
      "the item hovered inside the submenu content is actually clickable/selectable",
    ).toBeVisible();
  });

  // REGRESSION (`docs/backlog.md` row 85). "Calling .focus() directly on a
  // submenu's own sub-trigger element (as a test driving focus
  // programmatically does, and as an assistive-technology focus-
  // restoration path plausibly could) closes the entire menu and reverts
  // focus to the ROOT trigger, rather than simply focusing the sub-trigger
  // in place." Root cause: `DropdownMenuTrigger`'s own `onblur` decided
  // "did focus leave the whole menu" by synchronously reading `ctx.focus.
  // any_focused()` (a Rust-tracked roving-focus signal) the instant this
  // trigger blurred -- correct for every focus move this crate's own
  // click/keyboard code drives (all of which update that signal *before*
  // moving DOM focus), but never told about a focus move arriving any
  // other way. Confirmed RED on the unmodified tree with this exact
  // sequence: `trigger` ended up `data-state="closed"` and the "More
  // tools" sub-trigger was gone from the DOM entirely (the whole menu had
  // unmounted), with focus reverted to `trigger` itself -- exactly the
  // reported symptom. Fixed by construction:
  // `DropdownMenuContentRendered` now calls `use_outside_dismiss` (the
  // same DOM-truth-based "did focus/a click land outside the whole
  // widget" check `ContextMenu`'s own root already used, immune to this
  // by construction since it asks `element.contains(event.target)` rather
  // than trusting a signal), and `DropdownMenuTrigger`'s fragile `onblur`
  // branch is removed.
  test("a raw .focus() call on the sub-trigger does not close the whole menu (row 85)", async ({
    page,
  }) => {
    const trigger = page.getByRole("button", { name: "Open Menu" });
    const subTrigger = page.getByRole("menuitem", { name: "More tools" });
    // Mouse-click-opened, deliberately with NO keyboard navigation
    // afterward -- this is exactly the precondition the bug needs:
    // `ctx.focus` (the root's own roving-focus collection) is never
    // populated by a plain click-open.
    await expect(subTrigger).toBeVisible();

    await subTrigger.focus();
    // Give any (correctly, now, no-op) reactive close a moment to have
    // fired if it were going to -- avoids a false green from asserting
    // before a real regression's own close would have completed.
    await page.waitForTimeout(150);

    await expect(trigger, "the whole menu must not close").toHaveAttribute("data-state", "open");
    await expect(
      subTrigger,
      "the sub-trigger must simply be focused in place, not torn out of the DOM by the whole menu unmounting",
    ).toBeFocused();
  });
});

test.describe("APG Menu and Menubar pattern — ContextMenu.Sub", () => {
  test.beforeEach(async ({ page }) => {
    await goto(page, "context_menu");
    await page.getByRole("button", { name: "right click here" }).click({ button: "right" });
  });

  test('sub-trigger: role="menuitem", aria-haspopup="menu" or "true", aria-expanded reflects open state', async ({
    page,
  }) => {
    const subTrigger = page.getByRole("menuitem", { name: "More tools" });
    await expect(subTrigger).toBeVisible();

    const haspopup = await subTrigger.getAttribute("aria-haspopup");
    expect(haspopup, `OBSERVED: aria-haspopup="${haspopup}".`).toMatch(/^(menu|true)$/);
    await expect(subTrigger).toHaveAttribute("aria-expanded", "false");

    await subTrigger.click();

    await expect(subTrigger).toHaveAttribute("aria-expanded", "true");
  });

  test('submenu popup is role="menu" with an accessible name, every item is role="menuitem"', async ({
    page,
  }) => {
    const subTrigger = page.getByRole("menuitem", { name: "More tools" });
    await subTrigger.click();

    const submenu = page.getByRole("menu", { name: "More tools" });
    await expect(submenu).toHaveAttribute("data-state", "open");
    expect(
      await submenu.getAttribute("aria-labelledby"),
      "docs/backlog.md row 25's rule, applied to this new component -- " +
        "must not reopen the gap that row closed for ContextMenu's " +
        "existing (non-nested) submenu shape",
    ).toBe(await subTrigger.getAttribute("id"));

    const items = submenu.getByRole("menuitem");
    expect(await items.count(), "this submenu has 2 items (Rename/Archive)").toBe(2);
    for (const item of await items.all()) {
      expect(await item.getAttribute("aria-selected")).toBeNull();
    }
  });

  test('ArrowRight/Enter open the submenu and move focus to its first item; Left Arrow/Escape close it and return focus, leaving the root menu open', async ({
    page,
  }) => {
    const rootMenu = page.getByRole("menu").filter({ has: page.getByRole("menuitem", { name: "Edit" }) });
    const subTrigger = page.getByRole("menuitem", { name: "More tools" });

    // Edit(0) -> Undo(1, disabled, skipped) -> Duplicate(2) -> Delete(3) ->
    // More tools(4) -- see this demo's own top-of-file index-layout comment.
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown");
    await expect(subTrigger).toBeFocused();

    await page.keyboard.press("ArrowRight");
    await expect(subTrigger).toHaveAttribute("aria-expanded", "true");
    await expect(page.getByRole("menuitem", { name: "Rename" })).toBeFocused();

    await page.keyboard.press("ArrowLeft");
    await expect(subTrigger, "closes the submenu").toHaveAttribute("aria-expanded", "false");
    await expect(subTrigger, "returns focus to the parent menuitem").toBeFocused();
    await expect(rootMenu, "the root context menu stays open").toHaveAttribute(
      "data-state",
      "open",
    );

    // Re-open and close with Enter/Escape this time, same contract.
    await page.keyboard.press("Enter");
    await expect(page.getByRole("menuitem", { name: "Rename" })).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(subTrigger).toHaveAttribute("aria-expanded", "false");
    await expect(subTrigger).toBeFocused();
    await expect(rootMenu, "still open after the submenu's own Escape").toHaveAttribute(
      "data-state",
      "open",
    );
  });

  test("hover: opens after a short intent delay, not instantly; leaving to a sibling item closes it", async ({
    page,
  }) => {
    const subTrigger = page.getByRole("menuitem", { name: "More tools" });
    const submenu = page.getByRole("menu", { name: "More tools" });

    await subTrigger.hover();
    await page.waitForTimeout(BEFORE_INTENT_DELAY_MS);
    await expect(subTrigger).toHaveAttribute("aria-expanded", "false");

    await page.waitForTimeout(PAST_INTENT_DELAY_MS - BEFORE_INTENT_DELAY_MS);
    await expect(subTrigger).toHaveAttribute("aria-expanded", "true");
    await expect(submenu).toBeVisible();

    await page.getByRole("menuitem", { name: "Edit" }).hover();
    await expect(subTrigger).toHaveAttribute("aria-expanded", "false");
  });

  // REGRESSION -- see the identical `DropdownMenu.Sub` test above for the
  // full root-cause writeup (`primitives/src/menu_sub.rs`'s
  // `SubMenuState::hover_close`, `primitives/src/context_menu.rs`'s
  // `ContextMenuSubContentRendered`); this is the same defect and the same
  // fix, exercised against `ContextMenu.Sub` instead, since the two hosts
  // duplicate this hover-timer wiring independently (not one shared code
  // path) and both needed the same fix.
  test("hover: moving the pointer off the trigger and into the submenu content keeps it open and its item clickable", async ({
    page,
  }) => {
    const subTrigger = page.getByRole("menuitem", { name: "More tools" });
    const submenu = page.getByRole("menu", { name: "More tools" });
    const renameItem = page.getByRole("menuitem", { name: "Rename" });

    const triggerBox = await subTrigger.boundingBox();
    if (!triggerBox) throw new Error("sub-trigger has no bounding box");

    await page.mouse.move(triggerBox.x + 5, triggerBox.y + triggerBox.height / 2);
    await page.mouse.move(
      triggerBox.x + triggerBox.width / 2,
      triggerBox.y + triggerBox.height / 2,
      { steps: 10 },
    );
    await page.waitForTimeout(PAST_INTENT_DELAY_MS);
    await expect(subTrigger, "submenu opens on hover-intent").toHaveAttribute(
      "aria-expanded",
      "true",
    );
    await expect(submenu).toBeVisible();

    const renameBox = await renameItem.boundingBox();
    if (!renameBox) throw new Error("Rename item has no bounding box");

    await page.mouse.move(
      renameBox.x + renameBox.width / 2,
      renameBox.y + renameBox.height / 2,
      { steps: 15 },
    );

    await page.waitForTimeout(PAST_INTENT_DELAY_MS);
    await expect(
      subTrigger,
      "hovering the submenu's own content must keep it open past the " +
        "close-grace delay, not just the trigger",
    ).toHaveAttribute("aria-expanded", "true");
    await expect(submenu, "the submenu itself must still be visible").toBeVisible();
    await expect(renameItem, "and its item must still be visible").toBeVisible();

    await renameItem.click();
    await expect(
      page.getByText("Selected: rename"),
      "the item hovered inside the submenu content is actually clickable/selectable",
    ).toBeVisible();
  });

  // CONSTRUCTION-PARITY regression, not a reproduced defect. `docs/backlog.md`
  // row 85 reports this closing-everything symptom for "BOTH DropdownMenu
  // and ContextMenu" -- this lane's own investigation (`$S/round4/fix-menu-
  // toggle/`) could reproduce it for `DropdownMenu`/`DropdownMenu.Sub` (see
  // the identical test in the `DropdownMenu.Sub` describe block above) and
  // for `Menubar` (`playwright/menubar.spec.ts`), but this exact "mouse-
  // opened root, then raw `.focus()` directly on `ContextMenuSubTrigger`"
  // sequence was already GREEN on the unmodified tree: `ContextMenu`'s own
  // root dismissal already went through `use_outside_dismiss`
  // (`context_menu.rs`, pre-existing, unchanged by this fix) rather than a
  // signal-based `onblur` guard, so it never had the root-level failure
  // mode `DropdownMenuTrigger` did. `ContextMenuSubTrigger`'s own `onblur`
  // (the submenu-*level* instance) carried the identical fragile shape
  // (`focused() && !sub.focus.any_focused()`, `context_menu.rs`) as
  // `DropdownMenuSubTrigger`'s, so it is fixed by the same construction
  // (`crate::menu_sub::use_sub_outside_dismiss`) for uniformity and
  // defense-in-depth, not because this test caught it red -- kept here as
  // a permanent regression guard now that both hosts share the construction.
  test("a raw .focus() call on the sub-trigger does not close the whole menu (row 85, construction parity)", async ({
    page,
  }) => {
    const rootMenu = page.getByRole("menu").first();
    const subTrigger = page.getByRole("menuitem", { name: "More tools" });
    await expect(subTrigger).toBeVisible();

    await subTrigger.focus();
    await page.waitForTimeout(150);

    await expect(rootMenu, "the whole menu must not close").toHaveAttribute("data-state", "open");
    await expect(subTrigger, "the sub-trigger must simply be focused in place").toBeFocused();
  });
});
