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
/// A bar's pixel extent along its *value* axis (y for a vertical bar, x for
/// a horizontal one -- the caller places this pair on whichever screen axis
/// actually carries values, see `components::series::bar`): given the
/// value's own already-scaled pixel position and the zero baseline's pixel
/// position on that SAME axis, returns `(start, length)`, the lower pixel
/// coordinate and the span, ready to hand straight to an SVG `rect`'s
/// `x`/`width` (horizontal) or `y`/`height` (vertical) attributes.
///
/// Sign-aware by construction, not by a caller-side branch: a positive
/// value's bar runs from the baseline up to the value; a negative value's
/// runs from the baseline down to the value; a value sitting exactly on the
/// baseline is a zero-length bar at the baseline (drawn, not omitted -- an
/// explicit "no visible bar" that still occupies its slot, matching how a
/// stacked `None` renders as a zero-height span rather than skipping the
/// rect entirely). Because pixel space has no inherent sign (a *smaller*
/// pixel coordinate can mean a *larger* domain value, depending on which
/// axis and which way the scale runs), this compares the two PIXEL
/// positions directly rather than re-deriving the sign from the original
/// domain value -- correct for both a vertical bar's inverted y-scale
/// (`range: (plot_y1, plot_y0)`, larger domain value -> smaller pixel y) and
/// a horizontal bar's non-inverted x-scale (`range: (plot_x0, plot_x1)`,
/// larger domain value -> larger pixel x) with the exact same formula and no
/// per-orientation branch.
///
/// ```
/// use dioxus_primitives::chart::engine::geometry::bar_extent;
///
/// // Vertical bar, positive value: pixel y decreases as the value grows
/// // (inverted range), so the smaller pixel (40, the value's own position)
/// // is the rect's top, and the span reaches down to the baseline (100).
/// assert_eq!(bar_extent(40.0, 100.0), (40.0, 60.0));
/// // Vertical bar, negative value: the value's pixel position (140) sits
/// // BELOW the baseline (100) -- the rect starts at the baseline and
/// // extends down to the value.
/// assert_eq!(bar_extent(140.0, 100.0), (100.0, 40.0));
/// // Exactly at the baseline: a zero-length bar, still positioned there.
/// assert_eq!(bar_extent(100.0, 100.0), (100.0, 0.0));
/// ```
pub fn bar_extent(value_px: f64, zero_px: f64) -> (f64, f64) {
    if value_px <= zero_px {
        (value_px, zero_px - value_px)
    } else {
        (zero_px, value_px - zero_px)
    }
}

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

    // -- bar_extent -----------------------------------------------------

    #[test]
    fn bar_extent_positive_value_runs_up_to_the_baseline() {
        // Vertical bar, inverted y range: a positive value's own pixel (40)
        // is smaller (higher on screen) than the baseline's pixel (100).
        assert_eq!(bar_extent(40.0, 100.0), (40.0, 60.0));
    }

    #[test]
    fn bar_extent_negative_value_runs_down_from_the_baseline() {
        assert_eq!(bar_extent(140.0, 100.0), (100.0, 40.0));
    }

    #[test]
    fn bar_extent_at_the_baseline_is_zero_length_not_omitted() {
        assert_eq!(bar_extent(100.0, 100.0), (100.0, 0.0));
    }

    #[test]
    fn bar_extent_is_orientation_agnostic() {
        // Horizontal bar, non-inverted x range: a positive value's pixel
        // (260) is LARGER than the baseline's (200) -- the opposite pixel
        // relationship from the vertical case above, same formula, no
        // per-orientation branch.
        assert_eq!(bar_extent(260.0, 200.0), (200.0, 60.0));
        // Horizontal bar, negative value: pixel (140) is smaller than the
        // baseline's (200).
        assert_eq!(bar_extent(140.0, 200.0), (140.0, 60.0));
    }

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
