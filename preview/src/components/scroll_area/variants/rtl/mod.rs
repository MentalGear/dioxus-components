use super::super::component::*;
use dioxus::prelude::*;
use dioxus_primitives::direction::{Direction, DirectionProvider};
use dioxus_primitives::scroll_area::ScrollDirection;

/// RTL fixture (dev-docs/backlog.md row 13) -- see `tabs/variants/rtl/mod.rs`'s
/// doc for the general shape. `ScrollArea` wraps the browser's own native
/// scrollbar (no custom-drawn thumb), so this fixture exists to prove the
/// `dir`/`data-direction` attribute-presence contract
/// (`playwright/oracle/tier3-radix/rtl.spec.ts`) -- the physical
/// scrollbar-side repositioning itself is native UA behavior, not
/// something a Rust-level assertion can observe portably.
#[component]
pub fn Demo() -> Element {
    rsx! {
        div { dir: "rtl",
            DirectionProvider { direction: Direction::Rtl,
                ScrollArea {
                    width: "10em",
                    height: "10em",
                    border: "1px solid var(--primary-color-6)",
                    border_radius: "0.5em",
                    padding: "0 1em 1em 1em",
                    direction: ScrollDirection::Vertical,
                    tabindex: "0",
                    div {
                        for i in 1..=20 {
                            p { "Scrollable content item {i}" }
                        }
                    }
                }
            }
        }
    }
}
