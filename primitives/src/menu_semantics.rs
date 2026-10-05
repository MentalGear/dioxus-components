//! Single source of truth for the ARIA role/token triple shared by every
//! implementation of the APG "Menu Button" and "Menu and Menubar" pattern
//! class in this crate: [`dropdown_menu`](crate::dropdown_menu),
//! [`context_menu`](crate::context_menu), [`menubar`](crate::menubar)'s
//! submenus, and [`navbar`](crate::navbar)'s nav dropdowns.
//!
//! # Why this module exists
//!
//! Before this module, each of `dropdown_menu.rs`, `context_menu.rs`, and
//! `menubar.rs` hand-wrote its own `role`/`aria-haspopup` string literals.
//! `context_menu.rs` and `menubar.rs` correctly used the menu pattern's
//! roles; `dropdown_menu.rs` instead carried `role="listbox"` /
//! `role="option"` / `aria-haspopup="listbox"` -- upstream's original
//! markup, which is the APG **listbox** pattern's contract, not the
//! menu-button pattern's. `DropdownMenu` has no selection model at all (no
//! `value`/`selected` state on the root, no `aria-selected` on any item;
//! activating an item calls `on_select` and closes the menu -- action
//! semantics, not selection semantics), so the listbox roles were simply
//! wrong: assistive technology announced "list box / option N of M" and
//! implied a selection that does not exist. See `docs/backlog.md` row 24 and
//! `oracle/tier1-apg/menu-roles.spec.ts` for the oracle that caught this and
//! the APG citations backing it.
//!
//! Rather than fix `dropdown_menu.rs`'s three literals in place -- which
//! would leave the *next* menu-pattern component free to hand-write a
//! fourth, possibly-different set -- this module gives the whole pattern
//! class one shared definition. That risk was not hypothetical: `navbar.rs`
//! was exactly that fourth set, added after this module existed and still
//! hand-writing its own `role="menubar"`/`role="menu"`/`role="menuitem"`
//! literals instead of reading them from here -- found and fixed in the same
//! stage-1 pass that added this doc paragraph (docs/backlog.md rows 24, 25,
//! 41). `dropdown_menu.rs`, `context_menu.rs`, `menubar.rs`, and `navbar.rs`
//! all read their menu/menuitem/haspopup literals from here now, so a role
//! can only drift if this module itself is edited.
//!
//! # Scope
//!
//! This module governs the *pattern-class* roles only: the popup container's
//! role, an activatable item's role, and the `aria-haspopup` token a trigger
//! uses to announce that popup. It intentionally does not cover:
//! - `menubar`'s and `navbar`'s own top-level container role
//!   (`role="menubar"`) -- a distinct element of the Menu and Menubar
//!   pattern, not shared with a menu-button's popup. `Navbar`'s top-level
//!   `div` is the same element in the same pattern as `Menubar`'s (a
//!   roving-tabindex row of triggers, not a popup this module governs), not
//!   a coincidental lookalike -- confirmed by reading both components in
//!   full during the stage-1 pass that added `navbar.rs` to this module's
//!   consumers, rather than assumed from the matching literal alone.
//! - `navbar`'s items. `Navbar` is a *navigation* menu: its items are links
//!   (`NavbarItem`), never checkable, so [`MENU_ITEM_CHECKBOX_ROLE`] /
//!   [`MENU_ITEM_RADIO_ROLE`] are not offered to it.
//!
//! # Checkable items
//!
//! APG permits `menuitemcheckbox` / `menuitemradio` as item roles in a menu
//! (`content/patterns/menubar/menu-and-menubar-pattern.html`, "WAI-ARIA
//! Roles, States, and Properties": "The items contained in a menu ... have
//! any of the following roles: menuitem, menuitemcheckbox, menuitemradio",
//! and "When a menuitemcheckbox or menuitemradio is checked, aria-checked is
//! set to true"; same pinned commit as below). WAI-ARIA 1.2 makes
//! `aria-checked` *required* on both roles, and says "If a menu or menubar
//! contains more than one group of menuitemradio elements, or if the menu
//! contains one group and other, unrelated menu items, authors SHOULD
//! contain each set of related menuitemradio elements in an element using
//! the group role" (<https://www.w3.org/TR/wai-aria-1.2/#menuitemradio>).
//!
//! `DropdownMenu`, `ContextMenu` and `Menubar` each have a `*CheckboxItem`
//! and a `*RadioGroup`/`*RadioItem` (shadcn's `CheckboxItem`/`RadioGroup`/
//! `RadioItem`), all built once in [`crate::menu_item`] and routed through
//! [`MENU_ITEM_CHECKBOX_ROLE`], [`MENU_ITEM_RADIO_ROLE`] and
//! [`MENU_GROUP_ROLE`] below, so "a menu item is checkable" has exactly one
//! definition: `role` is owned by the primitive (never by the caller),
//! `aria-checked` is always present (`"true"`/`"false"`, never absent), and
//! a plain `*Item` stays `role="menuitem"` with no `aria-checked` at all.
//! `oracle/tier1-apg/menu-roles.spec.ts` grades both halves: the checkable
//! roles are legal exactly where a checkable item is rendered, and nowhere
//! else.

/// The `role` for a menu pattern's popup content container.
///
/// APG Menu and Menubar pattern, "WAI-ARIA Roles, States, and Properties":
/// "The element serving as the menu has a role of either `menu` or
/// `menubar`." (`content/patterns/menubar/menu-and-menubar-pattern.html`,
/// pinned commit `7e4034b262bc0d25332e330d8a582aaf34113829` of
/// `w3c/aria-practices` -- see `playwright/oracle/reference/README.md`).
/// This constant is the `menu` half -- every popup governed by this module
/// is a menu, never a menubar.
pub(crate) const MENU_ROLE: &str = "menu";

/// The `role` for an activatable item inside a [`MENU_ROLE`] container.
///
/// Same APG section: "The items contained in a menu ... have any of the
/// following roles: `menuitem`, `menuitemcheckbox`, `menuitemradio`." This is
/// the plain action item; the other two are [`MENU_ITEM_CHECKBOX_ROLE`] and
/// [`MENU_ITEM_RADIO_ROLE`].
pub(crate) const MENU_ITEM_ROLE: &str = "menuitem";

/// The `role` for a checkable item inside a [`MENU_ROLE`] container that
/// toggles on/off independently of its siblings.
///
/// WAI-ARIA 1.2 makes `aria-checked` a required state on this role
/// (`true`, `false` or `mixed`) -- this crate always emits `true`/`false`,
/// never omits it. A plain action item must NEVER carry this role: an
/// assistive technology announces "checkbox, not checked" for it, which is a
/// lie when activating the item runs a command.
pub(crate) const MENU_ITEM_CHECKBOX_ROLE: &str = "menuitemcheckbox";

/// The `role` for a checkable item inside a [`MENU_ROLE`] container that is
/// one of a mutually exclusive set (exactly one `aria-checked="true"` per
/// [`MENU_GROUP_ROLE`] group).
pub(crate) const MENU_ITEM_RADIO_ROLE: &str = "menuitemradio";

/// The `role` for the element that groups a set of [`MENU_ITEM_RADIO_ROLE`]
/// items, so assistive technology reads them as one mutually exclusive set.
///
/// WAI-ARIA 1.2, `menuitemradio`: menu item radios "are owned by an element
/// with role menu or menubar, or by a role group which itself is owned by an
/// element with role menu or menubar", and a group SHOULD wrap each set of
/// related radios when the menu holds more than one set or other unrelated
/// items (<https://www.w3.org/TR/wai-aria-1.2/#menuitemradio>).
pub(crate) const MENU_GROUP_ROLE: &str = "group";

/// The `aria-haspopup` token for a trigger that opens a [`MENU_ROLE`] popup.
///
/// APG Menu Button pattern, "WAI-ARIA Roles, States, and Properties": "The
/// element with role `button` has `aria-haspopup` set to either `menu` or
/// `true`." (`content/patterns/menu-button/menu-button-pattern.html`, same
/// pinned commit.) Either token satisfies the pattern; this module picks
/// `menu` as the one literal every trigger in this pattern class uses.
pub(crate) const MENU_TRIGGER_HASPOPUP: &str = "menu";
