use crate::components::{
    button::{Button, ButtonSize, ButtonVariant},
    drawer::{
        Drawer, DrawerClose, DrawerContent, DrawerDescription, DrawerFooter, DrawerHandle,
        DrawerHeader, DrawerSide, DrawerTitle,
    },
};
use dioxus::prelude::*;
use dioxus_icons::lucide::{Minus, Plus};

const GOAL_STEP: i32 = 10;
const MIN_GOAL: i32 = 200;
const MAX_GOAL: i32 = 400;

#[component]
pub fn Demo() -> Element {
    let mut open = use_signal(|| false);
    let mut top_open = use_signal(|| false);
    let mut goal = use_signal(|| 350);

    rsx! {
        div { display: "flex", gap: "0.5rem",
            Button { onclick: move |_| open.set(true), "Move Goal" }
            Button {
                variant: ButtonVariant::Outline,
                onclick: move |_| top_open.set(true),
                "Open from Top",
            }
        }

        Drawer { open: open(), on_open_change: move |v| open.set(v),
            DrawerContent {
                DrawerHandle {}
                DrawerHeader {
                    DrawerTitle { "Move Goal" }
                    DrawerDescription { "Set your daily activity goal." }
                }

                div {
                    display: "flex",
                    align_items: "center",
                    justify_content: "center",
                    gap: "1.5rem",
                    padding: "0 1rem 1rem",

                    Button {
                        variant: ButtonVariant::Outline,
                        size: ButtonSize::Icon,
                        border_radius: "50%",
                        aria_label: "Decrease goal",
                        disabled: goal() <= MIN_GOAL,
                        onclick: move |_| goal.set((goal() - GOAL_STEP).max(MIN_GOAL)),
                        Minus { size: "16px" }
                    }

                    div {
                        display: "flex",
                        flex_direction: "column",
                        align_items: "center",
                        flex: "1 1 0%",

                        div { font_size: "3rem", font_weight: "700", line_height: "1", "{goal()}" }
                        div {
                            margin_top: "0.5rem",
                            color: "var(--secondary-color-5)",
                            font_size: "0.75rem",
                            text_transform: "uppercase",
                            "Calories/day"
                        }
                    }

                    Button {
                        variant: ButtonVariant::Outline,
                        size: ButtonSize::Icon,
                        border_radius: "50%",
                        aria_label: "Increase goal",
                        disabled: goal() >= MAX_GOAL,
                        onclick: move |_| goal.set((goal() + GOAL_STEP).min(MAX_GOAL)),
                        Plus { size: "16px" }
                    }
                }

                DrawerFooter {
                    Button { onclick: move |_| open.set(false), "Submit" }
                    DrawerClose {
                        as: |attributes| rsx! {
                            Button { variant: ButtonVariant::Outline, attributes, "Cancel" }
                        },
                    }
                }
            }
        }

        Drawer {
            open: top_open(),
            on_open_change: move |v| top_open.set(v),
            side: DrawerSide::Top,
            DrawerContent {
                DrawerHeader {
                    DrawerTitle { "Notifications" }
                    DrawerDescription { "This drawer slides in from the top edge instead." }
                }
                DrawerFooter {
                    DrawerClose { "Close" }
                }
            }
        }
    }
}
