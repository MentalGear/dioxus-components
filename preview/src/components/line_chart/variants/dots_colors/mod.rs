use super::super::component::*;
use crate::components::card::{Card, CardContent, CardDescription, CardHeader, CardTitle};
use dioxus::prelude::*;

/// Port of shadcn's `chart-line-dots-colors.tsx`: a single series
/// ("visitors") over browser categories, each point's own dot colored
/// individually (`ChartDatum::color`) instead of one flat series color --
/// `series::line::render_dot`'s own doc: an inline `style` override, since
/// an SVG presentation attribute alone would lose to `chart/style.css`'s
/// own `[data-slot="chart-dot"] { fill: var(--series-color); }` rule.
fn generate_data() -> Vec<ChartDatum> {
    const ROWS: [(&str, f64, &str); 5] = [
        ("Chrome", 275.0, "var(--dx-chart-1)"),
        ("Safari", 200.0, "var(--dx-chart-2)"),
        ("Firefox", 187.0, "var(--dx-chart-3)"),
        ("Edge", 173.0, "var(--dx-chart-4)"),
        ("Other", 90.0, "var(--dx-chart-5)"),
    ];
    ROWS.iter()
        .map(|(label, value, color)| ChartDatum {
            label: label.to_string(),
            values: vec![Some(*value)],
            color: Some(color.to_string()),
        })
        .collect()
}

#[component]
pub fn Demo() -> Element {
    let config = ChartConfig::new().series("visitors", "Visitors", "var(--dx-chart-1)");

    rsx! {
        Card {
            CardHeader {
                CardTitle { "Line Chart - Dots Colors" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config, data: generate_data(), kind: ChartKind::Line,
                    Chart {
                        aria_label: "Visitors by browser, each dot its own color",
                        line: LineOptions { dots: true, ..Default::default() },
                    }
                    ChartTooltip { hide_label: true }
                }
            }
        }
    }
}
