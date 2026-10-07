use super::super::component::*;
use crate::Route;
use dioxus::prelude::*;

/// Click-activation fixture (`open_on_hover: false`).
///
/// Hovering a trigger neither opens its dropdown nor switches between
/// dropdowns, and the pointer leaving an open one does not close it: a
/// dropdown opens on click/Enter/ArrowDown/ArrowUp and closes on a second
/// click, Escape, or a press/focus outside it.
///
/// Every label below ("Click-only navigation"/"Layout"/"Overlays"/"Card"/
/// "Separator"/"Sheet"/"Drawer"/"Start page") deliberately differs from the
/// `main` variant's ("Example navigation"/"Inputs"/"Information"/
/// "Calendar"/"Home"/...):
/// `navbar.spec.ts` and `oracle/tier1-apg/keyboard-matrix.spec.ts` query those
/// words by name (substring, no `exact`) and this variant renders alongside
/// `main` on the same page, so reusing them would make those locators match
/// both.
#[component]
pub fn Demo() -> Element {
    rsx! {
        div {
            Navbar { aria_label: "Click-only navigation", open_on_hover: false,
                NavbarNav { index: 0usize,
                    NavbarTrigger { "Layout" }
                    NavbarContent {
                        NavbarItem {
                            index: 0usize,
                            value: "card".to_string(),
                            to: Route::component("card"),
                            "Card"
                        }
                        NavbarItem {
                            index: 1usize,
                            value: "separator".to_string(),
                            to: Route::component("separator"),
                            "Separator"
                        }
                    }
                }
                NavbarNav { index: 1usize,
                    NavbarTrigger { "Overlays" }
                    NavbarContent {
                        NavbarItem {
                            index: 0usize,
                            value: "sheet".to_string(),
                            to: Route::component("sheet"),
                            "Sheet"
                        }
                        NavbarItem {
                            index: 1usize,
                            value: "drawer".to_string(),
                            to: Route::component("drawer"),
                            "Drawer"
                        }
                    }
                }
                NavbarItem {
                    index: 2usize,
                    value: "start".to_string(),
                    to: Route::home(),
                    "Start page"
                }
            }
        }
    }
}
