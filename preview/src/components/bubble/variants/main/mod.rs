use super::super::component::*;
use dioxus::prelude::*;

#[component]
pub fn Demo() -> Element {
    rsx! {
        div { style: "display: flex; width: 100%; max-width: 28rem; flex-direction: column; gap: var(--dx-space-8);",
            Bubble {
                BubbleContent { "Default bubbles use the primary colour for the active user side of a chat." }
            }
            Bubble { variant: BubbleVariant::Secondary,
                BubbleContent { "Secondary bubbles are the standard neutral surface for assistant and conversation content." }
            }
            Bubble { variant: BubbleVariant::Muted,
                BubbleContent { "Muted bubbles lower the emphasis for quiet system notes or supporting content." }
            }
            Bubble { variant: BubbleVariant::Tinted, align: BubbleAlign::End,
                BubbleContent { "Tinted bubbles use a softer primary tint when a primary fill is too strong." }
            }
            Bubble { variant: BubbleVariant::Outline,
                BubbleContent { "Outline bubbles frame message content and give it a border." }
            }
            Bubble { variant: BubbleVariant::Destructive,
                BubbleContent { "Destructive bubbles flag errors or failed actions in a conversation." }
            }
            Bubble { variant: BubbleVariant::Ghost,
                BubbleContent {
                    p { style: "margin: 0 0 var(--dx-space-2)",
                        "Ghost bubbles work for assistant text and other content that should not be framed."
                    }
                    p { style: "margin: 0",
                        "They span the full width of the container, which suits long answers and rich content."
                    }
                }
            }
        }
    }
}
