//! shadcn's `chart-line-dots-custom`: `LineOptions::dot` draws your own mark at
//! every point -- here lucide's `GitCommitVertical` icon, 24px, as shadcn
//! does.

use super::super::component::*;
use crate::components::card::{Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle};
use dioxus::prelude::*;
use dioxus_icons::lucide::{GitCommitVertical, TrendingUp};

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
    let dot = DotRenderer(Callback::new(|ctx: DotContext| {
        let r = 24.0;
        rsx! {
            g { key: "{ctx.index}", "data-slot": "chart-custom-dot", "data-index": "{ctx.index}",
                GitCommitVertical {
                    x: "{ctx.cx - r / 2.0}",
                    y: "{ctx.cy - r / 2.0}",
                    width: "{r}",
                    height: "{r}",
                    fill: "var(--dx-card)",
                    stroke: "var(--series-color)",
                }
            }
        }
    }));

    rsx! {
        Card {
            CardHeader {
                CardTitle { "Line Chart - Custom Dots" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config: chart_config(), data: chart_data(), kind: ChartKind::Line,
                    Chart {
                        aria_label: "Visitors by month, desktop",
                        margin: ChartMargin { left: 12.0, right: 12.0, ..ChartMargin::NONE },
                        curve: Curve::Natural,
                        cursor: false,
                        line: LineOptions {
                            dot: Some(dot),
                            ..Default::default()
                        },
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
