//! Checkable menu items -- `menuitemcheckbox` and `menuitemradio` -- built
//! once for the whole menu family: [`crate::dropdown_menu`],
//! [`crate::context_menu`] and [`crate::menubar`].
//!
//! shadcn's `DropdownMenu`, `ContextMenu` and `Menubar` all have a
//! `CheckboxItem` and a `RadioGroup` + `RadioItem`. Before this module none
//! of the three had either, and writing them three times would have been the
//! same trap `crate::menu_semantics` and `crate::menu_root` exist to close:
//! three hosts hand-writing one contract, free to drift apart. So everything
//! that is *the same* for every host lives here, and each host adds only
//! what genuinely differs (which roving-focus collection the item registers
//! in, how a pointer press is committed, what "close the menu" means):
//!
//! - **Roles and ARIA** -- [`CheckableState::owned_attributes`]: `role`
//!   (`menuitemcheckbox`/`menuitemradio`, from [`crate::menu_semantics`]),
//!   `aria-checked` (always present, `"true"` or `"false"`), `data-state`
//!   (`checked`/`unchecked`), `aria-disabled`, `data-disabled`, `tabindex`.
//!   Owned by the primitive, so they win over a caller's own attributes.
//! - **State** -- [`use_checkbox_state`] (controlled or uncontrolled
//!   `checked`) and [`use_radio_group`] + [`use_radio_state`] (controlled or
//!   uncontrolled group `value`; an item is checked when its `value` equals
//!   the group's).
//! - **Props** -- [`MenuCheckboxItemProps`], [`MenuRadioItemProps`] and
//!   [`MenuRadioGroupProps`], re-exported by each host under its own name
//!   (`DropdownMenuCheckboxItemProps`, ...), so the three APIs cannot drift.
//! - **The radio group** -- [`render_radio_group`], a `role="group"`
//!   element, identical for every host.
//! - **Activation** -- [`CheckableState::activate`] flips a checkbox or
//!   chooses a radio's value; [`is_activation_key`] is the one definition of
//!   "Enter or Space".
//!
//! # Close semantics
//!
//! Radix's `onSelect` default, which shadcn inherits: selecting a checkable
//! item **closes the menu**. Every checkable item has a
//! `close_on_select` prop (default `true`) to opt out and keep the menu open
//! after a toggle -- the behaviour APG describes as optional for Space ("When
//! focus is on a menuitemcheckbox, changes the state without closing the
//! menu", `content/patterns/menubar/menu-and-menubar-pattern.html`, "Keyboard
//! Interaction", pinned commit `7e4034b262bc0d25332e330d8a582aaf34113829`),
//! and the default of Base UI (the shadcn Nova registry's primitive).
//!
//! # Controlled or uncontrolled
//!
//! Both are supported, as everywhere in this crate -- but note what
//! "uncontrolled" means in a menu: the state lives in the item, and a menu's
//! content unmounts when it closes, so an uncontrolled `default_checked` /
//! `default_value` item **starts over every time the menu opens** (the same
//! as Radix). Hold the state in the caller and pass `checked` / `value` for
//! anything that must survive a close, which is what shadcn's own demos do.
//!
//! # Typeahead
//!
//! A checkable item registers in the same roving-focus collection as a plain
//! item, with the same `text_value`, so it is a typeahead target
//! (`crate::typeahead`) and is skipped by arrow keys, Home/End and typeahead
//! when disabled, exactly like a plain item. A radio item falls back to its
//! `value` as its label; a checkbox item has no `value`, so it is
//! searchable only when `text_value` is set.

use dioxus::prelude::*;
use dioxus_attributes::attributes;

use crate::{merge_attributes, use_controlled};

/// The props for a checkbox item in a menu: [`DropdownMenuCheckboxItem`]
/// (`crate::dropdown_menu`), [`ContextMenuCheckboxItem`]
/// (`crate::context_menu`) and [`MenubarCheckboxItem`] (`crate::menubar`)
/// all take exactly these.
///
/// [`DropdownMenuCheckboxItem`]: crate::dropdown_menu::DropdownMenuCheckboxItem
/// [`ContextMenuCheckboxItem`]: crate::context_menu::ContextMenuCheckboxItem
/// [`MenubarCheckboxItem`]: crate::menubar::MenubarCheckboxItem
#[derive(Props, Clone, PartialEq)]
pub struct MenuCheckboxItemProps {
    /// The index of the item within the enclosing menu content (or submenu
    /// content). This is used to order the items for keyboard navigation,
    /// the same as a plain item's `index`.
    pub index: ReadSignal<usize>,

    /// Whether the item is checked. If not provided, the item is
    /// uncontrolled and uses [`Self::default_checked`].
    pub checked: ReadSignal<Option<bool>>,

    /// The initial checked state when the item is uncontrolled.
    #[props(default)]
    pub default_checked: bool,

    /// Callback fired with the new checked state when the item is toggled.
    #[props(default)]
    pub on_checked_change: Callback<bool>,

    /// Whether the item is disabled. A disabled item cannot be toggled and
    /// is skipped by roving focus and typeahead.
    #[props(default)]
    pub disabled: ReadSignal<bool>,

    /// Explicit label text used for typeahead search. A checkbox item has no
    /// `value` to fall back to, so it is only searchable when this is set.
    #[props(default)]
    pub text_value: ReadSignal<Option<String>>,

    /// Whether toggling the item closes the menu. Defaults to `true`, Radix's
    /// `onSelect` default; set `false` to keep the menu open so several
    /// items can be toggled in one visit.
    #[props(default = ReadSignal::new(Signal::new(true)))]
    pub close_on_select: ReadSignal<bool>,

    /// Additional attributes to apply to the item element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the item: its label, and any icon.
    pub children: Element,
}

/// The props for a radio item in a menu: [`DropdownMenuRadioItem`]
/// (`crate::dropdown_menu`), [`ContextMenuRadioItem`]
/// (`crate::context_menu`) and [`MenubarRadioItem`] (`crate::menubar`) all
/// take exactly these. It must be inside the matching `*RadioGroup`.
///
/// [`DropdownMenuRadioItem`]: crate::dropdown_menu::DropdownMenuRadioItem
/// [`ContextMenuRadioItem`]: crate::context_menu::ContextMenuRadioItem
/// [`MenubarRadioItem`]: crate::menubar::MenubarRadioItem
#[derive(Props, Clone, PartialEq)]
pub struct MenuRadioItemProps {
    /// The index of the item within the enclosing menu content (or submenu
    /// content). This is used to order the items for keyboard navigation,
    /// the same as a plain item's `index`.
    pub index: ReadSignal<usize>,

    /// The value of the item. The item is checked while the group's value
    /// equals this.
    pub value: ReadSignal<String>,

    /// Whether the item is disabled. A disabled item cannot be chosen and is
    /// skipped by roving focus and typeahead.
    #[props(default)]
    pub disabled: ReadSignal<bool>,

    /// Explicit label text used for typeahead search. Falls back to
    /// [`Self::value`].
    #[props(default)]
    pub text_value: ReadSignal<Option<String>>,

    /// Whether choosing the item closes the menu. Defaults to `true`,
    /// Radix's `onSelect` default; set `false` to keep the menu open.
    #[props(default = ReadSignal::new(Signal::new(true)))]
    pub close_on_select: ReadSignal<bool>,

    /// Additional attributes to apply to the item element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the item: its label, and any icon.
    pub children: Element,
}

/// The props for a radio group in a menu: [`DropdownMenuRadioGroup`]
/// (`crate::dropdown_menu`), [`ContextMenuRadioGroup`]
/// (`crate::context_menu`) and [`MenubarRadioGroup`] (`crate::menubar`) all
/// take exactly these.
///
/// [`DropdownMenuRadioGroup`]: crate::dropdown_menu::DropdownMenuRadioGroup
/// [`ContextMenuRadioGroup`]: crate::context_menu::ContextMenuRadioGroup
/// [`MenubarRadioGroup`]: crate::menubar::MenubarRadioGroup
#[derive(Props, Clone, PartialEq)]
pub struct MenuRadioGroupProps {
    /// The value of the checked radio item. If not provided, the group is
    /// uncontrolled and uses [`Self::default_value`].
    pub value: ReadSignal<Option<String>>,

    /// The initial value when the group is uncontrolled.
    #[props(default)]
    pub default_value: String,

    /// Callback fired with the item's value when a radio item is chosen.
    #[props(default)]
    pub on_value_change: Callback<String>,

    /// Additional attributes to apply to the group element. Give the group
    /// an accessible name with `aria-label` or `aria-labelledby`.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The radio items of the group.
    pub children: Element,
}

/// Which of the two checkable roles an item has.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum CheckableKind {
    Checkbox,
    Radio,
}

impl CheckableKind {
    fn role(self) -> &'static str {
        match self {
            Self::Checkbox => crate::menu_semantics::MENU_ITEM_CHECKBOX_ROLE,
            Self::Radio => crate::menu_semantics::MENU_ITEM_RADIO_ROLE,
        }
    }
}

/// What a checkable item reports (is it checked) and does (flip, or choose
/// its value) -- the half of a checkable item that is independent of the
/// host menu. `Copy`, so each host's handlers capture it freely.
#[derive(Clone, Copy)]
pub(crate) struct CheckableState {
    kind: CheckableKind,
    checked: Memo<bool>,
    activate: Callback<()>,
    /// A radio item's `value`, the label typeahead falls back to.
    text_fallback: Option<ReadSignal<String>>,
}

impl CheckableState {
    pub(crate) fn checked(&self) -> bool {
        (self.checked)()
    }

    /// Toggle a checkbox / choose a radio's value. Does not close the menu;
    /// that is the host's call, via the item's `close_on_select`.
    pub(crate) fn activate(&self) {
        self.activate.call(());
    }

    /// The item's typeahead label: the explicit `text_value`, else (radio)
    /// its `value`.
    pub(crate) fn text_value(&self, explicit: ReadSignal<Option<String>>) -> Option<String> {
        explicit().or_else(|| self.text_fallback.map(|value| value()))
    }

    /// The attributes every host's checkable item owns, to be merged *after*
    /// the caller's own so they win (`docs/backlog.md` row 93's
    /// duplicate-attribute hazard).
    ///
    /// `aria-checked` is always present: WAI-ARIA 1.2 makes it a required
    /// state on both roles, and an absent value reads as "not a checkbox".
    pub(crate) fn owned_attributes(
        &self,
        disabled: bool,
        tabindex: &'static str,
    ) -> Vec<Attribute> {
        let checked = self.checked();
        attributes!(div {
            role: self.kind.role(),
            aria_checked: if checked { "true" } else { "false" },
            "data-state": if checked { "checked" } else { "unchecked" },
            aria_disabled: disabled,
            "data-disabled": disabled,
            tabindex: tabindex,
        })
    }
}

/// The inputs every host's checkable item shares, whichever role it has --
/// what is left of a [`MenuCheckboxItemProps`]/[`MenuRadioItemProps`] once
/// the state half has been turned into a [`CheckableState`].
pub(crate) struct CheckableCommon {
    pub(crate) index: ReadSignal<usize>,
    pub(crate) disabled: ReadSignal<bool>,
    pub(crate) text_value: ReadSignal<Option<String>>,
    pub(crate) close_on_select: ReadSignal<bool>,
    pub(crate) attributes: Vec<Attribute>,
    pub(crate) children: Element,
}

impl From<MenuCheckboxItemProps> for CheckableCommon {
    fn from(props: MenuCheckboxItemProps) -> Self {
        Self {
            index: props.index,
            disabled: props.disabled,
            text_value: props.text_value,
            close_on_select: props.close_on_select,
            attributes: props.attributes,
            children: props.children,
        }
    }
}

impl From<MenuRadioItemProps> for CheckableCommon {
    fn from(props: MenuRadioItemProps) -> Self {
        Self {
            index: props.index,
            disabled: props.disabled,
            text_value: props.text_value,
            close_on_select: props.close_on_select,
            attributes: props.attributes,
            children: props.children,
        }
    }
}

/// The state of a checkbox item: controlled when `checked` is `Some`, else
/// internal, seeded from `default_checked`. Activating flips it and fires
/// `on_checked_change`. A hook -- call it unconditionally, once per render.
pub(crate) fn use_checkbox_state(
    checked: ReadSignal<Option<bool>>,
    default_checked: bool,
    on_checked_change: Callback<bool>,
) -> CheckableState {
    let (value, set_value) = use_controlled(checked, default_checked, on_checked_change);
    let activate = use_callback(move |()| set_value.call(!value()));
    CheckableState {
        kind: CheckableKind::Checkbox,
        checked: value,
        activate,
        text_fallback: None,
    }
}

#[derive(Clone, Copy)]
struct RadioGroupContext {
    value: Memo<String>,
    set_value: Callback<String>,
}

/// Provide a radio group's value to its [`use_radio_state`] items:
/// controlled when `value` is `Some`, else internal, seeded from
/// `default_value`. A hook -- call it unconditionally, once per render.
pub(crate) fn use_radio_group(
    value: ReadSignal<Option<String>>,
    default_value: String,
    on_value_change: Callback<String>,
) {
    let (value, set_value) = use_controlled(value, default_value, on_value_change);
    use_context_provider(|| RadioGroupContext { value, set_value });
}

/// The state of a radio item: checked while the enclosing group's value
/// equals this item's `value`; activating sets the group's value to it.
/// A hook -- call it unconditionally, once per render.
///
/// # Panics
///
/// Panics if there is no enclosing radio group, as every other context-bound
/// item in this crate does outside its parent.
pub(crate) fn use_radio_state(value: ReadSignal<String>) -> CheckableState {
    let group: RadioGroupContext = use_context();
    let checked = use_memo(move || (group.value)() == value());
    let activate = use_callback(move |()| group.set_value.call(value()));
    CheckableState {
        kind: CheckableKind::Radio,
        checked,
        activate,
        text_fallback: Some(value),
    }
}

/// The radio group element, shared by every host's `*RadioGroup`: a
/// `role="group"` (`crate::menu_semantics::MENU_GROUP_ROLE`) whose radio
/// items read the group's value. Call it from a host's component body; it
/// calls hooks.
pub(crate) fn render_radio_group(props: MenuRadioGroupProps) -> Element {
    use_radio_group(props.value, props.default_value, props.on_value_change);

    // Owned by this component -- see `CheckableState::owned_attributes`.
    let attributes = merge_attributes(vec![
        props.attributes,
        attributes!(div {
            role: crate::menu_semantics::MENU_GROUP_ROLE,
        }),
    ]);

    rsx! {
        div { ..attributes, {props.children} }
    }
}

/// Whether `key` activates a menu item: Enter or Space. The one definition
/// every checkable item's `onkeydown` reads.
pub(crate) fn is_activation_key(key: &Key) -> bool {
    matches!(key, Key::Enter) || matches!(key, Key::Character(c) if c == " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context_menu::{
        ContextMenu, ContextMenuCheckboxItem, ContextMenuItem, ContextMenuRadioGroup,
        ContextMenuRadioItem, ContextMenuTrigger,
    };
    use crate::dropdown_menu::{
        DropdownMenu, DropdownMenuCheckboxItem, DropdownMenuItem, DropdownMenuRadioGroup,
        DropdownMenuRadioItem, DropdownMenuTrigger,
    };
    use crate::menubar::{
        Menubar, MenubarCheckboxItem, MenubarItem, MenubarMenu, MenubarRadioGroup,
        MenubarRadioItem, MenubarTrigger,
    };

    fn render(component: fn() -> Element) -> String {
        let mut dom = VirtualDom::new(component);
        dom.rebuild_in_place();
        dioxus_ssr::render(&dom)
    }

    /// One menu per host: a plain item, a checked and an unchecked checkbox
    /// item, and a radio group of two with `b` chosen. The items sit directly
    /// under the host root rather than inside its `*Content`, which mounts
    /// only after an effect has flipped it open -- the items themselves only
    /// need the root's context, and that is what is under test.
    #[component]
    fn DropdownMenuWithCheckables() -> Element {
        rsx! {
            DropdownMenu {
                DropdownMenuTrigger { "t" }
                DropdownMenuItem::<String> { value: "plain".to_string(), index: 0usize, "Plain" }
                DropdownMenuCheckboxItem { index: 1usize, default_checked: true, "On" }
                DropdownMenuCheckboxItem { index: 2usize, "Off" }
                DropdownMenuRadioGroup { default_value: "b".to_string(), aria_label: "Pick",
                    DropdownMenuRadioItem { index: 3usize, value: "a".to_string(), "A" }
                    DropdownMenuRadioItem { index: 4usize, value: "b".to_string(), "B" }
                }
            }
        }
    }

    #[component]
    fn ContextMenuWithCheckables() -> Element {
        rsx! {
            ContextMenu {
                ContextMenuTrigger { "t" }
                ContextMenuItem { value: "plain".to_string(), index: 0usize, "Plain" }
                ContextMenuCheckboxItem { index: 1usize, default_checked: true, "On" }
                ContextMenuCheckboxItem { index: 2usize, "Off" }
                ContextMenuRadioGroup { default_value: "b".to_string(), aria_label: "Pick",
                    ContextMenuRadioItem { index: 3usize, value: "a".to_string(), "A" }
                    ContextMenuRadioItem { index: 4usize, value: "b".to_string(), "B" }
                }
            }
        }
    }

    #[component]
    fn MenubarWithCheckables() -> Element {
        rsx! {
            Menubar {
                MenubarMenu { index: 0usize,
                    MenubarTrigger { "t" }
                    MenubarItem { value: "plain".to_string(), index: 0usize, "Plain" }
                    MenubarCheckboxItem { index: 1usize, default_checked: true, "On" }
                    MenubarCheckboxItem { index: 2usize, "Off" }
                    MenubarRadioGroup { default_value: "b".to_string(), aria_label: "Pick",
                        MenubarRadioItem { index: 3usize, value: "a".to_string(), "A" }
                        MenubarRadioItem { index: 4usize, value: "b".to_string(), "B" }
                    }
                }
            }
        }
    }

    /// The contract every host must render for the menu above -- asserted
    /// against all of them with this one function, so a host that drifts
    /// (a hand-written role, a missing `aria-checked`) fails here.
    ///
    /// `plain_menuitems` is how many `role="menuitem"` the menu legitimately
    /// has: the one plain item, plus the trigger where the host's trigger is
    /// itself a menuitem (`Menubar`'s is).
    fn assert_checkable_contract(host: &str, html: &str, plain_menuitems: usize) {
        let count = |needle: &str| html.matches(needle).count();
        assert_eq!(count(r#"role="menuitemcheckbox""#), 2, "{host}: {html}");
        assert_eq!(count(r#"role="menuitemradio""#), 2, "{host}: {html}");
        assert_eq!(count(r#"role="group""#), 1, "{host}: {html}");
        // The plain item stays a plain menuitem.
        assert_eq!(
            count(r#"role="menuitem""#),
            plain_menuitems,
            "{host}: {html}"
        );
        // `aria-checked` is on every checkable item and only on those:
        // On + radio `b` are true, Off + radio `a` are false.
        assert_eq!(count(r#"aria-checked="true""#), 2, "{host}: {html}");
        assert_eq!(count(r#"aria-checked="false""#), 2, "{host}: {html}");
        assert_eq!(count("aria-checked="), 4, "{host}: {html}");
        assert_eq!(count(r#"data-state="checked""#), 2, "{host}: {html}");
        assert_eq!(count(r#"data-state="unchecked""#), 2, "{host}: {html}");
    }

    #[test]
    fn dropdown_menu_renders_the_checkable_contract() {
        assert_checkable_contract("DropdownMenu", &render(DropdownMenuWithCheckables), 1);
    }

    #[test]
    fn context_menu_renders_the_checkable_contract() {
        assert_checkable_contract("ContextMenu", &render(ContextMenuWithCheckables), 1);
    }

    #[test]
    fn menubar_renders_the_checkable_contract() {
        assert_checkable_contract("Menubar", &render(MenubarWithCheckables), 2);
    }

    /// A caller cannot override the primitive-owned role or `aria-checked`:
    /// both are merged after the caller's attributes.
    #[component]
    fn CheckboxTryingToOverrideItsRole() -> Element {
        rsx! {
            DropdownMenu {
                DropdownMenuCheckboxItem {
                    index: 0usize,
                    role: "menuitem",
                    aria_checked: "mixed",
                    "x"
                }
            }
        }
    }

    #[test]
    fn the_role_and_aria_checked_are_owned_by_the_primitive() {
        let html = render(CheckboxTryingToOverrideItsRole);
        assert_eq!(html.matches("role=").count(), 1, "{html}");
        assert!(html.contains(r#"role="menuitemcheckbox""#), "{html}");
        assert_eq!(html.matches("aria-checked=").count(), 1, "{html}");
        assert!(html.contains(r#"aria-checked="false""#), "{html}");
    }

    #[test]
    fn enter_and_space_activate_and_nothing_else_does() {
        assert!(is_activation_key(&Key::Enter));
        assert!(is_activation_key(&Key::Character(" ".to_string())));
        assert!(!is_activation_key(&Key::Character("a".to_string())));
        assert!(!is_activation_key(&Key::Escape));
        assert!(!is_activation_key(&Key::ArrowDown));
    }

    #[test]
    fn the_two_checkable_kinds_have_distinct_roles() {
        assert_eq!(CheckableKind::Checkbox.role(), "menuitemcheckbox");
        assert_eq!(CheckableKind::Radio.role(), "menuitemradio");
    }
}
