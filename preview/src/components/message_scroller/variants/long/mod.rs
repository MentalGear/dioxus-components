use super::super::component::*;
use crate::components::button::{Button, ButtonVariant};
use dioxus::prelude::*;

const ROWS: u32 = 2000;

/// Rows of varying height (one to four lines), so the browser's remembered
/// sizes and the first-render estimate genuinely differ.
fn text(i: u32) -> String {
    let lines = 1 + (i * 7) % 4;
    let mut text = format!("Message {i}.");
    for line in 1..lines {
        text.push_str(&format!(" Line {} of a longer message.", line + 1));
    }
    text
}

#[component]
fn Controls(count: Signal<u32>) -> Element {
    let scroller = use_message_scroller();
    rsx! {
        div { style: "display: flex; flex-wrap: wrap; align-items: center; gap: var(--dx-space-2);",
            Button {
                variant: ButtonVariant::Outline,
                onclick: move |_| {
                    scroller.scroll_to_message("m1000", ScrollToMessageOptions::default());
                },
                "Jump to message 1000"
            }
            Button {
                variant: ButtonVariant::Outline,
                onclick: move |_| count += 1,
                "Add message"
            }
            span { style: "margin-inline-start: auto; color: var(--dx-muted-foreground); font-size: var(--dx-text-xs);",
                "{count} messages in the DOM"
            }
        }
    }
}

/// Two thousand rows, all in the DOM. Only the rows in view are laid out and
/// painted (`content-visibility: auto`), and the last rows stay rendered so
/// following a reply is exact; native find-in-page still reaches every row.
#[component]
pub fn Demo() -> Element {
    let count = use_signal(|| ROWS);

    rsx! {
        MessageScrollerProvider { auto_scroll: true,
            div { style: "display: flex; width: 100%; max-width: 32rem; min-width: 0; flex-direction: column; gap: var(--dx-space-3);",
                div { style: "height: 22rem; overflow: hidden; border: 1px solid var(--dx-border); border-radius: var(--dx-radius-lg);",
                    MessageScroller {
                        MessageScrollerViewport { "aria-label": "Long conversation",
                            MessageScrollerContent { style: "padding: var(--dx-space-4); gap: var(--dx-space-2);",
                                for i in 0..count() {
                                    MessageScrollerItem { key: "{i}", message_id: "m{i}",
                                        div { style: "padding: var(--dx-space-2) var(--dx-space-3); border: 1px solid var(--dx-border); border-radius: var(--dx-radius-lg); font-size: var(--dx-text-sm);",
                                            "{text(i)}"
                                        }
                                    }
                                }
                            }
                        }
                        MessageScrollerButton {}
                    }
                }
                Controls { count }
            }
        }
    }
}
