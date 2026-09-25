use super::super::component::*;
use dioxus::prelude::*;

/// See `variants/main/mod.rs`'s own doc comment for why this dataset is
/// duplicated rather than shared -- every one of shadcn's eleven source
/// files does the same.
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

/// Ports shadcn's `chart-pie-separator-none.tsx`: a plain pie with the thin
/// per-slice separator stroke removed (`<Pie stroke="none" />`). This
/// crate's `chart-arc` draws no separator stroke at all in the first place
/// (`chart/style.css` has no `stroke` rule for it) -- there is no
/// `PieOptions` field for one to toggle off, so this demo is visually
/// identical to `main`'s own default: a documented simplification, not a
/// missing feature.
#[component]
pub fn Demo() -> Element {
    rsx! {
        Card {
            CardHeader {
                CardTitle { "Pie Chart - No Separator" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config: chart_config(), data: chart_data(), kind: ChartKind::Pie,
                    Chart { aria_label: "Visitors by browser" }
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
