use dioxus::prelude::*;
use dioxus_primitives::dioxus_attributes::attributes;
use dioxus_primitives::merge_attributes;
use dioxus_primitives::switch::{self, SwitchProps};

#[component]
pub fn Switch(props: SwitchProps) -> Element {
    let base = attributes!(button { class: "dx-switch" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/switch/style.css") }
        switch::Switch {
            checked: props.checked,
            default_checked: props.default_checked,
            disabled: props.disabled,
            required: props.required,
            name: props.name,
            value: props.value,
            on_checked_change: props.on_checked_change,
            attributes: merged,
            switch::SwitchThumb { class: "dx-switch-thumb" }
        }
    }
}
