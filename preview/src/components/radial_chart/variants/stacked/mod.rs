use super::super::component::*;
use dioxus::prelude::*;
use dioxus_icons::lucide::TrendingUp;

/// January's mobile and desktop visitors, stacked into one half-turn ring
/// (mobile first, from three o'clock).
fn chart_data() -> Vec<ChartDatum> {
    vec![ChartDatum {
        label: "January".to_string(),
        values: vec![Some(570.0), Some(1260.0)],
        ..Default::default()
    }]
}

fn chart_config() -> ChartConfig {
    ChartConfig::new()
        .series("mobile", "Mobile", "var(--dx-chart-2)")
        .series("desktop", "Desktop", "var(--dx-chart-1)")
}

/// `1830.0` as `"1,830"` (JavaScript's `toLocaleString()`).
fn thousands(value: f64) -> String {
    let digits = format!("{value:.0}");
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// A half-turn gauge with rounded segments, the total in its centre. Each
/// segment spans its share of the total.
#[component]
pub fn Demo() -> Element {
    let data = chart_data();
    let total: f64 = data[0].values.iter().flatten().sum();

    rsx! {
        Card {
            CardHeader {
                CardTitle { "Radial Chart - Stacked" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config: chart_config(), data, kind: ChartKind::RadialBar,
                    Chart {
                        aria_label: "Visitors in January, mobile and desktop",
                        radial: RadialOptions {
                            end_angle: 180.0,
                            inner_radius: Radius::Px(80.0),
                            outer_radius: Radius::Px(110.0),
                            stacked: true,
                            corner_radius: 5.0,
                            center_text: Some((thousands(total), "Visitors".to_string())),
                            center_text_raised: true,
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
