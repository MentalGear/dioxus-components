use super::super::component::*;
use crate::components::card::{Card, CardAction, CardContent, CardDescription, CardHeader, CardTitle};
use crate::components::select::{Select, SelectOption};
use dioxus::prelude::*;

/// The three ranges shadcn's own `chart-area-interactive` demo offers,
/// filtering the tail of the fixed 91-day series below.
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

/// The literal 91-row dataset from
/// `$S/refs/ui/apps/v4/registry/new-york-v4/charts/chart-area-interactive.tsx`
/// (`(date, desktop, mobile)`) -- ported verbatim rather than regenerated,
/// so this demo's shape (which days trend up/down, the two series'
/// relative sizes) matches the upstream demo exactly, not just its
/// statistical flavor. Formatted `"Mon D"` labels (matching the upstream
/// `tickFormatter`'s `toLocaleDateString(..., { month: "short", day:
/// "numeric" })` output) are built once in [`generate_data`], not stored
/// as separate ISO strings -- this crate's `ChartDatum::label` is the one
/// value both the hidden data table and the x-axis format from (see
/// `x_tick_format` below, which returns it unchanged rather than this
/// crate's own default first-3-characters truncation, which would cut
/// every label down to just `"Apr"`/`"May"`/`"Jun"` and lose the day
/// number).
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

/// `"2024-04-01"` -> `"Apr 1"` -- a fixed, hand-written month-name table
/// rather than a date-arithmetic dependency (this crate takes on none for
/// the chart engine; see `primitives/src/chart/engine/mod.rs`'s own doc),
/// enough for this literal dataset's own two months' worth of ISO dates.
fn format_date_label(iso: &str) -> String {
    let month = match &iso[5..7] {
        "04" => "Apr",
        "05" => "May",
        "06" => "Jun",
        other => other,
    };
    let day = iso[8..10].trim_start_matches('0');
    format!("{month} {day}")
}

fn generate_data() -> Vec<ChartDatum> {
    ROWS.iter()
        .map(|&(date, desktop, mobile)| ChartDatum {
            label: format_date_label(date),
            values: vec![Some(desktop), Some(mobile)],
            ..Default::default()
        })
        .collect()
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
        div { class: "dx-area-chart-gallery",
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
                    ChartContainer { config, data: visible, kind: ChartKind::Area,
                        Chart {
                            // Stacked, per the upstream demo's shared
                            // `stackId="a"` on both `<Area>`s -- unlike the
                            // generic `chart` package's own `main` variant
                            // (a deliberate, documented, non-stacked
                            // simplification of this same upstream demo).
                            aria_label: "Visitors by day, desktop and mobile",
                            stacked: true,
                            x_label: "Date",
                            x_tick_format: |label: String| label,
                        }
                        ChartTooltip {}
                        ChartLegend {}
                    }
                }
            }
        }
    }
}
