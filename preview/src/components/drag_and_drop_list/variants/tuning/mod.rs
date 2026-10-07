use super::super::component::*;
use crate::components::slider::Slider;
use dioxus::prelude::*;

/// Both tuning options, driven live: `ghost_opacity` (the dragged item's
/// ghost) and `drop_gap` (the room the other items make at the drop slot).
/// They start at the component's own defaults (0.9 and 25px).
#[component]
pub fn Demo() -> Element {
    let mut ghost_opacity = use_signal(|| 0.9_f64);
    let mut drop_gap = use_signal(|| 25.0_f64);

    let items: Vec<Element> = [
        ("review", "Review pull request"),
        ("deploy", "Deploy to staging"),
        ("notes", "Write release notes"),
        ("triage", "Triage new issues"),
        ("retro", "Plan the retrospective"),
        ("backup", "Verify nightly backups"),
    ]
    .map(|(key, label)| {
        rsx! {
            span { key: "{key}", "{label}" }
        }
    })
    .to_vec();

    rsx! {
        div {
            style: "display: flex; width: 100%; max-width: 460px; flex-direction: column; margin: 0 auto; gap: 20px;",
            div { style: "display: flex; flex-direction: column; gap: 14px;",
                div { style: "display: flex; flex-direction: column; gap: 6px;",
                    div {
                        style: "display: flex; justify-content: space-between; font-size: 13px; font-weight: 500;",
                        span { "Ghost opacity" }
                        span {
                            style: "color: var(--dx-muted-foreground); font-variant-numeric: tabular-nums;",
                            "{ghost_opacity:.2}"
                        }
                    }
                    Slider {
                        label: "Ghost opacity",
                        horizontal: true,
                        min: 0.1,
                        max: 1.0,
                        step: 0.05,
                        default_value: 0.9,
                        on_value_change: move |value: f64| ghost_opacity.set(value),
                    }
                }
                div { style: "display: flex; flex-direction: column; gap: 6px;",
                    div {
                        style: "display: flex; justify-content: space-between; font-size: 13px; font-weight: 500;",
                        span { "Drop gap" }
                        span {
                            style: "color: var(--dx-muted-foreground); font-variant-numeric: tabular-nums;",
                            "{drop_gap:.0}px"
                        }
                    }
                    Slider {
                        label: "Drop gap",
                        horizontal: true,
                        min: 0.0,
                        max: 60.0,
                        step: 5.0,
                        default_value: 25.0,
                        on_value_change: move |value: f64| drop_gap.set(value),
                    }
                }
            }
            p {
                style: "margin: 0; color: var(--dx-muted-foreground); font-size: 12px; line-height: 1.4;",
                "Drag an item, or press Enter on one and use the arrow keys, to see the ghost and the gap."
            }
            // Room below for the last items, which move with a transform and
            // so spill past the list's own box by up to the gap.
            div { style: "padding-block-end: 64px;",
                DragAndDropList {
                    items,
                    ghost_opacity: ghost_opacity() as f32,
                    drop_gap: format!("{}px", drop_gap()),
                }
            }
        }
    }
}
