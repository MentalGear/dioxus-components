//! The hidden data table's row model -- pure, unit-tested, no `dioxus`
//! types (see the `engine` module doc for why). `Chart` renders
//! [`table_rows`]' output as a real `<table>`; this module only decides
//! *what* each cell says, never how it's marked up.

use super::data::ChartDatum;
use super::scale::fmt_decimal;

/// One row of the chart's visually-hidden data table: the x-category label
/// and one pre-formatted cell per series, in
/// [`crate::chart::ChartConfig::series`] order (moved out of this `engine`
/// tree in the stage-2 chart round -- see `chart::config`'s own module
/// doc).
#[derive(Clone, PartialEq, Debug)]
pub struct TableRow {
    /// The row header (`th[scope=row]`) -- the datum's x-category label.
    pub label: String,
    /// One formatted value per series, aligned to [`ChartDatum::values`].
    /// Already display-ready: up to 2 decimal places, trimmed, or `"—"`
    /// for `None`.
    pub cells: Vec<String>,
}

/// Build one [`TableRow`] per datum, in order.
///
/// ```
/// use dioxus_primitives::chart::{engine::table::table_rows, ChartDatum};
///
/// let data = vec![ChartDatum { label: "January".to_string(), values: vec![Some(186.0), None], ..Default::default() }];
/// let rows = table_rows(&data);
/// assert_eq!(rows[0].label, "January");
/// assert_eq!(rows[0].cells, vec!["186".to_string(), "—".to_string()]);
/// ```
pub fn table_rows(data: &[ChartDatum]) -> Vec<TableRow> {
    data.iter()
        .map(|datum| TableRow {
            label: datum.label.clone(),
            cells: datum.values.iter().map(|v| format_cell(*v)).collect(),
        })
        .collect()
}

/// Build one [`TableRow`] per datum, same as [`table_rows`], but with only
/// the *first* configured series' value as its one cell. Used by `Chart`
/// for the polar-family stub kinds (`ChartKind::Pie`/`Radar`/`RadialBar` --
/// `components::chart`'s own module doc), whose real per-datum/per-category
/// data table shape is each owning lane's own job (`s2-polar`/`s2-radar`);
/// this MVP stand-in at least avoids either an empty table or a misleading
/// one that repeats every series' column for a family that doesn't draw
/// per-series marks per datum the way Area/Bar/Line do.
///
/// ```
/// use dioxus_primitives::chart::{engine::table::table_rows_single_series, ChartDatum};
///
/// let data = vec![ChartDatum { label: "A".to_string(), values: vec![Some(1.0), Some(2.0)], ..Default::default() }];
/// let rows = table_rows_single_series(&data);
/// assert_eq!(rows[0].cells, vec!["1".to_string()]);
/// ```
pub fn table_rows_single_series(data: &[ChartDatum]) -> Vec<TableRow> {
    data.iter()
        .map(|datum| TableRow {
            label: datum.label.clone(),
            cells: vec![format_cell(datum.values.first().copied().flatten())],
        })
        .collect()
}

/// Build one [`TableRow`] per datum, same shape as [`table_rows_single_series`]
/// (one value cell, from `values[0]`) plus a second cell: that datum's
/// share of every datum's `values[0]` summed, as a percentage. Used by
/// `Chart` for [`crate::chart::ChartKind::Pie`] specifically -- unlike
/// [`table_rows_single_series`] (still used for `Radar`/`RadialBar`,
/// where "percent of the total" isn't a meaningful reading of a radial
/// bar's own value), a pie slice's percentage of the whole is exactly
/// the visual information its wedge angle encodes, so the hidden table
/// should carry it too, not just the raw value the eye can't easily
/// recover a share from.
///
/// A negative or non-finite value contributes `0.0` to the sum (matching
/// `engine::polar::pie_layout`'s own treatment of a slice's angular
/// share), so this stays consistent with what a sighted user actually
/// sees drawn, even though the "value" cell itself still prints the raw
/// number unchanged.
///
/// ```
/// use dioxus_primitives::chart::{engine::table::table_rows_pie, ChartDatum};
///
/// let data = vec![
///     ChartDatum { label: "Chrome".to_string(), values: vec![Some(75.0)], ..Default::default() },
///     ChartDatum { label: "Safari".to_string(), values: vec![Some(25.0)], ..Default::default() },
/// ];
/// let rows = table_rows_pie(&data);
/// assert_eq!(rows[0].cells, vec!["75".to_string(), "75%".to_string()]);
/// assert_eq!(rows[1].cells, vec!["25".to_string(), "25%".to_string()]);
/// ```
pub fn table_rows_pie(data: &[ChartDatum]) -> Vec<TableRow> {
    let total: f64 = data
        .iter()
        .filter_map(|d| d.values.first().copied().flatten())
        .filter(|v| v.is_finite() && *v > 0.0)
        .sum();
    data.iter()
        .map(|datum| {
            let value = datum.values.first().copied().flatten();
            let percent = match value {
                Some(v) if v.is_finite() && v > 0.0 && total > 0.0 => {
                    format!("{}%", fmt_decimal(v / total * 100.0, 1))
                }
                _ => "0%".to_string(),
            };
            TableRow {
                label: datum.label.clone(),
                cells: vec![format_cell(value), percent],
            }
        })
        .collect()
}

/// Format one data table cell: `"—"` (em dash, matching this crate's other
/// "no value" convention) for `None`, else up to 2 decimal places.
fn format_cell(value: Option<f64>) -> String {
    match value {
        None => "—".to_string(),
        Some(v) => fmt_decimal(v, 2),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_rows_formats_values_and_gaps() {
        let data = vec![
            ChartDatum {
                label: "January".to_string(),
                values: vec![Some(186.0), Some(80.5), None],
                ..Default::default()
            },
            ChartDatum {
                label: "February".to_string(),
                values: vec![Some(305.125), None, Some(0.0)],
                ..Default::default()
            },
        ];
        let rows = table_rows(&data);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].label, "January");
        assert_eq!(rows[0].cells, vec!["186", "80.5", "—"]);
        assert_eq!(rows[1].label, "February");
        // 305.125 rounds to 2 decimals (half-way rounds away from zero).
        assert_eq!(rows[1].cells, vec!["305.13", "—", "0"]);
    }

    #[test]
    fn table_rows_of_no_data_is_empty() {
        assert!(table_rows(&[]).is_empty());
    }

    #[test]
    fn table_rows_single_series_keeps_only_the_first_value() {
        let data = vec![
            ChartDatum {
                label: "January".to_string(),
                values: vec![Some(186.0), Some(80.5)],
                ..Default::default()
            },
            ChartDatum {
                label: "February".to_string(),
                values: vec![None, Some(200.0)],
                ..Default::default()
            },
            ChartDatum {
                label: "March".to_string(),
                values: vec![],
                ..Default::default()
            },
        ];
        let rows = table_rows_single_series(&data);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].cells, vec!["186".to_string()]);
        // The first series' own gap stays a gap, even though the second
        // series has a value -- only `values[0]` is ever read.
        assert_eq!(rows[1].cells, vec!["—".to_string()]);
        // No series at all (an empty `values`) is a gap too, not a panic.
        assert_eq!(rows[2].cells, vec!["—".to_string()]);
    }

    #[test]
    fn table_rows_pie_adds_a_percent_column() {
        let data = vec![
            ChartDatum {
                label: "Chrome".to_string(),
                values: vec![Some(275.0)],
                ..Default::default()
            },
            ChartDatum {
                label: "Safari".to_string(),
                values: vec![Some(200.0)],
                ..Default::default()
            },
            ChartDatum {
                label: "Other".to_string(),
                values: vec![Some(25.0)],
                ..Default::default()
            },
        ];
        let rows = table_rows_pie(&data);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].cells, vec!["275".to_string(), "55%".to_string()]);
        assert_eq!(rows[1].cells, vec!["200".to_string(), "40%".to_string()]);
        assert_eq!(rows[2].cells, vec!["25".to_string(), "5%".to_string()]);
    }

    #[test]
    fn table_rows_pie_handles_a_none_value_and_an_all_zero_total() {
        let with_gap = vec![
            ChartDatum {
                label: "A".to_string(),
                values: vec![Some(10.0)],
                ..Default::default()
            },
            ChartDatum {
                label: "B".to_string(),
                values: vec![None],
                ..Default::default()
            },
        ];
        let rows = table_rows_pie(&with_gap);
        assert_eq!(rows[0].cells, vec!["10".to_string(), "100%".to_string()]);
        assert_eq!(rows[1].cells, vec!["—".to_string(), "0%".to_string()]);

        let all_zero = vec![ChartDatum {
            label: "A".to_string(),
            values: vec![Some(0.0)],
            ..Default::default()
        }];
        let rows = table_rows_pie(&all_zero);
        assert_eq!(rows[0].cells, vec!["0".to_string(), "0%".to_string()]);
    }
}
