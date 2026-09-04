use dioxus::prelude::*;
use dioxus_primitives::toggle_group::{self, ToggleGroupProps, ToggleItemProps};

// docs/backlog.md row 32: `#[css_module]` is gone -- see checkbox/component.rs's
// header comment for the full delivery-mechanism rationale (asset!() +
// document::Link, embedded in both exported entry points so a
// `dx components add toggle_group`-copied component needs no extra wiring).
//
// This component is also on the row-32 "not already namespaced" lane: its
// item class used to be the bare `dx-toggle-item`, not
// `dx-toggle-group-item`. `#[css_module]`'s hash kept it collision-safe
// regardless, but a plain `dx-toggle-item` isn't provably this component's
// own once the hash is gone. Renamed to `dx-toggle-group-item` in the same
// change that drops the macro, in both this file and `style.css` -- see
// `scripts/check-dx-class-prefix.sh`.
#[component]
pub fn ToggleGroup(props: ToggleGroupProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/toggle_group/style.css") }
        toggle_group::ToggleGroup {
            class: "dx-toggle-group",
            default_pressed: props.default_pressed,
            pressed: props.pressed,
            on_pressed_change: props.on_pressed_change,
            disabled: props.disabled,
            allow_multiple_pressed: props.allow_multiple_pressed,
            horizontal: props.horizontal,
            roving_loop: props.roving_loop,
            attributes: props.attributes,
            {props.children}
        }
    }
}

#[component]
pub fn ToggleItem(props: ToggleItemProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/toggle_group/style.css") }
        toggle_group::ToggleItem {
            class: "dx-toggle-group-item",
            index: props.index,
            disabled: props.disabled,
            attributes: props.attributes,
            {props.children}
        }
    }
}
