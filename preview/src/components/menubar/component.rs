use dioxus::prelude::*;
use dioxus_primitives::dioxus_attributes::attributes;
use dioxus_primitives::menubar::{
    self, MenubarContentProps, MenubarItemProps, MenubarMenuProps, MenubarProps,
    MenubarTriggerProps,
};
use dioxus_primitives::merge_attributes;

#[component]
pub fn Menubar(props: MenubarProps) -> Element {
    let base = attributes!(div { class: "dx-menubar" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/menubar/style.css") }
        menubar::Menubar {
            disabled: props.disabled,
            roving_loop: props.roving_loop,
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn MenubarMenu(props: MenubarMenuProps) -> Element {
    let base = attributes!(div { class: "dx-menubar-menu" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/menubar/style.css") }
        menubar::MenubarMenu {
            index: props.index,
            disabled: props.disabled,
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn MenubarTrigger(props: MenubarTriggerProps) -> Element {
    let base = attributes!(button { class: "dx-menubar-trigger" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/menubar/style.css") }
        menubar::MenubarTrigger { attributes: merged, {props.children} }
    }
}

#[component]
pub fn MenubarContent(props: MenubarContentProps) -> Element {
    let base = attributes!(div { class: "dx-menubar-content" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/menubar/style.css") }
        menubar::MenubarContent { id: props.id, attributes: merged, {props.children} }
    }
}

#[component]
pub fn MenubarItem(props: MenubarItemProps) -> Element {
    let base = attributes!(div { class: "dx-menubar-item" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/menubar/style.css") }
        menubar::MenubarItem {
            index: props.index,
            value: props.value,
            disabled: props.disabled,
            text_value: props.text_value,
            on_select: props.on_select,
            attributes: merged,
            {props.children}
        }
    }
}
