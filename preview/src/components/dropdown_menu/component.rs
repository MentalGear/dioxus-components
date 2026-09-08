use dioxus::prelude::*;
use dioxus_icons::lucide::ChevronRight;
use dioxus_primitives::dioxus_attributes::attributes;
use dioxus_primitives::dropdown_menu::{
    self, DropdownMenuContentProps, DropdownMenuItemProps, DropdownMenuProps,
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
            attributes: merged,
            {props.children}
            ChevronRight { class: "dx-dropdown-menu-sub-trigger-icon", size: "16px" }
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
            on_select: props.on_select,
            attributes: merged,
            {props.children}
        }
    }
}
