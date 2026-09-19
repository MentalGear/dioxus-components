use super::super::component::*;
use crate::components::card::{Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle};
use dioxus::prelude::*;
use dioxus_icons::lucide::TrendingUp;

/// Port of shadcn's `chart-line-multiple.tsx`
/// (`$S/refs/ui/apps/v4/registry/new-york-v4/charts/chart-line-multiple.tsx`,
/// clone commit `a87a63b`): two series (desktop/mobile), `Curve::Monotone`
/// (the source's own `type="monotone"` -- an exact match, unlike the
/// `"natural"` demos this gallery's other variants substitute for). The
/// tooltip keeps its label row (no `hideLabel` here, matching the source):
/// with two series sharing one tooltip, the label is what tells them apart
/// from each other's own category.
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
                CardTitle { "Line Chart - Multiple" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config, data: generate_data(), kind: ChartKind::Line,
                    Chart {
                        aria_label: "Visitors by month, desktop and mobile",
                        x_label: "Month",
                        curve: Curve::Monotone,
                    }
                    ChartTooltip {}
                }
            }
            CardFooter {
                div { style: "display: flex; flex-direction: column; align-items: flex-start; gap: var(--dx-space-2); font-size: var(--dx-text-sm);",
                    div { style: "display: flex; align-items: center; gap: var(--dx-space-2); font-weight: 600; line-height: 1;",
                        "Trending up by 5.2% this month"
                        TrendingUp { size: "16px" }
                    }
                    div { style: "color: var(--secondary-color-5); line-height: 1;",
                        "Showing total visitors for the last 6 months"
                    }
                }
            }
        }
    }
}
