use super::super::component::*;
use crate::components::{
    bubble::{Bubble, BubbleContent, BubbleVariant},
    button::{Button, ButtonVariant},
    message::{Message, MessageAlign, MessageContent},
};
use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
struct Turn {
    id: u32,
    mine: bool,
    text: String,
}

const FIRST_LOADED: u32 = 24;
const BATCH: u32 = 6;

/// Rows of different heights, so a position held by pixel offset would drift
/// if the scroller did not hold it by row.
fn turn(id: u32) -> Turn {
    let filler = match id % 3 {
        0 => " Short.",
        1 => " A somewhat longer line that wraps onto a second row in a narrow column.",
        _ => " A much longer one, which keeps going so that the heights of the rows differ from each other by quite a lot.",
    };
    Turn {
        id,
        mine: id.is_multiple_of(2),
        text: format!("Message {id}.{filler}"),
    }
}

#[component]
pub fn Demo() -> Element {
    let mut turns = use_signal(|| {
        (FIRST_LOADED..FIRST_LOADED + 16)
            .map(turn)
            .collect::<Vec<_>>()
    });
    let mut oldest = use_signal(|| FIRST_LOADED);

    rsx! {
        MessageScrollerProvider {
            div { style: "display: flex; width: 100%; max-width: 32rem; min-width: 0; flex-direction: column; gap: var(--dx-space-3);",
                div { style: "height: 22rem; overflow: hidden; border: 1px solid var(--dx-border); border-radius: var(--dx-radius-lg);",
                    MessageScroller {
                        MessageScrollerViewport { "aria-label": "Conversation history",
                            MessageScrollerContent { style: "padding: var(--dx-space-4);",
                                for turn in turns() {
                                    MessageScrollerItem {
                                        key: "{turn.id}",
                                        message_id: "m{turn.id}",
                                        Message {
                                            align: if turn.mine { MessageAlign::End } else { MessageAlign::Start },
                                            MessageContent {
                                                Bubble {
                                                    variant: if turn.mine { BubbleVariant::Default } else { BubbleVariant::Muted },
                                                    BubbleContent { "{turn.text}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        MessageScrollerButton { direction: MessageScrollerDirection::Start }
                        MessageScrollerButton {}
                    }
                }
                div { style: "display: flex; flex-wrap: wrap; align-items: center; gap: var(--dx-space-2);",
                    Button {
                        variant: ButtonVariant::Outline,
                        disabled: oldest() == 0,
                        onclick: move |_| {
                            let from = oldest().saturating_sub(BATCH);
                            let older = (from..oldest()).map(turn).collect::<Vec<_>>();
                            turns.write().splice(0..0, older);
                            oldest.set(from);
                        },
                        "Load earlier messages"
                    }
                }
            }
        }
    }
}
