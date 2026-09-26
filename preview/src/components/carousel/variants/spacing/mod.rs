use super::super::component::*;
use crate::components::card::{Card, CardContent};
use dioxus::prelude::*;
use dioxus_icons::lucide::{ChevronLeft, ChevronRight};

/// shadcn's "Spacing" demo (`carousel-spacing.tsx`): the inter-slide gap is
/// a compositional choice at the call site (`--dx-carousel-gap` on
/// `CarouselContent`), not a hardcoded property -- see docs.md's own
/// "Sizes and spacing" section. Four presets, matching shadcn's own
/// `-ml-1/pl-1` ... `-ml-4/pl-4` scale exactly (`--dx-space-1` = 4px ...
/// `--dx-space-4` = 16px, this component's own default, set on
/// `.dx-carousel-content` in `style.css`).
///
/// Each row also overrides `CarouselItem`'s own `flex-basis` down to 50%
/// (two slides per view) -- with only one slide visible at a time (the
/// primitive's own default), a gap between slides is never on screen at
/// all, so a single-per-view row would not actually demonstrate anything.
#[component]
pub fn Demo() -> Element {
    let presets = [
        ("--dx-space-1 (4px)", "var(--dx-space-1)"),
        ("--dx-space-2 (8px)", "var(--dx-space-2)"),
        ("--dx-space-3 (12px)", "var(--dx-space-3)"),
        ("--dx-space-4 (16px, default)", "var(--dx-space-4)"),
    ];
    rsx! {
        div { style: "width: 100%; max-width: 26rem; margin: 0 auto; display: flex; flex-direction: column; gap: 1.5rem;",
            for (label , gap) in presets {
                div { key: "{label}",
                    p {
                        style: "margin: 0 0 0.5rem; font-size: 0.875rem; color: var(--secondary-color-3);",
                        "{label}"
                    }
                    Carousel { aria_label: "Spacing preset: {label}",
                        CarouselPrevious { ChevronLeft {} }
                        CarouselNext { ChevronRight {} }
                        CarouselContent { style: "--dx-carousel-gap: {gap};",
                            for i in 0..4usize {
                                CarouselItem { key: "{i}", index: i, style: "flex-basis: 50%;",
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
    }
}
