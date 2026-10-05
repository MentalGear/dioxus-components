use dioxus::prelude::*;
use dioxus_icons::lucide::{Check, ChevronRight};
use dioxus_primitives::dioxus_attributes::attributes;
use dioxus_primitives::dropdown_menu::{
    self, DropdownMenuCheckboxItemProps, DropdownMenuContentProps, DropdownMenuItemProps,
    DropdownMenuProps, DropdownMenuRadioGroupProps, DropdownMenuRadioItemProps,
    DropdownMenuSubContentProps, DropdownMenuSubItemProps, DropdownMenuSubProps,
    DropdownMenuSubTriggerProps, DropdownMenuTriggerProps,
};
use dioxus_primitives::merge_attributes;

#[component]
pub fn DropdownMenu(props: DropdownMenuProps) -> Element {
    let base = attributes!(div {
        class: "dx-dropdown-menu",
    });
    let merged = merge_attributes(vec![base, props.attributes.clone()]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/dropdown_menu/style.css") }
        dropdown_menu::DropdownMenu {
            open: props.open,
            default_open: props.default_open,
            on_open_change: props.on_open_change,
            disabled: props.disabled,
            roving_loop: props.roving_loop,
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn DropdownMenuTrigger(props: DropdownMenuTriggerProps) -> Element {
    let base = attributes!(button {
        class: "dx-dropdown-menu-trigger",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/dropdown_menu/style.css") }
        dropdown_menu::DropdownMenuTrigger { as: props.r#as, attributes: merged, {props.children} }
    }
}

#[component]
pub fn DropdownMenuContent(props: DropdownMenuContentProps) -> Element {
    let base = attributes!(div {
        class: "dx-dropdown-menu-content",
    });
    let merged = merge_attributes(vec![base, props.attributes.clone()]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/dropdown_menu/style.css") }
        dropdown_menu::DropdownMenuContent { id: props.id, attributes: merged, {props.children} }
    }
}

#[component]
pub fn DropdownMenuItem<T: Clone + PartialEq + 'static>(
    props: DropdownMenuItemProps<T>,
) -> Element {
    let base = attributes!(div {
        class: "dx-dropdown-menu-item",
    });
    let merged = merge_attributes(vec![base, props.attributes.clone()]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/dropdown_menu/style.css") }
        dropdown_menu::DropdownMenuItem {
            disabled: props.disabled,
            value: props.value,
            index: props.index,
            text_value: props.text_value,
            on_select: props.on_select,
            attributes: merged,
            {props.children}
        }
    }
}

/// Themed wrapper for [`dropdown_menu::DropdownMenuSub`] -- row 68's nested
/// submenu. Renders no element itself (neither does the primitive it
/// wraps), so unlike every other wrapper in this file there is no `class`/
/// `attributes` to attach -- the `document::Link` below still matters,
/// since this is the entry point a caller composing a submenu reaches for
/// first.
#[component]
pub fn DropdownMenuSub(props: DropdownMenuSubProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/dropdown_menu/style.css") }
        dropdown_menu::DropdownMenuSub {
            open: props.open,
            default_open: props.default_open,
            on_open_change: props.on_open_change,
            roving_loop: props.roving_loop,
            open_on_hover: props.open_on_hover,
            {props.children}
        }
    }
}

/// Themed wrapper for [`dropdown_menu::DropdownMenuSubTrigger`]. Carries
/// both `dx-dropdown-menu-item` (same hover/focus/disabled chrome every
/// plain item gets) and `dx-dropdown-menu-sub-trigger` (the open-state
/// highlight) -- see `style.css`. Appends a `ChevronRight` icon after the
/// caller's own content, the same "submenu, opens to the side" affordance
/// `select/component.rs`'s `SelectTrigger` uses `ChevronDown` for -- a real
/// icon element, not CSS-generated `content:` text, so it never risks
/// becoming part of the trigger's accessible name the way generated text
/// content can (`playwright/oracle/tier1-apg/menu-submenu.spec.ts` matches
/// this trigger by its visible "More tools" name; a decorative icon with no
/// text/title, like every other icon this crate composes this way, cannot
/// change that name).
#[component]
pub fn DropdownMenuSubTrigger(props: DropdownMenuSubTriggerProps) -> Element {
    let base = attributes!(div {
        class: "dx-dropdown-menu-item dx-dropdown-menu-sub-trigger",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/dropdown_menu/style.css") }
        dropdown_menu::DropdownMenuSubTrigger {
            index: props.index,
            id: props.id,
            disabled: props.disabled,
            text_value: props.text_value,
            attributes: merged,
            {props.children}
            ChevronRight {
                class: "dx-dropdown-menu-item-indicator dx-dropdown-menu-sub-trigger-icon",
                size: "16px",
            }
        }
    }
}

/// Themed wrapper for [`dropdown_menu::DropdownMenuSubContent`]. Carries
/// both `dx-dropdown-menu-content` (the shared menu chrome: background,
/// padding, open/close animation) and `dx-dropdown-menu-sub-content` (the
/// side-anchored, not below-anchored, static fallback position) -- see
/// `style.css`.
#[component]
pub fn DropdownMenuSubContent(props: DropdownMenuSubContentProps) -> Element {
    let base = attributes!(div {
        class: "dx-dropdown-menu-content dx-dropdown-menu-sub-content",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/dropdown_menu/style.css") }
        dropdown_menu::DropdownMenuSubContent { id: props.id, attributes: merged, {props.children} }
    }
}

/// Themed wrapper for [`dropdown_menu::DropdownMenuSubItem`]. Reuses the
/// plain `dx-dropdown-menu-item` chrome -- a sub-item looks like any other
/// item once you're inside its submenu.
#[component]
pub fn DropdownMenuSubItem<T: Clone + PartialEq + 'static>(
    props: DropdownMenuSubItemProps<T>,
) -> Element {
    let base = attributes!(div {
        class: "dx-dropdown-menu-item",
    });
    let merged = merge_attributes(vec![base, props.attributes.clone()]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/dropdown_menu/style.css") }
        dropdown_menu::DropdownMenuSubItem {
            disabled: props.disabled,
            value: props.value,
            index: props.index,
            text_value: props.text_value,
            on_select: props.on_select,
            attributes: merged,
            {props.children}
        }
    }
}

/// Themed wrapper for [`dropdown_menu::DropdownMenuCheckboxItem`] --
/// shadcn's `DropdownMenuCheckboxItem`. Carries `dx-dropdown-menu-item` (the
/// same hover/focus/disabled chrome every plain item gets) and
/// `dx-dropdown-menu-checkable-item` (the reserved indicator slot, see
/// `style.css`), and appends a `Check` icon after the caller's own content.
/// The icon carries `dx-dropdown-menu-item-indicator`, so it sits in the
/// slot the item reserved at the inline end -- the same construction as the
/// sub-trigger's chevron -- and a long label can never meet it. It is drawn
/// only while the primitive reports `data-state="checked"`; an unchecked
/// item keeps the slot, so labels never shift when an item is toggled.
#[component]
pub fn DropdownMenuCheckboxItem(props: DropdownMenuCheckboxItemProps) -> Element {
    let base = attributes!(div {
        class: "dx-dropdown-menu-item dx-dropdown-menu-checkable-item",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/dropdown_menu/style.css") }
        dropdown_menu::DropdownMenuCheckboxItem {
            index: props.index,
            checked: props.checked,
            default_checked: props.default_checked,
            on_checked_change: props.on_checked_change,
            disabled: props.disabled,
            text_value: props.text_value,
            close_on_select: props.close_on_select,
            attributes: merged,
            {props.children}
            Check { class: "dx-dropdown-menu-item-indicator", size: "16px" }
        }
    }
}

/// Themed wrapper for [`dropdown_menu::DropdownMenuRadioGroup`] -- shadcn's
/// `DropdownMenuRadioGroup`. A `role="group"` with no chrome of its own.
#[component]
pub fn DropdownMenuRadioGroup(props: DropdownMenuRadioGroupProps) -> Element {
    let base = attributes!(div {
        class: "dx-dropdown-menu-radio-group",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/dropdown_menu/style.css") }
        dropdown_menu::DropdownMenuRadioGroup {
            value: props.value,
            default_value: props.default_value,
            on_value_change: props.on_value_change,
            attributes: merged,
            {props.children}
        }
    }
}

/// Themed wrapper for [`dropdown_menu::DropdownMenuRadioItem`] -- shadcn's
/// `DropdownMenuRadioItem`. Identical chrome and indicator slot to
/// [`DropdownMenuCheckboxItem`]; the `Check` marks the chosen item (shadcn's
/// current registry draws a check for radio items too, not a dot).
#[component]
pub fn DropdownMenuRadioItem(props: DropdownMenuRadioItemProps) -> Element {
    let base = attributes!(div {
        class: "dx-dropdown-menu-item dx-dropdown-menu-checkable-item",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/dropdown_menu/style.css") }
        dropdown_menu::DropdownMenuRadioItem {
            index: props.index,
            value: props.value,
            disabled: props.disabled,
            text_value: props.text_value,
            close_on_select: props.close_on_select,
            attributes: merged,
            {props.children}
            Check { class: "dx-dropdown-menu-item-indicator", size: "16px" }
        }
    }
}

/// A non-interactive heading for a group of items -- shadcn's
/// `DropdownMenuLabel`. Pure presentation (the primitive has no label
/// component): give it an `id` and point a [`DropdownMenuRadioGroup`]'s
/// `aria-labelledby` at it so the group is named.
#[component]
pub fn DropdownMenuLabel(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div {
        class: "dx-dropdown-menu-label",
    });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/dropdown_menu/style.css") }
        div { ..merged, {children} }
    }
}
