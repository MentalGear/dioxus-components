use super::super::component::*;
use crate::components::button::{Button, ButtonVariant};
use dioxus::prelude::*;

#[component]
pub fn Demo() -> Element {
    rsx! {
        div { style: "display: flex; align-items: center; gap: 1.5rem;",
            Spinner {}
            Button {
                variant: ButtonVariant::Outline,
                disabled: true,
                Spinner { label: "Saving" }
                "Saving..."
            }
        }
    }
}
