use super::super::component::*;
use dioxus::prelude::*;

/// Visitor counts per browser.
fn chart_data() -> Vec<ChartDatum> {
    [
        ("Chrome", 275.0, "var(--dx-chart-1)"),
        ("Safari", 200.0, "var(--dx-chart-2)"),
        ("Firefox", 187.0, "var(--dx-chart-3)"),
        ("Edge", 173.0, "var(--dx-chart-4)"),
        ("Other", 90.0, "var(--dx-chart-5)"),
    ]
    .into_iter()
    .map(|(browser, visitors, color)| ChartDatum {
        label: browser.to_string(),
        values: vec![Some(visitors)],
        color: Some(color.to_string()),
    })
    .collect()
}

fn chart_config() -> ChartConfig {
    ChartConfig::new().series("visitors", "Visitors", "var(--dx-chart-1)")
}

/// A swatch and name per slice below the pie. shadcn's chart is a 300px square
/// with the legend inside it (`*:basis-1/4` wraps it to two rows, 52px):
/// the legend takes its room from the square, leaving shadcn's pie (radius
/// 95.2, centre 124px down) in the 300x248 above it.
#[component]
pub fn Demo() -> Element {
    rsx! {
        Card {
            CardHeader {
                CardTitle { "Pie Chart - Legend" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer {
                    config: chart_config(),
                    data: chart_data(),
                    kind: ChartKind::Pie,
                    style: "max-inline-size: 300px",
                    Chart { width: 300.0, legend_size: 52.0, aria_label: "Visitors by browser" }
                    ChartLegend { style: "--dx-chart-legend-item-basis: 25%; gap: var(--dx-space-2); translate: 0 calc(-1 * var(--dx-space-2))" }
                }
            }
        }
    }
}
