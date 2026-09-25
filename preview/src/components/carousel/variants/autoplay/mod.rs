use super::super::component::*;
use dioxus::prelude::*;
use dioxus_icons::lucide::{ChevronLeft, ChevronRight, Play};

/// The APG "auto-rotating" carousel: `CarouselRotationControl` is placed
/// **first**, before `CarouselPrevious`/`CarouselNext`, so it is the first
/// focusable element in the carousel (APG's own explicit requirement). Its
/// icon is deliberately static/decorative -- the actual play/stop state is
/// carried by the button's own toggling accessible name ("Start automatic
/// slide show"/"Stop automatic slide show"), not duplicated into a second,
/// demo-local signal that could drift from it (e.g. once autoplay
/// self-stops at the last slide without `loop`, nothing here would learn
/// that to flip a local icon).
///
/// `delay_ms: 1200` is shorter than `CarouselAutoplayProps::delay_ms`'s own
/// default (4000ms) purely so this demo (and the Playwright tests that
/// drive it) don't need to wait as long per tick -- a real consumer would
/// typically leave it at the default or pick something slower.
#[component]
pub fn Demo() -> Element {
    rsx! {
        // See `variants/main/mod.rs`'s own comment for the `26rem` derivation.
        div { style: "width: 100%; max-width: 26rem; margin: 0 auto;",
            Carousel { aria_label: "Autoplay gallery",
                CarouselRotationControl { Play {} }
                CarouselAutoplay { delay_ms: 1200u64 }
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
