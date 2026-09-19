use super::super::component::*;
use crate::components::card::{Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle};
use dioxus::prelude::*;
use dioxus_icons::lucide::TrendingUp;

/// One series over six months -- ports
/// `$S/refs/ui/apps/v4/registry/new-york-v4/charts/chart-area-default.tsx`
/// (shadcn's `chart-area-default` demo) data, config, and copy verbatim.
/// The default `Curve::Monotone` interpolation matches shadcn's own
/// `type="natural"` (Recharts' `"natural"` curve is a natural cubic
/// spline, distinct in theory from this crate's Steffen-monotone spline,
/// but both are smooth interpolations through the same points and shadcn's
/// own docs use the two names for the same visual idea across its demos --
/// see `primitives/src/chart/engine/curve.rs`'s own module doc for why
/// this crate implements Steffen's monotone formula specifically).
fn generate_data() -> Vec<ChartDatum> {
    const ROWS: [(&str, f64); 6] = [
        ("January", 186.0),
        ("February", 305.0),
        ("March", 237.0),
        ("April", 73.0),
        ("May", 209.0),
        ("June", 214.0),
    ];
    ROWS.iter()
        .map(|&(label, desktop)| ChartDatum {
            label: label.to_string(),
            values: vec![Some(desktop)],
        })
        .collect()
}

#[component]
pub fn Demo() -> Element {
    let config = ChartConfig::new().series("desktop", "Desktop", "var(--dx-chart-1)");

    rsx! {
        div { class: "dx-area-chart-gallery",
            Card {
                CardHeader {
                    CardTitle { "Area Chart" }
                    CardDescription { "Showing total visitors for the last 6 months" }
                }
                CardContent {
                    ChartContainer { config, data: generate_data(), kind: ChartKind::Area,
                        Chart { aria_label: "Visitors by month, desktop" }
                        ChartTooltip {}
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
