use super::super::component::*;
use dioxus::prelude::*;
use dioxus_primitives::direction::{Direction, DirectionProvider};

/// RTL fixture (dev-docs/backlog.md row 13) -- see `tabs/variants/rtl/mod.rs`'s
/// doc for the general shape. A fixed pixel `width` (rather than the
/// `main` variant's default sizing) so
/// `playwright/oracle/tier3-radix/rtl.spec.ts` can click at a precise
/// fraction of the track and assert the exact value Radix's own
/// `getValueFromPointer` would produce for that position under RTL (a
/// click near the left end resolves to a value near `max`) --
/// see `primitives/src/slider.rs`'s `mirror_offset` doc for the citation.
#[component]
pub fn Demo() -> Element {
    let mut current_value = use_signal(|| 50.0);

    rsx! {
        div { dir: "rtl",
            DirectionProvider { direction: Direction::Rtl,
                div {
                    display: "flex",
                    flex_direction: "column",
                    align_items: "center",
                    width: "300px",

                    div { style: "margin-bottom: 15px; font-size: 16px; font-weight: bold;",
                        "{current_value:.0}%"
                    }

                    Slider {
                        label: "RTL Slider",
                        horizontal: true,
                        min: 0.0,
                        max: 100.0,
                        step: 1.0,
                        default_value: 50.0,
                        on_value_change: move |value: f64| {
                            current_value.set(value);
                        },
                    }
                }
            }
        }
    }
}
