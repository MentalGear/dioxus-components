use crate::components::button::{Button, ButtonVariant};
use crate::components::dialog::DialogTitle;

use super::super::component::{Command, CommandDialog, CommandEmpty, CommandGroup, CommandItem};
use dioxus::prelude::*;

/// `overlay: false` keeps the palette modal and drops only the dim and the blur behind it.
#[component]
pub fn Demo() -> Element {
    let mut open = use_signal(|| false);
    let mut query = use_signal(String::new);

    rsx! {
        Button {
            variant: ButtonVariant::Outline,
            onclick: move |_| open.set(true),
            "Palette without overlay"
        }
        CommandDialog {
            overlay: false,
            open: open(),
            on_open_change: move |v| {
                open.set(v);
                if !v {
                    query.set(String::new());
                }
            },
            DialogTitle { class: "dx-command-sr-only", "Command Palette" }
            Command::<String> {
                query: Some(query()),
                on_query_change: move |next| query.set(next),
                on_value_change: move |_| open.set(false),
                input_aria_label: "Search commands without overlay",
                placeholder: "Type a command or search...",
                list_aria_label: "Commands without overlay",
                CommandEmpty { "No results found." }
                CommandGroup {
                    CommandItem::<String> {
                        index: 0usize,
                        value: "new-file".to_string(),
                        text_value: "New File",
                        "New File"
                    }
                }
            }
        }
    }
}
