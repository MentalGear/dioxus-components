use super::super::component::*;
use crate::components::button::{Button, ButtonVariant};
use dioxus::prelude::*;
use dioxus_icons::lucide::Inbox;

#[component]
pub fn Demo() -> Element {
    rsx! {
        Empty {
            style: "border: 1px dashed var(--primary-color-6); width: 100%; max-width: 28rem;",
            EmptyHeader {
                EmptyMedia { variant: EmptyMediaVariant::Icon,
                    Inbox {}
                }
                EmptyTitle { "No messages yet" }
                EmptyDescription { "You don't have any messages. Start a conversation to see it here." }
            }
            EmptyContent {
                Button { variant: ButtonVariant::Outline, "New message" }
            }
        }
    }
}
