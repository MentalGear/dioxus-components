use super::super::component::*;
use dioxus::prelude::*;
use dioxus_icons::lucide::{ChevronLeft, ChevronRight};

/// `align` (`start` | `center` | `end`, default `start`): where each slide
/// rests against the scrollport. Not a shadcn demo of its own -- shadcn's
/// `opts={{ align }}` is Embla's own generic options bag, never
/// demonstrated at anything but `start` in any `base` example -- but it is
/// a real primitive option (`primitives/src/carousel.rs`'s own
/// `CarouselAlign`), so this demo exists to show all three side by side.
/// Each row uses `--dx-carousel-per-view: 2` and `--dx-carousel-peek: 20%`
/// -- with only whole slides and no leftover space, `start`/`center`/`end`
/// would all look identical, so this deliberately leaves a fractional
/// remainder for the alignment to actually be visible.
#[component]
pub fn Demo() -> Element {
    let rows = [
        ("Start (default)", CarouselAlign::Start),
        ("Center", CarouselAlign::Center),
        ("End", CarouselAlign::End),
    ];
    rsx! {
        div { style: "width: 100%; max-width: 26rem; margin: 0 auto; display: flex; flex-direction: column; gap: 1.5rem;",
            for (label , align) in rows {
                div { key: "{label}",
                    p {
                        style: "margin: 0 0 0.5rem; font-size: 0.875rem; color: var(--secondary-color-3);",
                        "{label}"
                    }
                    Carousel {
                        aria_label: "Alignment demo: {label}",
                        align,
                        default_value: 2usize,
                        CarouselPrevious { ChevronLeft {} }
                        CarouselNext { ChevronRight {} }
                        CarouselContent {
                            style: "--dx-carousel-per-view: 2; --dx-carousel-peek: 20%;",
                            for i in 0..5usize {
                                CarouselItem { key: "{i}", index: i,
                                    div {
                                        style: "display: flex; align-items: center; justify-content: center; height: 6rem; border: 1px solid var(--primary-color-6); border-radius: var(--dx-radius-lg); font-size: 1.5rem;",
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
}
