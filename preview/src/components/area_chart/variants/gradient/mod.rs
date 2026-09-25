use super::super::component::*;
use crate::components::card::{Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle};
use dioxus::prelude::*;
use dioxus_icons::lucide::TrendingUp;

/// Ports
/// `$S/refs/ui/apps/v4/registry/new-york-v4/charts/chart-area-gradient.tsx`
/// (shadcn's `chart-area-gradient` demo): the same stacked two-series data
/// as `stacked`/`legend`, with `AreaOptions::gradient: true` -- a
/// top-to-bottom `<linearGradient>` per series
/// (`primitives/src/chart/components/series/area.rs`, id'd
/// `"{chart-id}-gradient-{series-slot}"`, SSR-stable) instead of a flat
/// fill, matching the upstream demo's own `stopOpacity` `0.8`/`0.1` pair.
fn generate_data() -> Vec<ChartDatum> {
    const ROWS: [(&str, f64, f64); 6] = [
        ("January", 186.0, 80.0),
        ("February", 305.0, 200.0),
        ("March", 237.0, 120.0),
        ("April", 73.0, 190.0),
        ("May", 209.0, 130.0),
        ("June", 214.0, 140.0),
    ];
    ROWS.iter()
        .map(|&(label, desktop, mobile)| ChartDatum {
            label: label.to_string(),
            values: vec![Some(desktop), Some(mobile)],
            ..Default::default()
        })
        .collect()
}

#[component]
pub fn Demo() -> Element {
    let config = ChartConfig::new()
        .series("desktop", "Desktop", "var(--dx-chart-1)")
        .series("mobile", "Mobile", "var(--dx-chart-2)");

    rsx! {
        div { class: "dx-area-chart-gallery",
            Card {
                CardHeader {
                    CardTitle { "Area Chart - Gradient" }
                    CardDescription { "Showing total visitors for the last 6 months" }
                }
                CardContent {
                    ChartContainer { config, data: generate_data(), kind: ChartKind::Area,
                        Chart {
                            aria_label: "Visitors by month, desktop and mobile, stacked, gradient fill",
                            stacked: true,
                            area: AreaOptions {
                                gradient: true,
                                ..Default::default()
                            },
                        }
                        ChartTooltip {}
                    }
                }
                CardFooter {
                    div { style: "display: flex; width: 100%; align-items: flex-start; gap: 8px; font-size: var(--dx-text-sm);",
                        div { style: "display: grid; gap: 8px;",
                            div { style: "display: flex; align-items: center; gap: 8px; font-weight: 600;",
                                "Trending up by 5.2% this month"
                                TrendingUp { size: "16px" }
                            }
                            div { style: "color: var(--secondary-color-5);", "January - June 2024" }
                        }
                    }
                }
            }
        }
    }
}
