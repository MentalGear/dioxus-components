use super::super::component::*;
use dioxus::prelude::*;
use dioxus_icons::lucide::{FileArchive, FileSpreadsheet, FileText, X};

const FILES: &[(&str, &str)] = &[
    ("sales-dashboard.pdf", "PDF · 2.4 MB"),
    ("q3-forecast.xlsx", "Spreadsheet · 840 KB"),
    ("design-assets.zip", "Archive · 18 MB"),
];

#[component]
pub fn Demo() -> Element {
    let mut removed = use_signal(Vec::<&'static str>::new);
    rsx! {
        div { style: "display: flex; width: 100%; max-width: 28rem; flex-direction: column; align-items: flex-start; gap: var(--dx-space-3);",
            for (name , meta) in FILES.iter().copied().filter(|(name, _)| !removed.read().contains(name)) {
                Attachment { key: "{name}",
                    AttachmentMedia {
                        FileIcon { name }
                    }
                    AttachmentContent {
                        AttachmentTitle { "{name}" }
                        AttachmentDescription { "{meta}" }
                    }
                    AttachmentActions {
                        AttachmentAction {
                            aria_label: "Remove {name}",
                            onclick: move |_| removed.write().push(name),
                            X {}
                        }
                    }
                }
            }
            if removed.read().len() == FILES.len() {
                p { role: "status", style: "margin: 0; font-size: var(--dx-text-sm); color: var(--dx-muted-foreground)",
                    "All attachments removed"
                }
            }
        }
    }
}

/// The media icon for a file, by its name.
#[component]
fn FileIcon(name: &'static str) -> Element {
    match name {
        "q3-forecast.xlsx" => rsx! {
            FileSpreadsheet {}
        },
        "design-assets.zip" => rsx! {
            FileArchive {}
        },
        _ => rsx! {
            FileText {}
        },
    }
}
