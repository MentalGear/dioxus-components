use super::super::component::*;
use dioxus::prelude::*;

/// Ports shadcn's `chart-pie-stacked.tsx`: two concentric rings (desktop,
/// mobile), one per configured series -- this crate's `series::pie::render`
/// draws `ctx.config.series.len() > 1` as `render_rings` (one ring per
/// series, series 0 innermost) automatically, the same `ChartConfig`/
/// `ChartDatum` shape every Cartesian family uses (unlike this gallery's
/// other ten single-series demos).
fn chart_data() -> Vec<ChartDatum> {
    [
        ("January", 186.0, 80.0),
        ("February", 305.0, 200.0),
        ("March", 237.0, 120.0),
        ("April", 73.0, 190.0),
        ("May", 209.0, 130.0),
        ("June", 214.0, 140.0),
    ]
    .into_iter()
    .map(|(month, desktop, mobile)| ChartDatum {
        label: month.to_string(),
        values: vec![Some(desktop), Some(mobile)],
        ..Default::default()
    })
    .collect()
}

fn chart_config() -> ChartConfig {
    ChartConfig::new()
        .series("desktop", "Desktop", "var(--dx-chart-1)")
        .series("mobile", "Mobile", "var(--dx-chart-2)")
}

#[component]
pub fn Demo() -> Element {
    rsx! {
        Card {
            CardHeader {
                CardTitle { "Pie Chart - Stacked" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config: chart_config(), data: chart_data(), kind: ChartKind::Pie,
                    Chart {
                        aria_label: "Visitors by month, desktop and mobile",
                        pie: PieOptions { inner_radius: 30.0, ..Default::default() },
                    }
                    ChartTooltip {}
                    ChartLegend {}
                }
            }
            CardFooter {
                div { "Trending up by 5.2% this month" }
                div { "Showing total visitors for the last 6 months" }
            }
        }
    }
}
