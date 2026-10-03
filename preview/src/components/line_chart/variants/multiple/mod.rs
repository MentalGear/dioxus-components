//! shadcn's `chart-line-multiple`: two series on monotone curves.

use super::super::component::*;
use crate::components::card::{Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle};
use dioxus::prelude::*;
use dioxus_icons::lucide::TrendingUp;

/// The chart's rows, `(month, desktop, mobile)`.
pub(crate) fn chart_data() -> Vec<ChartDatum> {
    const ROWS: [(&str, f64, f64); 6] = [
        ("January", 186.0, 80.0),
        ("February", 305.0, 200.0),
        ("March", 237.0, 120.0),
        ("April", 73.0, 190.0),
        ("May", 209.0, 130.0),
        ("June", 214.0, 140.0),
    ];
    ROWS.iter()
        .map(|&(label, desktop, mobile)| ChartDatum {
            label: label.to_string(),
            values: vec![Some(desktop), Some(mobile)],
            ..Default::default()
        })
        .collect()
}

/// The series, in draw order.
pub(crate) fn chart_config() -> ChartConfig {
    ChartConfig::new()
        .series("desktop", "Desktop", "var(--dx-chart-1)")
        .series("mobile", "Mobile", "var(--dx-chart-2)")
}

#[component]
pub fn Demo() -> Element {
    rsx! {
        Card {
            CardHeader {
                CardTitle { "Line Chart - Multiple" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config: chart_config(), data: chart_data(), kind: ChartKind::Line,
                    Chart {
                        aria_label: "Visitors by month, desktop and mobile",
                        margin: ChartMargin { left: 12.0, right: 12.0, ..ChartMargin::NONE },
                        curve: Curve::Monotone,
                        cursor: false,
                    }
                    ChartTooltip {}
                }
            }
            CardFooter { class: "dx-chart-footer",
                div { class: "dx-chart-footer-trend",
                    "Trending up by 5.2% this month"
                    TrendingUp { size: "16px" }
                }
                div { class: "dx-chart-footer-caption", "Showing total visitors for the last 6 months" }
            }
        }
    }
}
