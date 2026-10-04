use super::super::component::*;
use crate::components::avatar::{Avatar, AvatarFallback};
use crate::components::bubble::{Bubble, BubbleContent, BubbleVariant};
use dioxus::prelude::*;

#[component]
pub fn Demo() -> Element {
    rsx! {
        div { style: "display: flex; width: 100%; max-width: 28rem; min-width: 0; flex-direction: column; gap: var(--dx-space-10);",
            Message {
                MessageAvatar {
                    Person { name: "Shadcn", initials: "CN" }
                }
                MessageContent {
                    Bubble { variant: BubbleVariant::Muted,
                        BubbleContent { "Something went wrong. Any idea?" }
                    }
                }
            }
            Message {
                MessageAvatar {
                    Person { name: "Evil Rabbit", initials: "ER" }
                }
                MessageContent {
                    Bubble { variant: BubbleVariant::Muted,
                        BubbleContent { "I checked the latest deployment and found the build failed during dependency installation." }
                    }
                }
            }
            Message { align: MessageAlign::End,
                MessageAvatar {
                    Person { name: "Me", initials: "ME" }
                }
                MessageContent {
                    Bubble { variant: BubbleVariant::Muted,
                        BubbleContent { "Can you share the exact error message from the logs?" }
                    }
                }
            }
            Message {
                MessageAvatar {
                    Person { name: "Shadcn", initials: "CN" }
                }
                MessageContent {
                    Bubble { variant: BubbleVariant::Destructive,
                        BubbleContent { "Error: the build failed with exit code 1." }
                    }
                }
            }
        }
    }
}

/// A message avatar: the root is `role="img"`, so it carries a name; the
/// fallback is restyled to sit on the avatar slot's muted circle.
#[component]
fn Person(name: &'static str, initials: &'static str) -> Element {
    rsx! {
        Avatar { aria_label: name,
            AvatarFallback { style: "background: var(--dx-muted); font-size: var(--dx-text-xs);", "{initials}" }
        }
    }
}
