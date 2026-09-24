use dioxus::prelude::*;
use dioxus_icons::lucide::X;
use dioxus_primitives::dioxus_attributes::attributes;
use dioxus_primitives::merge_attributes;
use dioxus_primitives::tag_group::{
    self, TagGroupEmptyProps, TagGroupLabelProps, TagGroupMultiProps, TagGroupProps, TagListProps,
};

// docs/backlog.md row 32: `#[css_module]` is gone -- see checkbox/component.rs's
// header comment for the full delivery-mechanism rationale (asset!() +
// document::Link, embedded in every exported entry point of this file so a
// `dx components add tag_group`-copied component needs no extra wiring).
//
// This component is also on the row-32 "not already namespaced" lane, and
// its `.dx-remove-button` is one of the two genuine cross-component
// collisions row 32 found: `drag_and_drop_list` defines its own,
// differently-sized `.dx-remove-button` (26x26px + `margin-left: 10px` vs
// this one's unsized, `margin-left: 0.25rem`), and unhashing both under the
// same short name would silently restyle whichever sheet lost the CSS load
// order. `dx-tag` and `dx-tag-list` were equally out of namespace even
// without an active collision today. All three were renamed under the full
// `dx-tag-group-` namespace (`dx-tag` -> `dx-tag-group-tag`, `dx-tag-list`
// -> `dx-tag-group-list`, `dx-remove-button` -> `dx-tag-group-remove-button`)
// in the same change that drops the macro, in both this file and
// `style.css` -- see `scripts/check-dx-class-prefix.sh`.
#[component]
pub fn TagGroup(props: TagGroupProps<String>) -> Element {
    let base = attributes!(div { class: "dx-tag-group" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/tag_group/style.css") }
        tag_group::TagGroup {
            value: props.value,
            default_value: props.default_value,
            on_value_change: props.on_value_change,
            disabled: props.disabled,
            selectable: props.selectable,
            allow_empty_selection: props.allow_empty_selection,
            escape_clears_selection: props.escape_clears_selection,
            roving_loop: props.roving_loop,
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn TagGroupMulti(props: TagGroupMultiProps<String>) -> Element {
    let base = attributes!(div { class: "dx-tag-group" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/tag_group/style.css") }
        tag_group::TagGroupMulti {
            values: props.values,
            default_values: props.default_values,
            on_values_change: props.on_values_change,
            disabled: props.disabled,
            selectable: props.selectable,
            allow_empty_selection: props.allow_empty_selection,
            escape_clears_selection: props.escape_clears_selection,
            roving_loop: props.roving_loop,
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn TagGroupLabel(props: TagGroupLabelProps) -> Element {
    let base = attributes!(div { class: "dx-tag-group-label" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/tag_group/style.css") }
        tag_group::TagGroupLabel { id: props.id, attributes: merged, {props.children} }
    }
}

#[component]
pub fn TagGroupEmpty(props: TagGroupEmptyProps) -> Element {
    let base = attributes!(div { class: "dx-tag-group-empty" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/tag_group/style.css") }
        tag_group::TagGroupEmpty { attributes: merged, {props.children} }
    }
}

#[component]
pub fn TagList(props: TagListProps) -> Element {
    let base = attributes!(div { class: "dx-tag-group-list" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/tag_group/style.css") }
        tag_group::TagList { attributes: merged, {props.children} }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct TagProps {
    pub value: ReadSignal<String>,
    #[props(default)]
    pub text_value: ReadSignal<Option<String>>,
    pub index: ReadSignal<usize>,
    #[props(default)]
    pub id: ReadSignal<Option<String>>,
    #[props(default)]
    pub disabled: ReadSignal<bool>,
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
    pub children: Element,
}

#[component]
pub fn Tag(props: TagProps) -> Element {
    let base = attributes!(div { class: "dx-tag-group-tag" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/tag_group/style.css") }
        tag_group::TagOption::<String> {
            value: props.value,
            text_value: props.text_value,
            disabled: props.disabled,
            id: props.id,
            index: props.index,
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn RemoveButton(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(button { class: "dx-tag-group-remove-button" });
    let merged = merge_attributes(vec![base, attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/tag_group/style.css") }
        tag_group::TagRemoveButton {
            attributes: merged,
            {children}
            X { size: "12px" }
        }
    }
}
