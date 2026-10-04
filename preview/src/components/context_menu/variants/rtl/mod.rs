use super::super::component::*;
use dioxus::prelude::*;
use dioxus_primitives::direction::{Direction, DirectionProvider};

/// RTL fixture (dev-docs/backlog.md row 13) -- see
/// `dropdown_menu/variants/rtl/mod.rs`'s doc for why this includes a
/// [`ContextMenuSub`] and why every label below ("RTL context zone"/
/// "Modify"/"Extra tools"/"Relabel"/"Stash"/"Erase") deliberately differs
/// from the `main` variant's ("right click here"/"Edit"/"More tools"/
/// "Rename"/"Archive"/"Delete") -- `context-menu.spec.ts` and the shared
/// `oracle/tier1-apg/{keyboard-matrix,menu-roles,menu-submenu}.spec.ts`
/// all query those words by name, and this variant renders alongside
/// `main` on the same page.
///
/// A checkbox item and a radio group ("Pinned"/"Ascending"/"Descending") are
/// appended after "Erase" (indices 3-5) so the checkable indicator slot is
/// exercised in RTL too -- see `playwright/menu-indicator-gap.spec.ts`.
#[component]
pub fn Demo() -> Element {
    let mut selected = use_signal(|| None);
    let mut pinned = use_signal(|| true);
    let mut order = use_signal(|| "ascending".to_string());

    rsx! {
        div { dir: "rtl",
            DirectionProvider { direction: Direction::Rtl,
                ContextMenu {
                    ContextMenuTrigger { "RTL context zone" }
                    ContextMenuContent {
                        ContextMenuItem {
                            value: "modify".to_string(),
                            index: 0usize,
                            on_select: move |v| selected.set(Some(v)),
                            "Modify"
                        }
                        ContextMenuSub {
                            ContextMenuSubTrigger { index: 1usize, "Extra tools" }
                            ContextMenuSubContent {
                                ContextMenuSubItem {
                                    value: "relabel".to_string(),
                                    index: 0usize,
                                    on_select: move |v| selected.set(Some(v)),
                                    "Relabel"
                                }
                                ContextMenuSubItem {
                                    value: "stash".to_string(),
                                    index: 1usize,
                                    on_select: move |v| selected.set(Some(v)),
                                    "Stash"
                                }
                            }
                        }
                        ContextMenuItem {
                            value: "erase".to_string(),
                            index: 2usize,
                            on_select: move |v| selected.set(Some(v)),
                            "Erase"
                        }
                        ContextMenuCheckboxItem {
                            index: 3usize,
                            checked: Some(pinned()),
                            on_checked_change: move |checked| pinned.set(checked),
                            text_value: "Pinned",
                            "Pinned"
                        }
                        ContextMenuRadioGroup {
                            aria_label: "Order",
                            value: Some(order()),
                            on_value_change: move |value| order.set(value),
                            ContextMenuRadioItem {
                                index: 4usize,
                                value: "ascending".to_string(),
                                "Ascending"
                            }
                            ContextMenuRadioItem {
                                index: 5usize,
                                value: "descending".to_string(),
                                "Descending"
                            }
                        }
                    }
                }
                if let Some(item) = selected() {
                    span { margin_left: "10px", "Selected: {item}" }
                }
            }
        }
    }
}
