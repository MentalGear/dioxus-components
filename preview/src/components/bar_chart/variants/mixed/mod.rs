use super::super::component::*;
use dioxus::prelude::*;

/// Port of shadcn's `chart-bar-mixed.tsx`: the same horizontal browser
/// layout as this gallery's own `horizontal` variant, but every bar gets
/// its own distinct color (`ChartDatum::color`, one of the eight
/// `--dx-chart-<1..8>` categorical tokens per browser) instead of one flat
/// series color -- shadcn's own demo does this the same way, a per-row
/// `fill` referencing that row's own `chartConfig` entry.
fn generate_data() -> Vec<ChartDatum> {
    const ROWS: [(&str, f64, &str); 5] = [
        ("Chrome", 275.0, "var(--dx-chart-1)"),
        ("Safari", 200.0, "var(--dx-chart-2)"),
        ("Firefox", 187.0, "var(--dx-chart-3)"),
        ("Edge", 173.0, "var(--dx-chart-4)"),
        ("Other", 90.0, "var(--dx-chart-5)"),
    ];
    ROWS.iter()
        .map(|(label, value, color)| ChartDatum {
            label: label.to_string(),
            values: vec![Some(*value)],
            color: Some(color.to_string()),
        })
        .collect()
}

#[component]
pub fn Demo() -> Element {
    let config = ChartConfig::new().series("visitors", "Visitors", "var(--dx-chart-1)");

    rsx! {
        Card {
            CardHeader {
                CardTitle { "Bar Chart - Mixed" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config, data: generate_data(), kind: ChartKind::Bar,
                    Chart {
                        aria_label: "Visitors by browser, each its own color",
                        show_x_axis: false,
                        show_y_axis: false,
                        bar: BarOptions { horizontal: true, ..Default::default() },
                    }
                    ChartTooltip { hide_label: true }
                }
            }
            BarChartTrendFooter {}
        }
    }
}
