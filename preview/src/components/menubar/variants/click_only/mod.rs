use super::super::component::*;
use dioxus::prelude::*;

/// Click-activation fixture (`open_on_hover: false`).
///
/// Once a menu is open, moving the pointer across the other triggers does
/// **not** switch menus -- another menu opens only on click, Enter/Space or
/// the arrow keys. (The default `main` variant hover-switches.)
///
/// Every label below ("Insert"/"Tools"/"Table"/"Image"/"Spelling"/
/// "Statistics") deliberately differs from the `main` variant's
/// ("File"/"Edit"/"New"/"Open"/"Save"/"Cut"/"Copy"/"Paste"):
/// `menubar.spec.ts` and the oracle `keyboard-matrix`/`menu-roles` specs
/// query those words by name (substring, no `exact`), and this variant
/// renders alongside `main` on the same page, so reusing them would make
/// those locators match both.
#[component]
pub fn Demo() -> Element {
    rsx! {
        div {
            Menubar { open_on_hover: false,
                MenubarMenu { index: 0usize,
                    MenubarTrigger { "Insert" }
                    MenubarContent {
                        MenubarItem {
                            index: 0usize,
                            value: "table".to_string(),
                            on_select: move |value| {
                                tracing::info!("Selected value: {}", value);
                            },
                            "Table"
                        }
                        MenubarItem {
                            index: 1usize,
                            value: "image".to_string(),
                            on_select: move |value| {
                                tracing::info!("Selected value: {}", value);
                            },
                            "Image"
                        }
                    }
                }
                MenubarMenu { index: 1usize,
                    MenubarTrigger { "Tools" }
                    MenubarContent {
                        MenubarItem {
                            index: 0usize,
                            value: "spelling".to_string(),
                            on_select: move |value| {
                                tracing::info!("Selected value: {}", value);
                            },
                            "Spelling"
                        }
                        MenubarItem {
                            index: 1usize,
                            value: "statistics".to_string(),
                            on_select: move |value| {
                                tracing::info!("Selected value: {}", value);
                            },
                            "Statistics"
                        }
                    }
                }
            }
        }
    }
}
