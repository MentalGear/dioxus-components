//! shadcn's `chart-line-dots-colors`: one series over browsers with no x
//! axis, each point's 5px dot in its browser's color (`ChartDatum::color`).

use super::super::component::*;
use crate::components::card::{Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle};
use dioxus::prelude::*;
use dioxus_icons::lucide::TrendingUp;

/// The chart's rows, `(browser, visitors, color)`: each point's dot takes
/// its browser's color (`ChartDatum::color`).
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

/// The one series drawn.
pub(crate) fn chart_config() -> ChartConfig {
    ChartConfig::new().series("visitors", "Visitors", "var(--dx-chart-2)")
}

#[component]
pub fn Demo() -> Element {
    rsx! {
        Card {
            CardHeader {
                CardTitle { "Line Chart - Dots Colors" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config: chart_config(), data: chart_data(), kind: ChartKind::Line,
                    Chart {
                        aria_label: "Visitors by browser",
                        margin: ChartMargin { top: 24.0, left: 24.0, right: 24.0, ..ChartMargin::NONE },
                        show_x_axis: false,
                        curve: Curve::Natural,
                        cursor: false,
                        line: LineOptions {
                            dots: true,
                            dot_radius: 5.0,
                            ..Default::default()
                        },
                    }
                    ChartTooltip { indicator: TooltipIndicator::Line, name_key: "Visitors", hide_label: true }
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
