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

/// Formats a value as its bare number (`450`), or "—" for a gap. The "kcal" unit is a separate, muted span next to it, as in shadcn's `{value}<span>kcal</span>`.
fn amount(value: Option<f64>) -> String {
    match value {
        Some(v) => format!("{v}"),
        None => "—".to_string(),
    }
}

/// A `formatter` that rebuilds the default dot, name and value row by hand so it can append a "Total" row after the last series (`TooltipRow::is_last`).
/// `TooltipRow::total` is the sum across series for the hovered point.
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
                            // shadcn's `defaultIndex={1}`: open on the second day. No grid and no y axis, as in shadcn's tooltip demos
                            default_index: 1,
                            show_grid: false,
                            show_y_axis: false,
                            // `tickMargin={10}` and `cursor={false}`
                            tick_margin: 10.0,
                            cursor: false,
                            // The bottom series rounds its lower corners, the top one its upper corners (`radius={[0, 0, 4, 4]}` / `[4, 4, 0, 0]`)
                            bar: BarOptions {
                                series_radius: vec![
                                    BarRadius::corners(0.0, 0.0, 4.0, 4.0),
                                    BarRadius::corners(4.0, 4.0, 0.0, 0.0),
                                ],
                                ..Default::default()
                            },
                            x_label: "Date",
                            stacked: true,
                            x_tick_format: |v: String| short_weekday(&v),
                        }
                        ChartTooltip {
                            hide_label: true,
                            // shadcn's `className="w-[180px]"`
                            style: "width: 180px",
                            formatter: |row: TooltipRow| rsx! {
                                span {
                                    "data-slot": "chart-swatch",
                                    "data-indicator": "dot",
                                    role: "graphics-symbol",
                                    "aria-label": "{row.label}",
                                    style: "--series-color: {row.color}",
                                }
                                span { "data-slot": "chart-tooltip-name", "{row.label}" }
                                span { "data-slot": "chart-tooltip-value",
                                    {amount(row.value)}
                                    span { "data-slot": "chart-tooltip-unit", "kcal" }
                                }
                                if row.is_last {
                                    div { "data-slot": "chart-tooltip-total",
                                        "Total"
                                        span { "data-slot": "chart-tooltip-total-value",
                                            {amount(Some(row.total))}
                                            span { "data-slot": "chart-tooltip-unit", "kcal" }
                                        }
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
    fn amount_formats_a_bare_number_and_falls_back_for_a_gap() {
        assert_eq!(amount(Some(750.0)), "750");
        assert_eq!(amount(None), "—");
    }
}
