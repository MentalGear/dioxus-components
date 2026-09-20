use super::super::component::*;
use dioxus::prelude::*;
use dioxus_icons::lucide::{ChevronLeft, ChevronRight};

/// A custom picker (dot indicators) built entirely from `use_carousel()`'s
/// public `CarouselApi`, alongside the usual `CarouselPrevious`/`CarouselNext`
/// -- not a new primitive. See `CarouselIndicators` (`component.rs`).
#[component]
pub fn Demo() -> Element {
    rsx! {
        div { style: "max-width: 20rem; margin: 0 auto;",
            Carousel { aria_label: "Travel destinations",
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
                CarouselIndicators {}
            }
        }
    }
}
