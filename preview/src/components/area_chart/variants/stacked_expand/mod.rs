use super::super::component::*;
use crate::components::card::{Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle};
use dioxus::prelude::*;
use dioxus_icons::lucide::TrendingUp;

/// Three series stacked to 100% (`AreaOptions::stack_mode: StackMode::Expand`):
/// every month's stack fills the full height, so each area shows that series'
/// share rather than its raw value.
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
        AreaChartGallery {
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
                CardFooter { class: "dx-chart-footer",
                    div { class: "dx-chart-footer-trend",
                        "Trending up by 5.2% this month"
                        TrendingUp { size: "16px" }
                    }
                    div { class: "dx-chart-footer-caption", "January - June 2024" }
                }
            }
        }
    }
}
