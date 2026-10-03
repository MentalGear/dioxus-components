use super::super::component::*;
use crate::components::card::{Card, CardContent};
use dioxus::prelude::*;
use dioxus_icons::lucide::{ChevronLeft, ChevronRight};

/// shadcn's "API" demo (`carousel-api.tsx`): a real, visible "Slide {n} of
/// {m}" counter -- a genuine DOM element, not an aria-only label -- built
/// entirely from [`use_carousel`]'s public [`CarouselApi`], the same
/// public escape hatch docs.md's own "A custom picker" section documents.
///
/// This is also where the pre-shadcn-parity `indicators` demo's custom
/// dot-picker composition technique lives on, now that its own exported
/// helper's name (`CarouselIndicators`) was reused by the APG tablist
/// component it was renamed from `CarouselTabList` to (see `component.rs`'s
/// own doc on that rename) -- `use_carousel()` itself was never renamed or
/// removed, so this demo (and any consumer) can still build any custom
/// picker UI it wants directly on top of it.
#[component]
pub fn Demo() -> Element {
    rsx! {
        // shadcn's own `carousel-api.tsx` wrapper is `mx-auto max-w-xs`: a
        // 320px (20rem) slide with Previous/Next OUTSIDE it. Here
        // `.dx-carousel` reserves its own 6rem of `padding-inline` for those
        // buttons INSIDE its box (style.css), so the wrapper is `20rem +
        // 6rem` -- the same pairing `variants/main/mod.rs` documents -- or
        // the slide shrinks by the reservation (the old `10rem` wrapper left
        // a 64x83 sliver and wrapped the caption to "Slide 1 / of 5").
        // `width: 100%` is load-bearing next to `max-width` (row 94).
        div { style: "width: 100%; max-width: 26rem; margin: 0 auto;",
            Carousel { aria_label: "API demo gallery",
                // The slide track gets its own positioning context so
                // Previous/Next (absolutely centred on the nearest
                // positioned ancestor) centre on the SLIDES, not on the
                // taller root that also holds the caption and the dots
                // below -- measured ~30px low otherwise. Its negative
                // margin/matching padding re-create the root's own
                // `padding-inline` reservation, so the buttons sit exactly
                // where they do in every other demo.
                div { style: "position: relative; margin-inline: calc(-1 * var(--dx-space-12)); padding-inline: var(--dx-space-12);",
                    CarouselPrevious { ChevronLeft {} }
                    CarouselNext { ChevronRight {} }
                    CarouselContent {
                        for i in 0..5usize {
                            CarouselItem { key: "{i}", index: i,
                                Card {
                                    // shadcn: `CardContent className="flex aspect-square
                                    // items-center justify-center p-6"` + `text-4xl font-semibold`.
                                    // `.dx-card-content`'s own `0 space-6` padding stands in
                                    // for `p-6`: `aspect-ratio` here applies to the CONTENT box
                                    // (272 square at a 320 card) and the `Card`'s own block
                                    // padding (24 + 24) brings the whole card back to a true
                                    // 320 square, like every other demo -- an extra vertical
                                    // `p-6` on top would make the card 48px taller than wide.
                                    CardContent {
                                        style: "display: flex; align-items: center; justify-content: center; aspect-ratio: 1; font-size: var(--dx-text-4xl); font-weight: 600;",
                                        "{i + 1}"
                                    }
                                }
                            }
                        }
                    }
                }
                SlideCounter {}
                CustomDotPicker {}
            }
        }
    }
}

/// A real, visible counter -- `use_carousel()`'s `selected`/`count` are
/// plain, already-resolved values (not signals), so this simply reads
/// them straight into the rendered text; being a sibling of
/// `CarouselContent` inside `Carousel` is what gives it access to the
/// same `CarouselContext` `use_carousel()` reads.
#[component]
fn SlideCounter() -> Element {
    let api = use_carousel();
    rsx! {
        p {
            // shadcn: `text-muted-foreground py-2 text-center text-sm`.
            style: "margin: 0; padding-block: var(--dx-space-2); text-align: center; font-size: var(--dx-text-sm); color: var(--dx-muted-foreground);",
            "Slide {api.selected + 1} of {api.count}"
        }
    }
}

/// The pre-shadcn-parity `indicators` demo's own dot-picker composition,
/// carried forward verbatim (same `data-active` state attribute, same
/// `scroll_to(i)` click handler) -- demo-local markup, not an exported
/// component, since the point here is exactly that a caller CAN build
/// this without any dedicated primitive at all, matching docs.md's own "A
/// custom picker" section example. Deliberately does not reuse
/// `.dx-carousel-indicator`'s own class/CSS -- that class now belongs to
/// the renamed APG tablist `CarouselIndicator` (`role="tab"`,
/// `aria-selected`), a different ARIA pattern than this plain, non-tablist
/// picker (`data-active`, no roving tabindex).
///
/// All paint is in `style.css`'s `.dx-carousel-picker`/`-dot` rules, driven
/// by `data-active` -- NOT an inline `style` that changes with the state:
/// dioxus-interpreter-js 0.7.9 drops every `var()`-valued shorthand
/// (`border-radius`, `background`) from an inline `style` the first time
/// that attribute is re-set, which turned the previously- and newly-active
/// dots square. See the rule's own comment in `style.css`.
#[component]
fn CustomDotPicker() -> Element {
    let api = use_carousel();
    rsx! {
        div { class: "dx-carousel-picker", role: "group", "aria-label": "Slide picker",
            for i in 0..api.count {
                button {
                    key: "{i}",
                    class: "dx-carousel-picker-dot",
                    r#type: "button",
                    "data-active": i == api.selected,
                    "aria-label": "Go to slide {i + 1}",
                    onclick: move |_| api.scroll_to(i),
                }
            }
        }
    }
}
