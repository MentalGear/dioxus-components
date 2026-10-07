use crate::components::button::{Button, ButtonSize, ButtonVariant};
use dioxus::prelude::*;
use dioxus_icons::lucide::ArrowDown;
use dioxus_primitives::dioxus_attributes::attributes;
use dioxus_primitives::merge_attributes;
use dioxus_primitives::message_scroller::{
    self, MessageScrollerContentProps, MessageScrollerItemProps, MessageScrollerProps,
    MessageScrollerProviderProps, MessageScrollerRowsProps, MessageScrollerViewportProps,
};

// Re-exported so a demo (or a consumer's own page) can write
// `use crate::components::message_scroller::*;` and reach the commands hook,
// the scrollability signal and the option enums without a second import from
// `dioxus_primitives` -- the same convention `carousel`'s wrapper follows.
#[allow(unused_imports)] // a consumer-facing surface; this crate's own demos use a subset
pub use dioxus_primitives::message_scroller::{
    use_message_scroller, use_message_scroller_scrollable, DefaultScrollPosition,
    MessageScrollerApi, MessageScrollerDirection, ScrollAlign, ScrollBehavior,
    ScrollToMessageOptions, Scrollable, Virtualization,
};

/// The headless root: owns the scroll controller and renders no element.
#[component]
pub fn MessageScrollerProvider(props: MessageScrollerProviderProps) -> Element {
    rsx! {
        message_scroller::MessageScrollerProvider {
            auto_scroll: props.auto_scroll,
            default_scroll_position: props.default_scroll_position,
            scroll_edge_threshold: props.scroll_edge_threshold,
            scroll_previous_item_peek: props.scroll_previous_item_peek,
            scroll_margin: props.scroll_margin,
            virtualize: props.virtualize,
            {props.children}
        }
    }
}

/// The frame: fills its parent (place it in a height-constrained container).
#[component]
pub fn MessageScroller(props: MessageScrollerProps) -> Element {
    let base = attributes!(div {
        class: "dx-message-scroller",
        "data-slot": "message-scroller",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/message_scroller/style.css") }
        message_scroller::MessageScroller { attributes: merged, {props.children} }
    }
}

/// The scrollable element, with a bottom edge fade (`dx-scroll-fade-b`, from
/// `dx-effects.css`; without that file the fade is simply absent).
#[component]
pub fn MessageScrollerViewport(props: MessageScrollerViewportProps) -> Element {
    let base = attributes!(div {
        class: "dx-message-scroller-viewport dx-scroll-fade-b",
        "data-slot": "message-scroller-viewport",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/message_scroller/style.css") }
        message_scroller::MessageScrollerViewport {
            preserve_scroll_on_prepend: props.preserve_scroll_on_prepend,
            attributes: merged,
            {props.children}
        }
    }
}

/// The transcript column holding the rows.
#[component]
pub fn MessageScrollerContent(props: MessageScrollerContentProps) -> Element {
    let base = attributes!(div {
        class: "dx-message-scroller-content",
        "data-slot": "message-scroller-content",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/message_scroller/style.css") }
        message_scroller::MessageScrollerContent {
            spacer_class: props.spacer_class,
            attributes: merged,
            {props.children}
        }
    }
}

/// One transcript row: a message, marker, typing indicator or separator.
#[component]
pub fn MessageScrollerItem(props: MessageScrollerItemProps) -> Element {
    let base = attributes!(div {
        class: "dx-message-scroller-item",
        "data-slot": "message-scroller-item",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/message_scroller/style.css") }
        message_scroller::MessageScrollerItem {
            message_id: props.message_id,
            scroll_anchor: props.scroll_anchor,
            attributes: merged,
            {props.children}
        }
    }
}

/// A long transcript's rows, grouped in chunks of 20 that the browser skips as a unit
/// (one skippable element per row costs main-thread time per scroll frame in proportion to the
/// row count). Use it in place of a `for` loop of `MessageScrollerItem`s.
#[component]
pub fn MessageScrollerRows(props: MessageScrollerRowsProps) -> Element {
    let base = attributes!(div {
        class: "dx-message-scroller-chunk",
        "data-slot": "message-scroller-chunk",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/message_scroller/style.css") }
        message_scroller::MessageScrollerRows {
            count: props.count,
            start: props.start,
            render_row: props.render_row,
            attributes: merged,
        }
    }
}

/// The jump control: scrolls to the end (or start) of the transcript. Renders
/// the themed `Button` (a secondary icon button by default) with an arrow and a
/// screen-reader label unless `children` are given.
#[component]
pub fn MessageScrollerButton(
    #[props(default)] direction: MessageScrollerDirection,
    #[props(default = ScrollBehavior::Smooth)] behavior: ScrollBehavior,
    #[props(default = ButtonVariant::Secondary)] variant: ButtonVariant,
    #[props(default = ButtonSize::IconSm)] size: ButtonSize,
    #[props(default)] onclick: Option<EventHandler<MouseEvent>>,
    #[props(extends = GlobalAttributes)]
    #[props(extends = button)]
    attributes: Vec<Attribute>,
    children: Option<Element>,
) -> Element {
    let base = attributes!(button {
        class: "dx-message-scroller-button",
        "data-slot": "message-scroller-button",
    });
    let merged = merge_attributes(vec![base, attributes]);
    let label = match direction {
        MessageScrollerDirection::End => "Scroll to end",
        MessageScrollerDirection::Start => "Scroll to start",
    };

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/message_scroller/style.css") }
        message_scroller::MessageScrollerButton {
            direction,
            behavior,
            onclick,
            attributes: merged,
            r#as: move |attributes: Vec<Attribute>| {
                rsx! {
                    Button { variant, size, attributes,
                        if let Some(children) = children.clone() {
                            {children}
                        } else {
                            ArrowDown { size: "16px", "aria-hidden": "true" }
                            span { class: "dx-message-scroller-sr-only", "{label}" }
                        }
                    }
                }
            },
            {}
        }
    }
}
