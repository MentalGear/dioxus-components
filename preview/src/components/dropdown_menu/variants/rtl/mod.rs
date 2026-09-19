use super::super::component::{
    DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSub, DropdownMenuSubContent,
    DropdownMenuSubItem, DropdownMenuSubTrigger, DropdownMenuTrigger,
};
use dioxus::prelude::*;
use dioxus_primitives::direction::{Direction, DirectionProvider};

/// RTL fixture (dev-docs/backlog.md row 13) -- see `tabs/variants/rtl/mod.rs`'s
/// doc for the general shape. Includes a [`DropdownMenuSub`] (index 1,
/// between "Modify" and "Erase") so `playwright/oracle/tier3-radix/rtl.spec.ts`
/// can exercise the submenu's direction-aware open/close arrow key
/// (`crate::menu_sub::is_submenu_open_arrow_key`/`is_submenu_close_arrow_key`)
/// and its `ContentSide::Left` flip.
///
/// Every label below ("Launch Menu"/"Modify"/"Extra tools"/"Relabel"/
/// "Stash"/"Erase") deliberately differs from the `main` variant's
/// ("Open Menu"/"Edit"/"More tools"/"Rename"/"Archive"/"Delete"):
/// `dropdown-menu.spec.ts` and the shared
/// `oracle/tier1-apg/{keyboard-matrix,menu-roles,menu-submenu}.spec.ts`
/// all query those words by name (no `exact`), and this variant renders
/// alongside `main` on the same page (`preview/src/main.rs`'s
/// `ComponentHighlight`), so reusing them -- even as a substring -- would
/// make such a locator match both. Confirmed live: before this rename,
/// every one of those files' `Open Menu`/`Edit`/`More tools`/`Rename`
/// locators threw a strict-mode "resolved to 2 elements" error the moment
/// this fixture was registered. Distinct words side-step it without
/// touching any of those spec files, which this lane's brief puts out of
/// scope.
#[component]
pub fn Demo() -> Element {
    let mut selected = use_signal(|| None);

    rsx! {
        div { dir: "rtl",
            DirectionProvider { direction: Direction::Rtl,
                DropdownMenu { default_open: false,
                    DropdownMenuTrigger { "Launch Menu" }
                    DropdownMenuContent {
                        DropdownMenuItem::<String> {
                            value: "modify".to_string(),
                            index: 0usize,
                            on_select: move |v| selected.set(Some(v)),
                            "Modify"
                        }
                        DropdownMenuSub {
                            DropdownMenuSubTrigger { index: 1usize, "Extra tools" }
                            DropdownMenuSubContent {
                                DropdownMenuSubItem::<String> {
                                    value: "relabel".to_string(),
                                    index: 0usize,
                                    on_select: move |v| selected.set(Some(v)),
                                    "Relabel"
                                }
                                DropdownMenuSubItem::<String> {
                                    value: "stash".to_string(),
                                    index: 1usize,
                                    on_select: move |v| selected.set(Some(v)),
                                    "Stash"
                                }
                            }
                        }
                        DropdownMenuItem::<String> {
                            value: "erase".to_string(),
                            index: 2usize,
                            on_select: move |v| selected.set(Some(v)),
                            "Erase"
                        }
                    }
                }
                if let Some(op) = selected() {
                    "Selected: {op}"
                }
            }
        }
    }
}
