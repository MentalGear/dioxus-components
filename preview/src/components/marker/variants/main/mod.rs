use super::super::component::*;
use crate::components::spinner::Spinner;
use dioxus::prelude::*;
use dioxus_icons::lucide::{ChevronRight, CircleUserRound, Clock, FileText, GitBranch};

#[component]
pub fn Demo() -> Element {
    rsx! {
        div { style: "display: flex; width: 100%; max-width: 28rem; flex-direction: column; gap: var(--dx-space-6);",
            Marker {
                MarkerContent { "A default marker" }
            }
            Marker {
                MarkerIcon {
                    FileText {}
                }
                MarkerContent { "Marker with icon" }
            }
            Marker { role: "status",
                MarkerIcon {
                    Spinner {}
                }
                MarkerContent { "Marker with a spinner" }
            }
            Marker { role: "status",
                MarkerContent { class: "dx-shimmer", "Thinking..." }
            }
            Marker {
                r#as: move |attrs: Vec<Attribute>| rsx! {
                    a { href: "#", ..attrs,
                        MarkerIcon {
                            GitBranch {}
                        }
                        MarkerContent { "Marker as a link" }
                    }
                },
            }
            Marker {
                r#as: move |attrs: Vec<Attribute>| rsx! {
                    button { r#type: "button", ..attrs,
                        MarkerIcon {
                            Clock {}
                        }
                        MarkerContent { style: "flex: 1", "Marker as a button" }
                        MarkerIcon {
                            ChevronRight {}
                        }
                    }
                },
            }
            Marker {
                MarkerIcon {
                    CircleUserRound {}
                }
                MarkerContent { "Rhea joined the chat" }
            }
            Marker { style: "justify-content: center",
                MarkerContent { "Olivia Rose left the chat" }
            }
        }
    }
}
