use dioxus::prelude::*;
use dioxus_icons::lucide::ChevronRight;
use dioxus_primitives::context_menu::{
    self, ContextMenuContentProps, ContextMenuItemProps, ContextMenuProps,
    ContextMenuSubContentProps, ContextMenuSubItemProps, ContextMenuSubProps,
    ContextMenuSubTriggerProps, ContextMenuTriggerProps,
};

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
            background: "var(--primary-color)",
            border: "1px dashed var(--primary-color-6)",
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
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/context_menu/style.css") }
        context_menu::ContextMenuContent {
            class: "dx-context-menu-content",
            id: props.id,
            attributes: props.attributes,
            {props.children}
        }
    }
}

#[component]
pub fn ContextMenuItem(props: ContextMenuItemProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/context_menu/style.css") }
        context_menu::ContextMenuItem {
            class: "dx-context-menu-item",
            disabled: props.disabled,
            value: props.value,
            index: props.index,
            on_select: props.on_select,
            attributes: props.attributes,
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
/// real icon element, not CSS-generated `content:` text, matters here.
#[component]
pub fn ContextMenuSubTrigger(props: ContextMenuSubTriggerProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/context_menu/style.css") }
        context_menu::ContextMenuSubTrigger {
            class: "dx-context-menu-item dx-context-menu-sub-trigger",
            index: props.index,
            id: props.id,
            disabled: props.disabled,
            attributes: props.attributes,
            {props.children}
            ChevronRight { class: "dx-context-menu-sub-trigger-icon", size: "16px" }
        }
    }
}

/// Themed wrapper for [`context_menu::ContextMenuSubContent`]. Carries both
/// `dx-context-menu-content` (the shared menu chrome) and
/// `dx-context-menu-sub-content` (the side-anchored, not click-point,
/// static fallback position) -- see `style.css`.
#[component]
pub fn ContextMenuSubContent(props: ContextMenuSubContentProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/context_menu/style.css") }
        context_menu::ContextMenuSubContent {
            class: "dx-context-menu-content dx-context-menu-sub-content",
            id: props.id,
            attributes: props.attributes,
            {props.children}
        }
    }
}

/// Themed wrapper for [`context_menu::ContextMenuSubItem`]. Reuses the
/// plain `dx-context-menu-item` chrome.
#[component]
pub fn ContextMenuSubItem(props: ContextMenuSubItemProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/context_menu/style.css") }
        context_menu::ContextMenuSubItem {
            class: "dx-context-menu-item",
            disabled: props.disabled,
            value: props.value,
            index: props.index,
            on_select: props.on_select,
            attributes: props.attributes,
            {props.children}
        }
    }
}
