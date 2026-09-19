use super::super::component::*;
use dioxus::prelude::*;
use dioxus_primitives::direction::{Direction, DirectionProvider};

fn panel_content(label: &'static str) -> Element {
    rsx! {
        div {
            style: "display: flex; align-items: center; justify-content: center; height: 100%; padding: 1rem; box-sizing: border-box;",
            "{label}"
        }
    }
}

/// RTL fixture (dev-docs/backlog.md row 13) -- see `tabs/variants/rtl/mod.rs`'s
/// doc for the general shape. A single horizontal group (no Radix/shadcn
/// original exists for Resizable -- see `primitives/src/resizable.rs`'s
/// own doc on `ResizableGroupContext::text_direction` for the
/// extrapolation this lane based the key-flip on). Deliberately horizontal
/// only: a vertical `ResizablePanelGroup` never consults direction at all.
#[component]
pub fn Demo() -> Element {
    rsx! {
        div { dir: "rtl",
            DirectionProvider { direction: Direction::Rtl,
                ResizablePanelGroup {
                    direction: ResizableDirection::Horizontal,
                    height: "10rem",
                    border: "1px solid var(--secondary-color-3)",
                    border_radius: "0.5rem",
                    overflow: "hidden",

                    ResizablePanel { index: 0usize, default_size: 50.0, min_size: 20.0,
                        {panel_content("Left")}
                    }
                    ResizableHandle { index: 0usize, aria_label: "Divider" }
                    ResizablePanel { index: 1usize, default_size: 50.0, min_size: 20.0,
                        {panel_content("Right")}
                    }
                }
            }
        }
    }
}
