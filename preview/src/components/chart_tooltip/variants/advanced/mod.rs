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

/// See `variants/formatter/mod.rs`'s own copy of this helper for why `None`
/// is handled explicitly.
fn kcal(value: Option<f64>) -> String {
    match value {
        Some(v) => format!("{v} kcal"),
        None => "—".to_string(),
    }
}

/// Ports shadcn's `chart-tooltip-advanced.tsx`: a `formatter` that rebuilds
/// the default dot-swatch + name + value row by hand (so it can append the
/// extra "Total" row after the last series -- `TooltipRow::is_last`, this
/// crate's replacement for shadcn's `index === 1` -- generalizes to any
/// series count rather than a fixture-specific magic index).
/// `TooltipRow::total` is the precomputed cross-series sum shadcn instead
/// reads out of Recharts' own raw per-datum payload (`item.payload.running
/// + item.payload.swimming`) -- this crate's tooltip has no equivalent
/// free-form payload object, so the sum is computed once and handed to
/// every row instead.
#[component]
pub fn Demo() -> Element {
    rsx! {
        Gallery {
            Card {
                CardHeader {
                    CardTitle { "Tooltip - Advanced" }
                    CardDescription { "Tooltip with custom formatter and total." }
                }
                CardContent {
                    ChartContainer { config: chart_config(), data: chart_data(), kind: ChartKind::Bar,
                        Chart {
                            aria_label: "Running and swimming calories by day",
                            x_label: "Date",
                            stacked: true,
                            x_tick_format: |v: String| short_weekday(&v),
                        }
                        ChartTooltip {
                            hide_label: true,
                            formatter: |row: TooltipRow| rsx! {
                                span {
                                    "data-slot": "chart-swatch",
                                    "data-indicator": "dot",
                                    role: "graphics-symbol",
                                    "aria-label": "{row.label}",
                                    style: "--series-color: {row.color}",
                                }
                                span { "data-slot": "chart-tooltip-name", "{row.label}" }
                                span { "data-slot": "chart-tooltip-value", {kcal(row.value)} }
                                if row.is_last {
                                    div { "data-slot": "chart-tooltip-total",
                                        "Total"
                                        span { "data-slot": "chart-tooltip-total-value", {kcal(Some(row.total))} }
                                    }
                                }
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
        assert_eq!(kcal(Some(750.0)), "750 kcal");
        assert_eq!(kcal(None), "—");
    }
}
