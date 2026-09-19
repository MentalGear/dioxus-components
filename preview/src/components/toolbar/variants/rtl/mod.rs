use super::super::component::*;
use dioxus::prelude::*;
use dioxus_primitives::direction::{Direction, DirectionProvider};

/// RTL fixture (dev-docs/backlog.md row 13) -- see `tabs/variants/rtl/mod.rs`'s
/// doc for the general shape. Simplified from the `main` variant's
/// bold/italic/underline/align demo to just the roving-focus row itself,
/// since only `ToolbarButton`'s `ArrowLeft`/`ArrowRight` handling is under
/// test here.
#[component]
pub fn Demo() -> Element {
    rsx! {
        div { dir: "rtl",
            DirectionProvider { direction: Direction::Rtl,
                Toolbar { aria_label: "RTL toolbar",
                    ToolbarButton { index: 0usize, "One" }
                    ToolbarButton { index: 1usize, "Two" }
                    ToolbarButton { index: 2usize, "Three" }
                }
            }
        }
    }
}
