//! The two series stacked (desktop at the bottom), rounding only the stack's
//! outer ends (`BarOptions::series_radius`); shadcn's `chart-bar-stacked`
//! without its legend (see `stacked_legend`).

use super::super::component::*;
use dioxus::prelude::*;

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

/// The series, in draw (and stacking) order.
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
                CardTitle { "Bar Chart - Stacked" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config: chart_config(), data: chart_data(), kind: ChartKind::Bar,
                    Chart {
                        aria_label: "Visitors by month, desktop and mobile, stacked",
                        stacked: true,
                        tick_margin: 10.0,
                        bar: BarOptions {
                            series_radius: vec![
                                BarRadius::corners(0.0, 0.0, 4.0, 4.0),
                                BarRadius::corners(4.0, 4.0, 0.0, 0.0),
                            ],
                            ..Default::default()
                        },
                    }
                    ChartTooltip { hide_label: true }
                }
            }
            BarChartTrendFooter {}
        }
    }
}
