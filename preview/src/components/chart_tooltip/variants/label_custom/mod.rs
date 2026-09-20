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

/// shadcn's own `chartConfig` here adds a THIRD entry, `activities: { label:
/// "Activities" }`, with no color/series of its own -- purely so
/// `labelKey="activities"` has something to look up (`getPayloadConfigFromPayload`,
/// chart.tsx). This crate's `ChartConfig` has no such label-only, seriesless
/// entry (every entry is a drawn series -- see `chart/mod.rs`'s own module
/// doc on why `ChartConfig` stays a plain ordered `Vec`, not a free-form
/// string-keyed map); `ChartTooltipProps::label_key` exposes the same
/// *observable* result -- a fixed "Activities" heading -- directly as a
/// literal string instead, so only the two real series are configured here.
fn chart_config() -> ChartConfig {
    ChartConfig::new()
        .series("running", "Running", "var(--dx-chart-1)")
        .series("swimming", "Swimming", "var(--dx-chart-2)")
}

/// Ports shadcn's `chart-tooltip-label-custom.tsx`
/// (`<ChartTooltipContent labelKey="activities" indicator="line" />`): the
/// label row reads "Activities" regardless of which day is hovered, instead
/// of that day's own date.
///
/// Uses [`ChartTooltipFull`], not the themed `ChartTooltip`: `label_key`/
/// `indicator` are fields `crate::components::chart::ChartTooltip`'s
/// wrapper doesn't forward yet (see `ChartTooltipFull`'s own doc comment in
/// `component.rs`).
#[component]
pub fn Demo() -> Element {
    rsx! {
        Gallery {
            Card {
                CardHeader {
                    CardTitle { "Tooltip - Custom label" }
                    CardDescription { "Tooltip with custom label from chartConfig." }
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
                            label_key: Some("Activities".to_string()),
                            indicator: TooltipIndicator::Line,
                        }
                    }
                }
            }
        }
    }
}
