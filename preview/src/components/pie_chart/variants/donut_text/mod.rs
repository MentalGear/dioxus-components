use super::super::component::*;
use dioxus::prelude::*;
use dioxus_icons::lucide::TrendingUp;

/// Visitor counts per browser.
fn chart_data() -> Vec<ChartDatum> {
    [
        ("Chrome", 275.0, "var(--dx-chart-1)"),
        ("Safari", 200.0, "var(--dx-chart-2)"),
        ("Firefox", 287.0, "var(--dx-chart-3)"),
        ("Edge", 173.0, "var(--dx-chart-4)"),
        ("Other", 190.0, "var(--dx-chart-5)"),
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

/// `1125.0` as `"1,125"` (JavaScript's `toLocaleString()`).
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

/// A donut with the grand total in its hole (`center_text`).
#[component]
pub fn Demo() -> Element {
    let data = chart_data();
    let total: f64 = data
        .iter()
        .filter_map(|d| d.values.first().copied().flatten())
        .sum();

    rsx! {
        Card {
            CardHeader {
                CardTitle { "Pie Chart - Donut with Text" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config: chart_config(), data, kind: ChartKind::Pie,
                    Chart {
                        aria_label: "Visitors by browser",
                        pie: PieOptions {
                            inner_radius: Radius::Px(60.0),
                            center_text: Some((thousands(total), "Visitors".to_string())),
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
