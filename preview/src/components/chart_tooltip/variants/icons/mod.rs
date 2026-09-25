use super::super::component::*;
use crate::components::card::{Card, CardContent, CardDescription, CardHeader, CardTitle};
use dioxus::prelude::*;
use dioxus_icons::lucide::{Footprints, WavesHorizontal};

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

/// Adds an icon per series -- shadcn's `chartConfig` here sets `icon:
/// Footprints`/`icon: Waves` (lucide-react). `ChartSeries::icon` has no
/// builder method yet (`primitives/src/chart/config.rs`'s own doc: "set it
/// with a struct update ... once a lane gives this a builder method of its
/// own"), so each series is built via the normal `.series(...)` chain, then
/// mapped through a struct update that attaches the icon -- this crate's
/// `ChartConfig`/`ChartSeries` are `config.rs`-owned, not this lane's to add
/// a convenience method to.
///
/// This crate's vendored `dioxus_icons::lucide` set has no plain `Waves`
/// icon (only `WavesHorizontal`/`WavesVertical`/`WavesLadder`/
/// `WavesArrowUp`/`WavesArrowDown` -- lucide's own icon appears to have
/// been split into directional variants upstream of this vendored
/// snapshot); `WavesHorizontal` is the closest visual match for the
/// swimming series.
fn chart_config() -> ChartConfig {
    let base = ChartConfig::new()
        .series("running", "Running", "var(--dx-chart-1)")
        .series("swimming", "Swimming", "var(--dx-chart-2)");
    ChartConfig {
        series: base
            .series
            .into_iter()
            .map(|series| {
                let icon = if series.key == "running" {
                    ChartIcon(Callback::new(|()| rsx! { Footprints {} }))
                } else {
                    ChartIcon(Callback::new(|()| rsx! { WavesHorizontal {} }))
                };
                ChartSeries { icon: Some(icon), ..series }
            })
            .collect(),
    }
}

/// Ports shadcn's `chart-tooltip-icons.tsx` (`hideLabel`, each series'
/// `icon` renders in place of its indicator swatch). The icon itself
/// renders from `ChartSeries::icon` (read directly off `config` by the
/// primitive), not from any `ChartTooltipProps` field this demo needs to
/// set.
#[component]
pub fn Demo() -> Element {
    rsx! {
        Gallery {
            Card {
                CardHeader {
                    CardTitle { "Tooltip - Icons" }
                    CardDescription { "Tooltip with icons." }
                }
                CardContent {
                    ChartContainer { config: chart_config(), data: chart_data(), kind: ChartKind::Bar,
                        Chart {
                            aria_label: "Running and swimming calories by day",
                            x_label: "Date",
                            stacked: true,
                            x_tick_format: |v: String| short_weekday(&v),
                        }
                        ChartTooltip { hide_label: true }
                    }
                }
            }
        }
    }
}
