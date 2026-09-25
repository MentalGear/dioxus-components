//! Series stacking -- pure, unit-tested, no `dioxus` types (see the
//! `engine` module doc for why).

/// Stack each row's series values into per-series `(y0, y1)` spans.
///
/// **Scope, stated plainly (per `$S/chart-api.md`):** this implements only
/// non-negative stacking, with negative values kept on their own baseline
/// -- not Recharts'/d3's full `"sign"` stack-offset family. Concretely, per
/// row, independently of every other row: a running *positive* baseline
/// and a running *negative* baseline each start at `0.0`. Each series
/// value, in config order:
/// - `None` -> a zero-height span at the current positive baseline (it
///   draws nothing either way, so which baseline is arbitrary).
/// - `Some(v)` with `v >= 0.0` -> spans `[positive_baseline,
///   positive_baseline + v]`, and advances the positive baseline to `v`'s
///   top.
/// - `Some(v)` with `v < 0.0` -> spans `[negative_baseline,
///   negative_baseline + v]` (so `y1 < y0`), and advances the negative
///   baseline down to `v`'s bottom.
///
/// This is enough for a stacked bar/area whose series are all one sign (the
/// common case, and the only one `$S/chart-api.md`'s MVP demos need); a
/// diverging stack (some positive, some negative, in the same row) renders
/// as two independent towers meeting at zero, which is a reasonable and
/// common reading but not Recharts' own (its default also does exactly
/// this for the "none" offset -- `expand`/`silhouette`/`wiggle` are the
/// offsets that do something more elaborate, and are out of scope here).
///
/// ```
/// use dioxus_primitives::chart::stack;
///
/// let rows = vec![
///     vec![Some(1.0), Some(2.0)],
///     vec![Some(-1.0), Some(3.0)],
///     vec![None, Some(5.0)],
/// ];
/// let stacked = stack(&rows);
/// assert_eq!(stacked[0], vec![(0.0, 1.0), (1.0, 3.0)]);
/// assert_eq!(stacked[1], vec![(0.0, -1.0), (0.0, 3.0)]);
/// assert_eq!(stacked[2], vec![(0.0, 0.0), (0.0, 5.0)]);
/// ```
pub fn stack(rows: &[Vec<Option<f64>>]) -> Vec<Vec<(f64, f64)>> {
    rows.iter()
        .map(|row| {
            let mut pos_top = 0.0f64;
            let mut neg_top = 0.0f64;
            row.iter()
                .map(|value| match value {
                    None => (pos_top, pos_top),
                    Some(v) if *v >= 0.0 => {
                        let y0 = pos_top;
                        pos_top += v;
                        (y0, pos_top)
                    }
                    Some(v) => {
                        let y0 = neg_top;
                        neg_top += v;
                        (y0, neg_top)
                    }
                })
                .collect()
        })
        .collect()
}

/// Per-row stack normalization: [`Normal`](StackMode::Normal) (this
/// module's own [`stack`] behavior, unchanged) or
/// [`Expand`](StackMode::Expand) (a "100% stacked" / percent-stacked
/// chart, e.g. shadcn's `chart-area-stacked-expand`/`chart-bar-stacked`-
/// with-`stackOffset="expand"` demos).
///
/// Mirrors d3-shape's `stackOffsetExpand` (`d3/d3-shape@a82254afd6`,
/// `src/offset/expand.js`, ISC license -- see `normalize_rows` (this
/// module's own, private, pre-pass function) for the
/// exact correspondence): for each row, divide every series' value by that
/// row's own total, so every row's stacked top lands at exactly `1.0`
/// (100%) instead of the row's raw sum -- a chart reading "what share of
/// this row is each series", not "how much is each series".
///
/// `#[non_exhaustive]`-free and `Default` (defaulting to `Normal`, i.e. a
/// caller who never mentions this type gets today's existing [`stack`]
/// behavior unchanged) so a family's own per-chart options struct (e.g.
/// the `area` family's `AreaOptions`) can hold a plain `stack_mode:
/// StackMode` field with `..Default::default()` support, per this stage's
/// own construction: `ChartProps` itself never grows a `stack_mode` field
/// -- every family that wants percent stacking (area today; bar via the
/// stage-2 ledger's own cross-lane request, since both already share this
/// module's plain [`stack`]) reads the *same* [`stack_with_mode`], from
/// its own family-specific options struct, rather than each duplicating
/// this normalization.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum StackMode {
    /// [`stack`]'s own behavior: raw cumulative spans.
    #[default]
    Normal,
    /// Normalize each row to fractions of its own total before stacking --
    /// see this type's own doc.
    Expand,
}

/// [`stack`], with an additional [`StackMode`] -- [`StackMode::Normal`]
/// (the default) is byte-for-byte [`stack`] itself;
/// [`StackMode::Expand`] first replaces every row with
/// `normalize_rows`'s (this module's own, private, pre-pass function)
/// output, then stacks that exactly like `Normal`
/// does. A new, additive function rather than a changed signature on
/// [`stack`] itself: every existing caller/doctest/unit test of `stack`
/// (this module's own, plus `primitives/src/chart/components/chart.rs`'s,
/// pre-dating this stage's family split) keeps compiling and behaving
/// identically, and a family that has no notion of percent stacking (e.g.
/// `Line`, where "stacked" is already meaningless -- see `ChartProps::
/// stacked`'s own doc) never has to pass a `StackMode` it would always set
/// to `Normal`.
///
/// ```
/// use dioxus_primitives::chart::engine::stack::{stack_with_mode, StackMode};
///
/// let rows = vec![vec![Some(1.0), Some(3.0)], vec![Some(2.0), Some(2.0)]];
/// // Normal: raw cumulative spans, same as `stack(&rows)`.
/// assert_eq!(stack_with_mode(&rows, StackMode::Normal), vec![
///     vec![(0.0, 1.0), (1.0, 4.0)],
///     vec![(0.0, 2.0), (2.0, 4.0)],
/// ]);
/// // Expand: row 0 (total 4) -> shares 0.25/0.75; row 1 (total 4) -> 0.5/0.5.
/// // Every row's stacked top is exactly 1.0 regardless of its raw total.
/// assert_eq!(stack_with_mode(&rows, StackMode::Expand), vec![
///     vec![(0.0, 0.25), (0.25, 1.0)],
///     vec![(0.0, 0.5), (0.5, 1.0)],
/// ]);
/// ```
pub fn stack_with_mode(rows: &[Vec<Option<f64>>], mode: StackMode) -> Vec<Vec<(f64, f64)>> {
    match mode {
        StackMode::Normal => stack(rows),
        StackMode::Expand => stack(&normalize_rows(rows)),
    }
}

/// The [`StackMode::Expand`] pre-pass: for each row, independently, sum
/// every `Some` value (a `None` contributes nothing to the sum, matching
/// d3-shape's own `series[i][j][1] || 0` -- a missing value has no share
/// of the total to speak of), then divide every `Some` value in that row
/// by the sum. A row whose sum is exactly `0.0` (every value `None`, every
/// value `0.0`, or a mix that happens to cancel) is returned unchanged --
/// d3-shape's own `if (y)` guard (`src/offset/expand.js`): dividing by
/// zero would produce `NaN`/`Infinity` for a row with nothing meaningful
/// to normalize to, so this leaves it as-is rather than manufacturing a
/// value out of nothing (an all-zero row stacks to all-zero spans either
/// way, so "unchanged" and "normalized" agree whenever every value is
/// already `0.0`; only the pathological canceling-mix case can visibly
/// differ from a hypothetical "always divide" version, and d3's own
/// behavior is the one this crate matches).
fn normalize_rows(rows: &[Vec<Option<f64>>]) -> Vec<Vec<Option<f64>>> {
    rows.iter()
        .map(|row| {
            let total: f64 = row.iter().filter_map(|v| *v).sum();
            if total == 0.0 {
                row.clone()
            } else {
                row.iter().map(|v| v.map(|x| x / total)).collect()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_accumulates_positive_series_and_isolates_negative_ones() {
        let rows = vec![
            vec![Some(1.0), Some(2.0)],
            vec![Some(-1.0), Some(3.0)],
            vec![None, Some(5.0)],
        ];
        let stacked = stack(&rows);
        assert_eq!(stacked[0], vec![(0.0, 1.0), (1.0, 3.0)]);
        assert_eq!(
            stacked[1],
            vec![(0.0, -1.0), (0.0, 3.0)],
            "a negative value stacks on its own baseline, not the positive one"
        );
        assert_eq!(
            stacked[2],
            vec![(0.0, 0.0), (0.0, 5.0)],
            "None is a zero-height span, and doesn't disturb the running baseline"
        );
    }

    #[test]
    fn stack_multiple_negative_series_accumulate_downward() {
        let rows = vec![vec![Some(-2.0), Some(-3.0)]];
        let stacked = stack(&rows);
        assert_eq!(stacked[0], vec![(0.0, -2.0), (-2.0, -5.0)]);
    }

    #[test]
    fn stack_of_no_rows_is_empty() {
        let stacked: Vec<Vec<(f64, f64)>> = stack(&[]);
        assert!(stacked.is_empty());
    }

    // -- StackMode / stack_with_mode / normalize_rows --------------------

    #[test]
    fn stack_mode_default_is_normal() {
        assert_eq!(StackMode::default(), StackMode::Normal);
    }

    #[test]
    fn stack_with_mode_normal_matches_plain_stack_exactly() {
        let rows = vec![
            vec![Some(1.0), Some(2.0)],
            vec![Some(-1.0), Some(3.0)],
            vec![None, Some(5.0)],
        ];
        assert_eq!(stack_with_mode(&rows, StackMode::Normal), stack(&rows));
    }

    #[test]
    fn stack_with_mode_expand_normalizes_every_row_to_a_100_percent_total() {
        // Three series, per `chart-area-stacked-expand`'s own shape: every
        // row's cumulative top must land at exactly 1.0 regardless of the
        // row's raw magnitude (January sums to 311, June to 514).
        let rows = vec![
            vec![Some(186.0), Some(80.0), Some(45.0)], // January, total 311
            vec![Some(214.0), Some(140.0), Some(160.0)], // June, total 514
        ];
        let spans = stack_with_mode(&rows, StackMode::Expand);
        for row in &spans {
            let top = row.last().unwrap().1;
            assert!(
                (top - 1.0).abs() < 1e-9,
                "row top {top} is not 1.0: {row:?}"
            );
        }
        // The first series' own share is its value / the row's total.
        assert!((spans[0][0].1 - 186.0 / 311.0).abs() < 1e-9);
        assert!((spans[1][0].1 - 214.0 / 514.0).abs() < 1e-9);
    }

    #[test]
    fn stack_with_mode_expand_treats_a_gap_as_zero_share_not_a_disqualifier() {
        let rows = vec![vec![Some(1.0), None, Some(3.0)]];
        let spans = stack_with_mode(&rows, StackMode::Expand);
        // Total is 1.0 + 3.0 = 4.0 (the None contributes nothing); the gap
        // itself stays a zero-height span at the running baseline, exactly
        // like `stack`'s own None handling (see that function's doc).
        assert_eq!(spans[0][0], (0.0, 0.25));
        assert_eq!(spans[0][1], (0.25, 0.25));
        assert_eq!(spans[0][2], (0.25, 1.0));
    }

    #[test]
    fn normalize_rows_leaves_an_all_zero_row_unchanged_rather_than_dividing_by_zero() {
        let rows = vec![vec![Some(0.0), Some(0.0)], vec![None, None]];
        assert_eq!(normalize_rows(&rows), rows);
        // And stacking that unchanged row produces all-zero spans, same as
        // plain `stack` would -- Expand and Normal agree here.
        assert_eq!(stack_with_mode(&rows, StackMode::Expand), stack(&rows));
    }

    #[test]
    fn normalize_rows_of_no_rows_is_empty() {
        assert!(normalize_rows(&[]).is_empty());
    }
}
