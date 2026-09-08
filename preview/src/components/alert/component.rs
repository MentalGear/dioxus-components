use dioxus::prelude::*;

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
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/alert/style.css") }
        div {
            class: "dx-alert",
            role: "alert",
            "data-style": variant.class(),
            ..attributes,
            {children}
        }
    }
}

/// The alert's heading. A `div` (not an `h*`) to match shadcn/ui -- an
/// alert is a transient callout, not a document-outline heading.
#[component]
pub fn AlertTitle(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/alert/style.css") }
        div { class: "dx-alert-title", ..attributes, {children} }
    }
}

/// The alert's supporting body text.
#[component]
pub fn AlertDescription(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/alert/style.css") }
        div { class: "dx-alert-description", ..attributes, {children} }
    }
}
