use dioxus::prelude::*;
use dioxus_icons::lucide::LoaderCircle;
use dioxus_primitives::dioxus_attributes::attributes;
use dioxus_primitives::merge_attributes;

/// A loading indicator: a continuously rotating ring icon. Renders
/// `role="status"` with an accessible name (default `"Loading"`, overridable
/// via `label`) so assistive tech announces the busy state -- there is no
/// ARIA widget pattern beyond that live-region role, so no primitive
/// underneath.
#[component]
pub fn Spinner(
    /// The accessible name announced for the loading state.
    #[props(default = "Loading".to_string())]
    label: String,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
) -> Element {
    let base = attributes!(span { class: "dx-spinner", "aria-label": "{label}" });
    // `role="status"` is required live-region semantics, not a caller
    // default -- owned-wins.
    let owned = attributes!(span { role: "status" });
    let merged = merge_attributes(vec![base, attributes, owned]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/spinner/style.css") }
        span { ..merged, LoaderCircle {} }
    }
}
