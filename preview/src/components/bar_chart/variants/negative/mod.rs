//! shadcn's `chart-bar-negative`: positive and negative values around a zero
//! baseline with no x axis; each bar is labelled with its month just past its
//! end, in its own color (`BarOptions::category_labels`).

use super::super::component::*;
use dioxus::prelude::*;

/// The chart's rows, `(month, visitors)`; a negative month is drawn in
/// `--dx-chart-2`, a positive one in `--dx-chart-1` (`ChartDatum::color`).
pub(crate) fn chart_data() -> Vec<ChartDatum> {
    const ROWS: [(&str, f64); 6] = [
        ("January", 186.0),
        ("February", 205.0),
        ("March", -207.0),
        ("April", 173.0),
        ("May", -209.0),
        ("June", 214.0),
    ];
    ROWS.iter()
        .map(|&(label, visitors)| ChartDatum {
            label: label.to_string(),
            values: vec![Some(visitors)],
            color: Some(
                if visitors > 0.0 { "var(--dx-chart-1)" } else { "var(--dx-chart-2)" }.to_string(),
            ),
        })
        .collect()
}

/// The series, in draw (and stacking) order.
pub(crate) fn chart_config() -> ChartConfig {
    ChartConfig::new()
        .series("visitors", "Visitors", "var(--dx-chart-1)")
}

#[component]
pub fn Demo() -> Element {
    rsx! {
        Card {
            CardHeader {
                CardTitle { "Bar Chart - Negative" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config: chart_config(), data: chart_data(), kind: ChartKind::Bar,
                    Chart {
                        aria_label: "Visitors by month, positive and negative",
                        show_x_axis: false,
                        cursor: false,
                        bar: BarOptions {
                            category_labels: true,
                            ..Default::default()
                        },
                    }
                    ChartTooltip { hide_label: true, hide_indicator: true }
                }
            }
            BarChartTrendFooter {}
        }
    }
}
