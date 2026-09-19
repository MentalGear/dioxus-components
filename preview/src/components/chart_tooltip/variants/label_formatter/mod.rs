use super::super::component::*;
use crate::components::card::{Card, CardContent, CardDescription, CardHeader, CardTitle};
use dioxus::prelude::*;

/// See `variants/main/mod.rs`'s own doc comment for why this dataset is
/// duplicated rather than shared -- every one of shadcn's nine source
/// files does the same.
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
    })
    .collect()
}

fn chart_config() -> ChartConfig {
    ChartConfig::new()
        .series("running", "Running", "var(--dx-chart-1)")
        .series("swimming", "Swimming", "var(--dx-chart-2)")
}

/// Parses a `"YYYY-MM-DD"` date and renders it `"<Month> <day>, <year>"`
/// (e.g. `"July 16, 2024"`) -- ports shadcn's own `labelFormatter={(value)
/// => new Date(value).toLocaleDateString("en-US", { day: "numeric", month:
/// "long", year: "numeric" })}`. Unlike `short_weekday`
/// (`component.rs`, shared by all nine demos), this formatting is only
/// this one demo's own feature, so it stays local rather than joining that
/// shared helper. Falls back to the raw string when malformed, same
/// reasoning as `short_weekday`.
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

/// Ports shadcn's `chart-tooltip-label-formatter.tsx`: the label row shows
/// the active datum's date spelled out in full, instead of the raw
/// `"YYYY-MM-DD"` string.
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
