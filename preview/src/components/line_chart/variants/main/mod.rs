//! shadcn's `chart-line-default`: one series, a natural curve, no dots, 12px
//! side margins and no hover cursor; the tooltip hides its label row.

use super::super::component::*;
use crate::components::card::{Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle};
use dioxus::prelude::*;
use dioxus_icons::lucide::TrendingUp;

/// The chart's rows, `(month, desktop)`.
pub(crate) fn chart_data() -> Vec<ChartDatum> {
    const ROWS: [(&str, f64); 6] = [
        ("January", 186.0),
        ("February", 305.0),
        ("March", 237.0),
        ("April", 73.0),
        ("May", 209.0),
        ("June", 214.0),
    ];
    ROWS.iter()
        .map(|&(label, desktop)| ChartDatum {
            label: label.to_string(),
            values: vec![Some(desktop)],
            ..Default::default()
        })
        .collect()
}

/// The one series drawn.
pub(crate) fn chart_config() -> ChartConfig {
    ChartConfig::new().series("desktop", "Desktop", "var(--dx-chart-1)")
}

#[component]
pub fn Demo() -> Element {
    rsx! {
        Card {
            CardHeader {
                CardTitle { "Line Chart" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config: chart_config(), data: chart_data(), kind: ChartKind::Line,
                    Chart {
                        aria_label: "Visitors by month, desktop",
                        margin: ChartMargin { left: 12.0, right: 12.0, ..ChartMargin::NONE },
                        curve: Curve::Natural,
                        cursor: false,
                    }
                    ChartTooltip { hide_label: true }
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
