use super::super::component::*;
use dioxus::prelude::*;

/// Port of shadcn's `chart-bar-multiple.tsx`: two series (desktop/mobile)
/// grouped side by side per category, rather than stacked -- the `stacked`
/// and `stacked_legend` variants are this same shape, stacked instead.
///
/// Simplification, stated plainly: shadcn's own demo gives the tooltip a
/// dashed-line indicator (`<ChartTooltipContent indicator="dashed" />`).
/// `ChartTooltip` has no indicator-style prop yet (only `hide_label`/
/// `hide_indicator` -- `primitives/src/chart/components/tooltip.rs`, owned
/// by the `s2-tooltip` lane, not this one), so this demo uses the default
/// swatch indicator instead. Everything else -- the two-series data, the
/// grouped (non-stacked) bars, the Card chrome -- matches shadcn's demo.
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
                CardTitle { "Bar Chart - Multiple" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config, data: generate_data(), kind: ChartKind::Bar,
                    Chart { aria_label: "Visitors by month, desktop and mobile" }
                    ChartTooltip {}
                }
            }
            BarChartTrendFooter {}
        }
    }
}
