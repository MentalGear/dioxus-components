use super::super::component::{Accordion, AccordionContent, AccordionItem, AccordionTrigger};
use dioxus::prelude::*;

const ITEMS: [(&str, &str); 3] = [
    (
        "Shipping",
        "Orders leave the warehouse within two working days.",
    ),
    (
        "Returns",
        "Returns are free for 30 days, including the narwhal plush.",
    ),
    (
        "Warranty",
        "Every product carries a two year limited warranty.",
    ),
];

/// `hidden_until_found`: closed items stay mounted as `hidden="until-found"`, so the browser's
/// find-in-page (and a `#fragment` link) can reach them. The browser reveals the item and the
/// accordion opens it, closing the open one because `allow_multiple_open` is false.
#[component]
pub fn Demo() -> Element {
    rsx! {
        div { style: "display: flex; flex-direction: column; gap: var(--dx-space-3); width: 100%; max-width: 15rem;",
            Accordion { allow_multiple_open: false, hidden_until_found: true,
                for (i , (title , body)) in ITEMS.into_iter().enumerate() {
                    AccordionItem { index: i, default_open: i == 0,
                        AccordionTrigger { "{title}" }
                        AccordionContent {
                            div { padding_bottom: "1rem",
                                p { padding: "0", "{body}" }
                            }
                        }
                    }
                }
            }
            p { style: "margin: 0; font-size: var(--dx-text-sm); color: var(--dx-muted-foreground)",
                "Try Ctrl+F for \"narwhal\": its closed item opens and the open one closes."
            }
        }
    }
}
