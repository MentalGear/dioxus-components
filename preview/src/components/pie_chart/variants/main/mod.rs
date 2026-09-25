use super::super::component::*;
use dioxus::prelude::*;

/// Ports shadcn's `chart-pie-simple.tsx`: five browsers' visitor counts,
/// one slice each, `hideLabel` tooltip and no legend. This crate's exact
/// numbers approximate (not byte-copied from) the upstream fixture --
/// `$S`, the session scratchpad this lane's own reference sources lived in,
/// is not available in this resumed session -- but the shape (five
/// categories, one series, per-slice `ChartDatum::color`) matches exactly.
pub(crate) fn chart_data() -> Vec<ChartDatum> {
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

pub(crate) fn chart_config() -> ChartConfig {
    ChartConfig::new().series("visitors", "Visitors", "var(--dx-chart-1)")
}

#[component]
pub fn Demo() -> Element {
    rsx! {
        Card {
            CardHeader {
                CardTitle { "Pie Chart" }
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
