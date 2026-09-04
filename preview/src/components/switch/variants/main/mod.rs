use super::super::component::*;
use dioxus::prelude::*;

// docs/backlog.md row 32: no `#[css_module]` of its own here, and no
// `document::Link` needed either -- this `Demo` always renders the `Switch`
// themed wrapper below, which (as of this migration) now carries its own
// `document::Link` for `style.css`. `document::Link` dedupes on `(href,
// rel)`, so the link the wrapper inserts covers this page.
#[component]
pub fn Demo() -> Element {
    let mut checked = use_signal(|| false);
    rsx! {
        div { class: "dx-switch-example",
            Switch {
                checked: checked(),
                aria_label: "Switch Demo",
                on_checked_change: move |new_checked| {
                    checked.set(new_checked);
                },
            }
        }
    }
}
