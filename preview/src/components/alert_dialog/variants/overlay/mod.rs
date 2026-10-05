use crate::components::button::{Button, ButtonVariant};

use super::super::component::*;
use dioxus::prelude::*;

/// `overlay: false` keeps the alert dialog modal and drops only the dim and the blur behind it.
#[component]
pub fn Demo() -> Element {
    let mut open = use_signal(|| false);

    rsx! {
        Button {
            variant: ButtonVariant::Outline,
            onclick: move |_| open.set(true),
            "Alert without overlay"
        }
        AlertDialog {
            overlay: false,
            open: open(),
            on_open_change: move |v| open.set(v),
            AlertDialogTitle { "Discard draft" }
            AlertDialogDescription { "The page behind is inert but not dimmed." }
            AlertDialogActions {
                AlertDialogCancel { "Keep" }
                AlertDialogAction { "Discard" }
            }
        }
    }
}
