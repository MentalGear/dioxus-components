use super::super::component::*;
use dioxus::prelude::*;

#[component]
pub fn Demo() -> Element {
    rsx! {
        div { style: "display: flex; width: 100%; max-width: 28rem; flex-direction: column; gap: var(--dx-space-8);",
            BubbleGroup {
                Bubble { variant: BubbleVariant::Secondary,
                    BubbleContent { "I finished the audit pass." }
                }
                Bubble { variant: BubbleVariant::Secondary,
                    BubbleContent { "The registry output looks clean, but I found one stale route." }
                }
                Bubble { variant: BubbleVariant::Secondary,
                    BubbleContent { "Want me to remove it now?" }
                }
            }
            BubbleGroup {
                Bubble { variant: BubbleVariant::Tinted, align: BubbleAlign::End,
                    BubbleContent { "Yes, clean that up." }
                }
                Bubble { variant: BubbleVariant::Tinted, align: BubbleAlign::End,
                    BubbleContent { "Then rerun the registry build." }
                }
            }
        }
    }
}
