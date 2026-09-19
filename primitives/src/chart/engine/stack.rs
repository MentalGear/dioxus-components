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
}
