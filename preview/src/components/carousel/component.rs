use dioxus::prelude::*;
use dioxus_primitives::carousel::{
    self, CarouselAutoplayProps, CarouselContentProps, CarouselItemProps, CarouselPreviousProps,
    CarouselRotationControlProps, CarouselTabListProps, CarouselTabProps,
    CarouselVirtualContentProps,
};
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

    /// Whether Previous/Next (and the root's own arrow keys) wrap around
    /// at the ends. See [`carousel::Carousel`]'s own doc for the
    /// rewind-style semantics.
    #[props(default)]
    pub r#loop: ReadSignal<bool>,

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
            r#loop: props.r#loop,
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
        carousel::CarouselContent {
            id: props.id,
            draggable: props.draggable,
            attributes: merged,
            {props.children}
        }
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
pub fn CarouselVirtualContent<T: Clone + PartialEq + 'static>(
    props: CarouselVirtualContentProps<T>,
) -> Element {
    let base = attributes!(div {
        class: "dx-carousel-content"
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/carousel/style.css") }
        carousel::CarouselVirtualContent::<T> {
            id: props.id,
            items: props.items,
            render_item: props.render_item,
            radius: props.radius,
            virtualize: props.virtualize,
            draggable: props.draggable,
            attributes: merged,
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

#[component]
pub fn CarouselAutoplay(props: CarouselAutoplayProps) -> Element {
    rsx! {
        carousel::CarouselAutoplay {
            delay_ms: props.delay_ms,
            stop_on_interaction: props.stop_on_interaction,
            stop_on_mouse_enter: props.stop_on_mouse_enter,
            default_playing: props.default_playing,
        }
    }
}

#[component]
pub fn CarouselRotationControl(props: CarouselRotationControlProps) -> Element {
    let base = attributes!(button {
        class: "dx-carousel-rotation-control"
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/carousel/style.css") }
        carousel::CarouselRotationControl { attributes: merged, {props.children} }
    }
}

#[component]
pub fn CarouselTabList(props: CarouselTabListProps) -> Element {
    let base = attributes!(div {
        class: "dx-carousel-tab-list"
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/carousel/style.css") }
        carousel::CarouselTabList { attributes: merged, {props.children} }
    }
}

#[component]
pub fn CarouselTab(props: CarouselTabProps) -> Element {
    let base = attributes!(button {
        class: "dx-carousel-tab"
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/carousel/style.css") }
        carousel::CarouselTab { index: props.index, attributes: merged, {props.children} }
    }
}
