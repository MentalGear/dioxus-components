/**
 * ORACLE: tier 1 (APG) — menu pattern-class role contract.
 *
 * Rule source: the APG **Menu Button** pattern and the **Menu and Menubar**
 * pattern's own "WAI-ARIA Roles, States, and Properties" sections, quoted in
 * full below and cited to a specific file + pinned commit, per
 * `tier1-apg/README.md`'s rule-source policy.
 *
 *   "The element that opens the menu has role button."
 *   "The element with role button has aria-haspopup set to either menu or
 *   true."
 *   "When the menu is displayed, the element with role button has
 *   aria-expanded set to true. When the menu is hidden, aria-expanded is
 *   set to false."
 *   "The element that contains the menu items displayed by activating the
 *   button has role menu."
 *   — APG Menu Button pattern, "WAI-ARIA Roles, States, and Properties"
 *     `content/patterns/menu-button/menu-button-pattern.html`
 *     https://www.w3.org/WAI/ARIA/apg/patterns/menu-button/
 *
 *   "The element serving as the menu has a role of either menu or menubar."
 *   "The items contained in a menu ... have any of the following roles:
 *   menuitem, menuitemcheckbox, menuitemradio."
 *   "A parent menuitem has aria-haspopup set to either menu or true."
 *   "An element with role menu either has: aria-labelledby set to a value
 *   that refers to the menuitem or button that controls its display. A
 *   label provided by aria-label."
 *   — APG Menu and Menubar pattern, "WAI-ARIA Roles, States, and Properties"
 *     `content/patterns/menubar/menu-and-menubar-pattern.html`
 *     https://www.w3.org/WAI/ARIA/apg/patterns/menubar/
 *
 * Both quoted files are narrative pattern pages, not executable examples --
 * only three patterns' *example* pages are vendored under
 * `../reference/7e4034b/` (menu-button, combobox-select-only, radio; see
 * that directory's README), so these two are not present there. The text
 * above was read from a fresh, read-only clone of `w3c/aria-practices`
 * checked out at the SAME pinned commit already vendored elsewhere in this
 * repo (`7e4034b262bc0d25332e330d8a582aaf34113829`), the same technique
 * `keyboard-matrix.spec.ts`'s file header already documents and uses for
 * its own Menubar prose citations -- not invented from memory.
 *
 * Calibration (per `tier1-apg/README.md`): the menu-button pattern's own
 * *executable* example page IS vendored
 * (`../reference/7e4034b/content/patterns/menu-button/examples/
 * menu-button-actions.html`) and is used below, loaded over `file://`
 * (mirroring `focus-restore-reference.spec.ts`'s technique -- this repo's
 * established pattern for running a tier-1 rule against its vendored
 * reference; there is no static-http-server lane for `reference/` in this
 * codebase to follow instead). That page's own "Roles, States, and
 * Properties" table duplicates the prose above practically verbatim,
 * confirming the two agree. The Menu and Menubar pattern has no vendored
 * executable example at all (`ContextMenu` and `Menubar` describe blocks
 * below are graded directly against the prose, same as
 * `keyboard-matrix.spec.ts`'s own Menubar rows).
 *
 * Subject components -- four implementations of this one pattern class:
 *   - DropdownMenu   (menu-button pattern: single trigger + popup)
 *   - ContextMenu    (menu-and-menubar pattern via "context specific menu"
 *                      invocation, same as keyboard-matrix.spec.ts's own
 *                      ContextMenu describe block)
 *   - Menubar        (menu-and-menubar pattern; submenu popups graded here,
 *                      not the `menubar`-role top-level container itself)
 *   - Navbar         (menu-and-menubar pattern, hover/click-opened nav
 *                      dropdowns; same shape as Menubar's submenus -- popup
 *                      role/items graded here, not the `menubar`-role
 *                      top-level container. Added in the same pass that
 *                      routed `navbar.rs`'s role literals through
 *                      `menu_semantics` -- docs/backlog.md rows 24, 25, 41 --
 *                      since a fourth hand-written role set was exactly what
 *                      this file exists to catch)
 *
 * Contract asserted per component: popup is role="menu"; every plain item is
 * role="menuitem"; no item carries aria-selected (a menu's items are
 * activated, not selected -- aria-selected belongs to the listbox/grid/tree/
 * tablist pattern family, not this one); a single-trigger component's
 * trigger has aria-haspopup="menu" or "true" with aria-expanded reflecting
 * open state; the open popup's aria-labelledby resolves to that trigger's id.
 *
 * CHECKABLE ITEMS: DropdownMenu, ContextMenu and Menubar each have a
 * `*CheckboxItem` and a `*RadioGroup`/`*RadioItem` (shadcn's), all built once
 * in `primitives/src/menu_item.rs` and routed through
 * `primitives/src/menu_semantics.rs`. APG permits `menuitemcheckbox` /
 * `menuitemradio` item roles ("The items contained in a menu ... have any of
 * the following roles: menuitem, menuitemcheckbox, menuitemradio" and "When a
 * menuitemcheckbox or menuitemradio is checked, aria-checked is set to true",
 * menu-and-menubar-pattern.html, same pinned commit); WAI-ARIA 1.2 makes
 * `aria-checked` a required state on both roles and says "If a menu or
 * menubar contains more than one group of menuitemradio elements, or if the
 * menu contains one group and other, unrelated menu items, authors SHOULD
 * contain each set of related menuitemradio elements in an element using the
 * group role" (https://www.w3.org/TR/wai-aria-1.2/#menuitemradio). The
 * "Checkable items" describe blocks below grade the legal half -- the
 * checkable roles appear exactly where a checkable item is rendered, always
 * carry `aria-checked="true"|"false"`, and radios sit in a `group` with
 * exactly one checked -- and the illegal half: a menu with no checkable items
 * (DropdownMenu's `main`, Menubar's File menu, every Navbar nav dropdown)
 * contains no checkable role, no `group`, and no `aria-checked` anywhere, and
 * no menu ever contains the standalone `checkbox`/`radio`/`option`/`listbox`
 * roles (a menu's checkable items are `menuitemcheckbox`/`menuitemradio`).
 *
 * SCOPE NOTE, found verifying this file (contradicts nothing in the
 * contract above, but narrows what is asserted where): `ContextMenu`'s and
 * `Menubar`'s submenu popups carry no aria-labelledby/aria-label at all
 * (`primitives/src/context_menu.rs`, `primitives/src/menubar.rs` -- grepped,
 * zero matches), unlike `DropdownMenu`'s, which does. That is a real,
 * pre-existing accessible-name gap distinct from the listbox/option defect
 * this file's DropdownMenu rows exist to catch (`docs/backlog.md` row 24) --
 * filed as a fresh backlog candidate, not asserted here, so as not to turn
 * an unrelated, already-latent gap into a manufactured red on components
 * this task is not fixing. Likewise, `Menubar`'s own top-level
 * `MenubarTrigger` ("File"/"Edit") has neither aria-haspopup nor
 * aria-expanded at all (grepped, zero matches) despite being the "parent
 * menuitem" the Menu and Menubar prose above requires both of -- also a
 * real, separate, pre-existing gap, so the trigger-contract assertions
 * below run only against DropdownMenu and ContextMenu, whose triggers do
 * carry `aria-haspopup` today; the Menubar describe block asserts the
 * popup/item role contract only. `NavbarTrigger` (`primitives/src/
 * navbar.rs`) has the identical gap for the identical reason: it is the
 * same "parent menuitem" role, in the same pattern, as `MenubarTrigger`, and
 * carries neither `aria-haspopup` nor `aria-expanded` either (grepped for
 * "haspopup"/"expanded", zero matches) -- so the Navbar describe block below
 * asserts the same popup/item role contract only, not a trigger contract.
 */

import { test, expect } from "../../fixtures";
import { type Locator, type Page } from "@playwright/test";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { BASE_URL } from "../../base-url";
import { gotoHydrated } from "../../hydration";

const BASE = `${BASE_URL}/component/?name=`;
// dev-docs/backlog.md row 109: this suite clicks triggers right after
// navigating, on the SSG lane -- `gotoHydrated` so those clicks can't land
// before hydration attaches listeners. (`menuButtonActionsUrl` below is a
// vendored static W3C reference page, not this app, so it keeps a plain
// `page.goto` -- there is no hydration signal to wait for there.)
const goto = (page: Page, name: string) =>
  gotoHydrated(page, `${BASE}${name}&`, { timeout: 20 * 60 * 1000 });

/** Every distinct `role` attribute on a descendant of `popup`, sorted. */
const rolesInside = (popup: Locator): Promise<string[]> =>
  popup.evaluate((el) =>
    Array.from(new Set(Array.from(el.querySelectorAll("[role]")).map((n) => n.getAttribute("role") ?? ""))).sort(),
  );

/** Roles that may never appear inside any menu popup of this pattern class. */
const STANDALONE_WIDGET_ROLES = ["checkbox", "radio", "option", "listbox"];

/**
 * The checkable-role contract for one open popup: the roles inside it are
 * exactly `roles` (so a checkable role can only be there when `roles` says
 * so, and a plain demo's popup can never grow one), every checkable item has
 * `aria-checked` of exactly "true" or "false", a plain item has none, nothing
 * carries `aria-selected`, and each radio group has exactly one checked radio
 * and holds every radio of its set.
 */
async function expectCheckableContract(popup: Locator, roles: string[], what: string) {
  expect(await rolesInside(popup), `${what}: roles inside the popup`).toEqual([...roles].sort());
  for (const forbidden of STANDALONE_WIDGET_ROLES) {
    await expect(popup.locator(`[role="${forbidden}"]`), `${what}: no standalone role="${forbidden}"`).toHaveCount(0);
  }
  const checkable = popup.locator('[role="menuitemcheckbox"], [role="menuitemradio"]');
  for (const item of await checkable.all()) {
    const label = (await item.textContent())?.trim();
    await expect(item, `${what}: "${label}" has aria-checked true|false, never absent`).toHaveAttribute(
      "aria-checked",
      /^(true|false)$/,
    );
  }
  for (const plain of await popup.locator('[role="menuitem"]').all()) {
    const label = (await plain.textContent())?.trim();
    await expect(plain, `${what}: plain item "${label}" carries no aria-checked`).not.toHaveAttribute(
      "aria-checked",
      /.*/,
    );
  }
  await expect(popup.locator("[aria-selected]"), `${what}: nothing carries aria-selected`).toHaveCount(0);
  for (const group of await popup.locator('[role="group"]').all()) {
    const radios = group.locator('[role="menuitemradio"]');
    expect(await radios.count(), `${what}: a radio group holds at least two radios`).toBeGreaterThanOrEqual(2);
    expect(
      await group.locator('[role="menuitemradio"][aria-checked="true"]').count(),
      `${what}: exactly one radio is checked per group`,
    ).toBe(1);
    expect(
      await group.locator("[role]:not([role='menuitemradio'])").count(),
      `${what}: a radio group holds only radios`,
    ).toBe(0);
  }
  // Every radio lives in a group: none sits directly under the menu.
  expect(
    await popup.locator('[role="menuitemradio"]').count(),
    `${what}: every menuitemradio is inside a role="group"`,
  ).toBe(await popup.locator('[role="group"] [role="menuitemradio"]').count());
}

const menuButtonActionsUrl = pathToFileURL(
  path.resolve(
    __dirname,
    "../reference/7e4034b/content/patterns/menu-button/examples/menu-button-actions.html",
  ),
).href;

test.describe("CALIBRATION — APG Menu Button pattern's own vendored example", () => {
  test("W3C's own example carries the exact role/haspopup/labelledby contract asserted below", async ({
    page,
  }) => {
    await page.goto(menuButtonActionsUrl);

    const button = page.locator("#menubutton1");
    await expect(button, 'trigger: aria-haspopup="true"').toHaveAttribute(
      "aria-haspopup",
      "true",
    );
    await expect(button, "trigger: aria-expanded starts false").toHaveAttribute(
      "aria-expanded",
      "false",
    );

    await button.click();

    await expect(button, "trigger: aria-expanded flips true when open").toHaveAttribute(
      "aria-expanded",
      "true",
    );
    const menu = page.locator("#menu1");
    await expect(menu, 'popup: role="menu"').toHaveAttribute("role", "menu");
    await expect(menu, "popup: aria-labelledby resolves to the trigger's id").toHaveAttribute(
      "aria-labelledby",
      "menubutton1",
    );
    const items = menu.locator('[role="menuitem"]');
    await expect(items.first(), 'items: role="menuitem"').toBeVisible();
    expect(await items.count(), "every child is a menuitem").toBe(4);
    for (const item of await items.all()) {
      expect(
        await item.getAttribute("aria-selected"),
        "menu items are activated, not selected -- no aria-selected",
      ).toBeNull();
    }
  });
});

test.describe("APG Menu Button pattern — DropdownMenu", () => {
  test.beforeEach(async ({ page }) => {
    await goto(page, "dropdown_menu");
  });

  test("trigger: aria-haspopup is \"menu\" or \"true\", aria-expanded reflects open state", async ({
    page,
  }) => {
    const trigger = page.getByRole("button", { name: "Open Menu" });
    const haspopup = await trigger.getAttribute("aria-haspopup");
    expect(
      haspopup,
      `APG requires aria-haspopup="menu" or "true" on a menu button's trigger. ` +
        `OBSERVED: aria-haspopup="${haspopup}".`,
    ).toMatch(/^(menu|true)$/);
    await expect(trigger, "aria-expanded starts false").toHaveAttribute(
      "aria-expanded",
      "false",
    );

    await trigger.click();

    await expect(trigger, "aria-expanded flips true when open").toHaveAttribute(
      "aria-expanded",
      "true",
    );
  });

  test('popup is role="menu" (not "listbox"), with an accessible name via aria-labelledby', async ({
    page,
  }) => {
    const trigger = page.getByRole("button", { name: "Open Menu" });
    await trigger.click();

    const menu = page.getByRole("menu");
    await expect(
      menu,
      'DropdownMenuContent must render role="menu" -- the APG menu-button ' +
        'pattern\'s popup role, not role="listbox" (the APG listbox pattern, ' +
        "which this component does not implement: no value/selected state, " +
        "no aria-selected on any item).",
    ).toHaveAttribute("data-state", "open");
    await expect(menu, "popup's aria-labelledby resolves to the trigger's id").toHaveAttribute(
      "aria-labelledby",
      (await trigger.getAttribute("id")) ?? "",
    );
  });

  test('every item is role="menuitem" (not "option"), and none carries aria-selected', async ({
    page,
  }) => {
    await page.getByRole("button", { name: "Open Menu" }).click();

    const items = page.getByRole("menuitem");
    await expect(
      items.first(),
      'DropdownMenuItem must render role="menuitem" -- activating an item ' +
        "calls on_select and closes the menu (action semantics), never " +
        "marks anything selected, so the APG listbox pattern's " +
        '"option"/aria-selected contract never applied here.',
    ).toBeVisible();
    const count = await items.count();
    expect(
      count,
      "this demo has 5 items: Edit/Undo/Duplicate/Delete plus the 'More tools' sub-trigger, which is itself a menuitem per APG",
    ).toBe(5);
    for (const item of await items.all()) {
      expect(
        await item.getAttribute("aria-selected"),
        "no DropdownMenuItem carries aria-selected -- there is no selection " +
          "model on this component at all",
      ).toBeNull();
    }
    // Negative check: the old listbox contract must be fully gone.
    await expect(page.locator('[role="listbox"]'), "no role=\"listbox\" left").toHaveCount(0);
    await expect(page.locator('[role="option"]'), "no role=\"option\" left").toHaveCount(0);
  });
});

test.describe("APG Menu pattern (context-menu invocation) — ContextMenu", () => {
  test.beforeEach(async ({ page }) => {
    await goto(page, "context_menu");
  });

  test('trigger: aria-haspopup="menu" (matches the "menu" token the Menu Button pattern permits)', async ({
    page,
  }) => {
    const trigger = page.getByRole("button", { name: "right click here" });
    await expect(trigger).toHaveAttribute("aria-haspopup", "menu");
  });

  test('popup is role="menu", every item is role="menuitem", none carries aria-selected', async ({
    page,
  }) => {
    const trigger = page.getByRole("button", { name: "right click here" });
    await trigger.click({ button: "right" });

    const menu = page.getByRole("menu");
    await expect(menu).toHaveAttribute("data-state", "open");
    // docs/backlog.md row 53: `ContextMenuContentRendered`'s aria-labelledby
    // now comes from the shared `menu_root::content_labelledby_attributes`
    // helper (same construction as DropdownMenu's identical assertion
    // above) -- asserted here so a future drift between the two hosts'
    // shared plumbing shows up as a red test, not a silent divergence.
    await expect(menu, "popup's aria-labelledby resolves to the trigger's id").toHaveAttribute(
      "aria-labelledby",
      (await trigger.getAttribute("id")) ?? "",
    );

    const items = page.getByRole("menuitem");
    expect(
      await items.count(),
      "this demo has 5 items: 4 plus the 'More tools' sub-trigger, itself a menuitem",
    ).toBe(5);
    for (const item of await items.all()) {
      expect(
        await item.getAttribute("aria-selected"),
        "context menu items are activated, not selected",
      ).toBeNull();
    }
  });
});

test.describe("APG Menu and Menubar pattern — Menubar submenus", () => {
  test.beforeEach(async ({ page }) => {
    await goto(page, "menubar");
  });

  test('submenu popup is role="menu", every item is role="menuitem", none carries aria-selected', async ({
    page,
  }) => {
    const fileTrigger = page.getByRole("menuitem", { name: "File" });
    await fileTrigger.click();

    // `MenubarMenu`'s own always-rendered wrapper div is ALSO role="menu"
    // (menubar.rs) and also matches this filter, alongside the actual popup
    // content -- `.last()` picks the popup, mirroring keyboard-matrix.spec.ts's
    // identical disambiguation for the same two-elements-both-role-menu shape.
    const fileMenu = page
      .getByRole("menu")
      .filter({ has: page.getByRole("menuitem", { name: "New" }) })
      .last();
    await expect(fileMenu).toHaveAttribute("data-state", "open");
    // docs/backlog.md row 53: `MenubarContentRendered`'s aria-labelledby
    // now comes from the shared `menu_root::content_labelledby_attributes`
    // helper too (same construction as DropdownMenu's/ContextMenu's
    // identical assertions above) -- asserted here so a future drift
    // between the three hosts' shared plumbing shows up as a red test.
    await expect(
      fileMenu,
      "submenu's aria-labelledby resolves to the trigger's id",
    ).toHaveAttribute("aria-labelledby", (await fileTrigger.getAttribute("id")) ?? "");

    const items = fileMenu.getByRole("menuitem");
    expect(await items.count(), "File submenu has 3 items (New/Open/Save)").toBe(3);
    for (const item of await items.all()) {
      expect(
        await item.getAttribute("aria-selected"),
        "submenu items are activated, not selected",
      ).toBeNull();
    }
  });
});

test.describe("APG Menu and Menubar pattern — Navbar nav dropdowns", () => {
  test.beforeEach(async ({ page }) => {
    await goto(page, "navbar");
  });

  test('nav dropdown popup is role="menu", every item is role="menuitem", none carries aria-selected', async ({
    page,
  }) => {
    // Navbar opens on hover, not click (its own gesture, unlike Menubar's
    // click-driven trigger) -- see `navbar.spec.ts`'s identical "hover
    // navigation" test for the same technique. `NavbarTrigger`'s own text
    // ("Inputs") is always rendered, but its sibling `NavbarContent` popup
    // only mounts once open, so the outer `NavbarNav` wrapper (itself also
    // role="menu" -- see below) is what has to be located and hovered first.
    const inputsNav = page
      .getByRole("menu")
      .filter({ has: page.getByRole("menuitem", { name: "Inputs" }) })
      .first();
    await inputsNav.hover();
    await expect(inputsNav).toHaveAttribute("data-state", "open");

    // `NavbarNav`'s own always-rendered wrapper div is ALSO role="menu"
    // (navbar.rs) and also matches this filter, alongside the actual popup
    // content -- `.last()` picks the popup, mirroring the Menubar describe
    // block's identical disambiguation above for the same two-elements-
    // both-role-menu shape (both wrapper components mirror one another by
    // construction, not by coincidence -- see `menu_semantics.rs`'s module
    // doc, "Scope").
    const navMenu = page
      .getByRole("menu")
      .filter({ has: page.getByRole("menuitem", { name: "Calendar" }) })
      .last();
    await expect(navMenu).toHaveAttribute("data-state", "open");

    const items = navMenu.getByRole("menuitem");
    expect(
      await items.count(),
      "Inputs nav has 4 items (Calendar/Slider/Checkbox/Radio Group)",
    ).toBe(4);
    for (const item of await items.all()) {
      expect(
        await item.getAttribute("aria-selected"),
        "nav dropdown items are activated, not selected",
      ).toBeNull();
    }
  });
});

test.describe("APG Menu Button pattern — DropdownMenu: checkable items", () => {
  test.beforeEach(async ({ page }) => {
    await goto(page, "dropdown_menu");
  });

  test("a menu of checkbox items: menuitemcheckbox (plus the sub-trigger's menuitem), aria-checked true|false on each", async ({ page }) => {
    await page.getByRole("button", { name: "Checkboxes" }).click();
    const popup = page.getByRole("menu");
    await expect(popup).toHaveAttribute("data-state", "open");
    // The "More options" sub-trigger is the one plain menuitem.
    await expectCheckableContract(popup, ["menuitem", "menuitemcheckbox"], "DropdownMenu checkboxes");
    await expect(popup.getByRole("menuitemcheckbox")).toHaveCount(3);
  });

  test("a menu of radio items: one role=group of menuitemradio with exactly one checked", async ({ page }) => {
    await page.getByRole("button", { name: "Radio Group" }).click();
    const popup = page.getByRole("menu");
    await expect(popup).toHaveAttribute("data-state", "open");
    await expectCheckableContract(popup, ["group", "menuitemradio"], "DropdownMenu radio group");
    await expect(popup.getByRole("menuitemradio")).toHaveCount(3);
  });

  test("ILLEGAL ELSEWHERE: the plain demo's menu has no checkable role, no group, no aria-checked", async ({ page }) => {
    await page.getByRole("button", { name: "Open Menu" }).click();
    const popup = page.getByRole("menu");
    await expect(popup).toHaveAttribute("data-state", "open");
    await expectCheckableContract(popup, ["menuitem"], "DropdownMenu main");
  });
});

test.describe("APG Menu pattern (context-menu invocation) — ContextMenu: checkable items", () => {
  test.beforeEach(async ({ page }) => {
    await goto(page, "context_menu");
  });

  test("a menu of checkbox items: menuitemcheckbox (plus the sub-trigger's menuitem), aria-checked true|false on each", async ({ page }) => {
    await page.getByRole("button", { name: "Right click for checkboxes" }).click({ button: "right" });
    const popup = page.getByRole("menu");
    await expect(popup).toHaveAttribute("data-state", "open");
    // The "More options" sub-trigger is the one plain menuitem.
    await expectCheckableContract(popup, ["menuitem", "menuitemcheckbox"], "ContextMenu checkboxes");
    await expect(popup.getByRole("menuitemcheckbox")).toHaveCount(3);
  });

  test("a menu of radio items: two role=group sets of menuitemradio, exactly one checked in each", async ({ page }) => {
    await page.getByRole("button", { name: "Right click for radio group" }).click({ button: "right" });
    const popup = page.getByRole("menu");
    await expect(popup).toHaveAttribute("data-state", "open");
    await expectCheckableContract(popup, ["group", "menuitemradio"], "ContextMenu radio groups");
    await expect(popup.getByRole("group")).toHaveCount(2);
  });

  test("a menu that mixes plain, checkbox and radio items has each role exactly where it was rendered", async ({ page }) => {
    await page.getByRole("button", { name: "right click here" }).click({ button: "right" });
    const popup = page.getByRole("menu");
    await expect(popup).toHaveAttribute("data-state", "open");
    await expectCheckableContract(
      popup,
      ["group", "menuitem", "menuitemcheckbox", "menuitemradio"],
      "ContextMenu main",
    );
    // 4 plain items + the sub-trigger, 2 checkbox items, 2 radio items.
    await expect(popup.getByRole("menuitem")).toHaveCount(5);
    await expect(popup.getByRole("menuitemcheckbox")).toHaveCount(2);
    await expect(popup.getByRole("menuitemradio")).toHaveCount(2);
  });
});

test.describe("APG Menu and Menubar pattern — Menubar: checkable items", () => {
  test.beforeEach(async ({ page }) => {
    await goto(page, "menubar");
  });

  test("a menu mixing plain and checkbox items: menuitem + menuitemcheckbox only", async ({ page }) => {
    await page.getByRole("menuitem", { name: "View" }).click();
    const popup = page
      .getByRole("menu")
      .filter({ has: page.getByRole("menuitemcheckbox", { name: "Always Show Full URLs" }) })
      .last();
    await expect(popup).toHaveAttribute("data-state", "open");
    await expectCheckableContract(popup, ["menuitem", "menuitemcheckbox"], "Menubar View");
    await expect(popup.getByRole("menuitemcheckbox")).toHaveCount(2);
    await expect(popup.getByRole("menuitem")).toHaveCount(2);
  });

  test("a menu of radio items: one role=group of menuitemradio with exactly one checked", async ({ page }) => {
    await page.getByRole("menuitem", { name: "Accounts" }).click();
    const popup = page
      .getByRole("menu")
      .filter({ has: page.getByRole("menuitemradio", { name: "Benoit" }) })
      .last();
    await expect(popup).toHaveAttribute("data-state", "open");
    await expectCheckableContract(popup, ["group", "menuitemradio"], "Menubar Accounts");
    await expect(popup.getByRole("menuitemradio")).toHaveCount(3);
  });

  test("ILLEGAL ELSEWHERE: the plain File menu has no checkable role, no group, no aria-checked", async ({ page }) => {
    await page.getByRole("menuitem", { name: "File" }).click();
    const popup = page
      .getByRole("menu")
      .filter({ has: page.getByRole("menuitem", { name: "New" }) })
      .last();
    await expect(popup).toHaveAttribute("data-state", "open");
    await expectCheckableContract(popup, ["menuitem"], "Menubar File");
  });
});

test.describe("APG Menu and Menubar pattern — Navbar nav dropdowns: no checkable items", () => {
  test("ILLEGAL ELSEWHERE: a nav dropdown has no checkable role, no group, no aria-checked", async ({ page }) => {
    await goto(page, "navbar");
    const inputsNav = page
      .getByRole("menu")
      .filter({ has: page.getByRole("menuitem", { name: "Inputs" }) })
      .first();
    await inputsNav.hover();
    await expect(inputsNav).toHaveAttribute("data-state", "open");
    const navMenu = page
      .getByRole("menu")
      .filter({ has: page.getByRole("menuitem", { name: "Calendar" }) })
      .last();
    await expect(navMenu).toHaveAttribute("data-state", "open");
    const roles = await rolesInside(navMenu);
    for (const forbidden of ["menuitemcheckbox", "menuitemradio", "group", ...STANDALONE_WIDGET_ROLES]) {
      expect(roles, `Navbar items are links, never checkable: found role="${forbidden}"`).not.toContain(forbidden);
    }
    await expect(navMenu.locator("[aria-checked]")).toHaveCount(0);
  });
});
