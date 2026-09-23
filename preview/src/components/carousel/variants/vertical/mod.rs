use super::super::component::*;
use dioxus::prelude::*;
use dioxus_icons::lucide::{ChevronDown, ChevronUp};

/// `CarouselOrientation::Vertical` fell out of the horizontal
/// construction for free (see `primitives/src/carousel.rs`'s module
/// doc), so it ships in v1 rather than being deferred. A vertical
/// carousel needs an explicit height on `CarouselContent` -- there is
/// nothing else to derive one from, the same way `ScrollArea` needs one.
///
/// Unlike the horizontal variants (`main`/`indicators`/`rtl`/`multiple`),
/// this wrapper needs no compensating size bump for `.dx-carousel`'s own
/// arrow-reservation padding (style.css, `7ad791c`). That reservation
/// costs a horizontal wrapper track width because `CarouselContent` has no
/// explicit width of its own there -- it fills whatever's left of
/// `.dx-carousel`'s fixed `width: 100%` after `padding-inline` is carved
/// out of it (`box-sizing: border-box`). Here `CarouselContent` instead
/// gets an explicit, literal `height: 12rem` below, wired to nothing else
/// -- so the reservation's `padding-block` has no fixed budget to be
/// carved out of; `.dx-carousel` (a plain block box, no explicit height)
/// just grows 6rem taller to fit both, and the visible slide stays 12rem.
/// Measured live (lane `carousel-wrapper-width`): unchanged at 192px
/// across every viewport/theme swept, with or without the reservation.
///
/// This wrapper previously also carried its own `padding-block:
/// var(--dx-space-12)` -- headroom for the pre-`7ad791c` construction,
/// where Previous/Next sat `calc(-1 * var(--dx-space-12))` *outside*
/// `.dx-carousel`'s own box and could straddle
/// `.dx-component-preview-frame`'s border. `7ad791c` moved those buttons
/// to sit *inside* `.dx-carousel`'s own (now self-expanding) box instead,
/// so nothing here needs external headroom for them any more -- style.css's
/// own comment on the vertical `padding-block` rule already called this
/// redundant, and removing it and re-measuring confirms it: no button
/// escapes the frame at any swept viewport/theme without it.
#[component]
pub fn Demo() -> Element {
    rsx! {
        div { style: "width: 100%; max-width: 12rem; margin: 0 auto;",
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
