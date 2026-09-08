use super::super::component::{
    DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSub, DropdownMenuSubContent,
    DropdownMenuSubItem, DropdownMenuSubTrigger, DropdownMenuTrigger,
};
use dioxus::prelude::*;
use strum::IntoEnumIterator;

// docs/backlog.md row 32: no `#[css_module]` of its own here, and no
// `document::Link` needed either -- this `Demo` always renders the
// `DropdownMenu`/`DropdownMenuTrigger`/`DropdownMenuContent`/
// `DropdownMenuItem` themed wrappers below, each of which (as of this
// migration) now carries its own `document::Link` for `style.css`.
// `document::Link` dedupes on `(href, rel)`, so those links cover this page.
//
// Index layout (docs/component-backlog.md row 68 added the submenu):
// Edit=0, Undo=1 (disabled), Duplicate=2, the submenu trigger=3, Delete=4.
// Several existing Playwright specs hard-depend on this demo's *shape*,
// not just its presence, so the nested submenu is slotted in without
// disturbing any of it: `playwright/dropdown-menu.spec.ts` and
// `oracle/tier1-apg/keyboard-matrix.spec.ts` both require Edit first, Undo
// disabled (so a 2nd ArrowDown from Edit skips it and lands on Duplicate),
// and -- keyboard-matrix.spec.ts specifically -- Delete reachable via End
// as the *last* item and as ArrowDown's wrap-around target from it. Roving
// focus orders by `index` value, not by JSX/DOM position (`collection.rs`),
// so Delete keeps index 4 (bumped up from its old 3) precisely so it stays
// the highest index -- and therefore still "last" -- with the submenu
// trigger sliding in at index 3, between Duplicate and it.
#[derive(Clone, Copy, strum::Display, strum::EnumIter, PartialEq)]
enum Operation {
    Edit,
    Undo,
    Duplicate,
}

// The nested-submenu-only items (docs/component-backlog.md row 68), kept
// as their own small enum so the submenu's own indices -- scoped to
// `DropdownMenuSubContent`'s own roving-focus collection, never the root
// menu's -- read as obviously independent of `Operation` above.
#[derive(Clone, Copy, strum::Display, strum::EnumIter, PartialEq)]
enum ToolOperation {
    Rename,
    Archive,
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
                on_select: move |value: Operation| {
                    selected_operation.set(Some(value.to_string()));
                },
                {o.to_string()}
            }
        }
    });

    let tool_operations = ToolOperation::iter().enumerate().map(|(i, o)| {
        rsx! {
            DropdownMenuSubItem::<ToolOperation> {
                value: o,
                index: i,
                on_select: move |value: ToolOperation| {
                    selected_operation.set(Some(value.to_string()));
                },
                {o.to_string()}
            }
        }
    });

    rsx! {
        DropdownMenu { class: "dx-dropdown-menu", default_open: false,
            DropdownMenuTrigger { class: "dx-dropdown-menu-trigger", "Open Menu" }
            DropdownMenuContent { class: "dx-dropdown-menu-content",
                {operations}
                // Nested submenu (docs/component-backlog.md row 68): see
                // this file's top-of-file comment for why this sits at
                // index 3, between Duplicate (2) and Delete (4).
                DropdownMenuSub {
                    DropdownMenuSubTrigger { index: 3usize, "More tools" }
                    DropdownMenuSubContent { {tool_operations} }
                }
                DropdownMenuItem::<String> {
                    class: "dx-dropdown-menu-item",
                    value: "Delete".to_string(),
                    index: 4usize,
                    on_select: move |value: String| {
                        selected_operation.set(Some(value));
                    },
                    "Delete"
                }
            }
        }
        if let Some(op) = selected_operation() {
            "Selected: {op}"
        }
    }
}
