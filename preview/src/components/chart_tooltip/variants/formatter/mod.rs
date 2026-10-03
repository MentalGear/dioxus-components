use super::super::component::*;
use crate::components::card::{Card, CardContent, CardDescription, CardHeader, CardTitle};
use dioxus::prelude::*;

/// The six-day running and swimming dataset this gallery's demos share. Each demo keeps its own copy so it can be copied on its own.
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

/// Formats a value as "{value} kcal", or "—" for a gap.
fn kcal(value: Option<f64>) -> String {
    match value {
        Some(v) => format!("{v} kcal"),
        None => "—".to_string(),
    }
}

/// Hides the label and uses a `formatter` that lays each row out as the name on the left and "value kcal" on the right.
#[component]
pub fn Demo() -> Element {
    rsx! {
        Gallery {
            Card {
                CardHeader {
                    CardTitle { "Tooltip - Formatter" }
                    CardDescription { "Tooltip with custom formatter." }
                }
                CardContent {
                    ChartContainer { config: chart_config(), data: chart_data(), kind: ChartKind::Bar,
                        Chart {
                            aria_label: "Running and swimming calories by day",
                            // Open on the third day so every tooltip variant is visible
                            default_index: 2,
                            x_label: "Date",
                            stacked: true,
                            x_tick_format: |v: String| short_weekday(&v),
                        }
                        ChartTooltip {
                            hide_label: true,
                            formatter: |row: TooltipRow| rsx! {
                                span { "data-slot": "chart-tooltip-name", "{row.label}" }
                                span { "data-slot": "chart-tooltip-value", {kcal(row.value)} }
                            },
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kcal_formats_a_value_and_falls_back_for_a_gap() {
        assert_eq!(kcal(Some(450.0)), "450 kcal");
        assert_eq!(kcal(Some(305.5)), "305.5 kcal");
        assert_eq!(kcal(None), "—");
    }
}
