use super::super::component::*;
use dioxus::prelude::*;
use dioxus_icons::lucide::TrendingUp;

/// Desktop and mobile visitors per month, one color per month.
fn chart_data() -> Vec<ChartDatum> {
    [
        ("January", 186.0, 80.0, "var(--dx-chart-1)"),
        ("February", 305.0, 200.0, "var(--dx-chart-2)"),
        ("March", 237.0, 120.0, "var(--dx-chart-3)"),
        ("April", 173.0, 190.0, "var(--dx-chart-4)"),
        ("May", 209.0, 130.0, "var(--dx-chart-5)"),
    ]
    .into_iter()
    .map(|(month, desktop, mobile, color)| ChartDatum {
        label: month.to_string(),
        values: vec![Some(desktop), Some(mobile)],
        color: Some(color.to_string()),
    })
    .collect()
}

fn chart_config() -> ChartConfig {
    ChartConfig::new()
        .series("desktop", "Desktop", "var(--dx-chart-1)")
        .series("mobile", "Mobile", "var(--dx-chart-2)")
}

/// Two pies over the same months: desktop a full disc of radius 60, mobile a
/// ring from 70 to 90 around it (`rings`, one per series).
#[component]
pub fn Demo() -> Element {
    rsx! {
        Card {
            CardHeader {
                CardTitle { "Pie Chart - Stacked" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config: chart_config(), data: chart_data(), kind: ChartKind::Pie,
                    Chart {
                        aria_label: "Visitors by month, desktop and mobile",
                        pie: PieOptions {
                            rings: vec![
                                (Radius::Px(0.0), Radius::Px(60.0)),
                                (Radius::Px(70.0), Radius::Px(90.0)),
                            ],
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
                div { class: "dx-chart-footer-caption", "Showing total visitors for the last 6 months" }
            }
        }
    }
}
