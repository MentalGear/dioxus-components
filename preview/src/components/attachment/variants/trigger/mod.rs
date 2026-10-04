use super::super::component::*;
use dioxus::prelude::*;
use dioxus_icons::lucide::{FileText, X};

/// The trigger fills the card behind the actions, so activating the card and
/// removing it are separately clickable and focusable.
#[component]
pub fn Demo() -> Element {
    let mut opened = use_signal(|| "Nothing opened yet");
    let mut removed = use_signal(|| false);
    rsx! {
        div { style: "display: flex; width: 100%; max-width: 28rem; flex-direction: column; align-items: flex-start; gap: var(--dx-space-3);",
            if !removed() {
                Attachment {
                    AttachmentMedia {
                        FileText {}
                    }
                    AttachmentContent {
                        AttachmentTitle { "research-summary.pdf" }
                        AttachmentDescription { "PDF · 3.1 MB" }
                    }
                    AttachmentActions {
                        AttachmentAction {
                            aria_label: "Remove research-summary.pdf",
                            onclick: move |_| removed.set(true),
                            X {}
                        }
                    }
                    AttachmentTrigger {
                        aria_label: "Preview research-summary.pdf",
                        onclick: move |_| opened.set("Opened research-summary.pdf"),
                    }
                }
            }
            Attachment {
                AttachmentMedia {
                    FileText {}
                }
                AttachmentContent {
                    AttachmentTitle { "workspace-notes.md" }
                    AttachmentDescription { "Markdown · 12 KB" }
                }
                AttachmentTrigger {
                    r#as: move |attrs: Vec<Attribute>| rsx! {
                        a {
                            href: "#",
                            "aria-label": "Open workspace-notes.md",
                            ..attrs,
                        }
                    },
                }
            }
            p { role: "status", style: "margin: 0; font-size: var(--dx-text-sm); color: var(--dx-muted-foreground)",
                "{opened}"
            }
        }
    }
}
