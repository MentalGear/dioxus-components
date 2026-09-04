use super::super::component::AspectRatio;
use dioxus::prelude::*;

// docs/backlog.md row 32: no `#[css_module]` of its own here, and no
// `document::Link` needed either -- this `Demo` always renders the real
// `AspectRatio` themed wrapper below, which (as of this migration) now
// carries its own `document::Link` for `style.css`. `document::Link` dedupes
// on `(href, rel)`, so the one link the wrapper inserts covers this page.
#[component]
pub fn Demo() -> Element {
    rsx! {
        div {
            class: "dx-aspect-ratio-container",
            width: "20rem",
            max_width: "30vw",
            AspectRatio { ratio: 4.0 / 3.0,
                div {
                    background: "linear-gradient(to bottom right, var(--primary-color-5), var(--primary-color-3))",
                    width: "100%",
                    height: "100%",
                }
            }
        }
    }
}
