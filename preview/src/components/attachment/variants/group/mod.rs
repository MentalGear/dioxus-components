use super::super::component::*;
use dioxus::prelude::*;
use dioxus_icons::lucide::FileText;

const FILES: &[&str] = &[
    "sales-dashboard.pdf",
    "q3-forecast.pdf",
    "roadmap-2027.pdf",
    "design-review.pdf",
    "customer-interviews.pdf",
    "security-audit.pdf",
];

/// A horizontally scrolling, snapping row with an edge fade. These
/// attachments are presentational, so the group itself is focusable (and
/// labelled) to keep it scrollable from the keyboard.
#[component]
pub fn Demo() -> Element {
    rsx! {
        div { style: "width: 100%; max-width: 28rem; min-width: 0;",
            AttachmentGroup {
                tabindex: "0",
                role: "group",
                aria_label: "Attached files",
                for name in FILES.iter().copied() {
                    Attachment { key: "{name}",
                        AttachmentMedia {
                            FileText {}
                        }
                        AttachmentContent {
                            AttachmentTitle { "{name}" }
                            AttachmentDescription { "PDF · 2.4 MB" }
                        }
                    }
                }
            }
        }
    }
}
