use super::super::component::*;
use dioxus::prelude::*;
use dioxus_icons::lucide::FileText;

#[component]
pub fn Demo() -> Element {
    rsx! {
        div { style: "display: flex; width: 100%; max-width: 28rem; flex-direction: column; gap: var(--dx-space-6);",
            Marker {
                MarkerIcon {
                    FileText {}
                }
                MarkerContent { "Default: an inline marker" }
            }
            Marker { variant: MarkerVariant::Border,
                MarkerIcon {
                    FileText {}
                }
                MarkerContent { "Border: opened implementation notes" }
            }
            Marker { variant: MarkerVariant::Separator,
                MarkerContent { "Separator: Today" }
            }
        }
    }
}
