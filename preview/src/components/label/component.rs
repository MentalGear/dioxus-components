use dioxus::prelude::*;
use dioxus_primitives::dioxus_attributes::attributes;
use dioxus_primitives::label::{self, LabelProps};
use dioxus_primitives::merge_attributes;

#[component]
pub fn Label(props: LabelProps) -> Element {
    // `class: "dx-label"` and `attributes: props.attributes` used to be passed
    // to the primitive unmerged: `attributes` extends `GlobalAttributes`, so
    // the ad-hoc `class` here and any `class` already inside `props.attributes`
    // (e.g. Field's `dx-field-label` override) landed as two separate `class`
    // entries in the final Vec -- rendered as two literal `class="..."`
    // attributes on the same `<label>`, a WHATWG duplicate-attribute parse
    // error (found by the SSG lane's hydration-parity Rule 4 on the field
    // demo). Merge them the same way Table/Combobox do.
    let base = attributes!(label { class: "dx-label" });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/label/style.css") }
        label::Label {
            html_for: props.html_for,
            attributes: merged,
            {props.children}
        }
    }
}
