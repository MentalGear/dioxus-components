use super::super::component::*;
use crate::components::spinner::Spinner;
use dioxus::prelude::*;

/// Streaming and in-progress markers carry `role: "status"` so the update is
/// announced; the spinner sits in the decorative `MarkerIcon`.
#[component]
pub fn Demo() -> Element {
    rsx! {
        div { style: "display: flex; width: 100%; max-width: 28rem; flex-direction: column; gap: var(--dx-space-6);",
            Marker { role: "status",
                MarkerIcon {
                    Spinner {}
                }
                MarkerContent { "Compacting conversation" }
            }
            Marker { role: "status",
                MarkerIcon {
                    Spinner {}
                }
                MarkerContent { class: "dx-shimmer", "Thinking..." }
            }
            Marker { role: "status",
                MarkerContent { class: "dx-shimmer dx-shimmer-reverse", "Reading 12 files" }
            }
        }
    }
}
