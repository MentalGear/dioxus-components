use super::super::component::*;
use dioxus::prelude::*;
use dioxus_icons::lucide::{ArrowUpRight, GitMerge};

#[component]
pub fn Demo() -> Element {
    rsx! {
        div {
            display: "flex",
            flex_direction: "row",
            flex_wrap: "wrap",
            align_items: "flex-start",
            gap: "0.75rem",

            // An icon-only button has no text, so it needs an accessible name (axe `button-name`).
            Button {
                variant: ButtonVariant::Outline,
                size: ButtonSize::Icon,
                aria_label: "Open",
                ArrowUpRight { size: "16px" }
            }

            Button {
                variant: ButtonVariant::Outline,
                size: ButtonSize::Icon,
                border_radius: "50%",
                aria_label: "Open in new tab",
                ArrowUpRight { size: "16px" }
            }

            Button {
                variant: ButtonVariant::Outline,
                size: ButtonSize::Sm,
                GitMerge { size: "16px" }
                "Merge"
            }
        }
    }
}
