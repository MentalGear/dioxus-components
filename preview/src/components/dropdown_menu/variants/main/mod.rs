use super::super::component::{
    DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger,
};
use dioxus::prelude::*;
use strum::IntoEnumIterator;

// docs/backlog.md row 32: no `#[css_module]` of its own here, and no
// `document::Link` needed either -- this `Demo` always renders the
// `DropdownMenu`/`DropdownMenuTrigger`/`DropdownMenuContent`/
// `DropdownMenuItem` themed wrappers below, each of which (as of this
// migration) now carries its own `document::Link` for `style.css`.
// `document::Link` dedupes on `(href, rel)`, so those links cover this page.
#[derive(Clone, Copy, strum::Display, strum::EnumIter, PartialEq)]
enum Operation {
    Edit,
    Undo,
    Duplicate,
    Delete,
}

#[component]
pub fn Demo() -> Element {
    let mut selected_operation = use_signal(|| None);

    let operations = Operation::iter().enumerate().map(|(i, o)| {
        rsx! {
            DropdownMenuItem::<Operation> {
                class: "dx-dropdown-menu-item",
                value: o,
                index: i,
                disabled: matches!(o, Operation::Undo),
                on_select: move |value| {
                    selected_operation.set(Some(value));
                },
                {o.to_string()}
            }
        }
    });

    rsx! {
        DropdownMenu { class: "dx-dropdown-menu", default_open: false,
            DropdownMenuTrigger { class: "dx-dropdown-menu-trigger", "Open Menu" }
            DropdownMenuContent { class: "dx-dropdown-menu-content", {operations} }
        }
        if let Some(op) = selected_operation() {
            "Selected: {op}"
        }
    }
}
