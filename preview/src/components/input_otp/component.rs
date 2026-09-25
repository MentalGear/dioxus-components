use dioxus::prelude::*;
use dioxus_primitives::dioxus_attributes::attributes;
use dioxus_primitives::input_otp::{
    self, InputOtpGroupProps, InputOtpProps, InputOtpSeparatorProps, InputOtpSlotProps,
};
use dioxus_primitives::merge_attributes;

/// Composes [`input_otp::InputOtp`] (`dioxus_primitives`), attaching this
/// crate's `dx-input-otp-input` theming class to the primitive's real
/// (invisible) `<input>` -- see that primitive's module doc for why the
/// accessible surface lives on the real input rather than on a wrapper
/// `div`: it is simultaneously the interactive *and* the form-submittable
/// control, unlike `Checkbox`'s `<button>` + hidden `BubbleInput` split.
#[component]
pub fn InputOtp(props: InputOtpProps) -> Element {
    let base = attributes!(input { class: "dx-input-otp-input" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/input_otp/style.css") }
        input_otp::InputOtp {
            value: props.value,
            default_value: props.default_value,
            on_value_change: props.on_value_change,
            on_complete: props.on_complete,
            max_length: props.max_length,
            disabled: props.disabled,
            required: props.required,
            name: props.name,
            id: props.id,
            attributes: merged,
            {props.children}
        }
    }
}

/// Lays out one cluster of [`InputOtpSlot`]s -- composes
/// [`input_otp::InputOtpGroup`].
#[component]
pub fn InputOtpGroup(props: InputOtpGroupProps) -> Element {
    let base = attributes!(div { class: "dx-input-otp-group" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/input_otp/style.css") }
        input_otp::InputOtpGroup { attributes: merged, {props.children} }
    }
}

/// A single passcode digit box -- composes [`input_otp::InputOtpSlot`].
#[component]
pub fn InputOtpSlot(props: InputOtpSlotProps) -> Element {
    let base = attributes!(div { class: "dx-input-otp-slot" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/input_otp/style.css") }
        input_otp::InputOtpSlot { index: props.index, attributes: merged }
    }
}

/// A visual divider between [`InputOtpGroup`]s -- composes
/// [`input_otp::InputOtpSeparator`]. `props.children` is forwarded only
/// when the caller actually supplied custom separator content, so an
/// omitted body still falls through to the primitive's own default ("•")
/// rather than this wrapper needing to duplicate it.
#[component]
pub fn InputOtpSeparator(props: InputOtpSeparatorProps) -> Element {
    let base = attributes!(div { class: "dx-input-otp-separator" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/input_otp/style.css") }
        if let Some(children) = props.children {
            input_otp::InputOtpSeparator { attributes: merged, {children} }
        } else {
            input_otp::InputOtpSeparator { attributes: merged }
        }
    }
}
