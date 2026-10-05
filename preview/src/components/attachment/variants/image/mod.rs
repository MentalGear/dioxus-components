use super::super::component::*;
use dioxus::prelude::*;
use dioxus_icons::lucide::X;

// A self-contained gradient preview, so the demo needs no network.
const PREVIEW: &str = "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 96 96'%3E%3Cdefs%3E%3ClinearGradient id='g' x1='0' y1='0' x2='1' y2='1'%3E%3Cstop offset='0' stop-color='%236366f1'/%3E%3Cstop offset='1' stop-color='%23ec4899'/%3E%3C/linearGradient%3E%3C/defs%3E%3Crect width='96' height='96' fill='url(%23g)'/%3E%3C/svg%3E";

/// `AttachmentMediaVariant::Image` holds an `<img>`; `Vertical` stacks the
/// media above the content.
#[component]
pub fn Demo() -> Element {
    rsx! {
        div { style: "display: flex; width: 100%; max-width: 28rem; flex-wrap: wrap; align-items: flex-start; gap: var(--dx-space-3);",
            Attachment {
                AttachmentMedia { variant: AttachmentMediaVariant::Image,
                    img { src: PREVIEW, alt: "Gradient preview of workspace.png" }
                }
                AttachmentContent {
                    AttachmentTitle { "workspace.png" }
                    AttachmentDescription { "PNG · 1.1 MB" }
                }
                AttachmentActions {
                    AttachmentAction { aria_label: "Remove workspace.png",
                        X {}
                    }
                }
            }
            Attachment { orientation: AttachmentOrientation::Vertical,
                AttachmentMedia { variant: AttachmentMediaVariant::Image,
                    img { src: PREVIEW, alt: "Gradient preview of cover.png" }
                }
                AttachmentContent {
                    AttachmentTitle { "cover.png" }
                    AttachmentDescription { "PNG · 640 KB" }
                }
                AttachmentActions {
                    AttachmentAction { aria_label: "Remove cover.png",
                        X {}
                    }
                }
            }
            Attachment {
                state: AttachmentState::Uploading,
                orientation: AttachmentOrientation::Vertical,
                AttachmentMedia { variant: AttachmentMediaVariant::Image,
                    img { src: PREVIEW, alt: "Gradient preview of banner.png" }
                }
                AttachmentContent {
                    AttachmentTitle { "banner.png" }
                    AttachmentDescription { "Uploading..." }
                }
            }
        }
    }
}
