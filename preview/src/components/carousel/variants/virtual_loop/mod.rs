use super::super::component::*;
use crate::components::card::{Card, CardContent};
use dioxus::prelude::*;
use dioxus_icons::lucide::{ChevronLeft, ChevronRight};

/// `CarouselVirtualContent` with `loop: true` and a small data set (12
/// items) -- the seamless-loop path, and this component's own canonical
/// loop demo (the plain-children `looping`/`looping_rtl` demos were
/// retired -- see `component.rs`'s own doc on `LoopMode`; a
/// `LoopMode::Rewind` opt-in on the plain children API has no dedicated
/// demo of its own, since it is exactly the same visible behavior those
/// retired demos already showed, now behind an explicit prop instead of
/// `r#loop: true` alone). Paging past the last slide here slides
/// physically forward one already-mounted window slide at a time (never a
/// visible rewind across every intervening slide), and paging back past
/// the first slides physically backward -- see
/// `primitives/src/carousel.rs`'s own `CarouselVirtualContent` doc,
/// "Seamless loop," for the construction. `radius: 2` (the default) keeps
/// at most 5 slides mounted at once regardless of the 12-item data set --
/// `playwright/carousel.spec.ts` asserts this DOM-node-count bound
/// directly. See `virtual_loop_rtl` for the identical demo under RTL.
///
/// Also carries `CarouselAutoplay` (so the same seamless wrap is exercised
/// by an automatic tick, not only by explicit Next/Prev/keyboard) and
/// `CarouselIndicators` (the APG tablist picker, composed exactly the way
/// the `indicators` variant already does for the plain children API --
/// proof this data-driven root is a drop-in for every consumer of
/// `CarouselContext`, not just Previous/Next: `CarouselIndicator`'s own
/// `onfocus` calls the identical `carousel_ctx.set_selected`
/// `CarouselVirtualContent`'s own re-anchor effect already watches,
/// regardless of what changed it).
#[component]
pub fn Demo() -> Element {
    let items: Vec<String> = (0..12).map(|i| format!("{}", i + 1)).collect();
    rsx! {
        // See `variants/main/mod.rs`'s own comment for the `26rem` derivation.
        div { style: "width: 100%; max-width: 26rem; margin: 0 auto;",
            Carousel { aria_label: "Seamless looping gallery", r#loop: true,
                CarouselAutoplay { delay_ms: 1200u64 }
                CarouselPrevious { ChevronLeft {} }
                CarouselNext { ChevronRight {} }
                CarouselVirtualContent::<String> {
                    items,
                    render_item: move |(_idx, label): (usize, String)| rsx! {
                        Card {
                            CardContent {
                                style: "display: flex; align-items: center; justify-content: center; aspect-ratio: 1; font-size: var(--dx-text-3xl); font-weight: 600;",
                                "{label}"
                            }
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
