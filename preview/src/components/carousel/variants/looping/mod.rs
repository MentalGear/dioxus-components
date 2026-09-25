use super::super::component::*;
use dioxus::prelude::*;
use dioxus_icons::lucide::{ChevronLeft, ChevronRight};

/// `loop: true` -- rewind-style wraparound (approved fast-follow decision,
/// backlog row 91): Previous at the first slide goes to the last, Next at
/// the last goes to the first, and neither button is ever `disabled`. See
/// `primitives/src/carousel.rs`'s own `CarouselProps::r#loop` doc for why
/// this is a visible "rewind" (a real `scrollIntoView` all the way across
/// the intervening slides) rather than an embla-style seamless illusion --
/// no cloned edge slides are ever added.
#[component]
pub fn Demo() -> Element {
    rsx! {
        // See `variants/main/mod.rs`'s own comment for the `26rem` derivation.
        div { style: "width: 100%; max-width: 26rem; margin: 0 auto;",
            Carousel { aria_label: "Looping photo gallery", r#loop: true,
                CarouselPrevious { ChevronLeft {} }
                CarouselNext { ChevronRight {} }
                CarouselContent {
                    for i in 0..5usize {
                        CarouselItem { key: "{i}", index: i,
                            div {
                                style: "display: flex; align-items: center; justify-content: center; height: 12rem; border: 1px solid var(--primary-color-6); border-radius: var(--dx-radius-lg); font-size: 2rem;",
                                "{i + 1}"
                            }
                        }
                    }
                }
            }
        }
    }
}
