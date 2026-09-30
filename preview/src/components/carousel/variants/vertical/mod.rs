use super::super::component::*;
use crate::components::card::{Card, CardContent};
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
///
/// Sizing follows shadcn's own `carousel-orientation.tsx` (not this file's
/// pre-shadcn-parity single-slide layout): `--dx-carousel-per-view: 2`
/// (`basis-1/2`) against a `16.875rem` (270px) `CarouselContent` height --
/// so each of the 2 visible slides gets half that height, split across the
/// block axis, rather than one slide filling the whole box. Deliberately
/// no `aspect-ratio` on the slide's own `CardContent` here, unlike every
/// other demo: shadcn's own orientation example has none either (height
/// comes from `CarouselContent`'s explicit height, not from the slide's
/// own aspect ratio).
///
/// **The `Card` must be `box-sizing: border-box`.** `Card` (`.dx-card`)
/// carries `padding: 24px 0` plus a 1px border and no `box-sizing`, i.e.
/// the `content-box` default, so `height: 100%` alone means "the slide's
/// usable block size *plus* 50px of padding/border on top". Measured live
/// before this fix: each item is 135px (half of the 270px content box),
/// 119px of it usable after the 16px leading gap padding, and the Card
/// rendered 169px tall (119 + 48 + 2) -- card 2 overlapped card 1 and the
/// bottom card ran past the clip viewport. `border-box` makes the Card
/// exactly the slide's usable size; `CardContent` then takes the Card's
/// remaining height with `flex: 1` (the Card is a flex column) instead of a
/// second, again-overflowing `height: 100%`. The gap is the default
/// spacing (`--dx-space-4`, 16px), the same as the horizontal demos, so the
/// space between the two visible cards reads the same on both axes (an
/// earlier 4px gap, copied from shadcn's `-mt-1`/`pt-1`, was filled by the
/// Card's own shadow and looked like no gap at all).
#[component]
pub fn Demo() -> Element {
    rsx! {
        div { style: "width: 100%; max-width: 12rem; margin: 0 auto;",
            Carousel {
                aria_label: "Vertical scrolling demo",
                orientation: CarouselOrientation::Vertical,
                CarouselPrevious { ChevronUp {} }
                CarouselNext { ChevronDown {} }
                CarouselContent { style: "height: 16.875rem; --dx-carousel-per-view: 2;",
                    for i in 0..4usize {
                        CarouselItem { key: "{i}", index: i,
                            Card { style: "height: 100%; box-sizing: border-box;",
                                CardContent {
                                    style: "display: flex; flex: 1; min-height: 0; align-items: center; justify-content: center; font-size: var(--dx-text-3xl); font-weight: 600;",
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
