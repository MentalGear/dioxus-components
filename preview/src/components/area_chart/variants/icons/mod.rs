use super::super::component::*;
use crate::components::card::{Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle};
use dioxus::prelude::*;
use dioxus_icons::lucide::{TrendingDown, TrendingUp};

/// Ports
/// `$S/refs/ui/apps/v4/registry/new-york-v4/charts/chart-area-icons.tsx`
/// (shadcn's `chart-area-icons` demo): the same stacked two-series data as
/// `stacked`/`legend`/`gradient`, with each series' `chartConfig` icon
/// (`TrendingDown` for desktop, `TrendingUp` for mobile, matching the
/// upstream file exactly) ported to [`ChartSeries::icon`].
///
/// **Not yet visually wired up** -- stated plainly, not silently shipped:
/// `ChartSeries::icon` is a stage-2 *extension point* the refactor lane
/// added (`primitives/src/chart/config.rs`: "added ... unused until a
/// later lane wires it into `ChartLegend`/`ChartTooltip`"); that wiring is
/// `s2-tooltip`'s own task (`components::{tooltip,legend}`), not
/// `s2-area`'s, and had not landed when this variant was written. This
/// demo sets the config field correctly (so it renders the *rest* of the
/// chart faithfully, and is ready to show icons the moment
/// `ChartLegend`/`ChartTooltip` read this field) rather than skip the
/// variant or fake the icon rendering here.
fn generate_data() -> Vec<ChartDatum> {
    const ROWS: [(&str, f64, f64); 6] = [
        ("January", 186.0, 80.0),
        ("February", 305.0, 200.0),
        ("March", 237.0, 120.0),
        ("April", 73.0, 190.0),
        ("May", 209.0, 130.0),
        ("June", 214.0, 140.0),
    ];
    ROWS.iter()
        .map(|&(label, desktop, mobile)| ChartDatum {
            label: label.to_string(),
            values: vec![Some(desktop), Some(mobile)],
            ..Default::default()
        })
        .collect()
}

#[component]
pub fn Demo() -> Element {
    let mut config = ChartConfig::new()
        .series("desktop", "Desktop", "var(--dx-chart-1)")
        .series("mobile", "Mobile", "var(--dx-chart-2)");
    config.series[0].icon = Some(ChartIcon(Callback::new(|()| rsx! { TrendingDown {} })));
    config.series[1].icon = Some(ChartIcon(Callback::new(|()| rsx! { TrendingUp {} })));

    rsx! {
        div { class: "dx-area-chart-gallery",
            Card {
                CardHeader {
                    CardTitle { "Area Chart - Icons" }
                    CardDescription { "Showing total visitors for the last 6 months" }
                }
                CardContent {
                    ChartContainer { config, data: generate_data(), kind: ChartKind::Area,
                        Chart {
                            aria_label: "Visitors by month, desktop and mobile, stacked",
                            stacked: true,
                        }
                        ChartTooltip {}
                        ChartLegend {}
                    }
                }
                CardFooter {
                    div { style: "display: flex; width: 100%; align-items: flex-start; gap: 8px; font-size: var(--dx-text-sm);",
                        div { style: "display: grid; gap: 8px;",
                            div { style: "display: flex; align-items: center; gap: 8px; font-weight: 600;",
                                "Trending up by 5.2% this month"
                                TrendingUp { size: "16px" }
                            }
                            div { style: "color: var(--secondary-color-5);", "January - June 2024" }
                        }
                    }
                }
            }
        }
    }
}
