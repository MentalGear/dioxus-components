use super::super::component::*;
use dioxus::prelude::*;
use dioxus_icons::lucide::{ChevronLeft, ChevronRight};
use dioxus_primitives::direction::Direction;

/// `loop: true` under `dir="rtl"` -- closes the one integration gap the
/// separate `looping` (LTR) and `rtl` (non-loop) variants don't each cover
/// on their own: RTL's own key-swap (`ArrowLeft` -> next, `ArrowRight`
/// -> previous, `Direction::resolve_horizontal`) composed with `loop`'s
/// own wraparound (`step_prev`/`step_next`).
#[component]
pub fn Demo() -> Element {
    rsx! {
        // See `variants/main/mod.rs`'s own comment for the `26rem` derivation.
        div { style: "width: 100%; max-width: 26rem; margin: 0 auto;",
            Carousel { aria_label: "Looping RTL gallery", r#loop: true, dir: Direction::Rtl,
                CarouselPrevious { ChevronLeft {} }
                CarouselNext { ChevronRight {} }
                CarouselContent {
                    for i in 0..4usize {
                        CarouselItem { key: "{i}", index: i,
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
