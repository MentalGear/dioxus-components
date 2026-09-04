use dioxus::prelude::*;
use dioxus_icons::lucide::{GripVertical, X};
use dioxus_primitives::drag_and_drop_list::{
    self, DragAndDropContext, DragAndDropDropIndicatorProps, DragAndDropItemContext,
    DragAndDropListItemProps, DragAndDropListItemsProps,
};

// docs/backlog.md row 32: `#[css_module]` is gone -- see checkbox/component.rs's
// header comment for the full delivery-mechanism rationale (asset!() +
// document::Link, embedded in the wrapper so a `dx components add
// drag_and_drop_list`-copied component needs no extra wiring).
//
// This component is also on the row-32 "not already namespaced" lane, and
// its `.dx-remove-button` is one of the two genuine cross-component
// collisions row 32 found: `tag_group` defines its own, differently-sized
// `.dx-remove-button` (unsized vs this one's 26x26px + `margin-left: 10px`),
// and unhashing both under the same short name would silently restyle
// whichever sheet lost the CSS load order. Every out-of-namespace class here
// (`dx-dnd-list*`, `dx-item-icon`, `dx-item-body-div`, `dx-remove-button`,
// `dx-drop-indicator`) was renamed under the full `dx-drag-and-drop-list-`
// namespace in the same change that drops the macro, in this file,
// `style.css`, and the `variants/main` demo's own inline stylesheet (which
// hovers `.dx-dnd-list-item` to restyle a demo-only class) -- see
// `scripts/check-dx-class-prefix.sh`.
#[derive(Props, Clone, PartialEq)]
pub struct DragAndDropListProps {
    /// Items (labels) to be rendered.
    pub items: Vec<Element>,

    /// Set if the list items should be removable
    #[props(default)]
    pub is_removable: bool,

    /// Accessible label for the list
    #[props(default)]
    pub aria_label: Option<String>,

    /// Additional attributes to apply to the list element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the list component.
    pub children: Element,
}

#[component]
pub fn DragAndDropList(props: DragAndDropListProps) -> Element {
    let is_removable = props.is_removable;
    let aria_label = props
        .aria_label
        .clone()
        .unwrap_or_else(|| "Sortable list".to_string());
    // Keep a stable key per item so Dioxus moves keyed siblings instead of
    // swapping content between list items during reorder.
    let items: Vec<Element> = props
        .items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let key = item
                .as_ref()
                .ok()
                .and_then(|v| v.key.clone())
                .unwrap_or_else(|| idx.to_string());
            rsx! {
                DragIcon { key: "{key}" }
                div { class: "dx-drag-and-drop-list-item-body-div", {item} }
                if is_removable {
                    RemoveButton {}
                }
            }
        })
        .collect();

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/drag_and_drop_list/style.css") }
        drag_and_drop_list::DragAndDropList {
            class: "dx-drag-and-drop-list",
            items,
            aria_label: props.aria_label,
            attributes: props.attributes,
            drag_and_drop_list::DragAndDropInstructions {}
            DragAndDropListItems {
                aria_label,
            }
            drag_and_drop_list::DragAndDropLiveRegion {}
            {props.children}
        }
    }
}

#[component]
pub fn DragAndDropListItem(props: DragAndDropListItemProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/drag_and_drop_list/style.css") }
        drag_and_drop_list::DragAndDropListItem {
            class: "dx-drag-and-drop-list-item",
            index: props.index,
            // Forward the stable item key so the primitive tracks focus by
            // identity across reorders and removals instead of losing it.
            item_key: props.item_key.clone(),
            attributes: props.attributes,
            {props.children}
        }
    }
}

#[component]
pub fn DragAndDropListItems(props: DragAndDropListItemsProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/drag_and_drop_list/style.css") }
        drag_and_drop_list::DragAndDropListItems {
            class: "dx-drag-and-drop-list-ul",
            aria_label: props.aria_label,
            attributes: props.attributes,
            for item in drag_and_drop_list::use_drag_and_drop_list_items() {
                Fragment {
                    key: "{item.key}",
                    DragAndDropDropIndicator {
                        index: item.index,
                        position: "before",
                    }
                    DragAndDropListItem {
                        index: item.index,
                        item_key: item.key.clone(),
                        {item.children}
                    }
                    DragAndDropDropIndicator {
                        index: item.index,
                        position: "after",
                    }
                }
            }
        }
    }
}

#[component]
pub fn DragAndDropDropIndicator(props: DragAndDropDropIndicatorProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/drag_and_drop_list/style.css") }
        drag_and_drop_list::DragAndDropDropIndicator {
            class: "dx-drag-and-drop-list-drop-indicator",
            index: props.index,
            position: props.position,
            attributes: props.attributes,
        }
    }
}

#[component]
fn DragIcon() -> Element {
    rsx! {
        GripVertical {
            class: "dx-drag-and-drop-list-item-icon",
            "aria-hidden": "true",
            size: "16px",
        }
    }
}

#[component]
pub fn RemoveButton(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let mut ctx: DragAndDropContext = use_context();
    let item_ctx: DragAndDropItemContext = use_context();
    let index = item_ctx.index();
    let label = format!("Remove item {}", index + 1);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/drag_and_drop_list/style.css") }
        button {
            class: "dx-drag-and-drop-list-remove-button",
            r#type: "button",
            aria_label: "{label}",
            draggable: "false",
            onpointerdown: move |event| event.stop_propagation(),
            onmousedown: move |event| event.stop_propagation(),
            onmouseup: move |event| event.stop_propagation(),
            ondragstart: move |event| {
                event.prevent_default();
                event.stop_propagation();
            },
            onkeydown: move |event| event.stop_propagation(),
            onclick: move |event| {
                event.stop_propagation();
                ctx.remove(index);
            },
            ..attributes,
            {children}
            X { size: "14px" }
        }
    }
}
