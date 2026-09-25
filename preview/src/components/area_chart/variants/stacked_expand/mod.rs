use super::super::component::*;
use crate::components::card::{Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle};
use dioxus::prelude::*;
use dioxus_icons::lucide::TrendingUp;

/// Ports
/// `$S/refs/ui/apps/v4/registry/new-york-v4/charts/chart-area-stacked-expand.tsx`
/// (shadcn's `chart-area-stacked-expand` demo, `stackOffset="expand"`):
/// three series, percent-stacked via `AreaOptions::stack_mode:
/// StackMode::Expand` (`primitives/src/chart/engine/stack.rs`'s
/// `stack_with_mode`, d3 `offset/expand.js` semantics) so every month's
/// stack reaches exactly 100% regardless of its raw total.
///
/// The grid is shown (unlike an earlier draft of this demo): stage-2's
/// §4(c) construction made `components::layout::build` itself
/// `StackMode`-aware, so `Chart`'s shared grid lines/y-axis ticks now
/// reflect the same normalized 0..1 domain the marks draw against -- no
/// more mismatch to work around by hiding the grid.
fn generate_data() -> Vec<ChartDatum> {
    const ROWS: [(&str, f64, f64, f64); 6] = [
        ("January", 186.0, 80.0, 45.0),
        ("February", 305.0, 200.0, 100.0),
        ("March", 237.0, 120.0, 150.0),
        ("April", 73.0, 190.0, 50.0),
        ("May", 209.0, 130.0, 100.0),
        ("June", 214.0, 140.0, 160.0),
    ];
    ROWS.iter()
        .map(|&(label, desktop, mobile, other)| ChartDatum {
            label: label.to_string(),
            values: vec![Some(desktop), Some(mobile), Some(other)],
            ..Default::default()
        })
        .collect()
}

#[component]
pub fn Demo() -> Element {
    let config = ChartConfig::new()
        .series("desktop", "Desktop", "var(--dx-chart-1)")
        .series("mobile", "Mobile", "var(--dx-chart-2)")
        .series("other", "Other", "var(--dx-chart-3)");

    rsx! {
        div { class: "dx-area-chart-gallery",
            Card {
                CardHeader {
                    CardTitle { "Area Chart - Stacked Expanded" }
                    CardDescription { "Showing total visitors for the last 6 months" }
                }
                CardContent {
                    ChartContainer { config, data: generate_data(), kind: ChartKind::Area,
                        Chart {
                            aria_label: "Visitors by month, desktop, mobile, and other, as a percentage of the month's total",
                            stacked: true,
                            area: AreaOptions {
                                stack_mode: StackMode::Expand,
                                ..Default::default()
                            },
                        }
                        ChartTooltip {}
                    }
                }
                CardFooter {
                    div { style: "display: flex; width: 100%; align-items: flex-start; gap: 8px; font-size: var(--dx-text-sm);",
                        div { style: "display: grid; gap: 8px;",
                            div { style: "display: flex; align-items: center; gap: 8px; font-weight: 600;",
                                "Trending up by 5.2% this month"
                                TrendingUp { size: "16px" }
                            }
                            div { style: "color: var(--secondary-color-5);", "January - June 2024" }
                        }
                    }
                }
            }
        }
    }
}
