use super::super::component::*;
use dioxus::prelude::*;

#[component]
pub fn Demo() -> Element {
    let mut value = use_signal(String::new);

    rsx! {
        div { style: "display: flex; flex-direction: column; gap: 1rem; align-items: flex-start;",
            InputOtp {
                id: "otp-main",
                max_length: 6,
                aria_label: "One-time passcode",
                on_value_change: move |v: String| value.set(v),
                InputOtpGroup {
                    InputOtpSlot { index: 0 }
                    InputOtpSlot { index: 1 }
                    InputOtpSlot { index: 2 }
                }
                InputOtpSeparator {}
                InputOtpGroup {
                    InputOtpSlot { index: 3 }
                    InputOtpSlot { index: 4 }
                    InputOtpSlot { index: 5 }
                }
            }
            p { id: "input-otp-value", "Value: {value}" }

            InputOtp {
                id: "otp-disabled",
                max_length: 4,
                aria_label: "Disabled passcode",
                disabled: true,
                default_value: "12",
                InputOtpGroup {
                    InputOtpSlot { index: 0 }
                    InputOtpSlot { index: 1 }
                    InputOtpSlot { index: 2 }
                    InputOtpSlot { index: 3 }
                }
            }
        }
    }
}
