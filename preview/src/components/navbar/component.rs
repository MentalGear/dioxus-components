use dioxus::prelude::*;
use dioxus_icons::lucide::ChevronDown;
use dioxus_primitives::navbar::{
    self, NavbarContentProps, NavbarItemProps, NavbarNavProps, NavbarProps, NavbarTriggerProps,
};

#[component]
pub fn Navbar(props: NavbarProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/navbar/style.css") }
        navbar::Navbar {
            class: "dx-navbar",
            disabled: props.disabled,
            roving_loop: props.roving_loop,
            attributes: props.attributes,
            {props.children}
        }
    }
}

#[component]
pub fn NavbarNav(props: NavbarNavProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/navbar/style.css") }
        navbar::NavbarNav {
            class: "dx-navbar-nav",
            index: props.index,
            disabled: props.disabled,
            attributes: props.attributes,
            {props.children}
        }
    }
}

#[component]
pub fn NavbarTrigger(props: NavbarTriggerProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/navbar/style.css") }
        navbar::NavbarTrigger { class: "dx-navbar-trigger", attributes: props.attributes,
            {props.children}
            ChevronDown {
                class: "dx-navbar-expand-icon",
                size: "20px",
                stroke: "var(--secondary-color-4)",
            }
        }
    }
}

#[component]
pub fn NavbarContent(props: NavbarContentProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/navbar/style.css") }
        navbar::NavbarContent {
            class: "dx-navbar-content",
            id: props.id,
            attributes: props.attributes,
            {props.children}
        }
    }
}

#[component]
pub fn NavbarItem(props: NavbarItemProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/navbar/style.css") }
        navbar::NavbarItem {
            class: "dx-navbar-item".to_string(),
            index: props.index,
            value: props.value,
            disabled: props.disabled,
            new_tab: props.new_tab,
            to: props.to,
            active_class: props.active_class,
            attributes: props.attributes,
            on_select: props.on_select,
            onclick: props.onclick,
            onmounted: props.onmounted,
            {props.children}
        }
    }
}
