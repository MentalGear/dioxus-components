use super::super::component::*;
use dioxus::prelude::*;
use dioxus_icons::lucide::{ChevronLeft, ChevronRight};
use dioxus_primitives::direction::{Direction, DirectionProvider};

/// The explicit `LoopMode::Rewind` opt-in (see `LoopMode`'s own doc, and
/// docs.md's "Looping" section): `r#loop: true` alone (the default
/// `LoopMode::Seamless`) does nothing for the plain children API -- this
/// demo is what actually exercises the rewind behavior the retired
/// `looping`/`looping_rtl` demos used to show unconditionally. Unlike
/// those, the wrap now jumps **instantly** rather than animating smoothly
/// across every intervening slide (matching the APG reference's own
/// basic-style example) -- see `primitives/src/carousel.rs`'s own
/// `is_rewind_wrap` doc.
///
/// Two rows -- LTR (5 slides) and RTL (4 slides) -- cover both directions
/// and the RTL key-swap together, the same split the retired demos used
/// to have as two separate pages.
#[component]
pub fn Demo() -> Element {
    rsx! {
        div { style: "width: 100%; max-width: 26rem; margin: 0 auto; display: flex; flex-direction: column; gap: 1.5rem;",
            div {
                Carousel { aria_label: "Rewind-loop gallery", r#loop: true, loop_mode: LoopMode::Rewind,
                    CarouselPrevious { ChevronLeft {} }
                    CarouselNext { ChevronRight {} }
                    CarouselContent {
                        for i in 0..5usize {
                            CarouselItem { key: "{i}", index: i,
                                div {
                                    style: "display: flex; align-items: center; justify-content: center; height: 10rem; border: 1px solid var(--primary-color-6); border-radius: var(--dx-radius-lg); font-size: 2rem;",
                                    "{i + 1}"
                                }
                            }
                        }
                    }
                }
            }
            div { dir: "rtl",
                DirectionProvider { direction: Direction::Rtl,
                    Carousel {
                        aria_label: "Rewind-loop gallery (RTL)",
                        r#loop: true,
                        loop_mode: LoopMode::Rewind,
                        CarouselPrevious { ChevronLeft {} }
                        CarouselNext { ChevronRight {} }
                        CarouselContent {
                            for i in 0..4usize {
                                CarouselItem { key: "{i}", index: i,
                                    div {
                                        style: "display: flex; align-items: center; justify-content: center; height: 10rem; border: 1px solid var(--primary-color-6); border-radius: var(--dx-radius-lg); font-size: 2rem;",
                                        "{i + 1}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
