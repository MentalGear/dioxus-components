use super::super::component::*;
use dioxus::prelude::*;

// Index layout (docs/component-backlog.md row 68 added the submenu):
// Edit=0, Undo=1 (disabled), Duplicate=2, Delete=3, the submenu trigger=4.
// `playwright/context-menu.spec.ts`'s "keyboard navigation" test
// hard-depends on this demo's existing shape -- Edit first, Undo disabled
// (so a 2nd ArrowDown from Edit skips it and lands on Duplicate) -- so
// those first four items are left exactly as they were; the submenu is
// added on at index 4 rather than interleaved (unlike `dropdown_menu`'s
// demo, no context_menu spec depends on which item Home/End treat as
// first/last, so there is no ordering constraint left to satisfy there).
#[component]
pub fn Demo() -> Element {
    let mut selected_item = use_signal(|| None);

    rsx! {
        ContextMenu {
            ContextMenuTrigger { "right click here" }
            ContextMenuContent {
                ContextMenuItem {
                    value: "edit".to_string(),
                    index: 0usize,
                    on_select: move |value| {
                        selected_item.set(Some(value));
                    },
                    "Edit"
                }
                ContextMenuItem {
                    value: "undo".to_string(),
                    index: 1usize,
                    disabled: true,
                    on_select: move |value| {
                        selected_item.set(Some(value));
                    },
                    "Undo"
                }
                ContextMenuItem {
                    value: "duplicate".to_string(),
                    index: 2usize,
                    on_select: move |value| {
                        selected_item.set(Some(value));
                    },
                    "Duplicate"
                }
                ContextMenuItem {
                    value: "delete".to_string(),
                    index: 3usize,
                    on_select: move |value| {
                        selected_item.set(Some(value));
                    },
                    "Delete"
                }
                // Nested submenu (docs/component-backlog.md row 68): see
                // this file's top-of-file comment for why this is appended
                // at index 4 rather than interleaved with the items above.
                ContextMenuSub {
                    ContextMenuSubTrigger { index: 4usize, "More tools" }
                    ContextMenuSubContent {
                        ContextMenuSubItem {
                            value: "rename".to_string(),
                            index: 0usize,
                            on_select: move |value| {
                                selected_item.set(Some(value));
                            },
                            "Rename"
                        }
                        ContextMenuSubItem {
                            value: "archive".to_string(),
                            index: 1usize,
                            on_select: move |value| {
                                selected_item.set(Some(value));
                            },
                            "Archive"
                        }
                    }
                }
            }
        }

        if let Some(item) = selected_item() {
            span { margin_left: "10px", "Selected: {item}" }
        }
    }
}
