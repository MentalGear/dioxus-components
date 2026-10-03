use super::super::component::*;
use dioxus::prelude::*;

/// Which of the two series is currently drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Series {
    Desktop,
    Mobile,
}

impl Series {
    const ALL: [Series; 2] = [Series::Desktop, Series::Mobile];

    fn key(self) -> &'static str {
        match self {
            Series::Desktop => "desktop",
            Series::Mobile => "mobile",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Series::Desktop => "Desktop",
            Series::Mobile => "Mobile",
        }
    }

    fn color(self) -> &'static str {
        match self {
            Series::Desktop => "var(--dx-chart-2)",
            Series::Mobile => "var(--dx-chart-1)",
        }
    }
}

/// 90 days of desktop/mobile visitor counts as `(date label, desktop, mobile)`,
/// generated from the row index so the server and the client render the same
/// data. Labels are pre-formatted as "Apr 1"; see the identity `x_tick_format`
/// in `Demo`.
fn generate_data() -> Vec<(String, f64, f64)> {
    const MONTHS: [(&str, u32); 3] = [("Apr", 30), ("May", 31), ("Jun", 30)];
    let mut data = Vec::with_capacity(91);
    let mut i: f64 = 0.0;
    for (name, days) in MONTHS {
        for day in 1..=days {
            let desktop = (222.0 + 120.0 * (i * 0.11).sin() + i * 0.9).max(12.0).round();
            let mobile = (150.0 + 90.0 * (i * 0.17 + 1.3).sin() + i * 0.6).max(8.0).round();
            data.push((format!("{name} {day}"), desktop, mobile));
            i += 1.0;
        }
    }
    data
}

/// Group an integer's digits by thousands with `,` (12345 -> "12,345").
fn format_total(value: f64) -> String {
    let n = value.round() as i64;
    let digits = n.unsigned_abs().to_string();
    let mut grouped = String::new();
    for (count, ch) in digits.chars().rev().enumerate() {
        if count > 0 && count % 3 == 0 {
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
    let raw = generate_data();

    let total_desktop: f64 = raw.iter().map(|(_, desktop, _)| desktop).sum();
    let total_mobile: f64 = raw.iter().map(|(_, _, mobile)| mobile).sum();
    let total_of = move |series: Series| match series {
        Series::Desktop => total_desktop,
        Series::Mobile => total_mobile,
    };

    let current = active();
    let config = ChartConfig::new().series(current.key(), current.label(), current.color());
    let data: Vec<ChartDatum> = raw
        .iter()
        .map(|(label, desktop, mobile)| {
            let value = match current {
                Series::Desktop => *desktop,
                Series::Mobile => *mobile,
            };
            ChartDatum {
                label: label.clone(),
                values: vec![Some(value)],
                ..Default::default()
            }
        })
        .collect();

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/bar_chart/style.css") }
        Card {
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
                            onclick: move |_| active.set(series),
                            span { class: "dx-bar-chart-interactive-toggle-label", "{series.label()}" }
                            span { class: "dx-bar-chart-interactive-toggle-total", "{format_total(total_of(series))}" }
                        }
                    }
                }
            }
            CardContent {
                ChartContainer { config, data, kind: ChartKind::Bar,
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
                        // and the default 3-character truncation would cut them
                        // down to the month.
                        x_tick_format: |label: String| label,
                    }
                    ChartTooltip {}
                }
            }
        }
    }
}
