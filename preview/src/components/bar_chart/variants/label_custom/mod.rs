//! shadcn's `chart-bar-label-custom`: horizontal bars with no axes; each bar
//! carries its month inside and its value past its end
//! (`BarOptions::inside_labels`), over vertical gridlines.

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
        .series("desktop", "Desktop", "var(--dx-chart-2)")
}

#[component]
pub fn Demo() -> Element {
    rsx! {
        Card {
            CardHeader {
                CardTitle { "Bar Chart - Custom Label" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config: chart_config(), data: chart_data(), kind: ChartKind::Bar,
                    Chart {
                        aria_label: "Visitors by month, desktop",
                        margin: ChartMargin { right: 16.0, ..ChartMargin::NONE },
                        show_x_axis: false,
                        cursor: false,
                        bar: BarOptions {
                            horizontal: true,
                            radius: BarRadius::all(4.0),
                            inside_labels: true,
                            ..Default::default()
                        },
                    }
                    ChartTooltip { indicator: TooltipIndicator::Line }
                }
            }
            BarChartTrendFooter {}
        }
    }
}
