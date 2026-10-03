use super::super::component::*;
use dioxus::prelude::*;
use dioxus_icons::lucide::TrendingUp;

/// A single-category dataset: one gauge-style ring with the visitor count
/// centered in the hole.
fn chart_data() -> Vec<ChartDatum> {
    vec![ChartDatum {
        label: "Visitors".to_string(),
        values: vec![Some(1999.0)],
        color: Some("var(--dx-chart-1)".to_string()),
    }]
}

fn chart_config() -> ChartConfig {
    ChartConfig::new().series("visitors", "Visitors", "var(--dx-chart-1)")
}

/// A single ring with a two-line total centered in its hole
/// (`RadialOptions::center_text`).
#[component]
pub fn Demo() -> Element {
    rsx! {
        Card {
            CardHeader {
                CardTitle { "Radial Chart - Text" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config: chart_config(), data: chart_data(), kind: ChartKind::RadialBar,
                    Chart {
                        width: 300.0,
                        height: 300.0,
                        aria_label: "Visitors",
                        radial: RadialOptions {
                            inner_radius: 80.0,
                            grid: true,
                            center_text: Some(("1,999".to_string(), "Visitors".to_string())),
                            ..Default::default()
                        },
                    }
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
