use dioxus::prelude::*;
use dioxus_icons::lucide::{Check, ChevronRight};
use dioxus_primitives::context_menu::{
    self, ContextMenuCheckboxItemProps, ContextMenuContentProps, ContextMenuItemProps,
    ContextMenuProps, ContextMenuRadioGroupProps, ContextMenuRadioItemProps,
    ContextMenuSubContentProps, ContextMenuSubItemProps, ContextMenuSubProps,
    ContextMenuSubTriggerProps, ContextMenuTriggerProps,
};
use dioxus_primitives::dioxus_attributes::attributes;
use dioxus_primitives::merge_attributes;

#[component]
pub fn ContextMenu(props: ContextMenuProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/context_menu/style.css") }
        context_menu::ContextMenu {
            disabled: props.disabled,
            open: props.open,
            default_open: props.default_open,
            on_open_change: props.on_open_change,
            roving_loop: props.roving_loop,
            attributes: props.attributes,
            {props.children}
        }
    }
}

#[component]
pub fn ContextMenuTrigger(props: ContextMenuTriggerProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/context_menu/style.css") }
        context_menu::ContextMenuTrigger {
            padding: "20px",
            background: "var(--dx-background)",
            border: "1px dashed var(--dx-border)",
            border_radius: ".5rem",
            cursor: "context-menu",
            user_select: "none",
            text_align: "center",
            attributes: props.attributes,
            {props.children}
        }
    }
}

#[component]
pub fn ContextMenuContent(props: ContextMenuContentProps) -> Element {
    let base = attributes!(div { class: "dx-context-menu-content" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/context_menu/style.css") }
        context_menu::ContextMenuContent { id: props.id, attributes: merged, {props.children} }
    }
}

#[component]
pub fn ContextMenuItem(props: ContextMenuItemProps) -> Element {
    let base = attributes!(div { class: "dx-context-menu-item" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/context_menu/style.css") }
        context_menu::ContextMenuItem {
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

/// Themed wrapper for [`context_menu::ContextMenuSub`] -- row 68's nested
/// submenu. Renders no element itself (neither does the primitive it
/// wraps), so there is no `class`/`attributes` to attach.
#[component]
pub fn ContextMenuSub(props: ContextMenuSubProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/context_menu/style.css") }
        context_menu::ContextMenuSub {
            open: props.open,
            default_open: props.default_open,
            on_open_change: props.on_open_change,
            roving_loop: props.roving_loop,
            {props.children}
        }
    }
}

/// Themed wrapper for [`context_menu::ContextMenuSubTrigger`]. Carries both
/// `dx-context-menu-item` (the same hover/focus/disabled chrome every plain
/// item gets) and `dx-context-menu-sub-trigger` (the open-state highlight)
/// -- see `style.css`. Appends a `ChevronRight` icon after the caller's own
/// content, same as `dropdown_menu/component.rs`'s identical
/// `DropdownMenuSubTrigger` wrapper -- see that component's doc for why a
/// real icon element, not CSS-generated `content:` text, matters here. The
/// icon carries `dx-context-menu-item-indicator`, so it is positioned in the
/// item's reserved indicator slot (`style.css`) and a long label can never
/// meet it.
#[component]
pub fn ContextMenuSubTrigger(props: ContextMenuSubTriggerProps) -> Element {
    let base = attributes!(div { class: "dx-context-menu-item dx-context-menu-sub-trigger" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/context_menu/style.css") }
        context_menu::ContextMenuSubTrigger {
            index: props.index,
            id: props.id,
            disabled: props.disabled,
            text_value: props.text_value,
            attributes: merged,
            {props.children}
            ChevronRight {
                class: "dx-context-menu-item-indicator dx-context-menu-sub-trigger-icon",
                size: "16px",
            }
        }
    }
}

/// Themed wrapper for [`context_menu::ContextMenuSubContent`]. Carries both
/// `dx-context-menu-content` (the shared menu chrome) and
/// `dx-context-menu-sub-content` (the side-anchored, not click-point,
/// static fallback position) -- see `style.css`.
#[component]
pub fn ContextMenuSubContent(props: ContextMenuSubContentProps) -> Element {
    let base = attributes!(div { class: "dx-context-menu-content dx-context-menu-sub-content" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/context_menu/style.css") }
        context_menu::ContextMenuSubContent { id: props.id, attributes: merged, {props.children} }
    }
}

/// Themed wrapper for [`context_menu::ContextMenuSubItem`]. Reuses the
/// plain `dx-context-menu-item` chrome.
#[component]
pub fn ContextMenuSubItem(props: ContextMenuSubItemProps) -> Element {
    let base = attributes!(div { class: "dx-context-menu-item" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/context_menu/style.css") }
        context_menu::ContextMenuSubItem {
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

/// Themed wrapper for [`context_menu::ContextMenuCheckboxItem`] -- shadcn's
/// `ContextMenuCheckboxItem`. Carries `dx-context-menu-item` (the same
/// hover/focus/disabled chrome every plain item gets) and
/// `dx-context-menu-checkable-item` (the reserved indicator slot, see
/// `style.css`), and appends a `Check` icon after the caller's own content,
/// same as `dropdown_menu/component.rs`'s identical
/// `DropdownMenuCheckboxItem` wrapper -- see that component's doc. The icon
/// carries `dx-context-menu-item-indicator`, so it sits in the item's
/// reserved inline-end slot and a long label can never meet it; it is drawn
/// only while the primitive reports `data-state="checked"`.
#[component]
pub fn ContextMenuCheckboxItem(props: ContextMenuCheckboxItemProps) -> Element {
    let base = attributes!(div { class: "dx-context-menu-item dx-context-menu-checkable-item" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/context_menu/style.css") }
        context_menu::ContextMenuCheckboxItem {
            index: props.index,
            checked: props.checked,
            default_checked: props.default_checked,
            on_checked_change: props.on_checked_change,
            disabled: props.disabled,
            text_value: props.text_value,
            close_on_select: props.close_on_select,
            attributes: merged,
            {props.children}
            Check { class: "dx-context-menu-item-indicator", size: "16px" }
        }
    }
}

/// Themed wrapper for [`context_menu::ContextMenuRadioGroup`] -- shadcn's
/// `ContextMenuRadioGroup`. A `role="group"` with no chrome of its own.
#[component]
pub fn ContextMenuRadioGroup(props: ContextMenuRadioGroupProps) -> Element {
    let base = attributes!(div { class: "dx-context-menu-radio-group" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/context_menu/style.css") }
        context_menu::ContextMenuRadioGroup {
            value: props.value,
            default_value: props.default_value,
            on_value_change: props.on_value_change,
            attributes: merged,
            {props.children}
        }
    }
}

/// Themed wrapper for [`context_menu::ContextMenuRadioItem`] -- shadcn's
/// `ContextMenuRadioItem`. Identical chrome and indicator slot to
/// [`ContextMenuCheckboxItem`]; the `Check` marks the chosen item.
#[component]
pub fn ContextMenuRadioItem(props: ContextMenuRadioItemProps) -> Element {
    let base = attributes!(div { class: "dx-context-menu-item dx-context-menu-checkable-item" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/context_menu/style.css") }
        context_menu::ContextMenuRadioItem {
            index: props.index,
            value: props.value,
            disabled: props.disabled,
            text_value: props.text_value,
            close_on_select: props.close_on_select,
            attributes: merged,
            {props.children}
            Check { class: "dx-context-menu-item-indicator", size: "16px" }
        }
    }
}

/// A non-interactive heading for a group of items -- shadcn's
/// `ContextMenuLabel`. Pure presentation (the primitive has no label
/// component): give it an `id` and point a [`ContextMenuRadioGroup`]'s
/// `aria-labelledby` at it so the group is named.
#[component]
pub fn ContextMenuLabel(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div { class: "dx-context-menu-label" });
    let merged = merge_attributes(vec![base, attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/context_menu/style.css") }
        div { ..merged, {children} }
    }
}
