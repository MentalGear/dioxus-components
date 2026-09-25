use super::super::component::*;
use dioxus::prelude::*;

/// See `variants/main/mod.rs`'s own doc comment for why this dataset is
/// duplicated rather than shared.
const BROWSERS: [(&str, f64, &str); 5] = [
    ("Chrome", 275.0, "var(--dx-chart-1)"),
    ("Safari", 200.0, "var(--dx-chart-2)"),
    ("Firefox", 187.0, "var(--dx-chart-3)"),
    ("Edge", 173.0, "var(--dx-chart-4)"),
    ("Other", 90.0, "var(--dx-chart-5)"),
];

fn chart_data() -> Vec<ChartDatum> {
    BROWSERS
        .iter()
        .map(|&(browser, visitors, color)| ChartDatum {
            label: browser.to_string(),
            values: vec![Some(visitors)],
            color: Some(color.to_string()),
        })
        .collect()
}

fn chart_config() -> ChartConfig {
    ChartConfig::new().series("visitors", "Visitors", "var(--dx-chart-1)")
}

/// Ports shadcn's `chart-pie-label-custom.tsx`: a fully custom per-slice
/// label -- shadcn draws its own `<text>`/percentage combination via a
/// render-prop `label` function; this crate's equivalent `PieLabels::List`
/// (the exact same mechanism `label_list` uses, just with different
/// caller-supplied text -- see `PieLabels::List`'s own doc) is given each
/// slice's percentage share of the total, computed once up front.
#[component]
pub fn Demo() -> Element {
    let total: f64 = BROWSERS.iter().map(|&(_, v, _)| v).sum();
    let labels: Vec<String> = BROWSERS
        .iter()
        .map(|&(_, v, _)| format!("{:.0}%", v / total * 100.0))
        .collect();

    rsx! {
        Card {
            CardHeader {
                CardTitle { "Pie Chart - Custom Label" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config: chart_config(), data: chart_data(), kind: ChartKind::Pie,
                    Chart {
                        aria_label: "Visitors by browser",
                        pie: PieOptions { labels: PieLabels::List(labels), ..Default::default() },
                    }
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
