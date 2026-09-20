use super::super::component::*;
use dioxus::prelude::*;
use dioxus_icons::lucide::{ChevronLeft, ChevronRight};

/// A "peek"/multi-item-per-view layout: each `CarouselItem` overrides the
/// primitive's own default `flex-basis: 100%` down to 40% via a plain
/// inline `style`, so roughly 2.5 slides are visible at once. Sizing is a
/// CSS decision, not a prop -- see the component's own docs.md.
#[component]
pub fn Demo() -> Element {
    rsx! {
        div { style: "max-width: 28rem; margin: 0 auto;",
            Carousel { aria_label: "Product gallery",
                CarouselPrevious { ChevronLeft {} }
                CarouselNext { ChevronRight {} }
                CarouselContent {
                    for i in 0..8usize {
                        CarouselItem { key: "{i}", index: i, style: "flex-basis: 40%;",
                            div {
                                style: "display: flex; align-items: center; justify-content: center; height: 8rem; border: 1px solid var(--primary-color-6); border-radius: var(--dx-radius-lg); font-size: 1.5rem;",
                                "{i + 1}"
                            }
                        }
                    }
                }
            }
        }
    }
}
