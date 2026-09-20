use super::super::component::*;
use crate::components::card::{Card, CardContent, CardDescription, CardHeader, CardTitle};
use dioxus::prelude::*;

/// See `variants/main/mod.rs`'s own doc comment for why this dataset is
/// duplicated rather than shared -- every one of shadcn's nine source
/// files does the same.
fn chart_data() -> Vec<ChartDatum> {
    [
        ("2024-07-15", 450.0, 300.0),
        ("2024-07-16", 380.0, 420.0),
        ("2024-07-17", 520.0, 120.0),
        ("2024-07-18", 140.0, 550.0),
        ("2024-07-19", 600.0, 350.0),
        ("2024-07-20", 480.0, 400.0),
    ]
    .into_iter()
    .map(|(date, running, swimming)| ChartDatum {
        label: date.to_string(),
        values: vec![Some(running), Some(swimming)],
        ..Default::default()
    })
    .collect()
}

fn chart_config() -> ChartConfig {
    ChartConfig::new()
        .series("running", "Running", "var(--dx-chart-1)")
        .series("swimming", "Swimming", "var(--dx-chart-2)")
}

/// Ports shadcn's `chart-tooltip-indicator-none.tsx`
/// (`<ChartTooltipContent hideIndicator />`): no swatch at all, next to
/// each row's name and value.
#[component]
pub fn Demo() -> Element {
    rsx! {
        Gallery {
            Card {
                CardHeader {
                    CardTitle { "Tooltip - No Indicator" }
                    CardDescription { "Tooltip with no indicator." }
                }
                CardContent {
                    ChartContainer { config: chart_config(), data: chart_data(), kind: ChartKind::Bar,
                        Chart {
                            aria_label: "Running and swimming calories by day",
                            x_label: "Date",
                            stacked: true,
                            x_tick_format: |v: String| short_weekday(&v),
                        }
                        ChartTooltip { hide_indicator: true }
                    }
                }
            }
        }
    }
}
