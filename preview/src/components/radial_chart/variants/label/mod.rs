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

/// Ports shadcn's `chart-radial-label.tsx`: each ring's own category name
/// drawn inside its own arc -- `PieLabels::List`, this family's own reuse
/// of Pie's per-arc label mechanism (`RadialOptions::labels`'s own doc).
#[component]
pub fn Demo() -> Element {
    let labels: Vec<String> = BROWSERS.iter().map(|&(browser, ..)| browser.to_string()).collect();

    rsx! {
        Card {
            CardHeader {
                CardTitle { "Radial Chart - Label" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config: chart_config(), data: chart_data(), kind: ChartKind::RadialBar,
                    Chart {
                        width: 300.0,
                        height: 300.0,
                        aria_label: "Visitors by browser",
                        radial: RadialOptions {
                            inner_radius: 30.0,
                            labels: PieLabels::List(labels),
                            ..Default::default()
                        },
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
