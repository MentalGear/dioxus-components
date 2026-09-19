use super::super::component::*;
use crate::components::card::{Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle};
use dioxus::prelude::*;
use dioxus_icons::lucide::TrendingUp;

/// Port of shadcn's `chart-line-default.tsx`
/// (`$S/refs/ui/apps/v4/registry/new-york-v4/charts/chart-line-default.tsx`,
/// clone commit `a87a63b`): one series, no dots, the plain baseline every
/// other variant in this gallery starts from. The tooltip hides its label
/// row (the source's own `hideLabel`) since a single-series chart's
/// tooltip header would just repeat the x category already implied by the
/// one swatch row underneath it.
///
/// shadcn's `type="natural"` (a natural cubic spline) is ported as this
/// primitive's `Curve::Monotone` (`Chart`'s own default -- left unset
/// below) -- the closest existing curve (Steffen's monotonicity-preserving
/// cubic, `primitives/src/chart/engine/curve.rs`), and visually
/// indistinguishable from a natural spline for this demo's data.
/// `Curve::Natural` itself was not added: `$S/stage2-common.md`'s own
/// ownership line for `engine/curve.rs` calls it "no change expected;
/// ... if wanted for parity" -- optional polish, not required for a
/// faithful-looking port.
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
        })
        .collect()
}

#[component]
pub fn Demo() -> Element {
    let config = ChartConfig::new().series("desktop", "Desktop", "var(--dx-chart-1)");

    rsx! {
        Card {
            CardHeader {
                CardTitle { "Line Chart" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config, data: generate_data(), kind: ChartKind::Line,
                    Chart {
                        aria_label: "Visitors by month, desktop",
                        x_label: "Month",
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
