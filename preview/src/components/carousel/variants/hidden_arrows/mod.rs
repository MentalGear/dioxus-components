use super::super::component::*;
use crate::components::card::{Card, CardContent};
use dioxus::prelude::*;
use dioxus_icons::lucide::{ChevronLeft, ChevronRight};

/// `nav_disabled_opacity: 0.0` on the root: the arrow at the first/last slide
/// fades out completely instead of dimming to the theme's 0.5, and is then
/// hidden from assistive technology (`visibility: hidden` -- the box stays,
/// so nothing moves when it goes). One setting covers both arrows.
///
/// One carousel on purpose. Every variant of a component renders on the same
/// page (`ComponentVariantHighlight`), so each carousel added here is hydrated
/// by every carousel test and by every visitor of the page. The option is two
/// inline custom properties on the root, so it behaves the same under RTL and
/// vertical layouts; `playwright/carousel-nav-opacity.spec.ts` checks both by
/// setting those properties on the existing `rtl` and `vertical` demos. See
/// docs.md's "Disabled arrows" section for the stylesheet-only form
/// (`--dx-carousel-nav-disabled-opacity`).
#[component]
pub fn Demo() -> Element {
    rsx! {
        div { style: "width: 100%; max-width: 26rem; margin: 0 auto;",
            Carousel { aria_label: "Gallery with hidden arrows", nav_disabled_opacity: 0.0,
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
        }
    }
}
