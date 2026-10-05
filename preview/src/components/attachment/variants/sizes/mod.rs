use super::super::component::*;
use dioxus::prelude::*;
use dioxus_icons::lucide::{FileText, X};

#[component]
pub fn Demo() -> Element {
    rsx! {
        div { style: "display: flex; width: 100%; max-width: 28rem; flex-direction: column; align-items: flex-start; gap: var(--dx-space-3);",
            Attachment { size: AttachmentSize::Default,
                AttachmentMedia {
                    FileText {}
                }
                AttachmentContent {
                    AttachmentTitle { "default.pdf" }
                    AttachmentDescription { "PDF · 2.4 MB" }
                }
                AttachmentActions {
                    AttachmentAction { aria_label: "Remove default.pdf",
                        X {}
                    }
                }
            }
            Attachment { size: AttachmentSize::Sm,
                AttachmentMedia {
                    FileText {}
                }
                AttachmentContent {
                    AttachmentTitle { "sm.pdf" }
                    AttachmentDescription { "PDF · 2.4 MB" }
                }
                AttachmentActions {
                    AttachmentAction { aria_label: "Remove sm.pdf",
                        X {}
                    }
                }
            }
            Attachment { size: AttachmentSize::Xs,
                AttachmentMedia {
                    FileText {}
                }
                AttachmentContent {
                    AttachmentTitle { "xs.pdf" }
                }
                AttachmentActions {
                    AttachmentAction { aria_label: "Remove xs.pdf",
                        X {}
                    }
                }
            }
        }
    }
}
