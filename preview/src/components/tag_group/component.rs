use dioxus::prelude::*;
use dioxus_icons::lucide::X;
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
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/tag_group/style.css") }
        tag_group::TagGroup {
            class: "dx-tag-group",
            value: props.value,
            default_value: props.default_value,
            on_value_change: props.on_value_change,
            disabled: props.disabled,
            selectable: props.selectable,
            allow_empty_selection: props.allow_empty_selection,
            escape_clears_selection: props.escape_clears_selection,
            roving_loop: props.roving_loop,
            attributes: props.attributes,
            {props.children}
        }
    }
}

#[component]
pub fn TagGroupMulti(props: TagGroupMultiProps<String>) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/tag_group/style.css") }
        tag_group::TagGroupMulti {
            class: "dx-tag-group",
            values: props.values,
            default_values: props.default_values,
            on_values_change: props.on_values_change,
            disabled: props.disabled,
            selectable: props.selectable,
            allow_empty_selection: props.allow_empty_selection,
            escape_clears_selection: props.escape_clears_selection,
            roving_loop: props.roving_loop,
            attributes: props.attributes,
            {props.children}
        }
    }
}

#[component]
pub fn TagGroupLabel(props: TagGroupLabelProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/tag_group/style.css") }
        tag_group::TagGroupLabel {
            class: "dx-tag-group-label",
            id: props.id,
            attributes: props.attributes,
            {props.children}
        }
    }
}

#[component]
pub fn TagGroupEmpty(props: TagGroupEmptyProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/tag_group/style.css") }
        tag_group::TagGroupEmpty {
            class: "dx-tag-group-empty",
            attributes: props.attributes,
            {props.children}
        }
    }
}

#[component]
pub fn TagList(props: TagListProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/tag_group/style.css") }
        tag_group::TagList {
            class: "dx-tag-group-list",
            attributes: props.attributes,
            {props.children}
        }
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
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/tag_group/style.css") }
        tag_group::TagOption::<String> {
            class: "dx-tag-group-tag",
            value: props.value,
            text_value: props.text_value,
            disabled: props.disabled,
            id: props.id,
            index: props.index,
            attributes: props.attributes,
            {props.children}
        }
    }
}

#[component]
pub fn RemoveButton(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/tag_group/style.css") }
        tag_group::TagRemoveButton {
            class: "dx-tag-group-remove-button",
            attributes,
            {children}
            X { size: "12px" }
        }
    }
}
