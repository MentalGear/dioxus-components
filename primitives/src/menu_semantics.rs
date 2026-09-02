//! Single source of truth for the ARIA role/token triple shared by every
//! implementation of the APG "Menu Button" and "Menu and Menubar" pattern
//! class in this crate: [`dropdown_menu`](crate::dropdown_menu),
//! [`context_menu`](crate::context_menu), and [`menubar`](crate::menubar)'s
//! submenus.
//!
//! # Why this module exists
//!
//! Before this module, each of the three components above hand-wrote its
//! own `role`/`aria-haspopup` string literals. `context_menu.rs` and
//! `menubar.rs` correctly used the menu pattern's roles; `dropdown_menu.rs`
//! instead carried `role="listbox"` / `role="option"` / `aria-haspopup=
//! "listbox"` -- upstream's original markup, which is the APG **listbox**
//! pattern's contract, not the menu-button pattern's. `DropdownMenu` has no
//! selection model at all (no `value`/`selected` state on the root, no
//! `aria-selected` on any item; activating an item calls `on_select` and
//! closes the menu -- action semantics, not selection semantics), so the
//! listbox roles were simply wrong: assistive technology announced
//! "list box / option N of M" and implied a selection that does not exist.
//! See `docs/backlog.md` row 24 and `oracle/tier1-apg/menu-roles.spec.ts`
//! for the oracle that caught this and the APG citations backing it.
//!
//! Rather than fix `dropdown_menu.rs`'s three literals in place -- which
//! would leave the *next* menu-pattern component free to hand-write a
//! fourth, possibly-different set -- this module gives the whole pattern
//! class one shared definition. `dropdown_menu.rs`, `context_menu.rs`, and
//! `menubar.rs` all read their menu/menuitem/haspopup literals from here, so
//! a role can only drift if this module itself is edited.
//!
//! # Scope
//!
//! This module governs the *pattern-class* roles only: the popup container's
//! role, an activatable item's role, and the `aria-haspopup` token a trigger
//! uses to announce that popup. It intentionally does not cover:
//! - `menubar`'s own top-level container role (`role="menubar"`) -- a
//!   distinct element of the Menu and Menubar pattern, not shared with a
//!   menu-button's popup.
//! - `menuitemcheckbox` / `menuitemradio` -- APG permits these as item roles
//!   in a menu (`content/patterns/menubar/menu-and-menubar-pattern.html`,
//!   "WAI-ARIA Roles, States, and Properties": item roles are "menuitem",
//!   "menuitemcheckbox", or "menuitemradio"), but none of `DropdownMenu`,
//!   `ContextMenu`, or `Menubar` has a checkable-item variant in this crate
//!   today (verified: no `CheckboxItem`/`RadioItem`/`checked` prop on any of
//!   their item types) -- there is nothing to route through this module for
//!   those roles yet.

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
/// following roles: `menuitem`, `menuitemcheckbox`, `menuitemradio`." This
/// module only has `menuitem` to offer (see the module doc's "Scope"
/// section for why the other two are out of scope here).
pub(crate) const MENU_ITEM_ROLE: &str = "menuitem";

/// The `aria-haspopup` token for a trigger that opens a [`MENU_ROLE`] popup.
///
/// APG Menu Button pattern, "WAI-ARIA Roles, States, and Properties": "The
/// element with role `button` has `aria-haspopup` set to either `menu` or
/// `true`." (`content/patterns/menu-button/menu-button-pattern.html`, same
/// pinned commit.) Either token satisfies the pattern; this module picks
/// `menu` as the one literal every trigger in this pattern class uses.
pub(crate) const MENU_TRIGGER_HASPOPUP: &str = "menu";
