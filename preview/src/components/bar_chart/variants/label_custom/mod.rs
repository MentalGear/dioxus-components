use super::super::component::*;
use dioxus::prelude::*;

/// Port of shadcn's `chart-bar-label-custom.tsx`: the same horizontal
/// browser layout as this gallery's own `horizontal`/`mixed` variants, plus
/// `BarOptions::inside_labels` -- each bar's own category name drawn
/// INSIDE it near its start (in a color meant to read against the bar's
/// fill) and its value just outside the bar's far end, replacing the
/// hidden default axis entirely (shadcn's own `<LabelList
/// dataKey="browser" position="insideLeft" />` + a second `<LabelList
/// dataKey="visitors" position="right" />`).
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
                CardTitle { "Bar Chart - Custom Label" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config, data: generate_data(), kind: ChartKind::Bar,
                    Chart {
                        aria_label: "Visitors by browser, labelled inside each bar",
                        show_x_axis: false,
                        show_y_axis: false,
                        show_grid: false,
                        bar: BarOptions {
                            horizontal: true,
                            inside_labels: true,
                            ..Default::default()
                        },
                    }
                }
            }
        }
    }
}
