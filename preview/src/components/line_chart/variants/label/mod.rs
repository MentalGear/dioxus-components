use super::super::component::*;
use crate::components::card::{Card, CardContent, CardDescription, CardHeader, CardTitle};
use dioxus::prelude::*;

/// Port of shadcn's `chart-line-label.tsx`: `LineOptions::labels:
/// LineLabels::Value` draws each defined point's own value just above it
/// (shadcn's `<LabelList position="top" />`), with the grid/y-axis hidden
/// (matching the source demo, whose whole point is that the labels replace
/// the axis as the readout).
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

    rsx! {
        Card {
            CardHeader {
                CardTitle { "Line Chart - Label" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config, data: generate_data(), kind: ChartKind::Line,
                    Chart {
                        aria_label: "Visitors by month, desktop, labelled",
                        show_grid: false,
                        show_y_axis: false,
                        line: LineOptions {
                            dots: true,
                            labels: LineLabels::Value,
                            ..Default::default()
                        },
                    }
                    ChartTooltip { hide_label: true }
                }
            }
        }
    }
}
