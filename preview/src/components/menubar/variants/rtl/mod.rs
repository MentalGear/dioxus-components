use super::super::component::*;
use dioxus::prelude::*;
use dioxus_primitives::direction::{Direction, DirectionProvider};

/// RTL fixture (dev-docs/backlog.md row 13) -- see `tabs/variants/rtl/mod.rs`'s
/// doc for the general shape, including why the labels below
/// ("Options"/"Modify"/"Create"/"Launch"/"Remove"/"Duplicate") deliberately
/// differ from the `main` variant's ("File"/"Edit"/"New"/"Open"/"Cut"/
/// "Copy"): `menubar.spec.ts` and
/// `oracle/tier1-apg/keyboard-matrix.spec.ts` both query these by name
/// (e.g. `getByRole("menuitem", { name: "File" })`, no `exact`), and this
/// variant renders alongside `main` on the same page (`preview/src/
/// main.rs`'s `ComponentHighlight`), so reusing those words -- even as a
/// substring -- would make such a locator match both. `MenubarMenu`'s
/// top-level trigger row is unconditionally horizontal, so its
/// `ArrowLeft`/`ArrowRight` roving focus between "Options"/"Modify" always
/// resolves this ambient RTL direction (no `horizontal` prop exists on
/// `Menubar` to set).
#[component]
pub fn Demo() -> Element {
    rsx! {
        div { dir: "rtl",
            DirectionProvider { direction: Direction::Rtl,
                Menubar {
                    MenubarMenu { index: 0usize,
                        MenubarTrigger { "Options" }
                        MenubarContent {
                            MenubarItem {
                                index: 0usize,
                                value: "create".to_string(),
                                on_select: move |_| {},
                                "Create"
                            }
                            MenubarItem {
                                index: 1usize,
                                value: "launch".to_string(),
                                on_select: move |_| {},
                                "Launch"
                            }
                        }
                    }
                    MenubarMenu { index: 1usize,
                        MenubarTrigger { "Modify" }
                        MenubarContent {
                            MenubarItem {
                                index: 0usize,
                                value: "remove".to_string(),
                                on_select: move |_| {},
                                "Remove"
                            }
                            MenubarItem {
                                index: 1usize,
                                value: "duplicate".to_string(),
                                on_select: move |_| {},
                                "Duplicate"
                            }
                        }
                    }
                }
            }
        }
    }
}
