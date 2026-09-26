use super::super::component::*;
use dioxus::prelude::*;
use dioxus_icons::lucide::{ChevronLeft, ChevronRight};
use dioxus_primitives::direction::{Direction, DirectionProvider};

/// `virtual_loop` under `dir="rtl"` -- closes the one integration gap that
/// variant and the plain `rtl` variant don't each cover on their own: RTL's
/// own key-swap composed with the seamless virtual loop's own physical
/// paging. See `rtl/mod.rs`'s own doc for the general RTL-fixture shape
/// this follows.
#[component]
pub fn Demo() -> Element {
    let items: Vec<String> = (0..12).map(|i| format!("{}", i + 1)).collect();
    rsx! {
        // See `variants/main/mod.rs`'s own comment for the `26rem` derivation.
        div { dir: "rtl", style: "width: 100%; max-width: 26rem; margin: 0 auto;",
            DirectionProvider { direction: Direction::Rtl,
                Carousel { aria_label: "Seamless looping gallery (RTL)", r#loop: true,
                    CarouselAutoplay { delay_ms: 1200u64 }
                    CarouselPrevious { ChevronLeft {} }
                    CarouselNext { ChevronRight {} }
                    CarouselVirtualContent::<String> {
                        items,
                        render_item: move |(_idx, label): (usize, String)| rsx! {
                            div {
                                style: "display: flex; align-items: center; justify-content: center; height: 12rem; border: 1px solid var(--primary-color-6); border-radius: var(--dx-radius-lg); font-size: 2rem;",
                                "{label}"
                            }
                        },
                    }
                    CarouselIndicators {
                        for i in 0..12usize {
                            CarouselIndicator { key: "{i}", index: i }
                        }
                    }
                }
            }
        }
    }
}
