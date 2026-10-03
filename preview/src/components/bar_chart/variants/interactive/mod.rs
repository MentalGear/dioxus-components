//! shadcn's `chart-bar-interactive`: 91 days of desktop and mobile visitors
//! with a toggle in the card header -- each button shows its series' total
//! (24,828 / 25,010) and picks the series drawn, in a fixed 250px-tall chart
//! as wide as the card, with the muted hover band behind the active day.

use super::super::component::*;
use dioxus::prelude::*;

/// Which of the two series the chart currently draws.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Series {
    Desktop,
    Mobile,
}

impl Series {
    const ALL: [Series; 2] = [Series::Desktop, Series::Mobile];

    const fn key(self) -> &'static str {
        match self {
            Series::Desktop => "desktop",
            Series::Mobile => "mobile",
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Series::Desktop => "Desktop",
            Series::Mobile => "Mobile",
        }
    }

    /// shadcn's config colors for this chart.
    const fn color(self) -> &'static str {
        match self {
            Series::Desktop => "var(--dx-chart-2)",
            Series::Mobile => "var(--dx-chart-1)",
        }
    }

    /// This series' position in [`ChartDatum::values`] of [`chart_data`].
    const fn index(self) -> usize {
        match self {
            Series::Desktop => 0,
            Series::Mobile => 1,
        }
    }
}

/// The 91-day dataset as `(date, desktop, mobile)` rows -- shadcn's own
/// (the same rows as the area and line/bar interactive charts).
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

/// Every row as `[desktop, mobile]`; the label is the ISO date, which the
/// axis and the tooltip format.
pub(crate) fn chart_data() -> Vec<ChartDatum> {
    ROWS.iter()
        .map(|&(date, desktop, mobile)| ChartDatum {
            label: date.to_string(),
            values: vec![Some(desktop), Some(mobile)],
            ..Default::default()
        })
        .collect()
}

/// Both series, as the header toggles them (the chart draws one at a time).
pub(crate) fn chart_config() -> ChartConfig {
    ChartConfig::new()
        .series(Series::Desktop.key(), Series::Desktop.label(), Series::Desktop.color())
        .series(Series::Mobile.key(), Series::Mobile.label(), Series::Mobile.color())
}

/// Thousands-grouped integer formatting, e.g. `12345.0` -> `"12,345"`.
fn format_total(value: f64) -> String {
    let n = value.round() as i64;
    let digits = n.unsigned_abs().to_string();
    let mut grouped = String::new();
    for (i, ch) in digits.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    let grouped: String = grouped.chars().rev().collect();
    if n < 0 {
        format!("-{grouped}")
    } else {
        grouped
    }
}

#[component]
pub fn Demo() -> Element {
    let mut active = use_signal(|| Series::Desktop);
    let data = chart_data();
    let totals: [f64; 2] = Series::ALL.map(|series| {
        data.iter()
            .filter_map(|d| d.values.get(series.index()).copied().flatten())
            .sum()
    });

    // Only the active series is configured, so only it is drawn.
    let current = active();
    let mut config = chart_config();
    config.series.retain(|s| s.key == current.key());
    let visible: Vec<ChartDatum> = data
        .iter()
        .map(|d| ChartDatum {
            label: d.label.clone(),
            values: vec![d.values[current.index()]],
            ..Default::default()
        })
        .collect();

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/bar_chart/style.css") }
        Card { class: "dx-bar-chart-interactive",
            CardHeader { class: "dx-bar-chart-interactive-header",
                div { class: "dx-bar-chart-interactive-heading",
                    CardTitle { "Bar Chart - Interactive" }
                    CardDescription { "Showing total visitors for the last 3 months" }
                }
                div { class: "dx-bar-chart-interactive-toggles",
                    for series in Series::ALL {
                        button {
                            key: "{series.key()}",
                            r#type: "button",
                            class: "dx-bar-chart-interactive-toggle",
                            "data-active": current == series,
                            "aria-pressed": current == series,
                            onclick: move |_| active.set(series),
                            span { class: "dx-bar-chart-interactive-toggle-label", "{series.label()}" }
                            span { class: "dx-bar-chart-interactive-toggle-total",
                                "{format_total(totals[series.index()])}"
                            }
                        }
                    }
                }
            }
            CardContent { class: "dx-bar-chart-interactive-content",
                ChartContainer { config, data: visible, kind: ChartKind::Bar,
                    Chart {
                        aria_label: "Visitors by day, {current.label()}",
                        // A fixed 250px height; the width follows the card.
                        height: 250.0,
                        margin: ChartMargin { left: 12.0, right: 12.0, ..ChartMargin::NONE },
                        x_label: "Date",
                        min_tick_gap: 32.0,
                        x_tick_format: |date: String| {
                            let (month, day, _) = date_parts(&date);
                            format!("{month} {day}")
                        },
                    }
                    ChartTooltip {
                        name_key: "Page Views",
                        label_format: |date: String| {
                            let (month, day, year) = date_parts(&date);
                            format!("{month} {day}, {year}")
                        },
                        style: "width: 150px",
                    }
                }
            }
        }
    }
}
