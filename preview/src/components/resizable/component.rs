use dioxus::prelude::*;
use dioxus_primitives::resizable::{
    self, ResizableHandleProps, ResizablePanelGroupProps, ResizablePanelProps,
};

// Plain enum, not markup -- re-exported so demo code never needs its own
// `dioxus_primitives::resizable::` import (scripts/check-preview-composition.sh's
// allowlist covers this shape for other components, e.g. `ContentSide`, via a
// module-root re-export from the themed wrapper; doing the same here keeps every
// non-`component.rs` file in this crate free of raw `dioxus_primitives::` text at
// all, which is simpler than adding an allowlist entry to a script this lane does
// not own).
pub use dioxus_primitives::resizable::ResizableDirection;

#[component]
pub fn ResizablePanelGroup(props: ResizablePanelGroupProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/resizable/style.css") }
        resizable::ResizablePanelGroup {
            class: "dx-resizable-panel-group",
            direction: props.direction,
            sizes: props.sizes,
            default_sizes: props.default_sizes,
            on_sizes_change: props.on_sizes_change,
            disabled: props.disabled,
            attributes: props.attributes,
            {props.children}
        }
    }
}

#[component]
pub fn ResizablePanel(props: ResizablePanelProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/resizable/style.css") }
        resizable::ResizablePanel {
            class: "dx-resizable-panel",
            index: props.index,
            default_size: props.default_size,
            min_size: props.min_size,
            max_size: props.max_size,
            collapsible: props.collapsible,
            collapsed_size: props.collapsed_size,
            attributes: props.attributes,
            {props.children}
        }
    }
}

#[component]
pub fn ResizableHandle(props: ResizableHandleProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/resizable/style.css") }
        resizable::ResizableHandle {
            class: "dx-resizable-handle",
            index: props.index,
            disabled: props.disabled,
            aria_label: props.aria_label,
            aria_labelledby: props.aria_labelledby,
            attributes: props.attributes,
            {props.children}
        }
    }
}
