use super::super::component::*;
use dioxus::prelude::*;

/// See `variants/main/mod.rs`'s own doc comment for why this dataset is
/// duplicated rather than shared.
fn chart_data() -> Vec<ChartDatum> {
    [
        ("Chrome", 275.0, "var(--dx-chart-1)"),
        ("Safari", 200.0, "var(--dx-chart-2)"),
        ("Firefox", 187.0, "var(--dx-chart-3)"),
        ("Edge", 173.0, "var(--dx-chart-4)"),
        ("Other", 90.0, "var(--dx-chart-5)"),
    ]
    .into_iter()
    .map(|(browser, visitors, color)| ChartDatum {
        label: browser.to_string(),
        values: vec![Some(visitors)],
        color: Some(color.to_string()),
    })
    .collect()
}

fn chart_config() -> ChartConfig {
    ChartConfig::new().series("visitors", "Visitors", "var(--dx-chart-1)")
}

/// Ports shadcn's `chart-pie-label.tsx`: each slice's own value drawn
/// inside it. shadcn's own demo floats the label outside the ring on a
/// leader line (Recharts' default `label` renderer); this crate always
/// centers a `PieLabels::Value` label at the slice's own centroid instead
/// (`primitives/src/chart/components/series/pie.rs`'s own module doc: the
/// polar engine's deliverable is `centroid`, not leader-line routing) -- a
/// documented simplification.
#[component]
pub fn Demo() -> Element {
    rsx! {
        Card {
            CardHeader {
                CardTitle { "Pie Chart - Label" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config: chart_config(), data: chart_data(), kind: ChartKind::Pie,
                    Chart {
                        aria_label: "Visitors by browser",
                        pie: PieOptions { labels: PieLabels::Value, ..Default::default() },
                    }
                    ChartTooltip { hide_label: true }
                }
            }
            CardFooter {
                div { "Trending up by 5.2% this month" }
                div { "Showing total visitors for the last 6 months" }
            }
        }
    }
}
