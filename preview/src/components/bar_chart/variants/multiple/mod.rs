//! shadcn's `chart-bar-multiple`: two series side by side, 4px apart, 4px
//! rounded; the tooltip uses the dashed indicator.

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
                CardTitle { "Bar Chart - Multiple" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config: chart_config(), data: chart_data(), kind: ChartKind::Bar,
                    Chart {
                        aria_label: "Visitors by month, desktop and mobile",
                        tick_margin: 10.0,
                        cursor: false,
                        bar: BarOptions {
                            radius: BarRadius::all(4.0),
                            ..Default::default()
                        },
                    }
                    ChartTooltip { indicator: TooltipIndicator::Dashed }
                }
            }
            BarChartTrendFooter {}
        }
    }
}
