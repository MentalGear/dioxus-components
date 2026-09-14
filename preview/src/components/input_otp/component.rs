use dioxus::prelude::*;
use dioxus_primitives::input_otp::{
    self, InputOtpGroupProps, InputOtpProps, InputOtpSeparatorProps, InputOtpSlotProps,
};

/// Composes [`input_otp::InputOtp`] (`dioxus_primitives`), attaching this
/// crate's `dx-input-otp-input` theming class to the primitive's real
/// (invisible) `<input>` -- see that primitive's module doc for why the
/// accessible surface lives on the real input rather than on a wrapper
/// `div`: it is simultaneously the interactive *and* the form-submittable
/// control, unlike `Checkbox`'s `<button>` + hidden `BubbleInput` split.
#[component]
pub fn InputOtp(props: InputOtpProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/input_otp/style.css") }
        input_otp::InputOtp {
            class: "dx-input-otp-input",
            value: props.value,
            default_value: props.default_value,
            on_value_change: props.on_value_change,
            on_complete: props.on_complete,
            max_length: props.max_length,
            disabled: props.disabled,
            required: props.required,
            name: props.name,
            id: props.id,
            attributes: props.attributes,
            {props.children}
        }
    }
}

/// Lays out one cluster of [`InputOtpSlot`]s -- composes
/// [`input_otp::InputOtpGroup`].
#[component]
pub fn InputOtpGroup(props: InputOtpGroupProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/input_otp/style.css") }
        input_otp::InputOtpGroup {
            class: "dx-input-otp-group",
            attributes: props.attributes,
            {props.children}
        }
    }
}

/// A single passcode digit box -- composes [`input_otp::InputOtpSlot`].
#[component]
pub fn InputOtpSlot(props: InputOtpSlotProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/input_otp/style.css") }
        input_otp::InputOtpSlot {
            class: "dx-input-otp-slot",
            index: props.index,
            attributes: props.attributes,
        }
    }
}

/// A visual divider between [`InputOtpGroup`]s -- composes
/// [`input_otp::InputOtpSeparator`]. `props.children` is forwarded only
/// when the caller actually supplied custom separator content, so an
/// omitted body still falls through to the primitive's own default ("•")
/// rather than this wrapper needing to duplicate it.
#[component]
pub fn InputOtpSeparator(props: InputOtpSeparatorProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/input_otp/style.css") }
        if let Some(children) = props.children {
            input_otp::InputOtpSeparator {
                class: "dx-input-otp-separator",
                attributes: props.attributes,
                {children}
            }
        } else {
            input_otp::InputOtpSeparator {
                class: "dx-input-otp-separator",
                attributes: props.attributes,
            }
        }
    }
}
