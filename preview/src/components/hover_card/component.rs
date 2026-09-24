use dioxus::prelude::*;
use dioxus_primitives::dioxus_attributes::attributes;
use dioxus_primitives::hover_card::{
    self, HoverCardContentProps, HoverCardProps, HoverCardTriggerProps,
};
use dioxus_primitives::merge_attributes;

#[component]
pub fn HoverCard(props: HoverCardProps) -> Element {
    let base = attributes!(div { class: "dx-hover-card" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/hover_card/style.css") }
        hover_card::HoverCard {
            open: props.open,
            default_open: props.default_open,
            on_open_change: props.on_open_change,
            disabled: props.disabled,
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn HoverCardTrigger(props: HoverCardTriggerProps) -> Element {
    let base = attributes!(button { class: "dx-hover-card-trigger" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/hover_card/style.css") }
        hover_card::HoverCardTrigger { id: props.id, attributes: merged, {props.children} }
    }
}

#[component]
pub fn HoverCardContent(props: HoverCardContentProps) -> Element {
    let base = attributes!(div { class: "dx-hover-card-content" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/hover_card/style.css") }
        hover_card::HoverCardContent {
            side: props.side,
            align: props.align,
            id: props.id,
            force_mount: props.force_mount,
            attributes: merged,
            {props.children}
        }
    }
}
