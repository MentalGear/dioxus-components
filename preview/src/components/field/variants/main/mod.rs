use super::super::component::*;
use crate::components::checkbox::Checkbox;
use crate::components::input::Input;
use dioxus::prelude::*;

#[component]
pub fn Demo() -> Element {
    rsx! {
        div { style: "width: 100%; max-width: 24rem;",
            FieldSet {
                FieldLegend { "Profile" }
                FieldGroup {
                    Field {
                        FieldLabel { html_for: "field-demo-username", "Username" }
                        Input { id: "field-demo-username", placeholder: "shadcn" }
                        FieldDescription { "This is your public display name." }
                    }
                    Field { invalid: true,
                        FieldLabel { html_for: "field-demo-email", "Email" }
                        Input {
                            id: "field-demo-email",
                            "aria-invalid": "true",
                            value: "not-an-email",
                        }
                        FieldError { "Enter a valid email address." }
                    }
                    FieldSeparator {}
                    Field { orientation: FieldOrientation::Horizontal,
                        Checkbox { id: "field-demo-marketing" }
                        FieldContent {
                            FieldLabel { html_for: "field-demo-marketing", "Marketing emails" }
                            FieldDescription { "Receive emails about new products and features." }
                        }
                    }
                }
            }
        }
    }
}
