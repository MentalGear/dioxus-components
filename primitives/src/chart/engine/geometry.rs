//! Turning one series' raw values into the point runs a line/area actually
//! draws -- pure, unit-tested, no `dioxus` types (see the `engine` module
//! doc for why). The one non-trivial piece of "series geometry" a
//! Line/Area mark needs beyond the scales themselves: deciding where a gap
//! in the data must break the drawn shape into more than one subpath,
//! rather than silently bridging across a missing point.
//!
//! Grouped (non-stacked, multi-series) bar positioning needs no function
//! of its own here: nesting a second [`super::scale::BandScale`] inside
//! one of an outer band scale's own bands (`range` set to that band's
//! `(start, start + width)`) already produces it -- see that type's own
//! doc and test.

/// Split one series' `values` (one entry per datum, `None` = missing) into
/// contiguous runs of `(x, value)` pairs, breaking at every `None` -- the
/// gap a line/area must not bridge by drawing a fake segment across it.
/// `xs` and `values` are positional (one x position, e.g. a
/// [`super::scale::BandScale`]'s per-datum centers, per value); a length
/// mismatch is handled defensively by stopping at the shorter of the two
/// rather than panicking. Values are returned *unscaled* -- the caller
/// maps each run's `y` through its own [`super::scale::LinearScale`]
/// afterward, so this function stays decoupled from any particular scale.
///
/// ```
/// use dioxus_primitives::chart::engine::geometry::plot_runs;
///
/// let xs = vec![0.0, 1.0, 2.0, 3.0];
/// let values = vec![Some(1.0), Some(2.0), None, Some(3.0)];
/// let runs = plot_runs(&xs, &values);
/// assert_eq!(runs, vec![vec![(0.0, 1.0), (1.0, 2.0)], vec![(3.0, 3.0)]]);
/// ```
pub fn plot_runs(xs: &[f64], values: &[Option<f64>]) -> Vec<Vec<(f64, f64)>> {
    let n = xs.len().min(values.len());
    let mut runs = Vec::new();
    let mut current: Vec<(f64, f64)> = Vec::new();
    for i in 0..n {
        match values[i] {
            Some(v) => current.push((xs[i], v)),
            None => {
                if !current.is_empty() {
                    runs.push(std::mem::take(&mut current));
                }
            }
        }
    }
    if !current.is_empty() {
        runs.push(current);
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plot_runs_breaks_at_none() {
        let xs = vec![0.0, 1.0, 2.0, 3.0];
        let values = vec![Some(1.0), Some(2.0), None, Some(3.0)];
        assert_eq!(
            plot_runs(&xs, &values),
            vec![vec![(0.0, 1.0), (1.0, 2.0)], vec![(3.0, 3.0)]]
        );
    }

    #[test]
    fn plot_runs_with_no_gaps_is_one_run() {
        let xs = vec![0.0, 1.0, 2.0];
        let values = vec![Some(1.0), Some(2.0), Some(3.0)];
        assert_eq!(
            plot_runs(&xs, &values),
            vec![vec![(0.0, 1.0), (1.0, 2.0), (2.0, 3.0)]]
        );
    }

    #[test]
    fn plot_runs_all_none_is_empty() {
        let xs = vec![0.0, 1.0];
        let values = vec![None, None];
        assert!(plot_runs(&xs, &values).is_empty());
    }

    #[test]
    fn plot_runs_leading_and_trailing_gaps() {
        let xs = vec![0.0, 1.0, 2.0, 3.0];
        let values = vec![None, Some(1.0), Some(2.0), None];
        assert_eq!(plot_runs(&xs, &values), vec![vec![(1.0, 1.0), (2.0, 2.0)]]);
    }

    #[test]
    fn plot_runs_mismatched_lengths_stops_at_the_shorter() {
        let xs = vec![0.0, 1.0];
        let values = vec![Some(1.0), Some(2.0), Some(3.0)];
        assert_eq!(plot_runs(&xs, &values), vec![vec![(0.0, 1.0), (1.0, 2.0)]]);
    }
}
