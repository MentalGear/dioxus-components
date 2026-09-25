use super::super::component::*;
use dioxus::prelude::*;

/// The same two-series stacking shadcn's `chart-bar-stacked.tsx` demo shows,
/// without its legend -- a deliberately bare baseline this gallery adds
/// alongside the faithful `stacked_legend` port (shadcn ships only the
/// legend-bearing version; splitting it in two here lets each variant show
/// exactly one thing: `Chart { stacked: true }` on its own here, plus
/// `ChartLegend` there). Same data as `stacked_legend`, so the two are
/// visibly the same chart, legend or not.
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
            ..Default::default()
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
                CardTitle { "Bar Chart - Stacked" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config, data: generate_data(), kind: ChartKind::Bar,
                    Chart { aria_label: "Visitors by month, desktop and mobile, stacked", stacked: true }
                    ChartTooltip { hide_label: true }
                }
            }
            BarChartTrendFooter {}
        }
    }
}
