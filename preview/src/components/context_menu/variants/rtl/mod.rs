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
#[component]
pub fn Demo() -> Element {
    let mut selected = use_signal(|| None);

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
                    }
                }
                if let Some(item) = selected() {
                    span { margin_left: "10px", "Selected: {item}" }
                }
            }
        }
    }
}
