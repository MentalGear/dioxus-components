//! shadcn's `chart-area-step`: the default chart drawn as midpoint steps
//! (`Curve::Step`); the series' icon replaces the tooltip swatch.

use super::super::component::*;
use crate::components::card::{Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle};
use dioxus::prelude::*;
use dioxus_icons::lucide::{Activity, TrendingUp};

/// The chart's rows, `(month, desktop)`, values in config order.
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

/// The series, in draw (and stacking) order: the first is the bottom layer.
pub(crate) fn chart_config() -> ChartConfig {
    let mut config = ChartConfig::new()
        .series("desktop", "Desktop", "var(--dx-chart-1)");
    config.series[0].icon = Some(ChartIcon(Callback::new(|()| rsx! { Activity {} })));
    config
}

#[component]
pub fn Demo() -> Element {
    rsx! {
        AreaChartGallery {
            Card {
                CardHeader {
                    CardTitle { "Area Chart - Step" }
                    CardDescription { "Showing total visitors for the last 6 months" }
                }
                CardContent {
                    ChartContainer { config: chart_config(), data: chart_data(), kind: ChartKind::Area,
                        Chart {
                            aria_label: "Visitors by month, desktop",
                            margin: ChartMargin { left: 12.0, right: 12.0, ..ChartMargin::NONE },
                            curve: Curve::Step,
                            cursor: false,
                        }
                        ChartTooltip { hide_label: true }
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
