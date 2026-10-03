use super::super::component::*;
use dioxus::prelude::*;

/// One series with positive and negative values, each bar colored per datum
/// (`ChartDatum::color`); a zero line marks the baseline.
fn generate_data() -> Vec<ChartDatum> {
    const ROWS: [(&str, f64); 6] = [
        ("January", 186.0),
        ("February", 305.0),
        ("March", -237.0),
        ("April", 73.0),
        ("May", -209.0),
        ("June", 214.0),
    ];
    ROWS.iter()
        .map(|(label, value)| ChartDatum {
            label: label.to_string(),
            values: vec![Some(*value)],
            color: Some(if *value >= 0.0 { "var(--dx-chart-1)" } else { "var(--dx-chart-5)" }.to_string()),
        })
        .collect()
}

#[component]
pub fn Demo() -> Element {
    let config = ChartConfig::new().series("visitors", "Visitors", "var(--dx-chart-1)");

    rsx! {
        Card {
            CardHeader {
                CardTitle { "Bar Chart - Negative" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config, data: generate_data(), kind: ChartKind::Bar,
                    Chart { aria_label: "Visitors by month, positive and negative" }
                    ChartTooltip { hide_label: true }
                }
            }
        }
    }
}
