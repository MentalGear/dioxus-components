//! shadcn's `chart-area-interactive`: 91 days of desktop and mobile visitors,
//! stacked (mobile at the bottom) with a gradient fill at Recharts' default
//! 0.6 opacity, in a fixed 250px-tall chart that is as wide as the card. The
//! select narrows it to the last 30 or 7 days -- the days on or after
//! June 30 minus the range, so 91, 31 or 8 rows, as in shadcn.

use super::super::component::*;
use crate::components::card::{Card, CardAction, CardContent, CardDescription, CardHeader, CardTitle};
use crate::components::select::{Select, SelectOption};
use dioxus::prelude::*;

/// The ranges the select offers.
#[derive(Debug, Clone, Copy, PartialEq, strum::Display)]
pub(crate) enum TimeRange {
    #[strum(to_string = "Last 3 months")]
    Days90,
    #[strum(to_string = "Last 30 days")]
    Days30,
    #[strum(to_string = "Last 7 days")]
    Days7,
}

impl TimeRange {
    /// How many days before the last date the range starts.
    pub(crate) const fn days(self) -> usize {
        match self {
            TimeRange::Days90 => 90,
            TimeRange::Days30 => 30,
            TimeRange::Days7 => 7,
        }
    }
}

/// The 91-day dataset as `(date, desktop, mobile)` rows -- shadcn's own.
const ROWS: [(&str, f64, f64); 91] = [
    ("2024-04-01", 222.0, 150.0),
    ("2024-04-02", 97.0, 180.0),
    ("2024-04-03", 167.0, 120.0),
    ("2024-04-04", 242.0, 260.0),
    ("2024-04-05", 373.0, 290.0),
    ("2024-04-06", 301.0, 340.0),
    ("2024-04-07", 245.0, 180.0),
    ("2024-04-08", 409.0, 320.0),
    ("2024-04-09", 59.0, 110.0),
    ("2024-04-10", 261.0, 190.0),
    ("2024-04-11", 327.0, 350.0),
    ("2024-04-12", 292.0, 210.0),
    ("2024-04-13", 342.0, 380.0),
    ("2024-04-14", 137.0, 220.0),
    ("2024-04-15", 120.0, 170.0),
    ("2024-04-16", 138.0, 190.0),
    ("2024-04-17", 446.0, 360.0),
    ("2024-04-18", 364.0, 410.0),
    ("2024-04-19", 243.0, 180.0),
    ("2024-04-20", 89.0, 150.0),
    ("2024-04-21", 137.0, 200.0),
    ("2024-04-22", 224.0, 170.0),
    ("2024-04-23", 138.0, 230.0),
    ("2024-04-24", 387.0, 290.0),
    ("2024-04-25", 215.0, 250.0),
    ("2024-04-26", 75.0, 130.0),
    ("2024-04-27", 383.0, 420.0),
    ("2024-04-28", 122.0, 180.0),
    ("2024-04-29", 315.0, 240.0),
    ("2024-04-30", 454.0, 380.0),
    ("2024-05-01", 165.0, 220.0),
    ("2024-05-02", 293.0, 310.0),
    ("2024-05-03", 247.0, 190.0),
    ("2024-05-04", 385.0, 420.0),
    ("2024-05-05", 481.0, 390.0),
    ("2024-05-06", 498.0, 520.0),
    ("2024-05-07", 388.0, 300.0),
    ("2024-05-08", 149.0, 210.0),
    ("2024-05-09", 227.0, 180.0),
    ("2024-05-10", 293.0, 330.0),
    ("2024-05-11", 335.0, 270.0),
    ("2024-05-12", 197.0, 240.0),
    ("2024-05-13", 197.0, 160.0),
    ("2024-05-14", 448.0, 490.0),
    ("2024-05-15", 473.0, 380.0),
    ("2024-05-16", 338.0, 400.0),
    ("2024-05-17", 499.0, 420.0),
    ("2024-05-18", 315.0, 350.0),
    ("2024-05-19", 235.0, 180.0),
    ("2024-05-20", 177.0, 230.0),
    ("2024-05-21", 82.0, 140.0),
    ("2024-05-22", 81.0, 120.0),
    ("2024-05-23", 252.0, 290.0),
    ("2024-05-24", 294.0, 220.0),
    ("2024-05-25", 201.0, 250.0),
    ("2024-05-26", 213.0, 170.0),
    ("2024-05-27", 420.0, 460.0),
    ("2024-05-28", 233.0, 190.0),
    ("2024-05-29", 78.0, 130.0),
    ("2024-05-30", 340.0, 280.0),
    ("2024-05-31", 178.0, 230.0),
    ("2024-06-01", 178.0, 200.0),
    ("2024-06-02", 470.0, 410.0),
    ("2024-06-03", 103.0, 160.0),
    ("2024-06-04", 439.0, 380.0),
    ("2024-06-05", 88.0, 140.0),
    ("2024-06-06", 294.0, 250.0),
    ("2024-06-07", 323.0, 370.0),
    ("2024-06-08", 385.0, 320.0),
    ("2024-06-09", 438.0, 480.0),
    ("2024-06-10", 155.0, 200.0),
    ("2024-06-11", 92.0, 150.0),
    ("2024-06-12", 492.0, 420.0),
    ("2024-06-13", 81.0, 130.0),
    ("2024-06-14", 426.0, 380.0),
    ("2024-06-15", 307.0, 350.0),
    ("2024-06-16", 371.0, 310.0),
    ("2024-06-17", 475.0, 520.0),
    ("2024-06-18", 107.0, 170.0),
    ("2024-06-19", 341.0, 290.0),
    ("2024-06-20", 408.0, 450.0),
    ("2024-06-21", 169.0, 210.0),
    ("2024-06-22", 317.0, 270.0),
    ("2024-06-23", 480.0, 530.0),
    ("2024-06-24", 132.0, 180.0),
    ("2024-06-25", 141.0, 190.0),
    ("2024-06-26", 434.0, 380.0),
    ("2024-06-27", 448.0, 490.0),
    ("2024-06-28", 149.0, 200.0),
    ("2024-06-29", 103.0, 160.0),
    ("2024-06-30", 446.0, 400.0),
];

/// `"2024-04-01"` -> `("Apr", "1", "2024")`, from a fixed month-name table
/// instead of a date library.
fn date_parts(iso: &str) -> (&'static str, &str, &str) {
    let month = match &iso[5..7] {
        "04" => "Apr",
        "05" => "May",
        "06" => "Jun",
        _ => "",
    };
    (month, iso[8..10].trim_start_matches('0'), &iso[..4])
}

/// Every row, values in config order (mobile, desktop); the label is the ISO
/// date, which the axis and the tooltip format.
pub(crate) fn chart_data() -> Vec<ChartDatum> {
    ROWS.iter()
        .map(|&(date, desktop, mobile)| ChartDatum {
            label: date.to_string(),
            values: vec![Some(mobile), Some(desktop)],
            ..Default::default()
        })
        .collect()
}

/// The rows from `range.days()` days before the last date on: shadcn keeps
/// `date >= June 30 - days`, which includes both ends (91 / 31 / 8 rows).
pub(crate) fn visible_data(range: TimeRange) -> Vec<ChartDatum> {
    let all = chart_data();
    let start = all.len().saturating_sub(range.days() + 1);
    all[start..].to_vec()
}

/// The series, in draw (and stacking) order: mobile at the bottom.
pub(crate) fn chart_config() -> ChartConfig {
    ChartConfig::new()
        .series("mobile", "Mobile", "var(--dx-chart-2)")
        .series("desktop", "Desktop", "var(--dx-chart-1)")
}

#[component]
pub fn Demo() -> Element {
    let mut range = use_signal(|| TimeRange::Days90);

    rsx! {
        AreaChartGallery {
            Card {
                CardHeader {
                    CardTitle { "Area Chart - Interactive" }
                    CardDescription { "Showing total visitors for the last 3 months" }
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
                    ChartContainer { config: chart_config(), data: visible_data(range()), kind: ChartKind::Area,
                        Chart {
                            aria_label: "Visitors by day, mobile and desktop, stacked",
                            // A fixed 250px height; the width follows the card.
                            height: 250.0,
                            stacked: true,
                            curve: Curve::Natural,
                            cursor: false,
                            area: AreaOptions {
                                gradient: true,
                                fill_opacity: 0.6,
                                ..Default::default()
                            },
                            x_label: "Date",
                            min_tick_gap: 32.0,
                            x_tick_format: |date: String| {
                                let (month, day, _) = date_parts(&date);
                                format!("{month} {day}")
                            },
                        }
                        ChartTooltip {
                            label_format: |date: String| {
                                let (month, day, _) = date_parts(&date);
                                format!("{month} {day}")
                            },
                        }
                        ChartLegend {}
                    }
                }
            }
        }
    }
}
