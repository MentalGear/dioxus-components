use dioxus::prelude::*;
use dioxus_primitives::dioxus_attributes::attributes;
use dioxus_primitives::merge_attributes;
use dioxus_primitives::slider::{self, RangeSliderProps, SliderProps};

#[component]
pub fn Slider(props: SliderProps) -> Element {
    let base = attributes!(div { class: "dx-slider" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/slider/style.css") }
        slider::Slider {
            value: props.value,
            default_value: props.default_value,
            min: props.min,
            max: props.max,
            step: props.step,
            disabled: props.disabled,
            horizontal: props.horizontal,
            inverted: props.inverted,
            on_value_change: props.on_value_change,
            label: props.label,
            attributes: merged,
            slider::SliderTrack { class: "dx-slider-track",
                slider::SliderRange { class: "dx-slider-range" }
                slider::SliderThumb { class: "dx-slider-thumb" }
            }
        }
    }
}

#[component]
pub fn RangeSlider(props: RangeSliderProps) -> Element {
    let base = attributes!(div { class: "dx-slider" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/slider/style.css") }
        slider::RangeSlider {
            value: props.value,
            default_value: props.default_value,
            min: props.min,
            max: props.max,
            step: props.step,
            disabled: props.disabled,
            horizontal: props.horizontal,
            inverted: props.inverted,
            on_value_change: props.on_value_change,
            label: props.label,
            attributes: merged,
            slider::SliderTrack { class: "dx-slider-track",
                slider::SliderRange { class: "dx-slider-range" }
                slider::SliderThumb { class: "dx-slider-thumb", index: 0usize }
                slider::SliderThumb { class: "dx-slider-thumb", index: 1usize }
            }
        }
    }
}
