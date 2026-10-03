//! shadcn's `chart-bar-label`: each bar's value 12px above it
//! (`BarOptions::value_labels`), with 20px of top margin to fit the labels.

use super::super::component::*;
use dioxus::prelude::*;

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

/// The series, in draw (and stacking) order.
pub(crate) fn chart_config() -> ChartConfig {
    ChartConfig::new()
        .series("desktop", "Desktop", "var(--dx-chart-1)")
}

#[component]
pub fn Demo() -> Element {
    rsx! {
        Card {
            CardHeader {
                CardTitle { "Bar Chart - Label" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config: chart_config(), data: chart_data(), kind: ChartKind::Bar,
                    Chart {
                        aria_label: "Visitors by month, desktop",
                        margin: ChartMargin { top: 20.0, ..ChartMargin::NONE },
                        tick_margin: 10.0,
                        cursor: false,
                        bar: BarOptions {
                            radius: BarRadius::all(8.0),
                            value_labels: true,
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
