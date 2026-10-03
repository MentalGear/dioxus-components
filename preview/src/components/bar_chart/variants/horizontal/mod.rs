use super::super::component::*;
use dioxus::prelude::*;

/// One series over browser categories, drawn horizontally
/// (`BarOptions::horizontal`). Both default axes are hidden; the category
/// labels are drawn at the left edge of the plot.
fn generate_data() -> Vec<ChartDatum> {
    const ROWS: [(&str, f64); 5] = [
        ("Chrome", 275.0),
        ("Safari", 200.0),
        ("Firefox", 187.0),
        ("Edge", 173.0),
        ("Other", 90.0),
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
    let config = ChartConfig::new().series("visitors", "Visitors", "var(--dx-chart-1)");

    rsx! {
        Card {
            CardHeader {
                CardTitle { "Bar Chart - Horizontal" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config, data: generate_data(), kind: ChartKind::Bar,
                    Chart {
                        aria_label: "Visitors by browser",
                        show_x_axis: false,
                        show_y_axis: false,
                        bar: BarOptions { horizontal: true, ..Default::default() },
                    }
                    ChartTooltip { hide_label: true }
                }
            }
            BarChartTrendFooter {}
        }
    }
}
