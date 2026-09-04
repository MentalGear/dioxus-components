use dioxus::prelude::*;
use dioxus_primitives::color_picker::{
    self, Color, ColorAreaProps, ColorPickerContext,
};
use dioxus_primitives::popover;
use dioxus_primitives::use_controlled;
use dioxus_primitives::label::Label;
use dioxus_primitives::slider::*;
use palette::{encoding, FromColor, Hsv, IntoColor, RgbHue, Srgb};

use crate::components::input::Input;

// docs/backlog.md row 32: `#[css_module]` is gone -- see checkbox/component.rs's
// header comment for the full delivery-mechanism rationale (asset!() +
// document::Link, embedded in the wrapper so a `dx components add
// color_picker`-copied component needs no extra wiring).
//
// This component is also on the row-32 "not already namespaced" lane: before
// hashing came off, several of this sheet's classes were spelled
// `dx-color-*` (`dx-color-swatch`, `dx-color-slider*`, `dx-color-area*`,
// `dx-color-field*`) rather than `dx-color-picker-*`. `#[css_module]`'s hash
// suffix kept them collision-safe regardless, but the moment that hash is
// gone a plain `dx-color-swatch` is indistinguishable from any other
// component that might someday define the same short name. All such classes
// were renamed to carry the full `dx-color-picker-` namespace (e.g.
// `dx-color-swatch` -> `dx-color-picker-swatch`) in the same change that
// drops the macro, in both this file and `style.css` -- see
// `scripts/check-dx-class-prefix.sh`, which enforces this repo-wide.
fn format_color_hex(color: Color) -> String {
    format!("#{color:X}")
}

#[derive(Clone, Copy)]
struct ColorPickerRootContext {
    open: Memo<bool>,
    disabled: ReadSignal<bool>,
    color: ReadSignal<Hsv<encoding::Srgb, f64>>,
}

/// The props for the [`ColorPickerRoot`] component.
#[derive(Props, Clone, PartialEq)]
pub struct ColorPickerRootProps {
    /// The selected color
    #[props(default)]
    pub color: ReadSignal<Hsv<encoding::Srgb, f64>>,

    /// Callback when color changes
    #[props(default)]
    pub on_color_change: Callback<Hsv<encoding::Srgb, f64>>,

    /// Whether the color picker is disabled
    #[props(default)]
    pub disabled: ReadSignal<bool>,

    /// The controlled open state of the popover.
    pub open: ReadSignal<Option<bool>>,

    /// The default open state when uncontrolled.
    #[props(default)]
    pub default_open: bool,

    /// Callback fired when the open state changes.
    #[props(default)]
    pub on_open_change: Callback<bool>,

    /// Additional attributes to extend the color picker element
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the color picker element
    pub children: Element,
}

#[component]
pub fn ColorPickerRoot(props: ColorPickerRootProps) -> Element {
    let (open, set_open) = use_controlled(props.open, props.default_open, props.on_open_change);

    use_context_provider(|| ColorPickerRootContext {
        open,
        disabled: props.disabled,
        color: props.color,
    });

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/color_picker/style.css") }
        color_picker::ColorPicker {
            class: "dx-color-picker",
            color: props.color,
            on_color_change: props.on_color_change,
            disabled: props.disabled,
            attributes: props.attributes,
            popover::PopoverRoot {
                is_modal: false,
                open: Some(open()),
                on_open_change: move |v| set_open.call(v),
                {props.children}
            }
        }
    }
}

/// The props for the [`ColorPicker`] component.
#[derive(Props, Clone, PartialEq)]
pub struct ColorPickerProps {
    /// The selected color
    #[props(default)]
    pub color: ReadSignal<Hsv<encoding::Srgb, f64>>,

    /// Callback when color changes
    #[props(default)]
    pub on_color_change: Callback<Hsv<encoding::Srgb, f64>>,

    /// Whether the color picker is disabled
    #[props(default)]
    pub disabled: ReadSignal<bool>,

    /// Optional label on the trigger button
    #[props(default)]
    pub label: Option<String>,

    /// The controlled open state of the popover.
    pub open: ReadSignal<Option<bool>>,

    /// The default open state when uncontrolled.
    #[props(default)]
    pub default_open: bool,

    /// Callback fired when the open state changes.
    #[props(default)]
    pub on_open_change: Callback<bool>,

    /// Additional attributes to extend the color picker element
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// Additional content to append to the default color picker popover
    pub children: Element,
}

#[component]
pub fn ColorPicker(props: ColorPickerProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/color_picker/style.css") }
        ColorPickerRoot {
            color: props.color,
            on_color_change: props.on_color_change,
            disabled: props.disabled,
            open: props.open,
            default_open: props.default_open,
            on_open_change: props.on_open_change,
            attributes: props.attributes,
            ColorPickerTrigger {
                label: props.label,
            }
            ColorPickerPopover {
                ColorPickerSelect {}
                {props.children}
            }
        }
    }
}

/// The props for the [`ColorPickerTrigger`] component.
#[derive(Props, Clone, PartialEq)]
pub struct ColorPickerTriggerProps {
    /// Optional label on the trigger button
    #[props(default)]
    pub label: Option<String>,

    /// Additional attributes to extend the trigger button
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// Additional content to render inside the trigger button
    pub children: Element,
}

#[component]
pub fn ColorPickerTrigger(props: ColorPickerTriggerProps) -> Element {
    let ctx = use_context::<ColorPickerRootContext>();
    let aria_hex = use_memo(move || {
        let rgb: Color = Srgb::<f64>::from_color((ctx.color)()).into_format();
        format_color_hex(rgb)
    });

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/color_picker/style.css") }
        popover::PopoverTrigger {
            class: "dx-color-picker-button",
            disabled: if (ctx.disabled)() { true },
            aria_label: format!("Color picker {aria_hex}"),
            aria_expanded: (ctx.open)(),
            attributes: props.attributes,
            ColorSwatch { color: ctx.color }
            if let Some(label) = props.label { span { {label} } }
            {props.children}
        }
    }
}

/// The props for the [`ColorPickerPopover`] component.
#[derive(Props, Clone, PartialEq)]
pub struct ColorPickerPopoverProps {
    /// Additional attributes to extend the popover content
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the color picker popover
    pub children: Element,
}

#[component]
pub fn ColorPickerPopover(props: ColorPickerPopoverProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/color_picker/style.css") }
        popover::PopoverContent {
            class: "dx-color-picker-popover".to_string(),
            attributes: props.attributes,
            {props.children}
        }
    }
}

/// The props for the [`ColorField`] component.
#[derive(Props, Clone, PartialEq)]
pub struct ColorFieldProps {
    /// Optional label above the input field
    #[props(default)]
    pub label: Option<String>,

    /// Optional props for the text description element
    #[props(default)]
    pub description: Option<String>,

    /// Additional attributes to extend the color field element
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the color field element
    pub children: Element,
}

/// # ColorField
///
/// The [`ColorField`] allows users to edit a hex color. Reads and writes the
/// current color through the surrounding [`ColorPickerContext`].
#[component]
fn ColorField(props: ColorFieldProps) -> Element {
    let ctx = use_context::<ColorPickerContext>();
    let hex_from_hsv = |hsv: Hsv<encoding::Srgb, f64>| {
        let rgb: Color = Srgb::<f64>::from_color(hsv).into_format();
        format_color_hex(rgb)
    };
    let emit_rgb = move |rgb: Color| {
        let hsv: Hsv<encoding::Srgb, f64> = rgb.into_format::<f64>().into_color();
        ctx.set_color(hsv);
    };

    let mut value = use_signal(|| hex_from_hsv(ctx.color()));

    // Synchronize local text with external color changes. Only overwrite
    // when the field already holds a parseable hex — otherwise the user is
    // mid-edit and replacing their text would clobber the input.
    use_effect(move || {
        let external = ctx.color();
        let current = value();
        if let Ok(parsed) = current.parse::<Color>() {
            let external_rgb: Color = Srgb::<f64>::from_color(external).into_format();
            if parsed != external_rgb {
                value.set(hex_from_hsv(external));
            }
        } else if current.is_empty() {
            value.set(hex_from_hsv(external));
        }
    });

    rsx! {
        div {
            class: "dx-color-picker-field-container",
            ..props.attributes,
            if let Some(label) = props.label {
                Label {
                    html_for: "color_field",
                    class: "dx-color-picker-slider-title",
                    {label}
                }
            }
            Input {
                id: "color_field",
                placeholder: "Enter a color",
                value: "{value}",
                oninput: move |e: FormEvent| {
                    let mut input = e.value();

                    // Sanitize input: allow only '#' and hex digits, length limit.
                    input.retain(|c| c == '#' || c.is_ascii_hexdigit());

                    // Automatically prepend '#' if missing.
                    if !input.starts_with('#') && !input.is_empty() {
                        input.insert(0, '#');
                    }

                    input.truncate(7);
                    value.set(input.to_uppercase());

                    if let Ok(parsed) = input.parse::<Color>() {
                        emit_rgb(parsed);
                    }
                },
            }
            if let Some(text) = props.description {
                span { class: "dx-color-picker-field-description", {text} }
            }
            {props.children}
        }
    }
}

/// The props for the [`ColorSwatch`] component.
#[derive(Props, Clone, PartialEq)]
pub struct ColorSwatchProps {
    /// The selected color
    #[props(default)]
    pub color: ReadSignal<Hsv<encoding::Srgb, f64>>,

    /// Additional attributes to extend the color swatch element
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the color swatch element
    pub children: Element,
}

/// # ColorSwatch
///
/// The [`ColorSwatch`] displays a preview of a selected color.
#[component]
fn ColorSwatch(props: ColorSwatchProps) -> Element {
    let hex_color = use_memo(move || {
        let rgb: Color = Srgb::<f64>::from_color((props.color)()).into_format();
        format_color_hex(rgb)
    });

    rsx! {
        div {
            role: "img",
            aria_label: format!("Selected color {hex_color}"),
            class: "dx-color-picker-swatch",
            style: "--swatch-color: {hex_color}",
            ..props.attributes,
            {props.children}
        }
    }
}

/// The props for the [`ColorSlider`] component.
#[derive(Props, Clone, PartialEq)]
pub struct ColorSliderProps {
    pub title: ReadSignal<String>,

    /// Additional attributes to extend the color slider element
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the color slider element
    pub children: Element,
}

/// # ColorSlider
///
/// The [`ColorSlider`] allows users to adjust the hue of the color held by
/// the surrounding [`ColorPickerContext`].
#[component]
fn ColorSlider(props: ColorSliderProps) -> Element {
    let ctx = use_context::<ColorPickerContext>();
    let mut current_hue = use_signal(|| ctx.color().hue.into_positive_degrees());

    let thumb_color = use_memo(move || {
        Srgb::<f64>::from_color(Hsv::<encoding::Srgb, f64>::new(
            RgbHue::new(current_hue()),
            1.0,
            1.0,
        ))
        .into_format()
    });

    use_effect(move || {
        let value = ctx.color().hue.into_positive_degrees();
        let current = current_hue();

        let is_wrap_around = (value - current).abs() > 350.0;

        // Update the signal only if this is an actual new position,
        // and not a "flip" of the circle by the palette library.
        if !is_wrap_around && value != current {
            current_hue.set(value);
        }
    });

    let display_value = {
        let value = current_hue();
        format!("{value:.2}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
            + "°"
    };

    rsx! {

        div {
            class: "dx-color-picker-slider-container",
            ..props.attributes,
            label { class: "dx-color-picker-slider-title", {props.title} }
            output { class: "dx-color-picker-slider-output", "{display_value}" }
            Slider {
                class: "dx-color-picker-slider",
                label: "Color Slider",
                horizontal: true,
                max: 360.0,
                value: current_hue(),
                on_value_change: move |h: f64| {
                    // Allow the value to be exactly 360.0
                    // The palette will understand that 360.0 == 0.0, but the signal will remain 360.0 for the UI.
                    current_hue.set(h);
                    ctx.set_hue(h);
                },
                SliderTrack {
                    class: "dx-color-picker-slider-track",
                    SliderThumb {
                        class: "dx-color-picker-slider-thumb",
                        aria_label: "Hue",
                        aria_valuetext: format!("{:.0}°", current_hue()),
                        background_color: format_color_hex(thumb_color()),
                    }
                }
            }
            {props.children}
        }
    }
}

#[component]
fn ColorArea(props: ColorAreaProps) -> Element {
    rsx! {
        color_picker::ColorArea {
            class: "dx-color-picker-area-container",
            step: props.step,
            attributes: props.attributes,
            color_picker::AreaTrack {
                class: "dx-color-picker-area-track",
                color_picker::AreaThumb {
                    class: "dx-color-picker-area-thumb",
                    color_picker::AreaThumbSaturationInput {
                        class: "dx-color-picker-area-input",
                    }
                    color_picker::AreaThumbValueInput {
                        class: "dx-color-picker-area-input",
                    }
                }
            }
            {props.children}
        }
    }
}

/// The props for the [`ColorPickerSelect`] component.
#[derive(Props, Clone, PartialEq)]
pub struct ColorPickerSelectProps {
    /// Additional attributes to extend the color picker select element
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the color picker select element
    pub children: Element,
}

#[component]
pub fn ColorPickerSelect(props: ColorPickerSelectProps) -> Element {
    let ctx = use_context::<ColorPickerContext>();

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/color_picker/style.css") }
        div {
            class: "dx-color-picker-dialog",
            ..props.attributes,
            ColorArea {}
            ColorSlider { title: "Hue" }
            div {
                class: "dx-color-picker-input",
                ColorField { label: "Hex" }
                ColorSwatch { color: ctx.color() }
            }
        }
    }
}
