use dioxus::prelude::*;
use dioxus_icons::lucide::LoaderCircle;

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
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/spinner/style.css") }
        span {
            class: "dx-spinner",
            role: "status",
            "aria-label": "{label}",
            ..attributes,
            LoaderCircle {}
        }
    }
}
