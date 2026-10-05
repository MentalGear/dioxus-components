use super::super::component::*;
use crate::components::{
    bubble::{Bubble, BubbleContent, BubbleVariant},
    message::{Message, MessageAlign, MessageContent},
};
use dioxus::prelude::*;

const TURNS: [(bool, &str); 7] = [
    (true, "Hi! I am wiring up a saved conversation."),
    (false, "Great, what should it do when someone reopens it?"),
    (true, "Where should it open: the very bottom, or somewhere better?"),
    (false, "Somewhere better. Reopening at the absolute end drops you into the middle of an answer."),
    (true, "So where do I land?"),
    (
        false,
        "On your last message, with its reply below it. You see what you asked and where the answer starts, \
and you can read down from there without reconstructing the thread from the bottom edge. If the last turn is \
short enough to fit in the viewport the scroller falls back to the end, so you never see a blank gap under it.",
    ),
    (true, "And this last one is the message it opens on."),
];

const LAST_REPLY: &str = "That is the `LastAnchor` position. It is keyed on the rows marked with `scroll_anchor`, \
not on a role, so a system marker or a handoff event can be the anchor just as well. The viewport stays hidden \
until the position is applied, which is what keeps a server-rendered page from flashing the top of the thread first. \
\n\nThis reply is intentionally long: a turn that does not fit in the viewport is the case where opening at the \
last message differs from opening at the end. Scroll down to read the rest, or press the button to jump to the end.\
\n\nA saved thread is a place you are returning to, so the first thing on screen should be a place to start \
reading from, not the last line of whatever was said last. Opening at the end is still available: it is the \
default, and the right choice when the newest message is what you came back for.";

#[component]
pub fn Demo() -> Element {
    rsx! {
        MessageScrollerProvider { default_scroll_position: DefaultScrollPosition::LastAnchor,
            div { style: "width: 100%; max-width: 32rem; min-width: 0; height: 22rem; overflow: hidden; border: 1px solid var(--dx-border); border-radius: var(--dx-radius-lg);",
                MessageScroller {
                    MessageScrollerViewport { "aria-label": "Saved conversation",
                        MessageScrollerContent { style: "padding: var(--dx-space-4);",
                            for (i , (mine , text)) in TURNS.iter().enumerate() {
                                MessageScrollerItem {
                                    key: "{i}",
                                    message_id: "m{i}",
                                    scroll_anchor: *mine,
                                    Message { align: if *mine { MessageAlign::End } else { MessageAlign::Start },
                                        MessageContent {
                                            Bubble { variant: if *mine { BubbleVariant::Default } else { BubbleVariant::Muted },
                                                BubbleContent { "{text}" }
                                            }
                                        }
                                    }
                                }
                            }
                            MessageScrollerItem { message_id: "reply",
                                Message {
                                    MessageContent {
                                        Bubble { variant: BubbleVariant::Muted,
                                            BubbleContent { style: "white-space: pre-line;", "{LAST_REPLY}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    MessageScrollerButton {}
                }
            }
        }
    }
}
