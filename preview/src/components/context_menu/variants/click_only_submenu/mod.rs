use super::super::component::*;
use dioxus::prelude::*;

/// Click-activation submenu fixture (`ContextMenuSub { open_on_hover: false }`).
///
/// Hovering "Export as" does nothing -- the submenu opens only on click,
/// Enter/Space or the open arrow key (ArrowRight in LTR), and moving the
/// pointer off it does not close it. (The default `main` variant's "More
/// tools" submenu opens on hover.)
///
/// Every label below ("secondary-click this zone"/"Share"/"Export as"/"PDF
/// document"/"Plain text"/"Print") deliberately differs from the `main`
/// variant's ("right click here"/"Edit"/"More tools"/"Rename"/"Archive"/
/// "Delete"...): `context-menu.spec.ts` and the shared oracle
/// `keyboard-matrix`/`menu-roles`/`menu-submenu` specs query those words by
/// name (substring, no `exact`), and this variant renders alongside `main`
/// on the same page, so reusing them would make those locators match both.
#[component]
pub fn Demo() -> Element {
    let mut selected = use_signal(|| None);

    rsx! {
        ContextMenu {
            ContextMenuTrigger { "secondary-click this zone" }
            ContextMenuContent {
                ContextMenuItem {
                    value: "share".to_string(),
                    index: 0usize,
                    on_select: move |v| selected.set(Some(v)),
                    "Share"
                }
                ContextMenuSub { open_on_hover: false,
                    ContextMenuSubTrigger { index: 1usize, "Export as" }
                    ContextMenuSubContent {
                        ContextMenuSubItem {
                            value: "pdf".to_string(),
                            index: 0usize,
                            on_select: move |v| selected.set(Some(v)),
                            "PDF document"
                        }
                        ContextMenuSubItem {
                            value: "text".to_string(),
                            index: 1usize,
                            on_select: move |v| selected.set(Some(v)),
                            "Plain text"
                        }
                    }
                }
                ContextMenuItem {
                    value: "print".to_string(),
                    index: 2usize,
                    on_select: move |v| selected.set(Some(v)),
                    "Print"
                }
            }
        }
        if let Some(item) = selected() {
            span { margin_left: "10px", "Chosen: {item}" }
        }
    }
}
