use super::super::component::*;
use crate::components::button::{Button, ButtonSize, ButtonVariant};
use dioxus::prelude::*;
use dioxus_icons::lucide::Search;

#[component]
pub fn Demo() -> Element {
    rsx! {
        div { style: "display: flex; flex-direction: column; gap: 1rem; width: 100%; max-width: 20rem;",
            InputGroup {
                InputGroupAddon {
                    Search {}
                }
                InputGroupInput { placeholder: "Search..." }
            }
            InputGroup {
                InputGroupAddon { InputGroupText { "$" } }
                InputGroupInput { placeholder: "0.00" }
                InputGroupAddon { align: InputGroupAlign::End,
                    Button {
                        variant: ButtonVariant::Ghost,
                        size: ButtonSize::Sm,
                        "Convert"
                    }
                }
            }
        }
    }
}
