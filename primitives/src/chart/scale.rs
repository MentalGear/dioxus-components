//! Pure, unit-tested scale and path math for the chart primitive -- no `dioxus`
//! types anywhere in this file, matching `slider.rs`'s `ordered_range`/
//! `snap_value`/`closest_thumb_for` and `calendar.rs`'s date-grid helpers:
//! a chart's arithmetic is exercised with plain `#[test]` functions, never
//! Playwright, per `dev-docs/conformance-harness.md`'s own coverage table
//! ("Cargo unit tests for algorithms ... the algorithmic ones").
//!
//! ## Where each piece of math comes from
//!
//! - [`LinearScale::ticks`]/[`LinearScale::nice`]/[`nice_domain`] generate
//!   "round" numbers using the classic 1-2-5 step series (every step is
//!   `{1, 2, 5, 10} * 10^k`) -- the same construction described in Paul
//!   Heckbert's "Nice Numbers for Graph Labels" (*Graphics Gems*, 1990) and,
//!   independently re-derived here rather than transcribed, the same shape
//!   d3-array's `ticks`/`tickStep` (`src/ticks.js`) implement: the residual
//!   `step0 / 10^floor(log10(step0))` always lands in `[1, 10)`, and is
//!   rounded to the nearest of `{1, 2, 5, 10}` using the geometric-mean
//!   thresholds `sqrt(2) ≈ 1.41`, `sqrt(10) ≈ 3.16`, `sqrt(50) ≈ 7.07`.
//! - [`line_path`]/[`area_path`]'s `Curve::Monotone` arm implements
//!   monotone cubic Hermite interpolation (Fritsch & Carlson, "Monotone
//!   Piecewise Cubic Interpolation", *SIAM J. Numer. Anal.* 17(2), 1980):
//!   tangents start as the average of each point's two neighboring secant
//!   slopes, then are scaled down wherever `(m_k/Δ_k)² + (m_{k+1}/Δ_k)² > 9`
//!   so the resulting Hermite spline never overshoots between data points.
//!   This is the same curve family Recharts calls `"monotone"` (itself
//!   `d3-shape`'s `curveMonotoneX`) -- ported here as an independent
//!   implementation of the published algorithm, not a transcription of
//!   either project's source, so it carries no third-party license.
//! - [`stack`]'s "non-negative stacking, negative values keep their own
//!   baseline" rule is this crate's own scope decision (documented on the
//!   function itself), not a port of Recharts' considerably larger `"sign"`
//!   stack-offset family (`expand`/`wiggle`/`silhouette` are out of scope
//!   for the chart MVP -- see `$S/chart-api.md`).

/// A curve interpolation for [`line_path`]/[`area_path`].
///
/// `Monotone` matches Recharts' (and d3-shape's) `"monotone"` curve type;
/// `Step` matches `"step"`'s *after* variant (the step happens right at the
/// next point's x, not before it or at its midpoint) -- see this module's
/// doc for the exact algorithms.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Curve {
    /// Straight line segments between consecutive points.
    Linear,
    /// Monotone cubic Hermite interpolation (Fritsch-Carlson) -- smooth,
    /// and guaranteed to never overshoot past a data point's own value.
    Monotone,
    /// Step-after: a horizontal line to the next point's x, then a
    /// vertical line to its y.
    Step,
}

/// A linear (continuous, numeric-to-numeric) scale: maps a `domain` value
/// to a `range` value by simple proportional interpolation. Used for a
/// chart's y axis (value -> pixel) and, via [`nice_domain`], to pick that
/// axis's own bounds.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct LinearScale {
    /// The input interval, `(min, max)`. Either bound may be larger --
    /// `domain: (100.0, 0.0)` is a valid, deliberately-inverted scale (SVG
    /// y grows downward, so a chart typically inverts here).
    pub domain: (f64, f64),
    /// The output interval, `(min, max)`, in the same "either order is
    /// valid" sense as `domain`.
    pub range: (f64, f64),
}

impl LinearScale {
    /// Map `v` from [`Self::domain`] to [`Self::range`]. Not clamped --
    /// a value outside the domain maps to a point outside the range, which
    /// is what a chart wants for e.g. a cursor position mid-drag.
    ///
    /// ```
    /// use dioxus_primitives::chart::LinearScale;
    ///
    /// let s = LinearScale { domain: (0.0, 100.0), range: (0.0, 200.0) };
    /// assert_eq!(s.scale(50.0), 100.0);
    /// assert_eq!(s.scale(0.0), 0.0);
    /// assert_eq!(s.scale(100.0), 200.0);
    /// ```
    pub fn scale(&self, v: f64) -> f64 {
        let (d0, d1) = self.domain;
        let (r0, r1) = self.range;
        if d1 == d0 {
            return r0;
        }
        let t = (v - d0) / (d1 - d0);
        r0 + t * (r1 - r0)
    }

    /// Generate up to `count + 1` "nice" tick values spanning
    /// [`Self::domain`], stepping by `{1, 2, 5, 10} * 10^k` -- see the
    /// module doc. `count` is a target, not an exact count: d3's own
    /// `ticks` has the same property (the returned length depends on how
    /// evenly the domain divides by the chosen step).
    pub fn ticks(&self, count: usize) -> Vec<f64> {
        generate_ticks(self.domain.0, self.domain.1, count)
    }

    /// Extend [`Self::domain`] outward to the nearest "nice" step boundary
    /// on each end (d3-scale's `.nice()`), using a fixed target of
    /// `DEFAULT_NICE_TICKS` steps. A zero-width or non-finite domain is
    /// returned unchanged -- there is no meaningful step to nice-round to.
    ///
    /// ```
    /// use dioxus_primitives::chart::LinearScale;
    ///
    /// let s = LinearScale { domain: (0.32, 98.6), range: (0.0, 1.0) }.nice();
    /// assert_eq!(s.domain, (0.0, 100.0));
    /// ```
    pub fn nice(self) -> Self {
        let (d0, d1) = self.domain;
        if d0 == d1 || !d0.is_finite() || !d1.is_finite() {
            return self;
        }
        let ascending = d1 >= d0;
        let (lo, hi) = if ascending { (d0, d1) } else { (d1, d0) };
        let step = nice_step(lo, hi, DEFAULT_NICE_TICKS);
        let nice_lo = (lo / step).floor() * step;
        let nice_hi = (hi / step).ceil() * step;
        let domain = if ascending {
            (nice_lo, nice_hi)
        } else {
            (nice_hi, nice_lo)
        };
        Self { domain, ..self }
    }
}

/// The tick target [`LinearScale::nice`] and [`nice_domain`] both nice-round
/// against -- matches d3-scale's own default `.nice()` (no explicit count)
/// behavior. A chart's actually-*rendered* tick count
/// (`Chart::y_tick_count`, default `5`) is a separate, smaller request
/// against the already-niced domain -- the two can differ, the same way
/// d3's `.nice()` and a later `.ticks(n)` call are independent.
const DEFAULT_NICE_TICKS: usize = 10;

/// A band scale: divides `range` into `count` equal-width bands with even
/// padding between and around them, for a categorical axis (one band per
/// data point). Mirrors d3's `scaleBand` with `paddingInner == paddingOuter
/// == padding` (this crate's single `padding` field) and no rounding/align
/// -- this chart draws in SVG viewBox units, not integer pixels, so the
/// sub-pixel alignment d3's rounding solves for doesn't apply here.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct BandScale {
    /// The number of bands (one per category / data point).
    pub count: usize,
    /// The output interval, `(min, max)`.
    pub range: (f64, f64),
    /// Padding as a fraction of one band's step, applied both between
    /// bands and at the two outer edges equally. `0.0` = bands fill the
    /// range edge-to-edge with no gap; `1.0` = zero-width bands (all gap).
    pub padding: f64,
}

impl BandScale {
    /// The distance from one band's start to the next band's start.
    fn step(&self) -> f64 {
        let (r0, r1) = self.range;
        let n = (self.count.max(1)) as f64;
        (r1 - r0) / (n + self.padding)
    }

    /// The `(start, width)` of the `i`th band. `i >= count` extrapolates
    /// past the range rather than panicking, matching the other scale
    /// methods' "never panic on out-of-domain input" behavior.
    ///
    /// ```
    /// use dioxus_primitives::chart::BandScale;
    ///
    /// let s = BandScale { count: 3, range: (0.0, 120.0), padding: 0.0 };
    /// assert_eq!(s.band(0), (0.0, 40.0));
    /// assert_eq!(s.band(1), (40.0, 40.0));
    /// assert_eq!(s.band(2), (80.0, 40.0));
    /// ```
    pub fn band(&self, i: usize) -> (f64, f64) {
        if self.count == 0 {
            return (self.range.0, 0.0);
        }
        let step = self.step();
        let width = (step * (1.0 - self.padding)).max(0.0);
        let start = self.range.0 + step * self.padding + step * i as f64;
        (start, width)
    }

    /// The horizontal (or vertical) center of the `i`th band -- the point a
    /// hit band, axis label, or dot should align to.
    pub fn center(&self, i: usize) -> f64 {
        let (start, width) = self.band(i);
        start + width / 2.0
    }
}

/// Round `step0` (an ideal, not-yet-round step size) to the nearest of
/// `{1, 2, 5, 10} * 10^k` -- see the module doc for the algorithm and its
/// citation. Falls back to `1.0` for a non-positive or non-finite span,
/// which callers treat as "no meaningful step" rather than dividing by it.
fn nice_step(lo: f64, hi: f64, count: usize) -> f64 {
    let count = (count.max(1)) as f64;
    let span = (hi - lo).abs();
    if span == 0.0 || !span.is_finite() {
        return 1.0;
    }
    let rough_step = span / count;
    let magnitude = 10f64.powf(rough_step.log10().floor());
    let residual = rough_step / magnitude; // always in [1, 10)
    let step_multiplier = if residual >= 50f64.sqrt() {
        10.0
    } else if residual >= 10f64.sqrt() {
        5.0
    } else if residual >= 2f64.sqrt() {
        2.0
    } else {
        1.0
    };
    step_multiplier * magnitude
}

/// Generate ticks between `start` and `stop` (either order) stepping by a
/// [`nice_step`]-rounded amount, inclusive of an endpoint that lands
/// exactly on a step multiple. `start == stop` returns that single value
/// (a flat domain still gets one tick, e.g. for an all-equal-value chart).
fn generate_ticks(start: f64, stop: f64, count: usize) -> Vec<f64> {
    if start == stop {
        return vec![start];
    }
    if count == 0 {
        return Vec::new();
    }
    let ascending = stop >= start;
    let (lo, hi) = if ascending {
        (start, stop)
    } else {
        (stop, start)
    };
    let step = nice_step(lo, hi, count);
    if step <= 0.0 || !step.is_finite() {
        return Vec::new();
    }

    let mut r0 = (lo / step).round();
    let mut r1 = (hi / step).round();
    if r0 * step < lo {
        r0 += 1.0;
    }
    if r1 * step > hi {
        r1 -= 1.0;
    }

    let n = (r1 - r0) as i64;
    let mut out = Vec::new();
    if n >= 0 {
        out.reserve(n as usize + 1);
        for i in 0..=n {
            out.push((r0 + i as f64) * step);
        }
    }
    if !ascending {
        out.reverse();
    }
    out
}

/// Pick a y-axis domain from raw data bounds: always includes `0.0` (a
/// bar/area chart must never float its baseline away from zero, or the
/// heights it draws stop meaning what they say), then nice-rounds both
/// ends via [`LinearScale::nice`]. An empty span (e.g. all data, and zero,
/// coincide) falls back to `(lo, lo + 1.0)` so callers never build a scale
/// with a zero-width domain.
///
/// ```
/// use dioxus_primitives::chart::nice_domain;
///
/// assert_eq!(nice_domain(2.3, 47.8), (0.0, 50.0));
/// assert_eq!(nice_domain(-30.0, 60.0), (-30.0, 60.0));
/// assert_eq!(nice_domain(0.0, 0.0), (0.0, 1.0));
/// ```
pub fn nice_domain(min: f64, max: f64) -> (f64, f64) {
    let lo = min.min(0.0);
    let hi = max.max(0.0);
    if !lo.is_finite() || !hi.is_finite() {
        return (0.0, 1.0);
    }
    if lo == hi {
        return (lo, hi + 1.0);
    }
    LinearScale {
        domain: (lo, hi),
        range: (0.0, 1.0),
    }
    .nice()
    .domain
}

/// Format a coordinate for an SVG attribute: at most 3 decimal places (per
/// `$S/chart-api.md`'s numeric-formatting rule), trailing zeros and a
/// trailing `.` trimmed, and `-0` normalized to `0` (a value that rounds to
/// exactly zero from the negative side, e.g. a scale of a tiny negative
/// input, must not render the confusing, byte-different `-0`).
pub(crate) fn fmt_num(v: f64) -> String {
    if !v.is_finite() {
        return "0".to_string();
    }
    let rounded = (v * 1000.0).round() / 1000.0;
    let rounded = if rounded == 0.0 { 0.0 } else { rounded }; // normalize -0.0 -> 0.0
    let s = format!("{rounded:.3}");
    let s = s.trim_end_matches('0');
    let s = s.trim_end_matches('.');
    if s.is_empty() || s == "-" {
        "0".to_string()
    } else {
        s.to_string()
    }
}

/// Secant slopes between consecutive points: `Δ[k] = (y[k+1]-y[k]) /
/// (x[k+1]-x[k])`, one entry per adjacent pair (`len() == points.len() -
/// 1`). A zero (or, defensively, non-finite) run of x -- two points sharing
/// an x -- reports a flat `0.0` secant rather than an infinite one, which
/// [`monotone_tangents`] then treats exactly like any other flat run.
fn secants(points: &[(f64, f64)]) -> Vec<f64> {
    points
        .windows(2)
        .map(|w| {
            let (x0, y0) = w[0];
            let (x1, y1) = w[1];
            let dx = x1 - x0;
            if dx == 0.0 {
                0.0
            } else {
                (y1 - y0) / dx
            }
        })
        .collect()
}

/// Per-point tangents for a monotone cubic Hermite spline through `points`
/// (Fritsch-Carlson) -- see the module doc for the algorithm and citation.
/// Returns one tangent per point (`len() == points.len()`).
fn monotone_tangents(points: &[(f64, f64)]) -> Vec<f64> {
    let n = points.len();
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![0.0];
    }

    let d = secants(points);
    let mut m = vec![0.0; n];
    m[0] = d[0];
    m[n - 1] = d[n - 2];
    for k in 1..n - 1 {
        let (dk_1, dk) = (d[k - 1], d[k]);
        // A local extremum (the secants change sign) or a flat run keeps
        // the spline flat here too -- interpolating through it instead
        // would overshoot past the data point on one side, which is
        // exactly what "monotone" rules out.
        if dk_1 == 0.0 || dk == 0.0 || dk_1.signum() != dk.signum() {
            m[k] = 0.0;
        } else {
            m[k] = (dk_1 + dk) / 2.0;
        }
    }

    // Fritsch-Carlson step 2: shrink each interval's two tangents together
    // whenever they'd overshoot that interval's own secant by too much.
    for k in 0..n - 1 {
        let dk = d[k];
        if dk == 0.0 {
            m[k] = 0.0;
            m[k + 1] = 0.0;
            continue;
        }
        let mut alpha = m[k] / dk;
        let mut beta = m[k + 1] / dk;
        if alpha < 0.0 {
            m[k] = 0.0;
            alpha = 0.0;
        }
        if beta < 0.0 {
            m[k + 1] = 0.0;
            beta = 0.0;
        }
        let tau = alpha * alpha + beta * beta;
        if tau > 9.0 {
            let scale = 3.0 / tau.sqrt();
            m[k] = scale * alpha * dk;
            m[k + 1] = scale * beta * dk;
        }
    }

    m
}

fn linear_path(points: &[(f64, f64)]) -> String {
    let mut out = String::new();
    for (i, (x, y)) in points.iter().enumerate() {
        if i == 0 {
            out.push_str(&format!("M{} {}", fmt_num(*x), fmt_num(*y)));
        } else {
            out.push_str(&format!(" L{} {}", fmt_num(*x), fmt_num(*y)));
        }
    }
    out
}

/// Step-*after*: from each point, a horizontal line to the next point's x
/// (at the *current* point's y), then a vertical line up/down to the next
/// point's own y. The step transition happens right at the next point, not
/// before it or at the midpoint (those would be d3's `curveStepBefore`/
/// plain `curveStep`, not requested here).
fn step_after_path(points: &[(f64, f64)]) -> String {
    let mut out = format!("M{} {}", fmt_num(points[0].0), fmt_num(points[0].1));
    for w in points.windows(2) {
        let (_, y0) = w[0];
        let (x1, y1) = w[1];
        out.push_str(&format!(" L{} {}", fmt_num(x1), fmt_num(y0)));
        out.push_str(&format!(" L{} {}", fmt_num(x1), fmt_num(y1)));
    }
    out
}

/// Monotone cubic Hermite, converted to SVG cubic Bezier segments via the
/// standard Hermite-to-Bezier control-point construction (each segment's
/// control points sit a third of the way along x, offset by that
/// endpoint's own tangent).
fn monotone_path(points: &[(f64, f64)]) -> String {
    let tangents = monotone_tangents(points);
    let mut out = format!("M{} {}", fmt_num(points[0].0), fmt_num(points[0].1));
    for i in 0..points.len() - 1 {
        let (x0, y0) = points[i];
        let (x1, y1) = points[i + 1];
        let dx = (x1 - x0) / 3.0;
        let cp1x = x0 + dx;
        let cp1y = y0 + dx * tangents[i];
        let cp2x = x1 - dx;
        let cp2y = y1 - dx * tangents[i + 1];
        out.push_str(&format!(
            " C{} {} {} {} {} {}",
            fmt_num(cp1x),
            fmt_num(cp1y),
            fmt_num(cp2x),
            fmt_num(cp2y),
            fmt_num(x1),
            fmt_num(y1)
        ));
    }
    out
}

/// Build an SVG path `d` string tracing `points` in order, using `curve`'s
/// interpolation. Empty input renders nothing; a single point renders a
/// zero-length `M` (a caller drawing a single-datum line will pair this
/// with a dot mark, not expect a visible line).
pub fn line_path(points: &[(f64, f64)], curve: Curve) -> String {
    if points.is_empty() {
        return String::new();
    }
    if points.len() == 1 {
        let (x, y) = points[0];
        return format!("M{} {}", fmt_num(x), fmt_num(y));
    }
    match curve {
        Curve::Linear => linear_path(points),
        Curve::Step => step_after_path(points),
        Curve::Monotone => monotone_path(points),
    }
}

/// Build a closed SVG path `d` string for an area: the same top edge as
/// [`line_path`], then straight down to `baseline_y` under the last point,
/// straight back to `baseline_y` under the first point, and closed (`Z`).
pub fn area_path(points: &[(f64, f64)], baseline_y: f64, curve: Curve) -> String {
    if points.is_empty() {
        return String::new();
    }
    if points.len() == 1 {
        let (x, y) = points[0];
        return format!(
            "M{x} {y} L{x} {b} Z",
            x = fmt_num(x),
            y = fmt_num(y),
            b = fmt_num(baseline_y)
        );
    }
    let top = line_path(points, curve);
    let first_x = points[0].0;
    let last_x = points[points.len() - 1].0;
    format!(
        "{top} L{lx} {b} L{fx} {b} Z",
        lx = fmt_num(last_x),
        b = fmt_num(baseline_y),
        fx = fmt_num(first_x)
    )
}

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

    /// Assert two `f64` slices are equal to within float rounding noise --
    /// `1.1 * 200.0` and the like are not bit-exact in IEEE 754, and a
    /// scale/tick implementation has no reason to special-case that away
    /// (real output goes through [`fmt_num`]'s 3-decimal rounding before it
    /// ever reaches an attribute, which absorbs noise at this scale).
    fn assert_close(actual: &[f64], expected: &[f64]) {
        assert_eq!(
            actual.len(),
            expected.len(),
            "length mismatch: {actual:?} vs {expected:?}"
        );
        for (a, e) in actual.iter().zip(expected) {
            assert!((a - e).abs() < 1e-9, "{actual:?} not close to {expected:?}");
        }
    }

    // -- LinearScale --------------------------------------------------

    #[test]
    fn linear_scale_maps_domain_to_range() {
        let s = LinearScale {
            domain: (0.0, 100.0),
            range: (0.0, 200.0),
        };
        assert_eq!(s.scale(0.0), 0.0);
        assert_eq!(s.scale(50.0), 100.0);
        assert_eq!(s.scale(100.0), 200.0);
        assert_close(&[s.scale(-10.0)], &[-20.0]); // extrapolates below the domain
        assert_close(&[s.scale(110.0)], &[220.0]); // extrapolates above the domain
    }

    #[test]
    fn linear_scale_handles_inverted_range_for_svg_y() {
        // SVG y grows downward -- a chart maps a high value to a *small* y.
        let s = LinearScale {
            domain: (0.0, 100.0),
            range: (200.0, 0.0),
        };
        assert_eq!(s.scale(0.0), 200.0);
        assert_eq!(s.scale(100.0), 0.0);
        assert_eq!(s.scale(50.0), 100.0);
    }

    #[test]
    fn linear_scale_degenerate_domain_returns_range_start() {
        let s = LinearScale {
            domain: (5.0, 5.0),
            range: (0.0, 100.0),
        };
        assert_eq!(s.scale(5.0), 0.0);
        assert_eq!(s.scale(999.0), 0.0);
    }

    #[test]
    fn ticks_are_round_1_2_5_steps() {
        let s = LinearScale {
            domain: (0.0, 100.0),
            range: (0.0, 1.0),
        };
        assert_eq!(s.ticks(5), vec![0.0, 20.0, 40.0, 60.0, 80.0, 100.0]);

        // The canonical d3-array example: domain (0,1), count 5 -> step 0.2.
        // `0.2` has no exact binary representation, so `i as f64 * step`
        // carries the usual IEEE 754 noise (e.g. `0.6000000000000001`) --
        // compare approximately rather than bit-exact.
        let unit = LinearScale {
            domain: (0.0, 1.0),
            range: (0.0, 1.0),
        };
        assert_close(&unit.ticks(5), &[0.0, 0.2, 0.4, 0.6, 0.8, 1.0]);
    }

    #[test]
    fn ticks_handle_descending_domain() {
        let s = LinearScale {
            domain: (100.0, 0.0),
            range: (0.0, 1.0),
        };
        assert_eq!(s.ticks(5), vec![100.0, 80.0, 60.0, 40.0, 20.0, 0.0]);
    }

    #[test]
    fn ticks_of_a_flat_domain_is_the_single_value() {
        let s = LinearScale {
            domain: (7.0, 7.0),
            range: (0.0, 1.0),
        };
        assert_eq!(s.ticks(5), vec![7.0]);
    }

    #[test]
    fn nice_extends_domain_to_round_bounds() {
        let s = LinearScale {
            domain: (0.32, 98.6),
            range: (0.0, 1.0),
        }
        .nice();
        assert_eq!(s.domain, (0.0, 100.0));

        // Already-nice bounds are left alone.
        let s2 = LinearScale {
            domain: (0.0, 50.0),
            range: (0.0, 1.0),
        }
        .nice();
        assert_eq!(s2.domain, (0.0, 50.0));
    }

    #[test]
    fn nice_preserves_range_and_ignores_degenerate_domain() {
        let s = LinearScale {
            domain: (3.0, 3.0),
            range: (10.0, 20.0),
        }
        .nice();
        assert_eq!(s.domain, (3.0, 3.0));
        assert_eq!(s.range, (10.0, 20.0));
    }

    // -- BandScale ------------------------------------------------------

    #[test]
    fn band_scale_no_padding_fills_range_contiguously() {
        let s = BandScale {
            count: 3,
            range: (0.0, 120.0),
            padding: 0.0,
        };
        assert_eq!(s.band(0), (0.0, 40.0));
        assert_eq!(s.band(1), (40.0, 40.0));
        assert_eq!(s.band(2), (80.0, 40.0));
        assert_eq!(s.center(1), 60.0);
    }

    #[test]
    fn band_scale_with_padding_shrinks_bands_and_insets_edges() {
        let s = BandScale {
            count: 2,
            range: (0.0, 100.0),
            padding: 0.5,
        };
        // step = 100 / (2 + 0.5) = 40; outer inset = 40*0.5 = 20;
        // band width = 40*0.5 = 20.
        let (start0, width0) = s.band(0);
        assert!((start0 - 20.0).abs() < 1e-9);
        assert!((width0 - 20.0).abs() < 1e-9);
        let (start1, _) = s.band(1);
        assert!((start1 - 60.0).abs() < 1e-9);
    }

    #[test]
    fn band_scale_full_padding_is_zero_width() {
        let s = BandScale {
            count: 4,
            range: (0.0, 100.0),
            padding: 1.0,
        };
        for i in 0..4 {
            assert_eq!(s.band(i).1, 0.0);
        }
    }

    #[test]
    fn band_scale_zero_count_never_panics() {
        let s = BandScale {
            count: 0,
            range: (0.0, 100.0),
            padding: 0.2,
        };
        assert_eq!(s.band(0), (0.0, 0.0));
        assert_eq!(s.center(0), 0.0);
    }

    // -- nice_domain ------------------------------------------------------

    #[test]
    fn nice_domain_always_includes_zero() {
        assert_eq!(nice_domain(2.3, 47.8), (0.0, 50.0));
        assert_eq!(nice_domain(-30.0, 60.0), (-30.0, 60.0));
        assert_eq!(nice_domain(-50.0, -10.0), (-50.0, 0.0));
    }

    #[test]
    fn nice_domain_flat_data_gets_a_non_empty_span() {
        assert_eq!(nice_domain(0.0, 0.0), (0.0, 1.0));
        // All values identical and positive: 0 is still included.
        let (lo, hi) = nice_domain(5.0, 5.0);
        assert_eq!(lo, 0.0);
        assert!(hi >= 5.0);
    }

    // -- fmt_num ------------------------------------------------------

    #[test]
    fn fmt_num_trims_to_at_most_3_decimals() {
        assert_eq!(fmt_num(1.0), "1");
        assert_eq!(fmt_num(1.5), "1.5");
        assert_eq!(fmt_num(1.23456), "1.235");
        assert_eq!(fmt_num(-0.0), "0");
        assert_eq!(fmt_num(0.00049), "0");
        assert_eq!(fmt_num(-2.0), "-2");
    }

    // -- line_path / area_path ------------------------------------------------------

    #[test]
    fn linear_line_path_is_straight_segments() {
        let path = line_path(&[(0.0, 0.0), (10.0, 10.0), (20.0, 0.0)], Curve::Linear);
        assert_eq!(path, "M0 0 L10 10 L20 0");
    }

    #[test]
    fn step_after_line_path_steps_at_the_next_point() {
        let path = line_path(&[(0.0, 0.0), (10.0, 10.0), (20.0, 0.0)], Curve::Step);
        assert_eq!(path, "M0 0 L10 0 L10 10 L20 10 L20 0");
    }

    #[test]
    fn monotone_line_path_reduces_to_a_straight_line_when_collinear() {
        // Collinear points (constant slope) is the base case every
        // monotone-cubic implementation must reproduce exactly: the
        // Fritsch-Carlson tangents all equal the shared slope, so the
        // Bezier control points land exactly back on the line.
        let path = line_path(&[(0.0, 0.0), (10.0, 10.0), (20.0, 20.0)], Curve::Monotone);
        assert_eq!(
            path,
            "M0 0 C3.333 3.333 6.667 6.667 10 10 C13.333 13.333 16.667 16.667 20 20"
        );
    }

    #[test]
    fn monotone_tangents_are_zero_at_a_local_extremum() {
        // A peak in the middle: the tangent there must be 0, or the curve
        // would overshoot above the peak on one side -- the entire point
        // of "monotone" interpolation.
        let tangents = monotone_tangents(&[(0.0, 0.0), (1.0, 10.0), (2.0, 0.0)]);
        assert_eq!(tangents.len(), 3);
        assert_eq!(tangents[1], 0.0);
    }

    #[test]
    fn monotone_path_single_point_is_a_bare_move() {
        assert_eq!(line_path(&[(5.0, 5.0)], Curve::Monotone), "M5 5");
        assert_eq!(line_path(&[], Curve::Monotone), "");
    }

    #[test]
    fn area_path_closes_down_to_the_baseline() {
        let path = area_path(&[(0.0, 10.0), (10.0, 0.0)], 20.0, Curve::Linear);
        assert_eq!(path, "M0 10 L10 0 L10 20 L0 20 Z");
    }

    #[test]
    fn area_path_single_point_is_a_closed_sliver() {
        let path = area_path(&[(5.0, 5.0)], 20.0, Curve::Linear);
        assert_eq!(path, "M5 5 L5 20 Z");
    }

    #[test]
    fn area_path_empty_is_empty() {
        assert_eq!(area_path(&[], 20.0, Curve::Linear), "");
    }

    // -- stack ------------------------------------------------------

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
