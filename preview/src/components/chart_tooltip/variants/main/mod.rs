use super::super::component::*;
use crate::components::card::{Card, CardContent, CardDescription, CardHeader, CardTitle};
use dioxus::prelude::*;

/// The same six-day running/swimming dataset every one of this gallery's
/// nine demos shares -- ported verbatim (dates and calorie values) from
/// shadcn's own `chartData` const, repeated identically across all nine
/// `$S/refs/ui/apps/v4/registry/new-york-v4/charts/chart-tooltip-*.tsx`
/// source files rather than factored out there either, so each variant
/// here stays the same copy-pasteable, self-contained unit its source is.
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
    })
    .collect()
}

/// The matching `chartConfig` -- two series, no icons (the `icons` variant
/// adds those to its own local copy once `ChartSeries.icon` lands).
fn chart_config() -> ChartConfig {
    ChartConfig::new()
        .series("running", "Running", "var(--dx-chart-1)")
        .series("swimming", "Swimming", "var(--dx-chart-2)")
}

/// Ports shadcn's `chart-tooltip-default.tsx`: a stacked bar chart with
/// `ChartTooltipContent`'s own defaults (dot indicator, label shown, every
/// series' value shown) -- no props set on our `ChartTooltip` either, for
/// the same reason.
#[component]
pub fn Demo() -> Element {
    rsx! {
        Gallery {
            Card {
                CardHeader {
                    CardTitle { "Tooltip - Default" }
                    CardDescription { "Default tooltip with ChartTooltipContent." }
                }
                CardContent {
                    ChartContainer { config: chart_config(), data: chart_data(), kind: ChartKind::Bar,
                        Chart {
                            aria_label: "Running and swimming calories by day",
                            x_label: "Date",
                            stacked: true,
                            x_tick_format: |v: String| short_weekday(&v),
                        }
                        ChartTooltip {}
                    }
                }
            }
        }
    }
}
