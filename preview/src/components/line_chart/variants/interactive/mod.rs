use super::super::component::*;
use crate::components::card::{Card, CardAction, CardContent, CardDescription, CardHeader, CardTitle};
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

    /// This series' position in [`ChartDatum::values`] (and in
    /// [`Self::ALL`]) -- `generate_data` always writes `[desktop, mobile]`
    /// in that order.
    const fn index(self) -> usize {
        match self {
            Series::Desktop => 0,
            Series::Mobile => 1,
        }
    }
}

/// Two series over 91 days with a toggle in the card header: each button shows
/// its series' running total and picks which series the chart draws. The
/// buttons are real `button` elements with `aria-pressed`. The data is
/// generated from the row index so the server and the client render the same
/// thing.
fn generate_data() -> Vec<ChartDatum> {
    const MONTHS: [(&str, u32); 3] = [("Apr", 30), ("May", 31), ("Jun", 30)];
    let mut data = Vec::with_capacity(91);
    let mut i: f64 = 0.0;
    for (name, days) in MONTHS {
        for day in 1..=days {
            let desktop = (230.0 + 160.0 * (i * 0.13).sin() + i * 0.7).max(15.0).round();
            let mobile = (210.0 + 150.0 * (i * 0.16 + 0.8).sin() + i * 0.5).max(15.0).round();
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
    let data = generate_data();
    let totals: [f64; 2] = Series::ALL.map(|series| {
        data.iter()
            .filter_map(|d| d.values.get(series.index()).copied().flatten())
            .sum()
    });

    // Only the active series is configured, so only it is drawn.
    let current = active();
    let config = ChartConfig::new().series(current.key(), current.label(), "var(--dx-chart-1)");
    let visible: Vec<ChartDatum> = data
        .iter()
        .map(|d| ChartDatum {
            label: d.label.clone(),
            values: vec![d.values[current.index()]],
            ..Default::default()
        })
        .collect();

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/line_chart/style.css") }
        Card {
            CardHeader {
                CardTitle { "Line Chart - Interactive" }
                CardDescription { "Showing total visitors for the last 3 months" }
                CardAction {
                    div { class: "dx-line-chart-toggle-row",
                        for series in Series::ALL {
                            button {
                                key: "{series.key()}",
                                class: "dx-line-chart-toggle",
                                r#type: "button",
                                "data-active": series == current,
                                "aria-pressed": series == current,
                                onclick: move |_| active.set(series),
                                span { class: "dx-line-chart-toggle-label", "{series.label()}" }
                                span { class: "dx-line-chart-toggle-value",
                                    "{format_total(totals[series.index()])}"
                                }
                            }
                        }
                    }
                }
            }
            CardContent {
                ChartContainer { config, data: visible, kind: ChartKind::Line,
                    Chart {
                        aria_label: "Visitors by day, {current.label()}",
                        // A fixed 250px height; the width follows the card.
                        // `width` is only the size used for the server/first
                        // render (see `ChartProps::fit_width`).
                        fit_width: true,
                        width: 700.0,
                        height: 250.0,
                        x_label: "Date",
                        // Identity formatter: the labels are already "Mon D",
                        // and the default 3-character truncation would turn
                        // every tick into "Apr"/"May"/"Jun".
                        x_tick_format: |label: String| label,
                        curve: Curve::Monotone,
                    }
                    ChartTooltip {}
                }
            }
        }
    }
}
