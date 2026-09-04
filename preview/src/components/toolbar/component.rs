use dioxus::prelude::*;
use dioxus_primitives::toolbar::{self, ToolbarButtonProps, ToolbarProps, ToolbarSeparatorProps};
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};

#[component]
pub fn Toolbar(props: ToolbarProps) -> Element {
    let base = attributes!(div {
        class: "dx-toolbar",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/toolbar/style.css") }
        toolbar::Toolbar {
            aria_label: props.aria_label,
            disabled: props.disabled,
            horizontal: props.horizontal,
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn ToolbarButton(props: ToolbarButtonProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/toolbar/style.css") }
        toolbar::ToolbarButton {
            index: props.index,
            disabled: props.disabled,
            on_click: props.on_click,
            attributes: props.attributes,
            {props.children}
        }
    }
}

#[component]
pub fn ToolbarSeparator(props: ToolbarSeparatorProps) -> Element {
    let base = attributes!(div {
        class: "dx-toolbar-separator",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/toolbar/style.css") }
        toolbar::ToolbarSeparator {
            decorative: props.decorative,
            horizontal: props.horizontal,
            attributes: merged,
        }
    }
}

#[component]
pub fn ToolbarGroup(
    #[props(extends = GlobalAttributes)]
    #[props(extends = div)]
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div {
        class: "dx-toolbar-group",
    });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/toolbar/style.css") }
        div { ..merged, {children} }
    }
}
