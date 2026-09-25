use super::super::component::*;
use dioxus::prelude::*;

/// Port of shadcn's `chart-bar-default.tsx`: one series over six months,
/// rounded bars, a grid, an x-axis, and a `hideLabel` tooltip -- the
/// simplest possible bar chart, so this is this gallery's `main` demo (this
/// repo's `variants::main` convention corresponds to shadcn's own "default"
/// story, see `dev-docs/research/chart-2026-09-19.md` §1.2's naming).
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
                CardTitle { "Bar Chart" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config, data: generate_data(), kind: ChartKind::Bar,
                    Chart { aria_label: "Visitors by month, desktop" }
                    ChartTooltip { hide_label: true }
                }
            }
            BarChartTrendFooter {}
        }
    }
}
