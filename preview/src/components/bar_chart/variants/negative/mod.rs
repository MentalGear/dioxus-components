use super::super::component::*;
use dioxus::prelude::*;

/// Port of shadcn's `chart-bar-negative.tsx`: one series whose monthly
/// values swing between positive and negative, each bar colored per-datum
/// (`ChartDatum::color`, positive months get `--dx-chart-1`, negative
/// months a contrasting `--dx-chart-5`) rather than a single flat series
/// color -- shadcn's own demo does the identical thing via a per-cell
/// `fill` callback. `layout::render_grid` draws an explicit
/// `data-slot="chart-zero-line"` for `ChartKind::Bar` specifically (this
/// lane's own addition, `components::layout`'s module doc), so the
/// baseline every bar runs from/to is visible.
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
