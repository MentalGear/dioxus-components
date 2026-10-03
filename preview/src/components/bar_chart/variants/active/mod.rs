//! shadcn's `chart-bar-active`: one bar highlighted (`BarOptions::active_index`)
//! -- drawn at 0.8 opacity with a dashed outline in its own color.

use super::super::component::*;
use dioxus::prelude::*;

/// The chart's rows, `(browser, visitors, color)`: each bar takes its
/// browser's color (`ChartDatum::color`).
pub(crate) fn chart_data() -> Vec<ChartDatum> {
    const ROWS: [(&str, f64, &str); 5] = [
        ("Chrome", 187.0, "var(--dx-chart-1)"),
        ("Safari", 200.0, "var(--dx-chart-2)"),
        ("Firefox", 275.0, "var(--dx-chart-3)"),
        ("Edge", 173.0, "var(--dx-chart-4)"),
        ("Other", 90.0, "var(--dx-chart-5)"),
    ];
    ROWS.iter()
        .map(|&(label, visitors, color)| ChartDatum {
            label: label.to_string(),
            values: vec![Some(visitors)],
            color: Some(color.to_string()),
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
                CardTitle { "Bar Chart - Active" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config: chart_config(), data: chart_data(), kind: ChartKind::Bar,
                    Chart {
                        aria_label: "Visitors by browser",
                        tick_margin: 10.0,
                        x_tick_format: |label: String| label,
                        cursor: false,
                        bar: BarOptions {
                            radius: BarRadius::all(8.0),
                            active_index: Some(2),
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
