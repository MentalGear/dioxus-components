use dioxus::prelude::*;
use dioxus_primitives::dioxus_attributes::attributes;
use dioxus_primitives::merge_attributes;

/// The visual style of an [`Alert`]. shadcn/ui ships exactly these two
/// (`default`/`destructive`) -- there is no APG contract here (`role="alert"`
/// is a plain ARIA live-region role, not a widget pattern), so this mirrors
/// the canonical shape rather than inventing extra variants.
#[derive(Copy, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum AlertVariant {
    #[default]
    Default,
    Destructive,
}

impl AlertVariant {
    pub fn class(&self) -> &'static str {
        match self {
            AlertVariant::Default => "default",
            AlertVariant::Destructive => "destructive",
        }
    }
}

/// A short, callout-style message -- an inline status, warning, or tip that
/// doesn't need the interruption of a `Toast` or `Dialog`. Renders
/// `role="alert"` so assistive tech announces it as a live region; use a
/// plain `Alert` (not `role="status"`) only for content that genuinely
/// should interrupt, per the ARIA authoring practices for live regions.
#[component]
pub fn Alert(
    #[props(default)] variant: AlertVariant,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div { class: "dx-alert" });
    // `role="alert"` and `data-style` reflect this wrapper's own required
    // live-region semantics and typed `variant` prop, not a caller default --
    // owned-wins, merged after the caller's own attributes.
    let owned = attributes!(div { role: "alert", "data-style": variant.class() });
    let merged = merge_attributes(vec![base, attributes, owned]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/alert/style.css") }
        div { ..merged, {children} }
    }
}

/// The alert's heading. A `div` (not an `h*`) to match shadcn/ui -- an
/// alert is a transient callout, not a document-outline heading.
#[component]
pub fn AlertTitle(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div { class: "dx-alert-title" });
    let merged = merge_attributes(vec![base, attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/alert/style.css") }
        div { ..merged, {children} }
    }
}

/// The alert's supporting body text.
#[component]
pub fn AlertDescription(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div { class: "dx-alert-description" });
    let merged = merge_attributes(vec![base, attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/alert/style.css") }
        div { ..merged, {children} }
    }
}
