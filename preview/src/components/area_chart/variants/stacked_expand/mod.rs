//! shadcn's `chart-area-stacked-expand`: three series stacked to 100%
//! (`StackMode::Expand`), so each band is that series' share of the month;
//! `other` (the bottom band) is filled at 0.1.

use super::super::component::*;
use crate::components::card::{Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle};
use dioxus::prelude::*;
use dioxus_icons::lucide::TrendingUp;

/// The chart's rows, `(month, other, mobile, desktop)`, values in config order.
pub(crate) fn chart_data() -> Vec<ChartDatum> {
    const ROWS: [(&str, f64, f64, f64); 6] = [
        ("January", 45.0, 80.0, 186.0),
        ("February", 100.0, 200.0, 305.0),
        ("March", 150.0, 120.0, 237.0),
        ("April", 50.0, 190.0, 73.0),
        ("May", 100.0, 130.0, 209.0),
        ("June", 160.0, 140.0, 214.0),
    ];
    ROWS.iter()
        .map(|&(label, other, mobile, desktop)| ChartDatum {
            label: label.to_string(),
            values: vec![Some(other), Some(mobile), Some(desktop)],
            ..Default::default()
        })
        .collect()
}

/// The series, in draw (and stacking) order: the first is the bottom layer.
pub(crate) fn chart_config() -> ChartConfig {
    ChartConfig::new()
        .series("other", "Other", "var(--dx-chart-3)")
        .series("mobile", "Mobile", "var(--dx-chart-2)")
        .series("desktop", "Desktop", "var(--dx-chart-1)")
}

#[component]
pub fn Demo() -> Element {
    rsx! {
        AreaChartGallery {
            Card {
                CardHeader {
                    CardTitle { "Area Chart - Stacked Expanded" }
                    CardDescription { "Showing total visitors for the last 6 months" }
                }
                CardContent {
                    ChartContainer { config: chart_config(), data: chart_data(), kind: ChartKind::Area,
                        Chart {
                            aria_label: "Visitors by month, other, mobile and desktop, as a share of the month",
                            margin: ChartMargin { top: 12.0, left: 12.0, right: 12.0, ..ChartMargin::NONE },
                            stacked: true,
                            curve: Curve::Natural,
                            cursor: false,
                            area: AreaOptions {
                                stack_mode: StackMode::Expand,
                                series_fill_opacity: vec![0.1, 0.4, 0.4],
                                ..Default::default()
                            },
                        }
                        ChartTooltip { indicator: TooltipIndicator::Line }
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
