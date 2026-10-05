use crate::components::button::{Button, ButtonVariant};

use super::super::component::{Dialog, DialogDescription, DialogTitle};
use dioxus::prelude::*;

/// A modal `Dialog` dims the page behind it with the shared overlay scrim. `overlay: false` keeps
/// the modality (focus trap, inert page, click-outside dismissal) and drops only the dim and the
/// blur. A non-modal dialog has no `::backdrop`; its wrapper element paints the scrim instead, and
/// `overlay: false` removes that one too.
#[component]
pub fn Demo() -> Element {
    let mut modal_off = use_signal(|| false);
    let mut inline_on = use_signal(|| false);
    let mut inline_off = use_signal(|| false);

    rsx! {
        div { display: "flex", flex_wrap: "wrap", gap: "0.5rem",
            Button {
                variant: ButtonVariant::Outline,
                onclick: move |_| modal_off.set(true),
                "Dialog without overlay"
            }
            Button {
                variant: ButtonVariant::Outline,
                onclick: move |_| inline_on.set(true),
                "Non-modal dialog"
            }
            Button {
                variant: ButtonVariant::Outline,
                onclick: move |_| inline_off.set(true),
                "Non-modal dialog without overlay"
            }
        }

        Dialog {
            overlay: false,
            open: modal_off(),
            on_open_change: move |v| modal_off.set(v),
            DialogTitle { "No overlay" }
            DialogDescription { "The page behind is inert but not dimmed." }
        }
        Dialog {
            is_modal: false,
            open: inline_on(),
            on_open_change: move |v| inline_on.set(v),
            DialogTitle { "Non-modal" }
            DialogDescription { "No top layer: the scrim is the wrapper element." }
        }
        Dialog {
            is_modal: false,
            overlay: false,
            open: inline_off(),
            on_open_change: move |v| inline_off.set(v),
            DialogTitle { "Non-modal, no overlay" }
            DialogDescription { "Neither a top layer nor a scrim." }
        }
    }
}
