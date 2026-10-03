use super::super::component::*;
use dioxus::prelude::*;
use dioxus_icons::lucide::TrendingUp;

/// One browser's visitors.
fn chart_data() -> Vec<ChartDatum> {
    vec![ChartDatum {
        label: "Safari".to_string(),
        values: vec![Some(200.0)],
        color: Some("var(--dx-chart-2)".to_string()),
    }]
}

fn chart_config() -> ChartConfig {
    ChartConfig::new().series("visitors", "Visitors", "var(--dx-chart-2)")
}

/// `1260.0` as `"1,260"` (JavaScript's `toLocaleString()`).
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

/// A 250-degree gauge with rounded ends over a full muted ring, the value in
/// its centre.
#[component]
pub fn Demo() -> Element {
    let data = chart_data();
    let visitors = data[0].values[0].unwrap_or(0.0);

    rsx! {
        Card {
            CardHeader {
                CardTitle { "Radial Chart - Text" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config: chart_config(), data, kind: ChartKind::RadialBar,
                    Chart {
                        aria_label: "Visitors",
                        radial: RadialOptions {
                            start_angle: 0.0,
                            end_angle: 250.0,
                            inner_radius: Radius::Px(80.0),
                            outer_radius: Radius::Px(90.0),
                            grid: RadialGrid::Annulus { outer: 90.0, inner: 80.0 },
                            background: true,
                            corner_radius: 10.0,
                            center_text: Some((thousands(visitors), "Visitors".to_string())),
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
