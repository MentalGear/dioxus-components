use super::super::component::*;
use dioxus::prelude::*;
use dioxus_icons::lucide::TrendingUp;

/// Visitor counts per browser.
fn chart_data() -> Vec<ChartDatum> {
    [
        ("Chrome", 275.0, "var(--dx-chart-1)"),
        ("Safari", 200.0, "var(--dx-chart-2)"),
        ("Firefox", 187.0, "var(--dx-chart-3)"),
        ("Edge", 173.0, "var(--dx-chart-4)"),
        ("Other", 90.0, "var(--dx-chart-5)"),
    ]
    .into_iter()
    .map(|(browser, visitors, color)| ChartDatum {
        label: browser.to_string(),
        values: vec![Some(visitors)],
        color: Some(color.to_string()),
    })
    .collect()
}

fn chart_config() -> ChartConfig {
    ChartConfig::new().series("visitors", "Visitors", "var(--dx-chart-1)")
}

/// Rings sweeping from six o'clock (`start_angle: -90.0`) over 470 degrees, each
/// labelled with its browser along the ring.
#[component]
pub fn Demo() -> Element {
    let data = chart_data();
    let labels: Vec<String> = data.iter().map(|d| d.label.clone()).collect();

    rsx! {
        Card {
            CardHeader {
                CardTitle { "Radial Chart - Label" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config: chart_config(), data, kind: ChartKind::RadialBar,
                    Chart {
                        aria_label: "Visitors by browser",
                        radial: RadialOptions {
                            start_angle: -90.0,
                            end_angle: 380.0,
                            inner_radius: Radius::Px(30.0),
                            outer_radius: Radius::Px(110.0),
                            background: true,
                            labels: PieLabels::List(labels),
                            ..Default::default()
                        },
                    }
                    ChartTooltip { hide_label: true }
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
