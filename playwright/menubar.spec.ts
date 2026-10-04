import { test, expect } from "./fixtures";
import { type Locator, type Page } from "@playwright/test";
import { expectNoAxeViolations, EXCLUDE_VENDORED_CODE_HIGHLIGHT } from "./axe";
import { BASE_URL } from "./base-url";
import { gotoHydrated } from "./hydration";

test("pointer navigation", async ({ page }) => {
  await page.goto(`${BASE_URL}/component/?name=menubar&`, { timeout: 20 * 60 * 1000 }); // Increase timeout to 20 minutes
  const fileMenuButton = page.getByRole("menuitem", { name: "File" });
  await fileMenuButton.click();
  // Assert the menu is open
  const fileMenuContent = page.getByRole("menu").filter({ has: page.getByRole("menuitem", { name: "New" }) }).last();
  await expect(fileMenuContent).toHaveAttribute("data-state", "open");

  // After the menu is open, hover over the Edit menu item
  const editMenuButton = page.getByRole("menuitem", { name: "Edit" });
  await editMenuButton.hover();
  // Assert the Edit menu content is open
  const editMenuContent = page.getByRole("menu").filter({ has: page.getByRole("menuitem", { name: "Cut" }) }).last();
  await expect(editMenuContent).toHaveAttribute("data-state", "open");
  // Assert the File menu content is closed
  await expect(fileMenuContent).toHaveCount(0);

  // Click the Cut menu item
  const cutItem = editMenuContent.getByRole("menuitem", { name: "Cut" });
  await cutItem.click();
  // Assert the menu is closed after clicking a menu item
  await expect(fileMenuContent).toHaveCount(0);
});

test("keyboard navigation", async ({ page }) => {
  await page.goto(`${BASE_URL}/component/?name=menubar&`, { timeout: 20 * 60 * 1000 }); // Increase timeout to 20 minutes
  await page.locator("#component-preview-frame").first().getByRole("menubar").focus();
  const fileMenuButton = page.getByRole("menuitem", { name: "File" });
  // Go right with the keyboard
  await page.keyboard.press("ArrowRight");
  // Assert the focus is on the Edit menu item
  const editMenuButton = page.getByRole("menuitem", { name: "Edit" });
  await expect(editMenuButton).toBeFocused();
  // Go left with the keyboard
  await page.keyboard.press("ArrowLeft");
  // Assert the focus is on the File menu item
  await expect(fileMenuButton).toBeFocused();
  // Open the File menu
  await page.keyboard.press("ArrowDown");
  // Assert the File menu content is open
  const fileMenuContent = page.getByRole("menu").filter({ has: page.getByRole("menuitem", { name: "New" }) }).last();
  await expect(fileMenuContent).toHaveAttribute("data-state", "open");

  // assert the new item is focused
  const newItem = fileMenuContent.getByRole("menuitem", { name: "New" });
  await expect(newItem).toBeFocused();
  await expect(fileMenuContent.getByRole("menuitem", { name: "Open" })).toHaveAttribute("data-disabled", "true");
  await page.keyboard.press("ArrowDown");
  await expect(fileMenuContent.getByRole("menuitem", { name: "Save" })).toBeFocused();
  // Click the focused Save menu item
  await page.keyboard.press("Enter");
  // Assert the menu is closed after clicking a menu item
  await expect(fileMenuContent).toHaveCount(0);
});

// REGRESSION (`docs/backlog.md` row 85). The most directly reproducible
// instance of the whole class this row reports: `MenubarItem`'s own
// `onblur` used to close this menu unconditionally whenever `focused()`
// (it still held `menu_ctx.focus`'s roving focus the instant it blurred).
// Arrow-down onto an item so it genuinely holds DOM focus, then a raw
// `.focus()` call directly on `MenubarTrigger` -- confirmed RED on the
// unmodified tree: `fileMenuContent` was torn out of the DOM entirely
// (count 0) and focus reverted to `fileMenuButton`. No mouse-opened
// precondition needed at all here (contrast `dropdown-menu.spec.ts`'s
// identical-shaped test, which does need one) -- `MenubarItem`'s guard
// never had `DropdownMenuTrigger`'s `submenu_open_count`-style escape
// hatch, so it fired on this exact sequence unconditionally. Fixed by
// construction: `MenubarContentRendered` now calls `use_outside_dismiss`
// (`primitives/src/lib.rs`, the same DOM-truth-based check `ContextMenu`'s
// own root already used), scoped to `MenubarMenu`'s own wrapping element
// (`menu_ctx.root_id`, new) which covers both `MenubarTrigger` and
// `MenubarContent` -- and `MenubarTrigger`'s/`MenubarItem`'s fragile
// `onblur` branches are removed.
test("a raw .focus() call on the trigger does not close its own open content (row 85)", async ({ page }) => {
  await page.goto(`${BASE_URL}/component/?name=menubar&`, { timeout: 20 * 60 * 1000 });
  const fileMenuButton = page.getByRole("menuitem", { name: "File" });
  const fileMenuContent = page.getByRole("menu").filter({ has: page.getByRole("menuitem", { name: "New" }) }).last();

  await fileMenuButton.click();
  await expect(fileMenuContent).toHaveAttribute("data-state", "open");
  // Arrow-down so a `MenubarItem` genuinely holds DOM focus -- the
  // precondition `MenubarItem`'s own (removed) `onblur` needed.
  await page.keyboard.press("ArrowDown");
  const newItem = fileMenuContent.getByRole("menuitem", { name: "New" });
  await expect(newItem).toBeFocused();

  await fileMenuButton.focus();
  // Give any (correctly, now, no-op) reactive close a moment to have
  // fired if it were going to -- avoids a false green from asserting
  // before a real regression's own close would have completed.
  await page.waitForTimeout(150);

  await expect(fileMenuContent, "the content must not close").toHaveCount(1);
  await expect(fileMenuContent, "and must still report itself open").toHaveAttribute(
    "data-state",
    "open",
  );
  await expect(fileMenuButton, "the trigger must simply be focused in place").toBeFocused();
});

// --- Checkable items: MenubarCheckboxItem / MenubarRadioGroup / MenubarRadioItem ---
//
// shadcn's Menubar has CheckboxItem and RadioGroup + RadioItem; this one had
// neither. They are one construction shared with DropdownMenu and ContextMenu
// (`primitives/src/menu_item.rs`), graded here for Menubar's own wiring:
// role/aria contract, pointer (pointerdown, like every Menubar item) and
// keyboard behaviour, close semantics (closing a menu hands focus back to its
// trigger, the APG menubar rule), roving focus and typeahead. The shared role
// contract is also graded across all three hosts by
// `oracle/tier1-apg/menu-roles.spec.ts`; the label/indicator geometry by
// `menu-indicator-gap.spec.ts`.
//
// The `checkboxes` and `radio_group` variants keep the menu open after a
// toggle (`close_on_select: false`) -- the APG-optional behaviour for Space
// ("changes the state without closing the menu", menu-and-menubar-pattern.html,
// "Keyboard Interaction"). The `rtl` variant's "Pinned"/"Ascending" items use
// the primitive's default, Radix's `onSelect` default: selecting closes the menu.
const block = (variant: string) => `${BASE_URL}/component/block/?name=menubar&variant=${variant}&`;

/** The open popup of the menu that contains `inner` (the popup, not its always-rendered `role="menu"` wrapper). */
const popupHaving = (page: Page, inner: Locator) => page.getByRole("menu").filter({ has: inner }).last();

test.describe("Checkable items", () => {
  test("checkbox items: menuitemcheckbox with an always-present aria-checked; a click toggles, the menu stays open", async ({ page }) => {
    await gotoHydrated(page, block("checkboxes"));
    await page.getByRole("menuitem", { name: "View" }).click();
    const view = popupHaving(page, page.getByRole("menuitemcheckbox", { name: "Always Show Full URLs" }));
    await expect(view).toHaveAttribute("data-state", "open");

    await expect(view.getByRole("menuitemcheckbox"), "two checkbox items in View").toHaveCount(2);
    await expect(view.getByRole("menuitem"), "Reload and Force Reload stay plain menuitems").toHaveCount(2);
    const bookmarks = view.getByRole("menuitemcheckbox", { name: "Always Show Bookmarks Bar" });
    const fullUrls = view.getByRole("menuitemcheckbox", { name: "Always Show Full URLs" });
    await expect(bookmarks).toHaveAttribute("aria-checked", "false");
    await expect(bookmarks).toHaveAttribute("data-state", "unchecked");
    await expect(fullUrls).toHaveAttribute("aria-checked", "true");
    await expect(fullUrls).toHaveAttribute("data-state", "checked");
    // Plain items carry no aria-checked at all.
    await expect(view.getByRole("menuitem", { name: "Reload", exact: true })).not.toHaveAttribute("aria-checked", /.*/);

    await bookmarks.click();
    await expect(bookmarks).toHaveAttribute("aria-checked", "true");
    await expect(view, "close_on_select: false keeps the menu open").toHaveAttribute("data-state", "open");
    await fullUrls.click();
    await expect(fullUrls).toHaveAttribute("aria-checked", "false");
    await expect(view).toHaveAttribute("data-state", "open");
  });

  test("checkbox items: arrow keys skip the disabled item, Space and Enter toggle in place, typeahead reaches them", async ({ page }) => {
    await gotoHydrated(page, block("checkboxes"));
    const viewTrigger = page.getByRole("menuitem", { name: "View" });
    await viewTrigger.focus();
    await page.keyboard.press("ArrowDown");
    const view = popupHaving(page, page.getByRole("menuitemcheckbox", { name: "Always Show Full URLs" }));
    await expect(view).toHaveAttribute("data-state", "open");
    const bookmarks = view.getByRole("menuitemcheckbox", { name: "Always Show Bookmarks Bar" });
    const fullUrls = view.getByRole("menuitemcheckbox", { name: "Always Show Full URLs" });
    const reload = view.getByRole("menuitem", { name: "Reload", exact: true });

    await expect(bookmarks, "ArrowDown on the trigger opens and focuses the first item").toBeFocused();
    await page.keyboard.press("ArrowDown");
    await expect(fullUrls).toBeFocused();
    await page.keyboard.press("ArrowDown");
    await expect(reload, "roving focus runs through checkable and plain items alike").toBeFocused();
    // Reload(2) -> Force Reload(3, disabled, skipped) -> wraps to Bookmarks(0).
    await page.keyboard.press("ArrowDown");
    await expect(bookmarks).toBeFocused();

    // APG (Optional): Space on a menuitemcheckbox "changes the state without
    // closing the menu" -- what close_on_select: false gives.
    await page.keyboard.press("Space");
    await expect(bookmarks).toHaveAttribute("aria-checked", "true");
    await expect(view).toHaveAttribute("data-state", "open");
    await expect(bookmarks, "focus stays on the toggled item").toBeFocused();
    await page.keyboard.press("Enter");
    await expect(bookmarks).toHaveAttribute("aria-checked", "false");
    await expect(view).toHaveAttribute("data-state", "open");

    // Typeahead: both checkbox labels start with "Always", so repeating "a"
    // cycles them; a plain item (Reload, via its value) is a target too.
    await page.keyboard.press("a");
    await expect(fullUrls, "typeahead 'a' reaches the next checkbox item").toBeFocused();
    await page.waitForTimeout(1100);
    await page.keyboard.press("r");
    await expect(reload).toBeFocused();
    // The disabled Force Reload is never a candidate ("f" matches nothing else).
    await page.waitForTimeout(1100);
    await page.keyboard.press("f");
    await expect(reload, "typeahead 'f' must not land on the disabled Force Reload").toBeFocused();
  });

  test("radio groups: role=group named by its label, one checked item per group, a choice moves the check and nothing else", async ({ page }) => {
    await gotoHydrated(page, block("radio_group"));
    await page.getByRole("menuitem", { name: "Accounts" }).click();
    const group = page.getByRole("group", { name: "Switch account" });
    await expect(group, "the group is named by its heading via aria-labelledby").toBeVisible();
    await expect(group.getByRole("menuitemradio")).toHaveCount(3);
    const checked = () => group.locator('[role="menuitemradio"][aria-checked="true"]');
    await expect(checked()).toHaveCount(1);
    await expect(group.getByRole("menuitemradio", { name: "Benoit" })).toHaveAttribute("aria-checked", "true");

    await group.getByRole("menuitemradio", { name: "Luis" }).click();
    await expect(group.getByRole("menuitemradio", { name: "Luis" })).toHaveAttribute("aria-checked", "true");
    await expect(group.getByRole("menuitemradio", { name: "Benoit" })).toHaveAttribute("aria-checked", "false");
    await expect(checked(), "still exactly one radio checked after a choice").toHaveCount(1);
    await expect(
      popupHaving(page, group),
      "close_on_select: false keeps the menu open",
    ).toHaveAttribute("data-state", "open");

    // The second menu's group is separate, and still on its own value.
    await page.getByRole("menuitem", { name: "Theme" }).hover();
    const theme = page.getByRole("group", { name: "Theme" });
    await expect(theme.getByRole("menuitemradio", { name: "System" })).toHaveAttribute("aria-checked", "true");
    await expect(theme.locator('[role="menuitemradio"][aria-checked="true"]')).toHaveCount(1);
  });

  test("radio groups: Space and Enter choose in place, typeahead falls back to the value", async ({ page }) => {
    await gotoHydrated(page, block("radio_group"));
    await page.getByRole("menuitem", { name: "Accounts" }).focus();
    await page.keyboard.press("ArrowDown");
    const group = page.getByRole("group", { name: "Switch account" });
    const andy = group.getByRole("menuitemradio", { name: "Andy" });
    const benoit = group.getByRole("menuitemradio", { name: "Benoit" });
    const luis = group.getByRole("menuitemradio", { name: "Luis" });
    const menu = popupHaving(page, group);

    await expect(andy).toBeFocused();
    await page.keyboard.press("Space");
    await expect(andy).toHaveAttribute("aria-checked", "true");
    await expect(benoit).toHaveAttribute("aria-checked", "false");
    await expect(menu).toHaveAttribute("data-state", "open");

    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown");
    await expect(luis).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(luis).toHaveAttribute("aria-checked", "true");
    await expect(andy).toHaveAttribute("aria-checked", "false");
    await expect(menu).toHaveAttribute("data-state", "open");

    // A radio item has no text_value here: typeahead falls back to its value.
    await page.keyboard.press("b");
    await expect(benoit, "typeahead 'b' reaches Benoit via its value").toBeFocused();
  });

  test("by default (Radix onSelect default) choosing a checkable item closes the menu, and the new state is kept; by keyboard focus returns to its trigger", async ({ page }) => {
    await gotoHydrated(page, block("rtl"));
    const options = page.getByRole("menuitem", { name: "Options" });
    await options.click();
    const pinned = page.getByRole("menuitemcheckbox", { name: "Pinned" });
    const optionsMenu = popupHaving(page, pinned);
    await expect(optionsMenu).toHaveAttribute("data-state", "open");
    await expect(pinned).toHaveAttribute("aria-checked", "true");

    // Pointer: closes, and the (controlled) state survives the close. (Where
    // DOM focus lands after a pointer select is the plain MenubarItem's
    // behaviour too -- selection commits on pointerdown, the click then
    // focuses an item that is being unmounted -- so it is not asserted here.)
    await pinned.click();
    await expect(optionsMenu, "a checkbox item closes the menu by default").toHaveCount(0);
    await options.click();
    await expect(pinned, "the (controlled) state survived the close").toHaveAttribute("aria-checked", "false");
    await page.keyboard.press("Escape");

    // Keyboard: Enter toggles, closes, and -- APG menubar rule -- hands focus
    // back to the menu's own trigger. Create(0) Launch(1) Pinned(2).
    await options.focus();
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown");
    await expect(pinned).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(optionsMenu).toHaveCount(0);
    await expect(options, "closing hands focus back to the menu's own trigger (APG menubar rule)").toBeFocused();
    await page.keyboard.press("ArrowDown");
    await expect(pinned, "Enter toggled it back on").toHaveAttribute("aria-checked", "true");
    await page.keyboard.press("Escape");

    // Keyboard: Enter on a radio item chooses it and closes the menu.
    // Remove(0) Duplicate(1) Ascending(2) Descending(3).
    const modify = page.getByRole("menuitem", { name: "Modify" });
    await modify.focus();
    await page.keyboard.press("ArrowDown");
    const descending = page.getByRole("menuitemradio", { name: "Descending" });
    const modifyMenu = popupHaving(page, descending);
    await expect(modifyMenu).toHaveAttribute("data-state", "open");
    for (let i = 0; i < 3; i++) await page.keyboard.press("ArrowDown");
    await expect(descending).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(modifyMenu).toHaveCount(0);
    await expect(modify).toBeFocused();

    await modify.click();
    await expect(descending).toHaveAttribute("aria-checked", "true");
    await expect(page.getByRole("menuitemradio", { name: "Ascending" })).toHaveAttribute("aria-checked", "false");
  });
});

test.describe("Axe automated scan", () => {
  test("loaded (menus closed) has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=menubar&`, { timeout: 20 * 60 * 1000 });
    // Wait for render before scanning -- see input.spec.ts's identical
    // comment for why (avoids a false pre-hydration "no main"/"no h1").
    await expect(page.getByRole("menuitem", { name: "File" })).toBeVisible();
    await expectNoAxeViolations(page, "menubar: loaded", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });

  // docs/backlog.md row 25: Menubar's role="menu" popups carry no
  // aria-labelledby/aria-label at all, so an open menu has no accessible
  // name (APG menu-and-menubar pattern requires one).
  // Checkable items: role="menuitemcheckbox"/"menuitemradio" need aria-checked
  // and a menu/group context; a radio group needs to be a real `group`.
  test("View menu (checkbox items) open has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=menubar&`, { timeout: 20 * 60 * 1000 });
    await page.getByRole("menuitem", { name: "View" }).click();
    await expect(page.getByRole("menuitemcheckbox", { name: "Always Show Full URLs" })).toBeVisible();
    await expectNoAxeViolations(page, "menubar: View menu open", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });

  test("Accounts menu (radio group) open has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=menubar&`, { timeout: 20 * 60 * 1000 });
    await page.getByRole("menuitem", { name: "Accounts" }).click();
    await expect(page.getByRole("menuitemradio", { name: "Benoit" })).toBeVisible();
    await expectNoAxeViolations(page, "menubar: Accounts menu open", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });

  test("File menu open has no automatically detectable a11y issues", async ({ page }) => {
    await page.goto(`${BASE_URL}/component/?name=menubar&`, { timeout: 20 * 60 * 1000 });
    await page.getByRole("menuitem", { name: "File" }).click();
    const fileMenuContent = page.getByRole("menu").filter({ has: page.getByRole("menuitem", { name: "New" }) }).last();
    await expect(fileMenuContent).toHaveAttribute("data-state", "open");
    await expectNoAxeViolations(page, "menubar: File menu open", { excludeRegions: [EXCLUDE_VENDORED_CODE_HIGHLIGHT] });
  });
});
