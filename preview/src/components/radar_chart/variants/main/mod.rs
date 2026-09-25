use super::super::component::*;
use crate::components::card::{
    Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle,
};
use dioxus::prelude::*;
use dioxus_icons::lucide::TrendingUp;

/// Ported from `$S/refs/ui/apps/v4/registry/new-york-v4/charts/chart-radar-default.tsx`
/// (data, config, copy, card chrome). shadcn's simplest radar: one series,
/// the default polygon grid, no dots.
fn generate_data() -> Vec<ChartDatum> {
    const MONTHS: [(&str, f64); 6] = [
        ("January", 186.0),
        ("February", 305.0),
        ("March", 237.0),
        ("April", 273.0),
        ("May", 209.0),
        ("June", 214.0),
    ];
    MONTHS
        .iter()
        .map(|(label, desktop)| ChartDatum {
            label: label.to_string(),
            values: vec![Some(*desktop)],
            ..Default::default()
        })
        .collect()
}

#[component]
pub fn Demo() -> Element {
    let config = ChartConfig::new().series("desktop", "Desktop", "var(--dx-chart-1)");

    rsx! {
        Card {
            CardHeader {
                CardTitle { "Radar Chart" }
                CardDescription { "Showing total visitors for the last 6 months" }
            }
            CardContent {
                ChartContainer { config, data: generate_data(), kind: ChartKind::Radar,
                    Chart {
                        width: 300.0,
                        height: 300.0,
                        aria_label: "Total visitors by month, desktop",
                        radar: RadarOptions {
                            fill_opacity: 0.6,
                            ..Default::default()
                        },
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
