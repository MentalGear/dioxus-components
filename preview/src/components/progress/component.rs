use dioxus::prelude::*;
use dioxus_primitives::progress::{self, ProgressProps};

#[component]
pub fn Progress(props: ProgressProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/progress/style.css") }
        progress::Progress {
            class: "dx-progress",
            value: props.value,
            max: props.max,
            attributes: props.attributes,
            progress::ProgressIndicator { class: "dx-progress-indicator" }
        }
    }
}
