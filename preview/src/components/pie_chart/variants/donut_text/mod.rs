use super::super::component::*;
use dioxus::prelude::*;

/// See `variants/main/mod.rs`'s own doc comment for why this dataset is
/// duplicated rather than shared.
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

/// Ports shadcn's `chart-pie-donut-text.tsx`: a donut with the grand total
/// centered in its hole (`center_text`), gauge-style.
#[component]
pub fn Demo() -> Element {
    let data = chart_data();
    let total: f64 = data.iter().filter_map(|d| d.values.first().copied().flatten()).sum();

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
                            inner_radius: 60.0,
                            center_text: Some((format!("{total:.0}"), "Visitors".to_string())),
                            ..Default::default()
                        },
                    }
                    ChartTooltip { hide_label: true }
                }
            }
            CardFooter {
                div { "Trending up by 5.2% this month" }
                div { "Showing total visitors for the last 6 months" }
            }
        }
    }
}
