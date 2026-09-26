use super::super::component::*;
use crate::components::card::{Card, CardContent};
use dioxus::prelude::*;
use dioxus_icons::lucide::{ChevronLeft, ChevronRight};

/// shadcn's "Sizes" demo (`carousel-size.tsx`): whole slides only, never a
/// partial peek -- `--dx-carousel-per-view: 2` by default, `3` at a wider
/// (`lg`, 64rem) breakpoint, set via a plain `@media` rule on this demo's
/// own wrapper class. There is no per-instance prop for this -- it is a
/// CSS decision at the call site (docs.md's own "Sizes" section) -- so
/// this uses the same "demo-local `<style>` block" construction `button`'s
/// own `size` variant already established for responsive demo layout.
///
/// Replaces this component's own pre-shadcn-parity `multiple` variant,
/// which used a bare `flex-basis: 40%` override -- not `1/n` for any small
/// whole `n`, so it overflowed the track (`dev-docs/research/shadcn-carousel-parity.md`'s
/// own "2½ slides" finding). `playwright/carousel.spec.ts` was updated to
/// match (the old `multiple` variant name is retired; the "partially
/// visible neighbour" drag test that specifically needed a fractional
/// peek now targets the new `peek` variant instead, which exists
/// precisely for that opt-in case).
#[component]
pub fn Demo() -> Element {
    rsx! {
        style { {SIZES_DEMO_STYLE} }
        div { class: "dx-carousel-demo-sizes", style: "width: 100%; margin: 0 auto;",
            Carousel { aria_label: "Product gallery",
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

const SIZES_DEMO_STYLE: &str = r#"
.dx-carousel-demo-sizes {
  max-width: 12rem;
  --dx-carousel-per-view: 2;
}

@media (width >= 24rem) {
  .dx-carousel-demo-sizes {
    max-width: 20rem;
  }
}

@media (width >= 64rem) {
  .dx-carousel-demo-sizes {
    max-width: 24rem;
    --dx-carousel-per-view: 3;
  }
}
"#;
