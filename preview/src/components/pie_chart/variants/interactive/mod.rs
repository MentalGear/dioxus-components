use super::super::component::*;
use crate::components::select::{Select, SelectOption};
use dioxus::prelude::*;

/// The six months shadcn's own `chart-pie-interactive.tsx` fixture covers
/// -- a small `Copy` enum plays the role of its `activeMonth` state string,
/// same convention `bar_chart/variants/interactive/mod.rs`'s own `Series`
/// enum uses for an analogous "pick one, keep its index" interaction.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Month {
    January,
    February,
    March,
    April,
    May,
    June,
}

impl Month {
    const ALL: [Month; 6] =
        [Month::January, Month::February, Month::March, Month::April, Month::May, Month::June];

    fn label(self) -> &'static str {
        match self {
            Month::January => "January",
            Month::February => "February",
            Month::March => "March",
            Month::April => "April",
            Month::May => "May",
            Month::June => "June",
        }
    }

    fn index(self) -> usize {
        Self::ALL.iter().position(|m| *m == self).unwrap_or(0)
    }
}

/// See `variants/main/mod.rs`'s own doc comment for why this dataset is
/// duplicated rather than shared -- here keyed by month instead of
/// browser, matching shadcn's own `chart-pie-interactive.tsx` fixture
/// shape (one visitor count per month, one color per month).
fn chart_data() -> Vec<ChartDatum> {
    [
        ("January", 186.0, "var(--dx-chart-1)"),
        ("February", 305.0, "var(--dx-chart-2)"),
        ("March", 237.0, "var(--dx-chart-3)"),
        ("April", 173.0, "var(--dx-chart-4)"),
        ("May", 209.0, "var(--dx-chart-5)"),
        ("June", 214.0, "var(--dx-chart-1)"),
    ]
    .into_iter()
    .map(|(month, visitors, color)| ChartDatum {
        label: month.to_string(),
        values: vec![Some(visitors)],
        color: Some(color.to_string()),
    })
    .collect()
}

fn chart_config() -> ChartConfig {
    ChartConfig::new().series("visitors", "Visitors", "var(--dx-chart-1)")
}

/// Ports shadcn's `chart-pie-interactive.tsx`: a `Select` drives which
/// month's slice is "active" (grown, `data-active="true"`) and the donut's
/// own center text together -- `PieOptions::active_index`, set from the
/// selected `Month`'s own position in [`chart_data`], is the single value
/// both effects read from.
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
                        trigger_aria_label: Some("Select a month".to_string()),
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
                ChartContainer { config: chart_config(), data: data.clone(), kind: ChartKind::Pie,
                    Chart {
                        aria_label: "Visitors by month",
                        pie: PieOptions {
                            inner_radius: 60.0,
                            active_index: Some(active().index()),
                            center_text: Some((format!("{active_visitors:.0}"), "Visitors".to_string())),
                            ..Default::default()
                        },
                    }
                    ChartTooltip { hide_label: true }
                }
            }
        }
    }
}
