use super::super::component::*;
use crate::components::card::{
    Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle,
};
use dioxus::prelude::*;
use dioxus_icons::lucide::TrendingUp;

/// Ported from `$S/refs/ui/apps/v4/registry/new-york-v4/charts/chart-radar-lines-only.tsx`
/// (data, config, copy, card chrome). Two stroke-only series, no fill.
///
/// shadcn's own demo also sets `radialLines={false}` on its grid; this port
/// keeps the default polygon grid (with spokes) here instead, since
/// `grid_circle_no_lines` already demonstrates a spokes-off grid on its
/// own -- this variant's own point is the fill/stroke behavior below, not
/// the grid.
fn generate_data() -> Vec<ChartDatum> {
    const MONTHS: [(&str, f64, f64); 6] = [
        ("January", 186.0, 160.0),
        ("February", 185.0, 170.0),
        ("March", 207.0, 180.0),
        ("April", 173.0, 160.0),
        ("May", 160.0, 190.0),
        ("June", 174.0, 204.0),
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
                CardTitle { "Radar Chart - Lines Only" }
                CardDescription { "Showing total visitors for the last 6 months" }
            }
            CardContent {
                ChartContainer { config, data: generate_data(), kind: ChartKind::Radar,
                    Chart {
                        width: 300.0,
                        height: 300.0,
                        aria_label: "Total visitors by month, desktop and mobile",
                        radar: RadarOptions { lines_only: true, fill_opacity: 0.0, ..Default::default() },
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
