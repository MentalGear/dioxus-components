use super::super::component::*;
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
        // shadcn's own `carousel-api.tsx` wrapper: `mx-auto max-w-[10rem]
        // sm:max-w-xs` -- narrower than every other demo's container,
        // since this one has no Previous/Next-clearance content to size
        // around beyond the slide itself.
        div { style: "width: 100%; max-width: 10rem; margin: 0 auto;",
            Carousel { aria_label: "API demo gallery",
                CarouselPrevious { ChevronLeft {} }
                CarouselNext { ChevronRight {} }
                CarouselContent {
                    for i in 0..5usize {
                        CarouselItem { key: "{i}", index: i,
                            div {
                                style: "display: flex; align-items: center; justify-content: center; height: 8rem; border: 1px solid var(--primary-color-6); border-radius: var(--dx-radius-lg); font-size: 1.5rem;",
                                "{i + 1}"
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
            style: "margin: 0.5rem 0 0; text-align: center; font-size: 0.875rem; color: var(--secondary-color-3);",
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
#[component]
fn CustomDotPicker() -> Element {
    let api = use_carousel();
    rsx! {
        div {
            style: "display: flex; justify-content: center; gap: var(--dx-space-2); margin-block-start: var(--dx-space-2);",
            role: "group",
            "aria-label": "Slide picker",
            for i in 0..api.count {
                {
                    let active = i == api.selected;
                    let background = if active { "var(--primary-color)" } else { "var(--primary-color-6)" };
                    rsx! {
                        button {
                            key: "{i}",
                            r#type: "button",
                            style: "width: var(--dx-space-2); height: var(--dx-space-2); box-sizing: border-box; padding: 0; border: none; border-radius: var(--dx-radius-full); cursor: pointer; background: {background};",
                            "data-active": active,
                            "aria-label": "Go to slide {i + 1}",
                            onclick: move |_| api.scroll_to(i),
                        }
                    }
                }
            }
        }
    }
}
