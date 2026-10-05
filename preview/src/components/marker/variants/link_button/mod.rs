use super::super::component::*;
use dioxus::prelude::*;
use dioxus_icons::lucide::{ChevronRight, Clock, FileText};

/// `as` turns the whole marker into a real link or button.
#[component]
pub fn Demo() -> Element {
    let mut clicks = use_signal(|| 0);
    rsx! {
        div { style: "display: flex; width: 100%; max-width: 28rem; flex-direction: column; gap: var(--dx-space-6);",
            Marker {
                r#as: move |attrs: Vec<Attribute>| rsx! {
                    a { href: "#", ..attrs,
                        MarkerIcon {
                            FileText {}
                        }
                        MarkerContent { "View the pull request" }
                    }
                },
            }
            Marker {
                r#as: move |attrs: Vec<Attribute>| rsx! {
                    button {
                        r#type: "button",
                        onclick: move |_| clicks += 1,
                        ..attrs,
                        MarkerIcon {
                            Clock {}
                        }
                        MarkerContent { style: "flex: 1", "Show earlier activity ({clicks})" }
                        MarkerIcon {
                            ChevronRight {}
                        }
                    }
                },
            }
        }
    }
}
