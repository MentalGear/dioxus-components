use super::super::component::*;
use dioxus::prelude::*;
use dioxus_icons::lucide::{ChevronLeft, ChevronRight};

/// `CarouselVirtualContent` with `loop: true` and a small data set (12
/// items) -- the seamless-loop path. Unlike `looping`'s own
/// `CarouselItem`-based rewind (a real, visible scroll all the way across
/// the intervening slides), paging past the last slide here slides
/// physically forward one already-mounted window slide at a time, and
/// paging back past the first slides physically backward -- see
/// `primitives/src/carousel.rs`'s own `CarouselVirtualContent` doc,
/// "Seamless loop," for the construction. `radius: 2` (the default) keeps
/// at most 5 slides mounted at once regardless of the 12-item data set --
/// `playwright/carousel.spec.ts` asserts this DOM-node-count bound
/// directly.
#[component]
pub fn Demo() -> Element {
    let items: Vec<String> = (0..12).map(|i| format!("{}", i + 1)).collect();
    rsx! {
        // See `variants/main/mod.rs`'s own comment for the `26rem` derivation.
        div { style: "width: 100%; max-width: 26rem; margin: 0 auto;",
            Carousel { aria_label: "Seamless looping gallery", r#loop: true,
                CarouselPrevious { ChevronLeft {} }
                CarouselNext { ChevronRight {} }
                CarouselVirtualContent::<String> {
                    items,
                    render_item: move |(_idx, label): (usize, String)| rsx! {
                        div {
                            style: "display: flex; align-items: center; justify-content: center; height: 12rem; border: 1px solid var(--primary-color-6); border-radius: var(--dx-radius-lg); font-size: 2rem;",
                            "{label}"
                        }
                    },
                }
            }
        }
    }
}
