use super::super::component::*;
use dioxus::prelude::*;
use dioxus_icons::lucide::{ChevronLeft, ChevronRight};

/// `CarouselVirtualContent` with `loop: false` and a large data set (200
/// items) -- shows the DOM only ever holds a small window (`radius: 2` ->
/// at most 5 slides) regardless of `N`, and that Previous/Next genuinely
/// `disabled` at the real ends (no loop, so `wrap: false` clamps the
/// window there too) -- see `primitives/src/carousel.rs`'s own
/// `CarouselVirtualContent` doc, "Non-loop, virtualised."
#[component]
pub fn Demo() -> Element {
    let items: Vec<String> = (0..200).map(|i| format!("{}", i + 1)).collect();
    rsx! {
        // See `variants/main/mod.rs`'s own comment for the `26rem` derivation.
        div { style: "width: 100%; max-width: 26rem; margin: 0 auto;",
            Carousel { aria_label: "Large product catalog",
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
