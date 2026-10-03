//! shadcn's `chart-bar-mixed`: horizontal bars, each in its own color
//! (`ChartDatum::color`), with the full browser names on the y axis.

use super::super::component::*;
use dioxus::prelude::*;

/// The chart's rows, `(browser, visitors, color)`: each bar takes its
/// browser's color (`ChartDatum::color`).
pub(crate) fn chart_data() -> Vec<ChartDatum> {
    const ROWS: [(&str, f64, &str); 5] = [
        ("Chrome", 275.0, "var(--dx-chart-1)"),
        ("Safari", 200.0, "var(--dx-chart-2)"),
        ("Firefox", 187.0, "var(--dx-chart-3)"),
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
                CardTitle { "Bar Chart - Mixed" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config: chart_config(), data: chart_data(), kind: ChartKind::Bar,
                    Chart {
                        aria_label: "Visitors by browser",
                        margin: ChartMargin::NONE,
                        show_grid: false,
                        show_x_axis: false,
                        show_y_axis: true,
                        tick_margin: 10.0,
                        x_tick_format: |label: String| label,
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
