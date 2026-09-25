use dioxus::prelude::*;
use dioxus_icons::lucide::BadgeCheck;
use dioxus_primitives::dioxus_attributes::attributes;
use dioxus_primitives::merge_attributes;

#[derive(Copy, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum BadgeVariant {
    #[default]
    Primary,
    Secondary,
    Destructive,
    Outline,
}

impl BadgeVariant {
    pub fn class(&self) -> &'static str {
        match self {
            BadgeVariant::Primary => "primary",
            BadgeVariant::Secondary => "secondary",
            BadgeVariant::Destructive => "destructive",
            BadgeVariant::Outline => "outline",
        }
    }
}

/// The props for the [`Badge`] component.
#[derive(Props, Clone, PartialEq)]
pub struct BadgeProps {
    #[props(default)]
    pub variant: BadgeVariant,

    /// Additional attributes to extend the badge element
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the badge element
    pub children: Element,
}

#[component]
pub fn Badge(props: BadgeProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/badge/style.css") }
        BadgeElement {
            "padding": true,
            variant: props.variant,
            attributes: props.attributes,
            {props.children}
        }
    }
}

#[component]
fn BadgeElement(props: BadgeProps) -> Element {
    let base = attributes!(span { class: "dx-badge" });
    // `data-style` reflects this wrapper's own typed `variant` prop, not a
    // caller default -- owned-wins.
    let owned = attributes!(span { "data-style": props.variant.class() });
    let merged = merge_attributes(vec![base, props.attributes, owned]);
    rsx! {
        span { ..merged, {props.children} }
    }
}

#[component]
pub fn VerifiedIcon() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/badge/style.css") }
        BadgeCheck {
            size: "12px",
            stroke: "var(--secondary-color-4)",
        }
    }
}
