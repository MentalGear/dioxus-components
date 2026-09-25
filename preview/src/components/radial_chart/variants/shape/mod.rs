use super::super::component::*;
use dioxus::prelude::*;

/// See `variants/text/mod.rs`'s own doc comment -- a single-category
/// dataset, same shape shadcn's `chart-radial-shape.tsx` uses.
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

/// Ports shadcn's `chart-radial-shape.tsx`: a single ring with rounded
/// corners (`RadialOptions::corner_radius > 0.0`).
#[component]
pub fn Demo() -> Element {
    rsx! {
        Card {
            CardHeader {
                CardTitle { "Radial Chart - Shape" }
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
                            corner_radius: 10.0,
                            ..Default::default()
                        },
                    }
                }
            }
            CardFooter {
                div { "Trending up by 5.2% this month" }
                div { "Showing total visitors for the last 6 months" }
            }
        }
    }
}
