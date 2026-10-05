use super::super::component::*;
use crate::components::attachment::{
    Attachment, AttachmentContent, AttachmentDescription, AttachmentGroup, AttachmentMedia,
    AttachmentTitle,
};
use crate::components::bubble::{Bubble, BubbleContent};
use dioxus::prelude::*;
use dioxus_icons::lucide::FileText;

/// A message with files attached: the attachments sit above the bubble in the
/// message content, aligned to the sender's side.
#[component]
pub fn Demo() -> Element {
    rsx! {
        div { style: "display: flex; width: 100%; max-width: 28rem; min-width: 0; flex-direction: column; gap: var(--dx-space-10);",
            Message { align: MessageAlign::End,
                MessageContent {
                    AttachmentGroup {
                        tabindex: "0",
                        role: "group",
                        aria_label: "Attached files",
                        Attachment {
                            AttachmentMedia {
                                FileText {}
                            }
                            AttachmentContent {
                                AttachmentTitle { "report.pdf" }
                                AttachmentDescription { "PDF · 2.4 MB" }
                            }
                        }
                        Attachment {
                            AttachmentMedia {
                                FileText {}
                            }
                            AttachmentContent {
                                AttachmentTitle { "appendix.pdf" }
                                AttachmentDescription { "PDF · 1.2 MB" }
                            }
                        }
                    }
                    Bubble {
                        BubbleContent { "Can you summarise these two documents?" }
                    }
                }
            }
        }
    }
}
