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
                    Person { name: "Evil Rabbit", initials: "ER" }
                }
                MessageContent {
                    MessageHeader { "Evil Rabbit" }
                    Bubble { variant: BubbleVariant::Muted,
                        BubbleContent { "Did the migration finish?" }
                    }
                    MessageFooter { "10:42 AM" }
                }
            }
            Message { align: MessageAlign::End,
                MessageAvatar {
                    Person { name: "Shadcn", initials: "CN" }
                }
                MessageContent {
                    MessageHeader { "You" }
                    Bubble {
                        BubbleContent { "Yes, all 14 tables are on the new schema." }
                    }
                    MessageFooter { "Read 10:43 AM" }
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
