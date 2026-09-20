use super::super::component::*;
use dioxus::prelude::*;
use dioxus_icons::lucide::{ChevronDown, ChevronUp};

/// `CarouselOrientation::Vertical` fell out of the horizontal
/// construction for free (see `primitives/src/carousel.rs`'s module
/// doc), so it ships in v1 rather than being deferred. A vertical
/// carousel needs an explicit height on `CarouselContent` -- there is
/// nothing else to derive one from, the same way `ScrollArea` needs one.
///
/// `padding-block: var(--dx-space-12)` on this wrapper (absent from every
/// other variant's wrapper) exists only for this orientation: Previous/Next
/// now sit `calc(-1 * var(--dx-space-12))` outside the region on the block
/// axis (style.css, shadcn-parity), and this is the one demo where that
/// extends past `.dx-component-preview-frame`'s own padding -- measured
/// live, before this padding was added, as the button's box straddling the
/// frame's border (~15px poking above/below it in both themes; horizontal
/// variants stay clear by a wide margin). Reusing the same `--dx-space-12` token the
/// button offset itself uses (rather than a new literal) guarantees this
/// wrapper always gives at least as much room as the button needs, however
/// the frame's own padding is tuned later.
#[component]
pub fn Demo() -> Element {
    rsx! {
        div { style: "width: 100%; max-width: 12rem; margin: 0 auto; padding-block: var(--dx-space-12);",
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
