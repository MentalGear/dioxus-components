use super::super::component::*;
use dioxus::prelude::*;
use dioxus_icons::lucide::{ChevronLeft, ChevronRight};
use dioxus_primitives::direction::{Direction, DirectionProvider};

/// RTL fixture (dev-docs/backlog.md row 13's own convention -- see
/// `tabs/variants/rtl/mod.rs`'s doc for the general shape). `ArrowLeft`
/// now moves to the *next* slide and `ArrowRight` to the *previous* one
/// (Radix's roving-focus convention -- see the primitive's own module
/// doc, "Accessibility"). Scrolling itself needs no RTL-specific
/// handling at all: slide order in the DOM never changes, and the
/// browser's own `scrollIntoView` already resolves the correct physical
/// position under `dir="rtl"`.
#[component]
pub fn Demo() -> Element {
    rsx! {
        div { dir: "rtl", style: "width: 100%; max-width: 20rem; margin: 0 auto;",
            DirectionProvider { direction: Direction::Rtl,
                Carousel { aria_label: "Right-to-left slideshow demo",
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
