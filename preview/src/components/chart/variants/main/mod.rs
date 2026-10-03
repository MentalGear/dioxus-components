use super::super::component::*;
use crate::components::card::{Card, CardAction, CardContent, CardDescription, CardHeader, CardTitle};
use crate::components::select::{Select, SelectOption};
use dioxus::prelude::*;

/// The ranges the select offers; each shows the tail of a fixed 90-day series.
#[derive(Debug, Clone, Copy, PartialEq, strum::Display)]
enum TimeRange {
    #[strum(to_string = "Last 3 months")]
    Days90,
    #[strum(to_string = "Last 30 days")]
    Days30,
    #[strum(to_string = "Last 7 days")]
    Days7,
}

impl TimeRange {
    const fn days(self) -> usize {
        match self {
            TimeRange::Days90 => 90,
            TimeRange::Days30 => 30,
            TimeRange::Days7 => 7,
        }
    }
}

/// 90 days of desktop/mobile visitor counts, generated from the row index so
/// the server-rendered and client-rendered markup agree.
fn generate_data() -> Vec<ChartDatum> {
    const MONTHS: [(&str, u32); 3] = [("Apr", 30), ("May", 31), ("Jun", 30)];
    let mut data = Vec::with_capacity(91);
    let mut i: f64 = 0.0;
    for (name, days) in MONTHS {
        for day in 1..=days {
            let desktop = (222.0 + 120.0 * (i * 0.11).sin() + i * 0.9).max(12.0).round();
            let mobile = (120.0 + 70.0 * (i * 0.17 + 1.3).sin() + i * 0.4).max(8.0).round();
            data.push(ChartDatum {
                label: format!("{name} {day}"),
                values: vec![Some(desktop), Some(mobile)],
                ..Default::default()
            });
            i += 1.0;
        }
    }
    data
}

#[component]
pub fn Demo() -> Element {
    let mut range = use_signal(|| TimeRange::Days90);
    let all_data = generate_data();
    let days = range().days();
    let visible: Vec<ChartDatum> = all_data.iter().rev().take(days).rev().cloned().collect();

    let config = ChartConfig::new()
        .series("desktop", "Desktop", "var(--dx-chart-1)")
        .series("mobile", "Mobile", "var(--dx-chart-2)");

    rsx! {
        Card {
            CardHeader {
                CardTitle { "Visitors" }
                CardDescription { "Total visitors for the selected range" }
                CardAction {
                    Select::<TimeRange> {
                        default_value: TimeRange::Days90,
                        trigger_aria_label: Some("Select a time range".to_string()),
                        on_value_change: move |value: Option<TimeRange>| {
                            if let Some(value) = value {
                                range.set(value);
                            }
                        },
                        SelectOption::<TimeRange> {
                            index: 0usize,
                            value: TimeRange::Days90,
                            text_value: "Last 3 months",
                            "Last 3 months"
                        }
                        SelectOption::<TimeRange> {
                            index: 1usize,
                            value: TimeRange::Days30,
                            text_value: "Last 30 days",
                            "Last 30 days"
                        }
                        SelectOption::<TimeRange> {
                            index: 2usize,
                            value: TimeRange::Days7,
                            text_value: "Last 7 days",
                            "Last 7 days"
                        }
                    }
                }
            }
            CardContent {
                ChartContainer { config, data: visible, kind: ChartKind::Area,
                    Chart {
                        // Not stacked: the two series overlap, each a
                        // translucent fill.
                        aria_label: "Visitors by day, desktop and mobile",
                        // Daily dates on the x axis. The default
                        // `x_tick_format` keeps only the first 3 characters of
                        // a label, which would turn every "Apr 1".."Jun 30"
                        // tick into "Apr"/"May"/"Jun", so the full label is
                        // used. `max_x_ticks` thins 90 days to about 8 ticks
                        // and leaves the 7-day view labelled daily.
                        x_tick_format: |label: String| label,
                        max_x_ticks: 8,
                        x_label: "Date",
                    }
                    ChartTooltip {}
                    ChartLegend {}
                }
            }
        }
    }
}
