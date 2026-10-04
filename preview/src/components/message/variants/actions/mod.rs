use super::super::component::*;
use crate::components::bubble::{Bubble, BubbleContent, BubbleVariant};
use crate::components::button::{Button, ButtonSize, ButtonVariant};
use dioxus::prelude::*;
use dioxus_icons::lucide::{Copy, RefreshCw, ThumbsDown, ThumbsUp};

/// Message-level actions live in the footer. They are icon-only, so each
/// carries an `aria-label`.
#[component]
pub fn Demo() -> Element {
    let mut last = use_signal(|| "No action yet");
    rsx! {
        div { style: "display: flex; width: 100%; max-width: 28rem; min-width: 0; flex-direction: column; gap: var(--dx-space-6);",
            Message {
                MessageContent {
                    Bubble { variant: BubbleVariant::Ghost,
                        BubbleContent { "The deployment looks healthy. P95 latency is 186ms with 2 recent errors, so I would keep it live and monitor the error rate." }
                    }
                    MessageFooter {
                        Button {
                            variant: ButtonVariant::Ghost,
                            size: ButtonSize::IconSm,
                            aria_label: "Copy",
                            onclick: move |_| last.set("Copied"),
                            Copy {}
                        }
                        Button {
                            variant: ButtonVariant::Ghost,
                            size: ButtonSize::IconSm,
                            aria_label: "Good response",
                            onclick: move |_| last.set("Good response"),
                            ThumbsUp {}
                        }
                        Button {
                            variant: ButtonVariant::Ghost,
                            size: ButtonSize::IconSm,
                            aria_label: "Bad response",
                            onclick: move |_| last.set("Bad response"),
                            ThumbsDown {}
                        }
                        Button {
                            variant: ButtonVariant::Ghost,
                            size: ButtonSize::IconSm,
                            aria_label: "Retry",
                            onclick: move |_| last.set("Retry"),
                            RefreshCw {}
                        }
                    }
                }
            }
            p { role: "status", style: "margin: 0; font-size: var(--dx-text-sm); color: var(--dx-muted-foreground)",
                "{last}"
            }
        }
    }
}
