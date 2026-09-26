use super::super::component::*;
use crate::components::card::{Card, CardContent};
use dioxus::prelude::*;
use dioxus_icons::lucide::{ChevronLeft, ChevronRight};

/// An explicit, opt-in "peek" layout: `--dx-carousel-peek` shows a sliver
/// of the *next* slide alongside the current one; `--dx-carousel-per-view`
/// stays at its default (one whole slide). Not shadcn parity -- shadcn's
/// own `base` docs have no separate peek example at all
/// (`dev-docs/research/shadcn-carousel-parity.md`'s own finding: "Sizes
/// *is* their multi-slide demo, and it always shows whole slides") -- this
/// demo exists so the feature is demonstrated somewhere, clearly labeled
/// as an opt-in rather than the default (see docs.md's own "Sizes"
/// section).
#[component]
pub fn Demo() -> Element {
    rsx! {
        div { style: "width: 100%; max-width: 20rem; margin: 0 auto;",
            Carousel { aria_label: "Peek gallery",
                CarouselPrevious { ChevronLeft {} }
                CarouselNext { ChevronRight {} }
                CarouselContent { style: "--dx-carousel-peek: 20%;",
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
        }
    }
}
