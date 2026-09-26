use dioxus::prelude::*;
use dioxus_primitives::carousel::{
    self, CarouselAutoplayProps, CarouselContentProps, CarouselIndicatorProps,
    CarouselIndicatorsProps, CarouselItemProps, CarouselPreviousProps, CarouselRotationControlProps,
    CarouselVirtualContentProps,
};
use dioxus_primitives::direction::Direction;
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};

// Re-exported so a demo (or a consumer's own page) can write
// `use crate::components::carousel::*;` and reach the orientation enum
// and the `use_carousel()`/`CarouselApi` escape hatch (for a custom
// picker built directly on it, see the `api` variant, or docs.md's own "A
// custom picker" section) without a second import from `dioxus_primitives`
// directly -- the same convention `resizable`'s themed wrapper already
// follows for `ResizableDirection`.
#[allow(unused_imports)] // `CarouselApi` is only ever named as `use_carousel()`'s inferred
// return type within this crate's own demos (the `api` variant) -- re-exported anyway so a
// consumer building their own custom picker can name the type explicitly.
pub use dioxus_primitives::carousel::{
    CarouselAlign, CarouselApi, CarouselOrientation, LoopMode, use_carousel,
};

/// The props for the [`Carousel`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CarouselProps {
    /// The class of the carousel component.
    #[props(default)]
    pub class: String,

    /// The axis the carousel pages along.
    #[props(default)]
    pub orientation: ReadSignal<CarouselOrientation>,

    /// Where each slide rests against the scrollport. Defaults to
    /// `start`, matching shadcn. See [`carousel::Carousel`]'s own doc for
    /// what changes with `Center`/`End`.
    #[props(default)]
    pub align: ReadSignal<CarouselAlign>,

    /// The controlled selected slide index.
    pub value: ReadSignal<Option<usize>>,

    /// The initial selected slide index when uncontrolled.
    #[props(default)]
    pub default_value: usize,

    /// Whether Previous/Next (and the root's own arrow keys) wrap around
    /// at the ends at all. See [`LoopMode`]'s own doc for what actually
    /// decides whether that wrap is seamless, an instant rewind, or a
    /// no-op.
    #[props(default)]
    pub r#loop: ReadSignal<bool>,

    /// How `r#loop` wraps at the ends. Defaults to [`LoopMode::Seamless`].
    #[props(default)]
    pub loop_mode: ReadSignal<LoopMode>,

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
            align: props.align,
            value: props.value,
            default_value: props.default_value,
            r#loop: props.r#loop,
            loop_mode: props.loop_mode,
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

/// The APG tablist (dot-picker) style: `role="tablist"` of `role="tab"`
/// dots, automatic activation on arrow/Home/End. Renamed from
/// `CarouselTabList` (pre-1.0 fork rename, owner-approved -- no
/// deprecated alias). See docs.md's own "Indicators" section for when to
/// reach for `Tabs` instead.
#[component]
pub fn CarouselIndicators(props: CarouselIndicatorsProps) -> Element {
    let base = attributes!(div {
        class: "dx-carousel-indicators"
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/carousel/style.css") }
        carousel::CarouselIndicators { attributes: merged, {props.children} }
    }
}

/// One dot inside a [`CarouselIndicators`]. Renamed from `CarouselTab`.
#[component]
pub fn CarouselIndicator(props: CarouselIndicatorProps) -> Element {
    let base = attributes!(button {
        class: "dx-carousel-indicator"
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/carousel/style.css") }
        carousel::CarouselIndicator { index: props.index, attributes: merged, {props.children} }
    }
}
