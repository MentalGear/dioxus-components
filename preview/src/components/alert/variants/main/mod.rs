use super::super::component::*;
use dioxus::prelude::*;
use dioxus_icons::lucide::{CircleAlert, Info};

#[component]
pub fn Demo() -> Element {
    rsx! {
        div { style: "display: flex; flex-direction: column; gap: 1rem; width: 100%; max-width: 24rem;",
            Alert {
                Info {}
                AlertTitle { "You can add components to your app" }
                AlertDescription { "Run the CLI to add components to your project." }
            }
            Alert { variant: AlertVariant::Destructive,
                CircleAlert {}
                AlertTitle { "Unable to process your payment." }
                AlertDescription { "Please verify your billing information and try again." }
            }
        }
    }
}
