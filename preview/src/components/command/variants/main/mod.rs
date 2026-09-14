use crate::components::button::component::Button;
use crate::components::kbd::component::{Kbd, KbdGroup};

use super::super::component::{Command, CommandDialog, CommandEmpty, CommandGroup, CommandGroupLabel, CommandItem};
use dioxus::prelude::*;

// docs/backlog.md row 32: no `#[css_module]` of its own here, and no
// `document::Link` needed either -- the `CommandDialog`/`Command` themed
// wrappers below already carry their own `document::Link` for
// `style.css`, deduplicated by `document::Link`'s own `(href, rel)`
// dedupe (same reasoning as dialog/variants/main/mod.rs).
#[component]
pub fn Demo() -> Element {
    let mut open = use_signal(|| false);
    let mut query = use_signal(String::new);
    let mut last_run = use_signal(|| None::<String>);

    let commands: &[(&str, &str, &str)] = &[
        ("new-file", "New File", "N"),
        ("new-window", "New Window", "⇧N"),
        ("open-file", "Open File...", "O"),
        ("save-file", "Save File", "S"),
    ];
    let view_commands: &[(&str, &str)] = &[("toggle-sidebar", "Toggle Sidebar"), ("toggle-terminal", "Toggle Terminal")];

    rsx! {
        Button {
            r#type: "button",
            "data-style": "outline",
            onclick: move |_| open.set(true),
            "Open Command Palette"
        }
        if let Some(run) = last_run() {
            p { "data-testid": "command-last-run", "Ran: {run}" }
        }
        CommandDialog {
            open: open(),
            on_open_change: move |v| {
                open.set(v);
                if !v {
                    query.set(String::new());
                }
            },
            Command::<String> {
                query: Some(query()),
                on_query_change: move |next| query.set(next),
                on_value_change: move |value: Option<String>| {
                    if let Some(value) = value {
                        last_run.set(Some(value));
                    }
                    open.set(false);
                    query.set(String::new());
                },
                input_aria_label: "Search commands",
                placeholder: "Type a command or search...",
                list_aria_label: "Commands",
                CommandEmpty { "No results found." }
                CommandGroup {
                    CommandGroupLabel { "File" }
                    for (i , (value , label , key)) in commands.iter().enumerate() {
                        CommandItem::<String> {
                            index: i,
                            value: value.to_string(),
                            text_value: label.to_string(),
                            shortcut: rsx! {
                                KbdGroup {
                                    Kbd { "⌘" }
                                    Kbd { "{key}" }
                                }
                            },
                            "{label}"
                        }
                    }
                }
                CommandGroup {
                    CommandGroupLabel { "View" }
                    for (i , (value , label)) in view_commands.iter().enumerate() {
                        CommandItem::<String> {
                            index: commands.len() + i,
                            value: value.to_string(),
                            text_value: label.to_string(),
                            "{label}"
                        }
                    }
                }
            }
        }
    }
}
