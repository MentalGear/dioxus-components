use super::super::component::*;
use dioxus::prelude::*;
use dioxus_icons::lucide::{ChevronLeft, ChevronRight};

/// Slide 1's placeholder is a real `<button>` (the rest stay plain `div`s,
/// matching every other variant) rather than a second demo variant, so
/// this one page can prove both pointer-drag halves at once: dragging it
/// pages the carousel like any other point on the track, but a plain
/// click still activates the button -- see `playwright/carousel.spec.ts`'s
/// own drag describe block, which drives exactly this element.
#[component]
pub fn Demo() -> Element {
    let mut slide_one_clicked = use_signal(|| false);
    rsx! {
        div { style: "max-width: 20rem; margin: 0 auto;",
            Carousel { aria_label: "Featured photos",
                CarouselPrevious { ChevronLeft {} }
                CarouselNext { ChevronRight {} }
                CarouselContent {
                    for i in 0..5usize {
                        CarouselItem { key: "{i}", index: i,
                            if i == 0 {
                                button {
                                    r#type: "button",
                                    "data-testid": "carousel-slide-button",
                                    "data-clicked": slide_one_clicked(),
                                    style: "display: flex; align-items: center; justify-content: center; width: 100%; height: 12rem; border: 1px solid var(--primary-color-6); border-radius: var(--dx-radius-lg); font-size: 2rem; background: none; cursor: pointer; font: inherit; color: inherit;",
                                    onclick: move |_| slide_one_clicked.set(true),
                                    if slide_one_clicked() { "Clicked!" } else { "{i + 1}" }
                                }
                            } else {
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
}
