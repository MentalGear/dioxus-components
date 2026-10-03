//! shadcn's `chart-area-gradient`: the stacked chart filled with a vertical
//! gradient per series (`AreaOptions::gradient`).

use super::super::component::*;
use crate::components::card::{Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle};
use dioxus::prelude::*;
use dioxus_icons::lucide::TrendingUp;

/// The chart's rows, `(month, mobile, desktop)`, values in config order.
pub(crate) fn chart_data() -> Vec<ChartDatum> {
    const ROWS: [(&str, f64, f64); 6] = [
        ("January", 80.0, 186.0),
        ("February", 200.0, 305.0),
        ("March", 120.0, 237.0),
        ("April", 190.0, 73.0),
        ("May", 130.0, 209.0),
        ("June", 140.0, 214.0),
    ];
    ROWS.iter()
        .map(|&(label, mobile, desktop)| ChartDatum {
            label: label.to_string(),
            values: vec![Some(mobile), Some(desktop)],
            ..Default::default()
        })
        .collect()
}

/// The series, in draw (and stacking) order: the first is the bottom layer.
pub(crate) fn chart_config() -> ChartConfig {
    ChartConfig::new()
        .series("mobile", "Mobile", "var(--dx-chart-2)")
        .series("desktop", "Desktop", "var(--dx-chart-1)")
}

#[component]
pub fn Demo() -> Element {
    rsx! {
        AreaChartGallery {
            Card {
                CardHeader {
                    CardTitle { "Area Chart - Gradient" }
                    CardDescription { "Showing total visitors for the last 6 months" }
                }
                CardContent {
                    ChartContainer { config: chart_config(), data: chart_data(), kind: ChartKind::Area,
                        Chart {
                            aria_label: "Visitors by month, mobile and desktop, stacked",
                            margin: ChartMargin { left: 12.0, right: 12.0, ..ChartMargin::NONE },
                            stacked: true,
                            curve: Curve::Natural,
                            cursor: false,
                            area: AreaOptions {
                                gradient: true,
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
