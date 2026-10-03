//! shadcn's `chart-bar-horizontal`: bars running left to right
//! (`BarOptions::horizontal`) with the months on the y axis, which shadcn
//! pulls 20px into the left margin.

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
                CardTitle { "Bar Chart - Horizontal" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config: chart_config(), data: chart_data(), kind: ChartKind::Bar,
                    Chart {
                        aria_label: "Visitors by month, desktop",
                        margin: ChartMargin { left: -20.0, ..ChartMargin::NONE },
                        show_grid: false,
                        show_x_axis: false,
                        show_y_axis: true,
                        tick_margin: 10.0,
                        cursor: false,
                        bar: BarOptions {
                            horizontal: true,
                            radius: BarRadius::all(5.0),
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
