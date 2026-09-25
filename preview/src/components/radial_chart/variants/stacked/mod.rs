use super::super::component::*;
use dioxus::prelude::*;

/// A single category (January) with two series -- shadcn's
/// `chart-radial-stacked.tsx` stacks `desktop`/`mobile` cumulatively into
/// ONE ring instead of each getting its own (`RadialOptions::stacked`).
fn chart_data() -> Vec<ChartDatum> {
    vec![ChartDatum {
        label: "January".to_string(),
        values: vec![Some(186.0), Some(80.0)],
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
                CardDescription { "January 2024" }
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
            CardFooter {
                div { "Trending up by 5.2% this month" }
                div { "January 2024 total visitors" }
            }
        }
    }
}
