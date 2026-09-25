use dioxus::prelude::*;
use dioxus_primitives::dioxus_attributes::attributes;
use dioxus_primitives::merge_attributes;
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
    let base = attributes!(div { class: "dx-resizable-panel-group" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/resizable/style.css") }
        resizable::ResizablePanelGroup {
            direction: props.direction,
            sizes: props.sizes,
            default_sizes: props.default_sizes,
            on_sizes_change: props.on_sizes_change,
            disabled: props.disabled,
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn ResizablePanel(props: ResizablePanelProps) -> Element {
    let base = attributes!(div { class: "dx-resizable-panel" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/resizable/style.css") }
        resizable::ResizablePanel {
            index: props.index,
            default_size: props.default_size,
            min_size: props.min_size,
            max_size: props.max_size,
            collapsible: props.collapsible,
            collapsed_size: props.collapsed_size,
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn ResizableHandle(props: ResizableHandleProps) -> Element {
    let base = attributes!(div { class: "dx-resizable-handle" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/resizable/style.css") }
        resizable::ResizableHandle {
            index: props.index,
            disabled: props.disabled,
            aria_label: props.aria_label,
            aria_labelledby: props.aria_labelledby,
            attributes: merged,
            {props.children}
        }
    }
}
