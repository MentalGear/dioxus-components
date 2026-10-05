use super::super::component::*;
use crate::components::button::{Button, ButtonVariant};
use dioxus::prelude::*;
use dioxus_icons::lucide::{CircleAlert, FileText, Image, Loader};

/// The whole lifecycle in one place: the first five cards show each state, and
/// the button steps a sixth through them (idle, uploading, processing, done).
#[component]
pub fn Demo() -> Element {
    let mut step = use_signal(|| 0usize);
    let state = match step() % 4 {
        0 => AttachmentState::Idle,
        1 => AttachmentState::Uploading,
        2 => AttachmentState::Processing,
        _ => AttachmentState::Done,
    };
    let description = match state {
        AttachmentState::Idle => "Ready to upload",
        AttachmentState::Uploading => "Uploading...",
        AttachmentState::Processing => "Processing...",
        _ => "PDF · 2.4 MB",
    };
    rsx! {
        div { style: "display: flex; width: 100%; max-width: 28rem; flex-direction: column; align-items: flex-start; gap: var(--dx-space-3);",
            Attachment { state: AttachmentState::Idle,
                AttachmentMedia {
                    FileText {}
                }
                AttachmentContent {
                    AttachmentTitle { "idle.pdf" }
                    AttachmentDescription { "Ready to upload" }
                }
            }
            Attachment { state: AttachmentState::Uploading,
                AttachmentMedia {
                    Loader {}
                }
                AttachmentContent {
                    AttachmentTitle { "uploading.pdf" }
                    AttachmentDescription { "Uploading... 42%" }
                }
            }
            Attachment { state: AttachmentState::Processing,
                AttachmentMedia {
                    Image {}
                }
                AttachmentContent {
                    AttachmentTitle { "processing.png" }
                    AttachmentDescription { "Generating a preview..." }
                }
            }
            Attachment { state: AttachmentState::Error,
                AttachmentMedia {
                    CircleAlert {}
                }
                AttachmentContent {
                    AttachmentTitle { "error.pdf" }
                    AttachmentDescription { "Upload failed: the file is larger than 25 MB" }
                }
            }
            Attachment { state: AttachmentState::Done,
                AttachmentMedia {
                    FileText {}
                }
                AttachmentContent {
                    AttachmentTitle { "done.pdf" }
                    AttachmentDescription { "PDF · 2.4 MB" }
                }
            }
            Attachment { state,
                AttachmentMedia {
                    FileText {}
                }
                AttachmentContent {
                    AttachmentTitle { "lifecycle.pdf" }
                    AttachmentDescription { "{description}" }
                }
            }
            Button { variant: ButtonVariant::Outline, onclick: move |_| step += 1, "Advance lifecycle" }
        }
    }
}
