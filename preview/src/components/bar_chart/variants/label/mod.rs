use super::super::component::*;
use dioxus::prelude::*;

/// Port of shadcn's `chart-bar-label.tsx`: the same single-series, six-month
/// dataset as this gallery's own `main` demo, plus `BarOptions::
/// value_labels` -- each bar's own value drawn just above it (shadcn's
/// `<LabelList position="top" />`), and the grid/y-axis hidden (matching
/// the source demo, whose whole point is that the labels replace the axis
/// as the readout).
fn generate_data() -> Vec<ChartDatum> {
    const ROWS: [(&str, f64); 6] = [
        ("January", 186.0),
        ("February", 305.0),
        ("March", 237.0),
        ("April", 73.0),
        ("May", 209.0),
        ("June", 214.0),
    ];
    ROWS.iter()
        .map(|(label, value)| ChartDatum {
            label: label.to_string(),
            values: vec![Some(*value)],
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
                CardTitle { "Bar Chart - Label" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config, data: generate_data(), kind: ChartKind::Bar,
                    Chart {
                        aria_label: "Visitors by month, desktop, labelled",
                        show_grid: false,
                        show_y_axis: false,
                        bar: BarOptions { value_labels: true, ..Default::default() },
                    }
                    ChartTooltip { hide_label: true }
                }
            }
            BarChartTrendFooter {}
        }
    }
}
