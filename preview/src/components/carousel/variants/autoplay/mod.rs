use super::super::component::*;
use crate::components::card::{Card, CardContent};
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
                            Card {
                                CardContent {
                                    style: "display: flex; align-items: center; justify-content: center; aspect-ratio: 1; font-size: var(--dx-text-3xl); font-weight: 600;",
                                    "{i + 1}"
                                }
                            }
                        }
                    }
                }
            }
            // Deliberate divergence from shadcn's own Autoplay demo,
            // labeled rather than left silent: shadcn's `Autoplay` plugin
            // (`stopOnInteraction: true`, hover `stop()`/`reset()`) fully
            // stops the timer on hover and resets it to zero on leave. This
            // component instead follows the APG "auto-rotating carousel"
            // pattern's own accessibility-features prose: hovering (or
            // focusing) merely PAUSES rotation, which resumes with its
            // remaining time on un-hover (focus is sticky -- see
            // `CarouselAutoplayProps`' own doc for the full asymmetry) --
            // a pause a user can rely on to read a slide without losing
            // their place, not a restart.
            p {
                style: "margin: 0.5rem 0 0; text-align: center; font-size: 0.875rem; color: var(--secondary-color-3);",
                "Pauses (not stop-and-reset) on hover or focus, per the APG auto-rotating pattern."
            }
        }
    }
}
