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

/// Renders a `"YYYY-MM-DD"` date as `"<Month> <day>, <year>"` (e.g. `"July 16, 2024"`), falling back to the raw string when it is malformed.
fn long_date(iso_date: &str) -> String {
    const MONTHS: [&str; 12] = [
        "January", "February", "March", "April", "May", "June", "July", "August", "September",
        "October", "November", "December",
    ];
    let parts: Vec<&str> = iso_date.split('-').collect();
    if let [y, m, d] = parts.as_slice() {
        if let (Ok(year), Ok(month), Ok(day)) =
            (y.parse::<i64>(), m.parse::<u32>(), d.parse::<u32>())
        {
            if let Some(name) = month.checked_sub(1).and_then(|i| MONTHS.get(i as usize)) {
                return format!("{name} {day}, {year}");
            }
        }
    }
    iso_date.to_string()
}

/// The label row shows the hovered date spelled out in full instead of the raw `"YYYY-MM-DD"` string.
#[component]
pub fn Demo() -> Element {
    rsx! {
        Gallery {
            Card {
                CardHeader {
                    CardTitle { "Tooltip - Label Formatter" }
                    CardDescription { "Tooltip with label formatter." }
                }
                CardContent {
                    ChartContainer { config: chart_config(), data: chart_data(), kind: ChartKind::Bar,
                        Chart {
                            aria_label: "Running and swimming calories by day",
                            // Open on the third day so every tooltip variant is visible
                            default_index: 2,
                            x_label: "Date",
                            stacked: true,
                            x_tick_format: |v: String| short_weekday(&v),
                        }
                        ChartTooltip { label_format: |v: String| long_date(&v) }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_date_spells_out_the_fixture_dates() {
        assert_eq!(long_date("2024-07-16"), "July 16, 2024");
        assert_eq!(long_date("2024-01-01"), "January 1, 2024");
        assert_eq!(long_date("2024-12-31"), "December 31, 2024");
    }

    #[test]
    fn long_date_falls_back_to_the_raw_string_when_malformed() {
        assert_eq!(long_date("not-a-date"), "not-a-date");
        assert_eq!(long_date("2024-13-01"), "2024-13-01");
    }
}
