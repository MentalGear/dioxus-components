use super::super::component::*;
use crate::components::{
    bubble::{Bubble, BubbleContent, BubbleVariant},
    button::{Button, ButtonVariant},
    marker::{Marker, MarkerContent, MarkerIcon},
    message::{Message, MessageAlign, MessageContent},
    spinner::Spinner,
    switch::Switch,
};
use dioxus::prelude::*;

/// One row of the transcript.
#[derive(Clone, PartialEq)]
struct Turn {
    id: u32,
    mine: bool,
    text: String,
}

const SEED: [(bool, &str); 12] = [
    (true, "Can you walk me through how a streaming chat should scroll?"),
    (false, "Sure. The short version: move the reader only when they asked to move."),
    (true, "What does that mean while a reply is arriving?"),
    (false, "If you are at the live edge the new text stays in view. If you scroll away, it leaves you alone."),
    (true, "And when I send a new message?"),
    (false, "Your message lands near the top with a peek of the previous turn, and the reply grows into the room below it."),
    (true, "Then what happens once the reply fills the screen?"),
    (false, "Following takes over from the anchor, so the newest words stay visible until you scroll up."),
    (true, "How do I get back after scrolling up?"),
    (false, "A jump button appears at the bottom. It is a real button, so the keyboard reaches it too."),
    (true, "Does it work for a saved conversation?"),
    (false, "Yes: it opens at the end (or at your last message) without flashing the top first."),
];

const REPLY: &str = "Streaming makes the list grow while you are still reading it. \
So the scroller follows the newest words only while you are already at the live edge, \
and the moment you scroll, press a scroll key or drag the scrollbar, it stops following. \
Press the jump button to return to the live edge and keep following from there. \
Nothing here talks to the framework per token: the text updates, the resize is observed, \
and one small controller moves the viewport. \
That is why a long reply stays smooth: the work that runs at scroll frequency never leaves the \
browser, and Rust hears about a change only when the answer to \"can I scroll further?\" flips. \
A new message of yours lands near the top of the viewport with a peek of the previous turn above it, \
and the reply grows into the room beneath. Once it fills the screen, following takes over from the anchor, \
so the newest words stay in view until you decide to read something else. \
Scroll up at any time and the stream carries on below you without moving what you are reading.";

/// A real timer for the simulated stream (this demo has no async runtime of
/// its own; the browser's `setTimeout` is the clock).
async fn sleep_ms(ms: u32) {
    let _ = document::eval(&format!("await new Promise((r) => setTimeout(r, {ms}));")).await;
}

/// Reads `use_message_scroller_scrollable()`: the signal the controller feeds
/// with change-only messages.
#[component]
fn ScrollStatus() -> Element {
    let scrollable = use_message_scroller_scrollable();
    let state = scrollable();
    let label = match (state.start, state.end) {
        (false, false) => "none",
        (true, false) => "start",
        (false, true) => "end",
        (true, true) => "start end",
    };
    rsx! {
        span { style: "margin-inline-start: auto; color: var(--dx-muted-foreground); font-size: var(--dx-text-xs);",
            "Scrollable: "
            span { "data-testid": "scroll-status", "{label}" }
        }
    }
}

#[component]
pub fn Demo() -> Element {
    let mut turns = use_signal(|| {
        SEED.iter()
            .enumerate()
            .map(|(i, (mine, text))| Turn {
                id: i as u32,
                mine: *mine,
                text: text.to_string(),
            })
            .collect::<Vec<_>>()
    });
    let mut next_id = use_signal(|| SEED.len() as u32);
    let mut busy = use_signal(|| false);
    let mut thinking = use_signal(|| false);
    let mut auto_scroll = use_signal(|| true);

    let mut push = move |mine: bool, text: String| -> u32 {
        let id = next_id();
        next_id.set(id + 1);
        turns.write().push(Turn { id, mine, text });
        id
    };

    // Append an assistant message and stream `REPLY` into it word by word.
    // `with_user` first sends a user turn (a scroll anchor), so the demo shows
    // anchor-then-follow as well as plain follow.
    let mut stream = move |with_user: bool| {
        if busy() {
            return;
        }
        busy.set(true);
        spawn(async move {
            if with_user {
                push(true, "Tell me more about following the stream.".to_string());
                sleep_ms(120).await;
            }
            thinking.set(true);
            sleep_ms(500).await;
            thinking.set(false);
            let id = push(false, String::new());
            for word in REPLY.split(' ') {
                sleep_ms(20).await;
                if let Some(turn) = turns.write().iter_mut().find(|t| t.id == id) {
                    if !turn.text.is_empty() {
                        turn.text.push(' ');
                    }
                    turn.text.push_str(word);
                }
            }
            busy.set(false);
        });
    };

    rsx! {
        MessageScrollerProvider { auto_scroll: auto_scroll(),
            div { style: "display: flex; width: 100%; max-width: 32rem; min-width: 0; flex-direction: column; gap: var(--dx-space-3);",
                div { style: "height: 26rem; overflow: hidden; border: 1px solid var(--dx-border); border-radius: var(--dx-radius-lg);",
                    MessageScroller {
                        MessageScrollerViewport { "aria-label": "Conversation",
                            MessageScrollerContent { style: "padding: var(--dx-space-4);",
                                for turn in turns() {
                                    MessageScrollerItem {
                                        key: "{turn.id}",
                                        message_id: "m{turn.id}",
                                        scroll_anchor: turn.mine,
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
                                if thinking() {
                                    MessageScrollerItem { message_id: "thinking",
                                        Marker { role: "status",
                                            MarkerIcon {
                                                Spinner {}
                                            }
                                            MarkerContent { "Thinking..." }
                                        }
                                    }
                                }
                            }
                        }
                        MessageScrollerButton {}
                    }
                }
                div { style: "display: flex; flex-wrap: wrap; align-items: center; gap: var(--dx-space-2);",
                    Button { disabled: busy(), onclick: move |_| stream(false), "Stream a reply" }
                    Button {
                        variant: ButtonVariant::Outline,
                        disabled: busy(),
                        onclick: move |_| stream(true),
                        "Send a message"
                    }
                    Button {
                        variant: ButtonVariant::Outline,
                        onclick: move |_| {
                            let n = next_id();
                            push(false, format!("Message {n}"));
                        },
                        "Add message"
                    }
                    label { style: "display: inline-flex; align-items: center; gap: var(--dx-space-2); font-size: var(--dx-text-sm);",
                        Switch {
                            checked: auto_scroll(),
                            aria_label: "Auto-scroll",
                            on_checked_change: move |value| auto_scroll.set(value),
                        }
                        "Auto-scroll"
                    }
                    ScrollStatus {}
                }
            }
        }
    }
}
