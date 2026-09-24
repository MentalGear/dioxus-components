use super::super::component::*;
use dioxus::prelude::*;

/// Port of shadcn's `chart-bar-horizontal.tsx`: a single series
/// ("visitors") over browser categories, drawn with the category axis
/// running top-to-bottom and values running left-to-right --
/// `BarOptions::horizontal`. Like shadcn's own demo, both of `Chart`'s
/// default axes are hidden (`show_x_axis`/`show_y_axis: false`): this
/// family's own `render` draws its own category labels at the plot's left
/// edge when `horizontal` is set (`primitives/src/chart/components/series/
/// bar.rs`'s own module doc explains why that's this family's job, not
/// `components::layout`'s).
fn generate_data() -> Vec<ChartDatum> {
    const ROWS: [(&str, f64); 5] = [
        ("Chrome", 275.0),
        ("Safari", 200.0),
        ("Firefox", 187.0),
        ("Edge", 173.0),
        ("Other", 90.0),
    ];
    ROWS.iter()
        .map(|(label, value)| ChartDatum {
            label: label.to_string(),
            values: vec![Some(*value)],
            ..Default::default()
        })
        .collect()
}

#[component]
pub fn Demo() -> Element {
    let config = ChartConfig::new().series("visitors", "Visitors", "var(--dx-chart-1)");

    rsx! {
        Card {
            CardHeader {
                CardTitle { "Bar Chart - Horizontal" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config, data: generate_data(), kind: ChartKind::Bar,
                    Chart {
                        aria_label: "Visitors by browser",
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
