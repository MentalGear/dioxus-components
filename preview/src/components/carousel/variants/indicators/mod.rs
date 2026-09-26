use super::super::component::*;
use dioxus::prelude::*;

/// The APG "tabbed" carousel style: a `CarouselIndicators` of dot pickers
/// (`role="tablist"`, roving tabindex, automatic activation on
/// arrow/Home/End) in place of `CarouselPrevious`/`CarouselNext`, matching
/// the vendored reference's own structure (`carousel-2-tablist.html`) --
/// no separate Previous/Next buttons. `CarouselItem` renders
/// `role="tabpanel"` (instead of `group`) once a `CarouselIndicators` is
/// present. See `CarouselIndicators`' own doc for why this composes
/// `crate::collection` directly rather than `Tabs`/`TabTrigger` (manual
/// activation there is the wrong contract).
///
/// Merges what used to be two separate demos -- `tabs` (this one, renamed
/// from `CarouselTabList`/`CarouselTab`) and a plain `indicators` demo (a
/// custom `use_carousel()`-built dot picker, unrelated to this tablist
/// component despite the name collision) -- into this single "Indicators"
/// demo. The custom `use_carousel()` picker lives on as the `api` demo
/// instead.
#[component]
pub fn Demo() -> Element {
    rsx! {
        // See `variants/main/mod.rs`'s own comment for the `26rem` derivation.
        div { style: "width: 100%; max-width: 26rem; margin: 0 auto;",
            Carousel { aria_label: "Tabbed slide gallery",
                CarouselIndicators {
                    for i in 0..5usize {
                        CarouselIndicator { key: "{i}", index: i }
                    }
                }
                CarouselContent {
                    for i in 0..5usize {
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
