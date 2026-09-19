use super::super::component::*;
use dioxus::prelude::*;
use dioxus_primitives::direction::{Direction, DirectionProvider};

/// RTL fixture (dev-docs/backlog.md row 13): wraps the same shape as the
/// `main` variant in a [`DirectionProvider`] (`Direction::Rtl`) and sets
/// `dir="rtl"` on this page's own root -- `Tabs`'s own root then resolves
/// that ambient direction (no local `dir` prop override needed) and
/// applies it to `TabTrigger`'s `ArrowLeft`/`ArrowRight` roving focus and
/// its own `dir`/`data-direction` output. See
/// `playwright/oracle/tier3-radix/rtl.spec.ts` for the keyboard-flip
/// contract this fixture exists to be tested against.
///
/// Labelled "One"/"Two"/"Three" rather than reusing the `main` variant's
/// "Tab 1"/"Tab 2"/"Tab 3" (every component page renders every one of its
/// variants at once -- `preview/src/main.rs`'s `ComponentHighlight` --  so
/// identical labels make `main`'s own pre-existing `tabs.spec.ts` locators
/// ambiguous the moment a second variant exists; confirmed live: adding
/// this fixture with the `main` labels made every `tabs.spec.ts` assertion
/// that queries `getByRole("tab", { name: "Tab 2" })` (no `exact`, so a
/// substring match too) throw a strict-mode "resolved to 2 elements"
/// error). Distinct words side-step it without touching that spec file,
/// which this lane's brief puts out of scope.
#[component]
pub fn Demo() -> Element {
    rsx! {
        div { dir: "rtl",
            DirectionProvider { direction: Direction::Rtl,
                Tabs {
                    default_value: "tab1".to_string(),
                    horizontal: true,
                    max_width: "16rem",
                    TabList {
                        TabTrigger { value: "tab1".to_string(), index: 0usize, "One" }
                        TabTrigger { value: "tab2".to_string(), index: 1usize, "Two" }
                        TabTrigger { value: "tab3".to_string(), index: 2usize, "Three" }
                    }
                    TabContent { index: 0usize, value: "tab1".to_string(),
                        div {
                            width: "100%",
                            height: "5rem",
                            display: "flex",
                            align_items: "center",
                            justify_content: "center",
                            "One Content"
                        }
                    }
                    TabContent { index: 1usize, value: "tab2".to_string(),
                        div {
                            width: "100%",
                            height: "5rem",
                            display: "flex",
                            align_items: "center",
                            justify_content: "center",
                            "Two Content"
                        }
                    }
                    TabContent { index: 2usize, value: "tab3".to_string(),
                        div {
                            width: "100%",
                            height: "5rem",
                            display: "flex",
                            align_items: "center",
                            justify_content: "center",
                            "Three Content"
                        }
                    }
                }
            }
        }
    }
}
