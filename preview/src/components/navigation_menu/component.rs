use dioxus::prelude::*;
use dioxus_icons::lucide::ChevronDown;
use dioxus_primitives::navigation_menu::{
    self, NavigationMenuContentProps, NavigationMenuItemProps, NavigationMenuLinkProps,
    NavigationMenuListProps, NavigationMenuProps, NavigationMenuTriggerProps,
};

#[component]
pub fn NavigationMenu(props: NavigationMenuProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/navigation_menu/style.css") }
        navigation_menu::NavigationMenu {
            class: "dx-navigation-menu",
            disabled: props.disabled,
            attributes: props.attributes,
            {props.children}
        }
    }
}

#[component]
pub fn NavigationMenuList(props: NavigationMenuListProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/navigation_menu/style.css") }
        navigation_menu::NavigationMenuList {
            class: "dx-navigation-menu-list",
            attributes: props.attributes,
            {props.children}
        }
    }
}

#[component]
pub fn NavigationMenuItem(props: NavigationMenuItemProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/navigation_menu/style.css") }
        navigation_menu::NavigationMenuItem {
            class: "dx-navigation-menu-item",
            index: props.index,
            disabled: props.disabled,
            attributes: props.attributes,
            {props.children}
        }
    }
}

#[component]
pub fn NavigationMenuTrigger(props: NavigationMenuTriggerProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/navigation_menu/style.css") }
        navigation_menu::NavigationMenuTrigger {
            class: "dx-navigation-menu-trigger",
            id: props.id,
            attributes: props.attributes,
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
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/navigation_menu/style.css") }
        navigation_menu::NavigationMenuContent {
            class: "dx-navigation-menu-content",
            id: props.id,
            attributes: props.attributes,
            {props.children}
        }
    }
}

#[component]
pub fn NavigationMenuLink(props: NavigationMenuLinkProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/navigation_menu/style.css") }
        navigation_menu::NavigationMenuLink {
            class: "dx-navigation-menu-link",
            active: props.active,
            disabled: props.disabled,
            onclick: props.onclick,
            attributes: props.attributes,
            {props.children}
        }
    }
}
