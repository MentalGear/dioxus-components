use super::super::component::{
    DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSub, DropdownMenuSubContent,
    DropdownMenuSubItem, DropdownMenuSubTrigger, DropdownMenuTrigger,
};
use dioxus::prelude::*;

/// Click-activation submenu fixture (`DropdownMenuSub { open_on_hover: false }`).
///
/// Hovering "Export as" does nothing -- the submenu opens only on click,
/// Enter/Space or the open arrow key (ArrowRight in LTR), and moving the
/// pointer off it does not close it. (The default `main` variant's "More
/// tools" submenu opens on hover.)
///
/// Every label below ("Actions"/"Share"/"Export as"/"PDF document"/"Plain
/// text"/"Print") deliberately differs from the `main` variant's ("Open
/// Menu"/"Edit"/"More tools"/"Rename"/"Archive"/"Delete"...):
/// `dropdown-menu.spec.ts` and the shared oracle `keyboard-matrix`/
/// `menu-roles`/`menu-submenu` specs query those words by name (substring,
/// no `exact`), and this variant renders alongside `main` on the same page,
/// so reusing them would make those locators match both.
#[component]
pub fn Demo() -> Element {
    let mut selected = use_signal(|| None);

    rsx! {
        DropdownMenu { default_open: false,
            DropdownMenuTrigger { "Actions" }
            DropdownMenuContent {
                DropdownMenuItem::<String> {
                    value: "share".to_string(),
                    index: 0usize,
                    on_select: move |v| selected.set(Some(v)),
                    "Share"
                }
                DropdownMenuSub { open_on_hover: false,
                    DropdownMenuSubTrigger { index: 1usize, "Export as" }
                    DropdownMenuSubContent {
                        DropdownMenuSubItem::<String> {
                            value: "pdf".to_string(),
                            index: 0usize,
                            on_select: move |v| selected.set(Some(v)),
                            "PDF document"
                        }
                        DropdownMenuSubItem::<String> {
                            value: "text".to_string(),
                            index: 1usize,
                            on_select: move |v| selected.set(Some(v)),
                            "Plain text"
                        }
                    }
                }
                DropdownMenuItem::<String> {
                    value: "print".to_string(),
                    index: 2usize,
                    on_select: move |v| selected.set(Some(v)),
                    "Print"
                }
            }
        }
        if let Some(op) = selected() {
            "Chosen: {op}"
        }
    }
}
