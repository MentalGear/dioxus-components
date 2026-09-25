use dioxus::prelude::*;
use dioxus_primitives::dioxus_attributes::attributes;
use dioxus_primitives::merge_attributes;
use dioxus_primitives::progress::{self, ProgressProps};

#[component]
pub fn Progress(props: ProgressProps) -> Element {
    let base = attributes!(div { class: "dx-progress" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/progress/style.css") }
        progress::Progress {
            value: props.value,
            max: props.max,
            attributes: merged,
            progress::ProgressIndicator { class: "dx-progress-indicator" }
        }
    }
}
