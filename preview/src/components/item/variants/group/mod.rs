use super::super::component::*;
use crate::components::avatar::{ImageAvatar, AvatarImageSize};
use crate::components::button::{Button, ButtonVariant};
use dioxus::prelude::*;
use dioxus_icons::lucide::Plus;

// Bundled illustrations (`preview/assets/avatars`, CC0 -- see its README) for
// invented people: the demo is local, offline-safe, and shows no real person.
const PEOPLE: &[(&str, &str, &str, Asset)] = &[
    (
        "Sarah Chen",
        "sarah.chen@example.com",
        "SC",
        asset!("/assets/avatars/sarah-chen.svg"),
    ),
    (
        "Marcus Wright",
        "marcus.wright@example.com",
        "MW",
        asset!("/assets/avatars/marcus-wright.svg"),
    ),
    (
        "Lena Park",
        "lena.park@example.com",
        "LP",
        asset!("/assets/avatars/lena-park.svg"),
    ),
];

#[component]
pub fn Demo() -> Element {
    rsx! {
        div {
            display: "flex",
            flex_direction: "column",
            width: "100%",
            max_width: "28rem",

            ItemGroup {
                for (i , (name , email , initials , avatar)) in PEOPLE.iter().enumerate() {
                    Item {
                        ItemMedia {
                            ImageAvatar {
                                size: AvatarImageSize::Small,
                                src: "{avatar}",
                                alt: "{name}",
                                "{initials}"
                            }
                        }
                        ItemContent {
                            ItemTitle { "{name}" }
                            ItemDescription { "{email}" }
                        }
                        ItemActions {
                            Button {
                                variant: ButtonVariant::Ghost,
                                aria_label: "Add {name}",
                                PlusIcon {}
                            }
                        }
                    }
                    if i + 1 < PEOPLE.len() {
                        ItemSeparator {}
                    }
                }
            }
        }
    }
}

#[component]
fn PlusIcon() -> Element {
    rsx! {
        Plus { size: "16" }
    }
}
