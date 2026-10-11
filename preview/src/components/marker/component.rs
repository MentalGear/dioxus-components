use dioxus::prelude::*;
use dioxus_primitives::activity::use_motion_when;
use dioxus_primitives::dioxus_attributes::attributes;
use dioxus_primitives::merge_attributes;

/// The layout of a [`Marker`].
#[derive(Copy, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum MarkerVariant {
    /// An inline marker for status, notes and actions.
    #[default]
    Default,
    /// A centered label with a divider line on each side.
    Separator,
    /// The default marker with a border under the row.
    Border,
}

impl MarkerVariant {
    pub fn class(&self) -> &'static str {
        match self {
            MarkerVariant::Default => "default",
            MarkerVariant::Separator => "separator",
            MarkerVariant::Border => "border",
        }
    }
}

/// An inline conversation marker: a streaming status ("Thinking..."), a tool
/// activity line, a system note, a bordered row or a labeled separator. Plain
/// layout with no ARIA widget role, so there is no primitive underneath --
/// presentational by default; pass `role: "status"` for a streaming or
/// in-progress marker so it is announced, and never `role: "separator"` on a
/// labeled divider (its text would not be announced).
///
/// To make the whole marker a link or button, pass `as` and render the
/// element yourself (the marker's children go inside that callback, exactly as
/// with `Item`).
#[component]
pub fn Marker(
    #[props(default)] variant: MarkerVariant,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    r#as: Option<Callback<Vec<Attribute>, Element>>,
    children: Element,
) -> Element {
    let base = attributes!(div {
        class: "dx-marker",
        "data-slot": "marker",
    });
    // `data-variant` reflects this wrapper's own typed `variant` prop, not a
    // caller default -- owned-wins.
    let owned = attributes!(div { "data-variant": variant.class() });
    let merged = merge_attributes(vec![base, attributes, owned]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/marker/style.css") }
        if let Some(dynamic) = r#as {
            {dynamic.call(merged)}
        } else {
            div { ..merged, {children} }
        }
    }
}

/// A decorative icon slot. Hidden from assistive tech, so the adjacent
/// [`MarkerContent`] carries the meaning; an icon-only marker needs an
/// `aria-label` (or visible text) on the [`Marker`] itself.
#[component]
pub fn MarkerIcon(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(span {
        class: "dx-marker-icon",
        "data-slot": "marker-icon",
    });
    // Decorative by construction, not a caller default -- owned-wins.
    let owned = attributes!(span { "aria-hidden": "true" });
    let merged = merge_attributes(vec![base, attributes, owned]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/marker/style.css") }
        span { ..merged, {children} }
    }
}

/// The marker's text. Add the `dx-shimmer` class (from
/// `assets/dx-effects.css`) for the animated streaming-text effect. The sweep
/// is paused while the text is off-screen, in a skipped `content-visibility`
/// subtree or in a hidden tab (`primitives/src/activity.rs`); a marker without
/// the class observes nothing.
#[component]
pub fn MarkerContent(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let shimmer = attributes.iter().any(|a| {
        a.name == "class" && matches!(&a.value, dioxus::core::AttributeValue::Text(t) if t.contains("dx-shimmer"))
    });
    let shimmering = use_memo(use_reactive!(|shimmer| shimmer));
    let motion = use_motion_when(move || *shimmering.read());
    let base = attributes!(span {
        class: "dx-marker-content",
        "data-slot": "marker-content",
    });
    let merged = merge_attributes(vec![base, attributes, motion.attributes()]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/marker/style.css") }
        span { ..merged, {children} }
    }
}
