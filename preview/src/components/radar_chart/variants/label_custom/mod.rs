use super::super::component::*;
use crate::components::card::{
    Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle,
};
use dioxus::prelude::*;
use dioxus_icons::lucide::TrendingUp;

/// Ported from `$S/refs/ui/apps/v4/registry/new-york-v4/charts/chart-radar-label-custom.tsx`
/// (data, config, copy). shadcn's own demo replaces the category tick
/// renderer entirely with a custom multi-line `<text>` (each axis label
/// shows "desktop/mobile" values above the month name) via a render-prop
/// `tick` function. This crate's data-driven chart API deliberately has no
/// render-prop/children-introspection mechanism for internal ticks (same
/// reasoning as `primitives/src/chart/engine/data.rs`'s own module doc:
/// Dioxus's `Element` gives a parent no equivalent of React's
/// `Children.map`/`cloneElement`) -- ported here as the standard category
/// labels (`axis_labels: true`, the default) with this note, not the fully
/// custom tick markup.
fn generate_data() -> Vec<ChartDatum> {
    const MONTHS: [(&str, f64, f64); 6] = [
        ("January", 186.0, 80.0),
        ("February", 305.0, 200.0),
        ("March", 237.0, 120.0),
        ("April", 73.0, 190.0),
        ("May", 209.0, 130.0),
        ("June", 214.0, 140.0),
    ];
    MONTHS
        .iter()
        .map(|(label, desktop, mobile)| ChartDatum {
            label: label.to_string(),
            values: vec![Some(*desktop), Some(*mobile)],
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
        Card {
            CardHeader {
                CardTitle { "Radar Chart - Custom Label" }
                CardDescription { "Showing total visitors for the last 6 months" }
            }
            CardContent {
                ChartContainer { config, data: generate_data(), kind: ChartKind::Radar,
                    Chart {
                        width: 300.0,
                        height: 300.0,
                        aria_label: "Total visitors by month, desktop and mobile",
                        radar: RadarOptions { fill_opacity: 0.6, ..Default::default() },
                    }
                    ChartTooltip {}
                }
            }
            CardFooter {
                div { "Trending up by 5.2% this month " TrendingUp { size: "16px" } }
                div { "January - June 2024" }
            }
        }
    }
}
