use super::super::component::*;
use crate::components::card::{Card, CardContent, CardDescription, CardHeader, CardTitle};
use dioxus::prelude::*;

/// The six-day running and swimming dataset this gallery's demos share. Each demo keeps its own copy so it can be copied on its own.
fn chart_data() -> Vec<ChartDatum> {
    [
        ("2024-07-15", 450.0, 300.0),
        ("2024-07-16", 380.0, 420.0),
        ("2024-07-17", 520.0, 120.0),
        ("2024-07-18", 140.0, 550.0),
        ("2024-07-19", 600.0, 350.0),
        ("2024-07-20", 480.0, 400.0),
    ]
    .into_iter()
    .map(|(date, running, swimming)| ChartDatum {
        label: date.to_string(),
        values: vec![Some(running), Some(swimming)],
        ..Default::default()
    })
    .collect()
}

fn chart_config() -> ChartConfig {
    ChartConfig::new()
        .series("running", "Running", "var(--dx-chart-1)")
        .series("swimming", "Swimming", "var(--dx-chart-2)")
}

/// Each row's swatch is a thin, full-height bar instead of a dot.
#[component]
pub fn Demo() -> Element {
    rsx! {
        Gallery {
            Card {
                CardHeader {
                    CardTitle { "Tooltip - Line Indicator" }
                    CardDescription { "Tooltip with line indicator." }
                }
                CardContent {
                    ChartContainer { config: chart_config(), data: chart_data(), kind: ChartKind::Bar,
                        Chart {
                            aria_label: "Running and swimming calories by day",
                            // shadcn's `defaultIndex={1}`: open on the second day. No grid and no y axis, as in shadcn's tooltip demos
                            default_index: 1,
                            show_grid: false,
                            show_y_axis: false,
                            // `tickMargin={10}` and `cursor={false}`
                            tick_margin: 10.0,
                            cursor: false,
                            // The bottom series rounds its lower corners, the top one its upper corners (`radius={[0, 0, 4, 4]}` / `[4, 4, 0, 0]`)
                            bar: BarOptions {
                                series_radius: vec![
                                    BarRadius::corners(0.0, 0.0, 4.0, 4.0),
                                    BarRadius::corners(4.0, 4.0, 0.0, 0.0),
                                ],
                                ..Default::default()
                            },
                            x_label: "Date",
                            stacked: true,
                            x_tick_format: |v: String| short_weekday(&v),
                        }
                        ChartTooltip { indicator: TooltipIndicator::Line }
                    }
                }
            }
        }
    }
}
