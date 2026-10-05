use super::super::component::*;
use crate::components::bubble::{Bubble, BubbleAlign, BubbleContent, BubbleGroup, BubbleVariant};
use dioxus::prelude::*;

#[component]
pub fn Demo() -> Element {
    rsx! {
        div { style: "display: flex; width: 100%; max-width: 28rem; min-width: 0; flex-direction: column; gap: var(--dx-space-10);",
            Message { align: MessageAlign::End,
                MessageContent {
                    Bubble {
                        BubbleContent { "Deploying to prod real quick." }
                    }
                }
            }
            Message {
                MessageContent {
                    Bubble { variant: BubbleVariant::Muted,
                        BubbleContent { "It's 4:55 PM. On a Friday." }
                    }
                }
            }
            Message { align: MessageAlign::End,
                MessageContent {
                    Bubble {
                        BubbleContent { "It's a one-line change." }
                    }
                }
            }
            Message {
                MessageContent {
                    BubbleGroup {
                        Bubble { variant: BubbleVariant::Muted, align: BubbleAlign::Start,
                            BubbleContent { "It's always a one-line change. Make sure to run the tests this time." }
                        }
                        Bubble { variant: BubbleVariant::Muted, align: BubbleAlign::Start,
                            BubbleContent { "Alright, go for it." }
                        }
                    }
                }
            }
        }
    }
}
