use super::super::component::*;
use crate::components::select::{Select, SelectOption};
use dioxus::prelude::*;

/// The months the select offers.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Month {
    January,
    February,
    March,
    April,
    May,
}

impl Month {
    const ALL: [Month; 5] = [
        Month::January,
        Month::February,
        Month::March,
        Month::April,
        Month::May,
    ];

    fn label(self) -> &'static str {
        match self {
            Month::January => "January",
            Month::February => "February",
            Month::March => "March",
            Month::April => "April",
            Month::May => "May",
        }
    }

    fn index(self) -> usize {
        Self::ALL.iter().position(|m| *m == self).unwrap_or(0)
    }
}

/// Desktop visitors per month, one color per month.
fn chart_data() -> Vec<ChartDatum> {
    [
        ("January", 186.0, "var(--dx-chart-1)"),
        ("February", 305.0, "var(--dx-chart-2)"),
        ("March", 237.0, "var(--dx-chart-3)"),
        ("April", 173.0, "var(--dx-chart-4)"),
        ("May", 209.0, "var(--dx-chart-5)"),
    ]
    .into_iter()
    .map(|(month, desktop, color)| ChartDatum {
        label: month.to_string(),
        values: vec![Some(desktop)],
        color: Some(color.to_string()),
    })
    .collect()
}

fn chart_config() -> ChartConfig {
    ChartConfig::new().series("desktop", "Desktop", "var(--dx-chart-1)")
}

/// `1125.0` as `"1,125"` (JavaScript's `toLocaleString()`).
fn thousands(value: f64) -> String {
    let digits = format!("{value:.0}");
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// A `Select` picks the active month: its slice is drawn 10px further out
/// with a halo ring beyond the rim (`active_index` + `active_halo`), and its
/// value fills the donut's hole. A 300px square, as in shadcn.
#[component]
pub fn Demo() -> Element {
    let mut active = use_signal(|| Month::January);
    let data = chart_data();
    let active_visitors = data
        .get(active().index())
        .and_then(|d| d.values.first().copied().flatten())
        .unwrap_or(0.0);

    rsx! {
        Card {
            CardHeader {
                CardTitle { "Pie Chart - Interactive" }
                CardDescription { "January - June 2024" }
                CardAction {
                    Select::<Month> {
                        default_value: Month::January,
                        trigger_aria_label: Some("Select a value".to_string()),
                        on_value_change: move |value: Option<Month>| {
                            if let Some(value) = value {
                                active.set(value);
                            }
                        },
                        for month in Month::ALL {
                            SelectOption::<Month> {
                                key: "{month.label()}",
                                index: month.index(),
                                value: month,
                                text_value: month.label(),
                                {month.label()}
                            }
                        }
                    }
                }
            }
            CardContent {
                ChartContainer {
                    config: chart_config(),
                    data: data.clone(),
                    kind: ChartKind::Pie,
                    style: "max-inline-size: 300px",
                    Chart {
                        width: 300.0,
                        aria_label: "Visitors by month",
                        pie: PieOptions {
                            inner_radius: Radius::Px(60.0),
                            active_index: Some(active().index()),
                            active_halo: true,
                            center_text: Some((thousands(active_visitors), "Visitors".to_string())),
                            ..Default::default()
                        },
                    }
                    ChartTooltip { hide_label: true }
                }
            }
        }
    }
}
