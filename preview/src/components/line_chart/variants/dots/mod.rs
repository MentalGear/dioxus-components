use super::super::component::*;
use crate::components::card::{Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle};
use dioxus::prelude::*;
use dioxus_icons::lucide::TrendingUp;

/// Port of shadcn's `chart-line-dots.tsx`
/// (`$S/refs/ui/apps/v4/registry/new-york-v4/charts/chart-line-dots.tsx`,
/// clone commit `a87a63b`): `show_dots: true` draws a filled circle at
/// every data point, colored from the series (`chart/style.css`'s
/// `[data-slot="chart-dot"] { fill: var(--series-color); }`), matching the
/// source's own `dot={{ fill: "var(--color-desktop)" }}`.
///
/// The source's `chartConfig` also declares a `mobile` series (label/color)
/// that its `chartData` rows carry a value for, but only ONE `<Line
/// dataKey="desktop">` is ever rendered in its JSX -- `mobile` is declared
/// but never drawn. This port's `ChartConfig` has only the `desktop` series
/// to match the actual rendered output faithfully: this primitive's `Chart`
/// draws one mark per *configured* series, so there is no way to declare an
/// undrawn one the way the untyped JSX config happens to allow.
///
/// The source's `activeDot={{ r: 6 }}` (enlarge the hovered point) is not
/// yet ported -- it needs the active-index-aware dot the primitive-side
/// `LineOptions`/`data-active` work (`$S/stage2-common.md`'s ownership
/// table) adds to `components/series/line.rs`, landing in a follow-up
/// commit to this same file once that primitive change is in.
fn generate_data() -> Vec<ChartDatum> {
    const MONTHS: [(&str, f64); 6] = [
        ("January", 186.0),
        ("February", 305.0),
        ("March", 237.0),
        ("April", 73.0),
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
                CardTitle { "Line Chart - Dots" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config, data: generate_data(), kind: ChartKind::Line,
                    Chart {
                        aria_label: "Visitors by month, desktop",
                        x_label: "Month",
                        line: LineOptions {
                            dots: true,
                            ..Default::default()
                        },
                    }
                    ChartTooltip { hide_label: true }
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
