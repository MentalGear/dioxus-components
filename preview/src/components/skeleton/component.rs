use dioxus::prelude::*;
use dioxus_primitives::activity::use_motion;
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};

#[component]
pub fn Skeleton(#[props(extends=GlobalAttributes)] attributes: Vec<Attribute>) -> Element {
    let base = attributes!(div {
        class: "dx-skeleton",
    });
    // The pulse is paused while off-screen or in a hidden tab (`dx-components-theme.css`).
    let motion = use_motion();
    let merged = merge_attributes(vec![base, attributes, motion.attributes()]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/skeleton/style.css") }
        div { ..merged }
    }
}
