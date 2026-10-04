use super::super::component::*;
use crate::components::button::{Button, ButtonSize, ButtonVariant};
use dioxus::prelude::*;
use dioxus_icons::lucide::{ThumbsDown, ThumbsUp};

/// Reactions overlap the bubble edge, so the rows are spaced further apart than
/// usual. A static row is one image with a descriptive label; interactive
/// reactions are buttons.
#[component]
pub fn Demo() -> Element {
    let mut reaction = use_signal(|| "No reaction yet");
    rsx! {
        div { style: "display: flex; width: 100%; max-width: 28rem; flex-direction: column; gap: var(--dx-space-16);",
            Bubble {
                BubbleContent { "Reactions at the bottom end." }
                BubbleReactions {
                    side: BubbleSide::Bottom,
                    align: BubbleAlign::End,
                    role: "img",
                    aria_label: "Reactions: thumbs up, surprised, fire, and 8 more",
                    span { "👍" }
                    span { "😮" }
                    span { "🔥" }
                    span { "+8" }
                }
            }
            Bubble { variant: BubbleVariant::Secondary,
                BubbleContent { "Reactions at the bottom start." }
                BubbleReactions {
                    side: BubbleSide::Bottom,
                    align: BubbleAlign::Start,
                    role: "img",
                    aria_label: "Reaction: fire",
                    span { "🔥" }
                }
            }
            Bubble { variant: BubbleVariant::Secondary,
                BubbleContent { "Reactions at the top start." }
                BubbleReactions {
                    side: BubbleSide::Top,
                    align: BubbleAlign::Start,
                    role: "img",
                    aria_label: "Reactions: thumbs up, eyes",
                    span { "👍" }
                    span { "👀" }
                }
            }
            Bubble { variant: BubbleVariant::Muted,
                BubbleContent { "We are going to the movies first, then dinner. Are you in?" }
                BubbleReactions {
                    Button {
                        variant: ButtonVariant::Secondary,
                        size: ButtonSize::IconXs,
                        aria_label: "Thumbs up",
                        onclick: move |_| reaction.set("You agree"),
                        ThumbsUp {}
                    }
                    Button {
                        variant: ButtonVariant::Secondary,
                        size: ButtonSize::IconXs,
                        aria_label: "Thumbs down",
                        onclick: move |_| reaction.set("You disagree"),
                        ThumbsDown {}
                    }
                }
            }
            p { role: "status", style: "margin: 0; font-size: var(--dx-text-sm); color: var(--dx-muted-foreground)",
                "{reaction}"
            }
        }
    }
}
