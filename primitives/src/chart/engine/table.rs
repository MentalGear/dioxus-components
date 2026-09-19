//! The hidden data table's row model -- pure, unit-tested, no `dioxus`
//! types (see the `engine` module doc for why). `Chart` renders
//! [`table_rows`]' output as a real `<table>`; this module only decides
//! *what* each cell says, never how it's marked up.

use super::data::ChartDatum;
use super::scale::fmt_decimal;

/// One row of the chart's visually-hidden data table: the x-category label
/// and one pre-formatted cell per series, in
/// [`super::data::ChartConfig::series`] order.
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
/// let data = vec![ChartDatum { label: "January".to_string(), values: vec![Some(186.0), None] }];
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
            },
            ChartDatum {
                label: "February".to_string(),
                values: vec![Some(305.125), None, Some(0.0)],
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
}
