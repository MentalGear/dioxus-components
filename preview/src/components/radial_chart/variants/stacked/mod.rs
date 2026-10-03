use super::super::component::*;
use dioxus::prelude::*;
use dioxus_icons::lucide::TrendingUp;

/// A single category (January) with two series stacked cumulatively into one
/// ring instead of one ring each (`RadialOptions::stacked`).
fn chart_data() -> Vec<ChartDatum> {
    vec![ChartDatum {
        label: "January".to_string(),
        values: vec![Some(1260.0), Some(570.0)],
        ..Default::default()
    }]
}

fn chart_config() -> ChartConfig {
    ChartConfig::new()
        .series("desktop", "Desktop", "var(--dx-chart-1)")
        .series("mobile", "Mobile", "var(--dx-chart-2)")
}

#[component]
pub fn Demo() -> Element {
    let data = chart_data();
    let total: f64 = data[0].values.iter().filter_map(|v| *v).sum();

    rsx! {
        Card {
            CardHeader {
                CardTitle { "Radial Chart - Stacked" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config: chart_config(), data, kind: ChartKind::RadialBar,
                    Chart {
                        width: 300.0,
                        height: 300.0,
                        aria_label: "Visitors by month, desktop and mobile",
                        radial: RadialOptions {
                            inner_radius: 80.0,
                            stacked: true,
                            center_text: Some((format!("{total:.0}"), "Visitors".to_string())),
                            ..Default::default()
                        },
                    }
                    ChartTooltip {}
                    ChartLegend {}
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
