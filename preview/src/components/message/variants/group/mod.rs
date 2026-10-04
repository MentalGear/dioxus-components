use super::super::component::*;
use crate::components::avatar::{Avatar, AvatarFallback};
use crate::components::bubble::{Bubble, BubbleContent, BubbleVariant};
use dioxus::prelude::*;

/// An empty `MessageAvatar` on the earlier messages keeps them aligned with
/// the avatar on the last one.
#[component]
pub fn Demo() -> Element {
    rsx! {
        div { style: "display: flex; width: 100%; max-width: 28rem; min-width: 0; flex-direction: column; gap: var(--dx-space-10);",
            MessageGroup {
                Message {
                    MessageContent {
                        Bubble { variant: BubbleVariant::Muted,
                            BubbleContent { "I checked the registry addresses." }
                        }
                    }
                }
                Message {
                    MessageContent {
                        Bubble { variant: BubbleVariant::Muted,
                            BubbleContent { "The component and example JSON now live under the UI registry." }
                        }
                    }
                }
            }
            MessageGroup {
                Message { align: MessageAlign::End,
                    MessageContent {
                        Bubble { variant: BubbleVariant::Muted,
                            BubbleContent { "I checked the registry addresses." }
                        }
                    }
                }
                Message { align: MessageAlign::End,
                    MessageContent {
                        Bubble { variant: BubbleVariant::Muted,
                            BubbleContent { "The component and example JSON now live under the UI registry." }
                        }
                    }
                }
            }
            MessageGroup {
                Message {
                    MessageAvatar {}
                    MessageContent {
                        Bubble { variant: BubbleVariant::Muted,
                            BubbleContent { "I checked the registry addresses." }
                        }
                    }
                }
                Message {
                    MessageAvatar {
                        Person { name: "Shadcn", initials: "CN" }
                    }
                    MessageContent {
                        Bubble { variant: BubbleVariant::Muted,
                            BubbleContent { "The component and example JSON now live under the UI registry." }
                        }
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
