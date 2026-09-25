use super::super::component::*;
use crate::components::card::{Card, CardContent, CardDescription, CardHeader, CardTitle};
use dioxus::prelude::*;

/// Port of shadcn's `chart-line-label-custom.tsx`: `LineOptions::labels:
/// LineLabels::Custom` labels each point with its own category name
/// (shadcn's own custom `<LabelList content={...} />` draws the month
/// name above every point) instead of the default up-to-2-decimals value
/// -- the callback closes over this demo's own `generate_data()` output by
/// index, since [`LineLabels::Custom`]'s callback only ever receives the
/// point's index (this crate's `ChartDatum` values, not a free-form
/// payload object, are the source of truth a caller reaches back into).
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
        }
    }
}
