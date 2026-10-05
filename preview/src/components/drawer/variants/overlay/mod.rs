use crate::components::{
    button::{Button, ButtonVariant},
    drawer::{Drawer, DrawerClose, DrawerContent, DrawerDescription, DrawerFooter, DrawerHandle, DrawerHeader, DrawerTitle},
};
use dioxus::prelude::*;

/// `overlay: false` keeps the drawer modal and drops only the dim and the blur behind it.
#[component]
pub fn Demo() -> Element {
    let mut open = use_signal(|| false);

    rsx! {
        Button {
            variant: ButtonVariant::Outline,
            onclick: move |_| open.set(true),
            "Drawer without overlay"
        }
        Drawer { overlay: false, open: open(), on_open_change: move |v| open.set(v),
            DrawerContent {
                DrawerHandle {}
                DrawerHeader {
                    DrawerTitle { "No overlay" }
                    DrawerDescription { "The page behind is inert but not dimmed." }
                }
                DrawerFooter {
                    DrawerClose { "Dismiss" }
                }
            }
        }
    }
}
