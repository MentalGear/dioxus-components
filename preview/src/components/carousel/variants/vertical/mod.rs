use super::super::component::*;
use dioxus::prelude::*;
use dioxus_icons::lucide::{ChevronDown, ChevronUp};

/// `CarouselOrientation::Vertical` fell out of the horizontal
/// construction for free (see `primitives/src/carousel.rs`'s module
/// doc), so it ships in v1 rather than being deferred. A vertical
/// carousel needs an explicit height on `CarouselContent` -- there is
/// nothing else to derive one from, the same way `ScrollArea` needs one.
#[component]
pub fn Demo() -> Element {
    rsx! {
        div { style: "max-width: 12rem; margin: 0 auto;",
            Carousel {
                aria_label: "Vertical scrolling demo",
                orientation: CarouselOrientation::Vertical,
                CarouselPrevious { ChevronUp {} }
                CarouselNext { ChevronDown {} }
                CarouselContent { style: "height: 12rem;",
                    for i in 0..4usize {
                        CarouselItem { key: "{i}", index: i,
                            div {
                                style: "display: flex; align-items: center; justify-content: center; height: 100%; border: 1px solid var(--primary-color-6); border-radius: var(--dx-radius-lg); font-size: 2rem;",
                                "{i + 1}"
                            }
                        }
                    }
                }
            }
        }
    }
}
