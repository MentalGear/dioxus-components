use super::super::component::*;
use dioxus::prelude::*;

#[component]
pub fn Demo() -> Element {
    rsx! {
        div { style: "display: flex; width: 100%; max-width: 28rem; flex-direction: column; gap: var(--dx-space-8);",
            Bubble { align: BubbleAlign::Start,
                BubbleContent { "Aligned to the start of the conversation." }
            }
            Bubble { variant: BubbleVariant::Secondary, align: BubbleAlign::End,
                BubbleContent { "Aligned to the end of the conversation." }
            }
        }
    }
}
