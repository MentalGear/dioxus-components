use super::super::component::*;
use dioxus::prelude::*;

#[component]
pub fn Demo() -> Element {
    rsx! {
        div { style: "display: flex; flex-direction: column; align-items: center; gap: 1rem;",
            KbdGroup {
                Kbd { "Ctrl" }
                Kbd { "B" }
            }
            p { style: "display: flex; align-items: center; gap: 0.375rem; margin: 0;",
                "Press "
                Kbd { "⌘" }
                Kbd { "K" }
                " to open the command palette."
            }
        }
    }
}
