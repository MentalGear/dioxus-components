use dioxus::prelude::*;
use dioxus_icons::lucide::ChevronDown;
use dioxus_primitives::dioxus_attributes::attributes;
use dioxus_primitives::merge_attributes;
use dioxus_primitives::navbar::{
    self, NavbarContentProps, NavbarItemProps, NavbarNavProps, NavbarProps, NavbarTriggerProps,
};

#[component]
pub fn Navbar(props: NavbarProps) -> Element {
    let base = attributes!(nav { class: "dx-navbar" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/navbar/style.css") }
        navbar::Navbar {
            disabled: props.disabled,
            roving_loop: props.roving_loop,
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn NavbarNav(props: NavbarNavProps) -> Element {
    let base = attributes!(div { class: "dx-navbar-nav" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/navbar/style.css") }
        navbar::NavbarNav {
            index: props.index,
            disabled: props.disabled,
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn NavbarTrigger(props: NavbarTriggerProps) -> Element {
    let base = attributes!(button { class: "dx-navbar-trigger" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/navbar/style.css") }
        navbar::NavbarTrigger { attributes: merged,
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
    let base = attributes!(div { class: "dx-navbar-content" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/navbar/style.css") }
        navbar::NavbarContent { id: props.id, attributes: merged, {props.children} }
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
