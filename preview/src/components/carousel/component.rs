use dioxus::prelude::*;
use dioxus_primitives::carousel::{self, CarouselContentProps, CarouselItemProps, CarouselPreviousProps};
use dioxus_primitives::direction::Direction;
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};

// Re-exported so a demo (or a consumer's own page) can write
// `use crate::components::carousel::*;` and reach the orientation enum
// and the `use_carousel()`/`CarouselApi` escape hatch (for a custom
// indicator row, see the `indicators` variant) without a second import
// from `dioxus_primitives` directly -- the same convention `resizable`'s
// themed wrapper already follows for `ResizableDirection`.
#[allow(unused_imports)] // `CarouselApi` is only ever named as `use_carousel()`'s inferred
// return type in this file's own `CarouselIndicators` -- re-exported anyway so a
// consumer building their own custom picker can name the type explicitly.
pub use dioxus_primitives::carousel::{CarouselApi, CarouselOrientation, use_carousel};

/// The props for the [`Carousel`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CarouselProps {
    /// The class of the carousel component.
    #[props(default)]
    pub class: String,

    /// The axis the carousel pages along.
    #[props(default)]
    pub orientation: ReadSignal<CarouselOrientation>,

    /// The controlled selected slide index.
    pub value: ReadSignal<Option<usize>>,

    /// The initial selected slide index when uncontrolled.
    #[props(default)]
    pub default_value: usize,

    /// Called whenever the selected slide changes.
    #[props(default)]
    pub on_value_change: Callback<usize>,

    /// The text direction for the root-level `ArrowLeft`/`ArrowRight` keys.
    pub dir: Option<Direction>,

    /// Additional attributes to apply to the carousel element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the carousel component.
    pub children: Element,
}

#[component]
pub fn Carousel(props: CarouselProps) -> Element {
    let base = attributes!(div {
        class: format!("{} {}", props.class, "dx-carousel"),
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/carousel/style.css") }
        carousel::Carousel {
            orientation: props.orientation,
            value: props.value,
            default_value: props.default_value,
            on_value_change: props.on_value_change,
            dir: props.dir,
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn CarouselContent(props: CarouselContentProps) -> Element {
    let base = attributes!(div {
        class: "dx-carousel-content"
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/carousel/style.css") }
        carousel::CarouselContent { id: props.id, attributes: merged, {props.children} }
    }
}

#[component]
pub fn CarouselItem(props: CarouselItemProps) -> Element {
    let base = attributes!(div {
        class: "dx-carousel-item"
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/carousel/style.css") }
        carousel::CarouselItem {
            index: props.index,
            id: props.id,
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn CarouselPrevious(props: CarouselPreviousProps) -> Element {
    let base = attributes!(button {
        class: "dx-carousel-previous"
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/carousel/style.css") }
        carousel::CarouselPrevious { attributes: merged, {props.children} }
    }
}

#[component]
pub fn CarouselNext(props: CarouselPreviousProps) -> Element {
    let base = attributes!(button {
        class: "dx-carousel-next"
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/carousel/style.css") }
        carousel::CarouselNext { attributes: merged, {props.children} }
    }
}

/// A row of dot indicators, one per slide, for jumping directly to any
/// slide -- composed entirely from [`use_carousel`]'s public
/// [`CarouselApi`], not a new primitive. Used by the `indicators` demo
/// variant; exported so any consumer can drop it in verbatim the way
/// `dx components add` copies the rest of this file.
#[component]
pub fn CarouselIndicators() -> Element {
    let api = use_carousel();

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/carousel/style.css") }
        div { class: "dx-carousel-indicators", role: "group", "aria-label": "Slide indicators",
            for i in 0..api.count {
                button {
                    key: "{i}",
                    r#type: "button",
                    class: "dx-carousel-indicator",
                    "data-active": i == api.selected,
                    "aria-label": "Go to slide {i + 1}",
                    "aria-current": if i == api.selected { "true" } else { "false" },
                    onclick: move |_| api.scroll_to(i),
                }
            }
        }
    }
}
