use dioxus::prelude::*;
use dioxus_icons::lucide::ChevronDown;
use dioxus_primitives::dioxus_attributes::attributes;
use dioxus_primitives::merge_attributes;
use dioxus_primitives::navigation_menu::{
    self, NavigationMenuContentProps, NavigationMenuItemProps, NavigationMenuLinkProps,
    NavigationMenuListProps, NavigationMenuProps, NavigationMenuTriggerProps,
};

#[component]
pub fn NavigationMenu(props: NavigationMenuProps) -> Element {
    let base = attributes!(nav { class: "dx-navigation-menu" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/navigation_menu/style.css") }
        navigation_menu::NavigationMenu {
            disabled: props.disabled,
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn NavigationMenuList(props: NavigationMenuListProps) -> Element {
    let base = attributes!(ul { class: "dx-navigation-menu-list" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/navigation_menu/style.css") }
        navigation_menu::NavigationMenuList { attributes: merged, {props.children} }
    }
}

#[component]
pub fn NavigationMenuItem(props: NavigationMenuItemProps) -> Element {
    let base = attributes!(li { class: "dx-navigation-menu-item" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/navigation_menu/style.css") }
        navigation_menu::NavigationMenuItem {
            index: props.index,
            disabled: props.disabled,
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn NavigationMenuTrigger(props: NavigationMenuTriggerProps) -> Element {
    let base = attributes!(button { class: "dx-navigation-menu-trigger" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/navigation_menu/style.css") }
        navigation_menu::NavigationMenuTrigger {
            id: props.id,
            attributes: merged,
            {props.children}
            ChevronDown {
                class: "dx-navigation-menu-expand-icon",
                size: "16px",
                stroke: "currentColor",
            }
        }
    }
}

#[component]
pub fn NavigationMenuContent(props: NavigationMenuContentProps) -> Element {
    let base = attributes!(div { class: "dx-navigation-menu-content" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/navigation_menu/style.css") }
        navigation_menu::NavigationMenuContent { id: props.id, attributes: merged, {props.children} }
    }
}

#[component]
pub fn NavigationMenuLink(props: NavigationMenuLinkProps) -> Element {
    let base = attributes!(a { class: "dx-navigation-menu-link" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/navigation_menu/style.css") }
        navigation_menu::NavigationMenuLink {
            active: props.active,
            disabled: props.disabled,
            content_index: props.content_index,
            onclick: props.onclick,
            attributes: merged,
            {props.children}
        }
    }
}
