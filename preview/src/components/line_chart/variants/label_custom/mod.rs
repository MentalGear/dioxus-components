use super::super::component::*;
use crate::components::card::{Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle};
use dioxus::prelude::*;
use dioxus_icons::lucide::TrendingUp;

/// `LineLabels::Custom` labels each point with text you choose, here the
/// category name. The callback only receives the point index, so it looks the
/// text up in the chart data.
fn generate_data() -> Vec<ChartDatum> {
    const MONTHS: [(&str, f64); 6] = [
        ("January", 186.0),
        ("February", 305.0),
        ("March", 237.0),
        ("April", 73.0),
        ("May", 209.0),
        ("June", 214.0),
    ];
    MONTHS
        .iter()
        .map(|(label, desktop)| ChartDatum {
            label: label.to_string(),
            values: vec![Some(*desktop)],
            ..Default::default()
        })
        .collect()
}

#[component]
pub fn Demo() -> Element {
    let config = ChartConfig::new().series("desktop", "Desktop", "var(--dx-chart-1)");
    let data = generate_data();
    let category_labels: Vec<String> = data.iter().map(|d| d.label.clone()).collect();
    let custom_label = LineLabels::Custom(Callback::new(move |i: usize| {
        category_labels
            .get(i)
            .map(|label| label.chars().take(3).collect::<String>())
            .unwrap_or_default()
    }));

    rsx! {
        Card {
            CardHeader {
                CardTitle { "Line Chart - Custom Label" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config, data, kind: ChartKind::Line,
                    Chart {
                        aria_label: "Visitors by month, desktop, labelled by month",
                        show_grid: false,
                        show_x_axis: false,
                        show_y_axis: false,
                        line: LineOptions {
                            dots: true,
                            labels: custom_label,
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
