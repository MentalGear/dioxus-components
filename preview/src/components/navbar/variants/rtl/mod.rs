use super::super::component::*;
use crate::Route;
use dioxus::prelude::*;
use dioxus_primitives::direction::{Direction, DirectionProvider};

/// RTL fixture (dev-docs/backlog.md row 13) -- see `tabs/variants/rtl/mod.rs`'s
/// doc for the general shape, including why the labels below
/// ("Widgets"/"Details"/"Schedule"/"Range"/"Notice"/"Start") deliberately
/// differ from the `main` variant's ("Inputs"/"Information"/"Calendar"/
/// "Slider"/"Toast"/"Home"): `navbar.spec.ts` and
/// `oracle/tier1-apg/keyboard-matrix.spec.ts` both query these by name
/// (e.g. `getByRole('menuitem', { name: 'Inputs' })`, no `exact`), and
/// this variant renders alongside `main` on the same page
/// (`preview/src/main.rs`'s `ComponentHighlight`), so reusing those words
/// would make such a locator match both. `Navbar`'s top-level trigger row
/// is unconditionally horizontal, matching `Menubar`'s identical shape.
#[component]
pub fn Demo() -> Element {
    rsx! {
        div { dir: "rtl",
            DirectionProvider { direction: Direction::Rtl,
                Navbar { aria_label: "RTL navigation",
                    NavbarNav { index: 0usize,
                        NavbarTrigger { "Widgets" }
                        NavbarContent {
                            NavbarItem {
                                index: 0usize,
                                value: "schedule".to_string(),
                                to: Route::component("calendar"),
                                "Schedule"
                            }
                            NavbarItem {
                                index: 1usize,
                                value: "range".to_string(),
                                to: Route::component("slider"),
                                "Range"
                            }
                        }
                    }
                    NavbarNav { index: 1usize,
                        NavbarTrigger { "Details" }
                        NavbarContent {
                            NavbarItem {
                                index: 0usize,
                                value: "notice".to_string(),
                                to: Route::component("toast"),
                                "Notice"
                            }
                        }
                    }
                    NavbarItem {
                        index: 2usize,
                        value: "start".to_string(),
                        to: Route::home(),
                        "Start"
                    }
                }
            }
        }
    }
}
