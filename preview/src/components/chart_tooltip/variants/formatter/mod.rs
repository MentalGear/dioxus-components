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

/// `"{value} kcal"`, `"—"` for a gap -- shadcn's own formatter renders
/// `{value}<span>kcal</span>` with no extra handling for a missing value
/// since Recharts never calls `formatter` for one; this crate's `formatter`
/// is called for every row regardless (`TooltipRow::value` is `Option`), so
/// this demo handles `None` explicitly, matching `ChartTooltip`'s own
/// default value markup's "—" convention.
fn kcal(value: Option<f64>) -> String {
    match value {
        Some(v) => format!("{v} kcal"),
        None => "—".to_string(),
    }
}

/// Ports shadcn's `chart-tooltip-formatter.tsx`: `hideLabel` plus a
/// `formatter` that lays each row out as name-left, "value kcal"-right --
/// still the swatch-free layout `hideLabel`'s sibling demos use, but with
/// a custom unit suffix on the value.
///
/// Uses [`ChartTooltipFull`], not the themed `ChartTooltip`: `formatter` is
/// a field `crate::components::chart::ChartTooltip`'s wrapper doesn't
/// forward yet (see `ChartTooltipFull`'s own doc comment in
/// `component.rs`).
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
                            x_label: "Date",
                            stacked: true,
                            x_tick_format: |v: String| short_weekday(&v),
                        }
                        ChartTooltipFull {
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
