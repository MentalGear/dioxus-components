use crate::components::button::component::Button;

use super::super::component::{Dialog, DialogDescription, DialogTitle};
use dioxus::prelude::*;

// docs/backlog.md row 32: no `#[css_module]` of its own here, and no
// `document::Link` needed either -- this `Demo` always renders the `Dialog`
// themed wrapper below, which (as of this migration) now carries its own
// `document::Link` for `style.css`. `document::Link` dedupes on `(href,
// rel)`, so the link the wrapper inserts covers this page.
#[component]
pub fn Demo() -> Element {
    let mut open = use_signal(|| false);
    // Nested dialog, additive to this demo so the tier-3 scroll-lock oracle
    // (playwright/oracle/tier3-radix/scroll-lock.spec.ts) has a fixture for
    // the refcounted-lock case: two modals open at once, closing the inner
    // one must leave the page still locked. See docs/plan.md Phase 3.2.
    let mut nested_open = use_signal(|| false);

    rsx! {
        Button {
            r#type: "button",
            "data-style": "outline",
            onclick: move |_| open.set(true),
            "Show Dialog"
        }
        Dialog { open: open(), on_open_change: move |v| open.set(v),
            button {
                class: "dx-dialog-close",
                r#type: "button",
                aria_label: "Close",
                tabindex: if open() { "0" } else { "-1" },
                onclick: move |_| open.set(false),
                "×"
            }
            DialogTitle { "Item information" }
            DialogDescription { "Here is some additional information about the item." }
            Button {
                r#type: "button",
                "data-style": "outline",
                onclick: move |_| nested_open.set(true),
                "Open Nested Dialog"
            }
            // Rendered as a child of the outer Dialog's content -- not a
            // sibling -- so a still-open outer dialog's scroll lock isn't
            // released when this one closes. See docs/plan.md Phase 3.2.
            Dialog {
                open: nested_open(),
                on_open_change: move |v| nested_open.set(v),
                button {
                    class: "dx-dialog-close",
                    r#type: "button",
                    aria_label: "Close Nested",
                    tabindex: if nested_open() { "0" } else { "-1" },
                    onclick: move |_| nested_open.set(false),
                    "×"
                }
                DialogTitle { "Nested dialog" }
                DialogDescription { "A second, independently-opened dialog nested inside the first." }
            }
        }
    }
}
