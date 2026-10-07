use crate::components::{
    button::{Button, ButtonVariant},
    sheet::{Sheet, SheetClose, SheetContentClose, SheetDescription, SheetFooter, SheetHeader, SheetTitle},
};
use dioxus::prelude::*;

/// `overlay: false` keeps the sheet modal and drops only the dim and the blur behind it.
#[component]
pub fn Demo() -> Element {
    let mut open = use_signal(|| false);

    rsx! {
        Button {
            variant: ButtonVariant::Outline,
            onclick: move |_| open.set(true),
            "Sheet without overlay"
        }
        Sheet {
            overlay: false,
            open: open(),
            on_open_change: move |v| open.set(v),
            SheetHeader {
                SheetTitle { "No overlay" }
                SheetDescription { "The page behind is inert but not dimmed." }
            }
            SheetFooter {
                SheetClose {
                    as: |attributes| rsx! {
                        Button { variant: ButtonVariant::Outline, attributes, "Dismiss" }
                    },
                }
            }
            SheetContentClose {}
        }
    }
}
