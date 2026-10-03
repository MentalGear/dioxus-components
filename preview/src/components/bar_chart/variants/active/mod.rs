use super::super::component::*;
use dioxus::prelude::*;

/// Horizontal bars with the "Firefox" bar kept highlighted
/// (`BarOptions::active_index`); every other bar dims.
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
                CardTitle { "Bar Chart - Active" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config, data: generate_data(), kind: ChartKind::Bar,
                    Chart {
                        aria_label: "Visitors by browser, Firefox highlighted",
                        show_x_axis: false,
                        show_y_axis: false,
                        bar: BarOptions {
                            horizontal: true,
                            active_index: Some(2),
                            ..Default::default()
                        },
                    }
                    ChartTooltip { hide_label: true }
                }
            }
            BarChartTrendFooter {}
        }
    }
}
