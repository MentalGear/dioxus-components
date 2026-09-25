use super::super::component::*;
use dioxus::prelude::*;

/// See `variants/main/mod.rs`'s own doc comment for why this dataset is
/// duplicated rather than shared.
const BROWSERS: [(&str, f64, &str); 5] = [
    ("chrome", 275.0, "var(--dx-chart-1)"),
    ("safari", 200.0, "var(--dx-chart-2)"),
    ("firefox", 187.0, "var(--dx-chart-3)"),
    ("edge", 173.0, "var(--dx-chart-4)"),
    ("other", 90.0, "var(--dx-chart-5)"),
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

/// Ports shadcn's `chart-pie-label-list.tsx`: each slice's own category
/// name drawn inside it, instead of its value -- `PieLabels::List`, given
/// the same lower-case browser names as this variant's own dataset
/// (shadcn's `label` prop reads `dataKey` off the same underlying row).
#[component]
pub fn Demo() -> Element {
    let labels: Vec<String> = BROWSERS.iter().map(|&(browser, ..)| browser.to_string()).collect();

    rsx! {
        Card {
            CardHeader {
                CardTitle { "Pie Chart - Label List" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config: chart_config(), data: chart_data(), kind: ChartKind::Pie,
                    Chart {
                        aria_label: "Visitors by browser",
                        pie: PieOptions { labels: PieLabels::List(labels), ..Default::default() },
                    }
                }
            }
            CardFooter {
                div { "Trending up by 5.2% this month" }
                div { "Showing total visitors for the last 6 months" }
            }
        }
    }
}
