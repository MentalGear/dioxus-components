//! SVG path generation for a series' line/area, in one of three
//! [`Curve`] interpolations -- pure, unit-tested, no `dioxus` types (see
//! the `engine` module doc for why).
//!
//! `Curve::Monotone`'s tangent formula (Steffen, M. 1990, "A Simple Method
//! for Monotonic Interpolation in One Dimension", *Astronomy and
//! Astrophysics* 239, p. 443) is cross-checked line-by-line against
//! `d3-shape`'s implementation of it (`src/curve/monotone.js`'s `sign`/
//! `slope3`/`slope2`, ISC license, `d3/d3-shape@a82254afd6`) -- confirmed
//! (via `recharts/src/shape/Curve.tsx`) to be literally what Recharts'
//! `"monotone"` curve type computes, so matching it is what "the same
//! curve family as Recharts'/shadcn's charts" actually requires. This is
//! an independent Rust implementation of Steffen's published formula, not
//! a transcription of d3-shape's JS -- no line below is copied from it --
//! consulted only to get the tangent arithmetic (particularly the
//! `(sign(s0) + sign(s1)) * min(|s0|, |s1|, 0.5*|p|)` **three-way** min,
//! and the one-sided `slope2` end-condition) bit-for-bit right, since nearby
//! but subtly different formulas exist in the wild (e.g. a two-way
//! `min(|s0|, 0.5*|p|)` some secondary ports use, which is *not* what
//! Recharts/d3-shape actually compute).
//!
//! One deliberate, documented narrowing: this crate's own x positions
//! (band-scale centers, always strictly increasing for distinct data
//! points) can never produce d3-shape's degenerate "two points share an
//! x" case, so `slope3` reports a flat `0.0` secant for a zero-width
//! segment instead of reproducing d3's signed-zero/`Infinity` handling for
//! that case -- see that function's own doc.

/// `sign(x)`: `-1.0` for negative, `1.0` otherwise (including both
/// `+0.0` and `-0.0`) -- deliberately not [`f64::signum`], which returns
/// `-1.0` for `-0.0` where d3-shape's own `sign` (this file's citation)
/// returns `1.0`. That difference is exactly what makes
/// `sign(s0) + sign(s1)` reliably zero out the tangent at a local extremum
/// (opposite-signed secants) without also misfiring on an exactly-flat
/// secant on one side.
fn sign(x: f64) -> f64 {
    if x < 0.0 {
        -1.0
    } else {
        1.0
    }
}

/// The interior tangent at the middle point of three, `(x0,y0)`,
/// `(x1,y1)`, `(x2,y2)` -- Steffen's three-way-min formula (module doc).
///
/// `h0`/`h1` are the two segments' x-spans; a zero span (two points
/// sharing an x) reports a flat `0.0` secant for that segment rather than
/// d3-shape's `Infinity`-via-signed-zero handling -- see the module doc
/// for why that case cannot arise from this crate's own callers.
fn slope3(x0: f64, y0: f64, x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    let h0 = x1 - x0;
    let h1 = x2 - x1;
    let s0 = if h0 == 0.0 { 0.0 } else { (y1 - y0) / h0 };
    let s1 = if h1 == 0.0 { 0.0 } else { (y2 - y1) / h1 };
    let p = if h0 + h1 == 0.0 {
        0.0
    } else {
        (s0 * h1 + s1 * h0) / (h0 + h1)
    };
    let result = (sign(s0) + sign(s1)) * s0.abs().min(s1.abs()).min(0.5 * p.abs());
    if result == 0.0 {
        0.0 // normalizes a possible `-0.0`, matching d3-shape's `|| 0`
    } else {
        result
    }
}

/// The one-sided tangent at an end point: given the secant slope of its
/// own (only) segment, `(x0,y0)`-`(x1,y1)`, and the already-computed
/// tangent `t` at that segment's *other* end, returns the tangent at
/// `(x0,y0)` (or, symmetrically, at `(x1,y1)` when called with that
/// segment's endpoints reversed) -- d3-shape's `slope2` (module doc).
fn slope2(x0: f64, y0: f64, x1: f64, y1: f64, t: f64) -> f64 {
    let h = x1 - x0;
    if h == 0.0 {
        t
    } else {
        (3.0 * (y1 - y0) / h - t) / 2.0
    }
}

/// Per-point tangents for a monotone cubic Hermite spline through `points`
/// (Steffen 1990, via `slope3`/`slope2` -- see the module doc). Returns
/// one tangent per point (`len() == points.len()`).
fn monotone_tangents(points: &[(f64, f64)]) -> Vec<f64> {
    let n = points.len();
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![0.0];
    }
    if n == 2 {
        // Two points define a straight line: both tangents equal to the
        // one secant, which the Bezier control-point construction in
        // `monotone_path` reproduces exactly as that same line (there is
        // no interior point to apply the three-point `slope3` formula to).
        // d3-shape special-cases this to a plain `lineTo` rather than a
        // Bezier -- a differently-serialized but geometrically identical
        // curve.
        let (x0, y0) = points[0];
        let (x1, y1) = points[1];
        let secant = if x1 == x0 { 0.0 } else { (y1 - y0) / (x1 - x0) };
        return vec![secant, secant];
    }

    let mut m = vec![0.0; n];
    for i in 1..n - 1 {
        let (x0, y0) = points[i - 1];
        let (x1, y1) = points[i];
        let (x2, y2) = points[i + 1];
        m[i] = slope3(x0, y0, x1, y1, x2, y2);
    }
    let (x0, y0) = points[0];
    let (x1, y1) = points[1];
    m[0] = slope2(x0, y0, x1, y1, m[1]);
    let (x0, y0) = points[n - 2];
    let (x1, y1) = points[n - 1];
    m[n - 1] = slope2(x0, y0, x1, y1, m[n - 2]);
    m
}

/// A curve interpolation for [`line_path`]/[`area_path`] -- the curve
/// types shadcn/ui's charts pass to Recharts' `type` prop, which Recharts
/// maps onto d3-shape's curve factories.
///
/// | variant | Recharts `type` | d3-shape |
/// |---|---|---|
/// | [`Curve::Linear`] | `"linear"` | `curveLinear` |
/// | [`Curve::Natural`] | `"natural"` | `curveNatural` |
/// | [`Curve::Monotone`] | `"monotone"` | `curveMonotoneX` |
/// | [`Curve::Step`] | `"step"` | `curveStep` (midpoint) |
/// | [`Curve::StepBefore`] | `"stepBefore"` | `curveStepBefore` |
/// | [`Curve::StepAfter`] | `"stepAfter"` | `curveStepAfter` |
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Curve {
    /// Straight line segments between consecutive points.
    Linear,
    /// A natural cubic spline (second derivative zero at both ends) through
    /// every point -- the smooth curve most shadcn area/line demos use. Like
    /// any interpolating spline it may overshoot between points.
    Natural,
    /// Monotone cubic Hermite interpolation (Steffen 1990) -- smooth, and
    /// guaranteed to never overshoot past a data point's own value.
    Monotone,
    /// Midpoint step: each value holds until halfway to the next point,
    /// where a vertical riser jumps to the next value (half-width runs at
    /// both ends).
    Step,
    /// Step-before: the riser sits at the *previous* point's x, so each
    /// value is drawn from the previous point up to its own.
    StepBefore,
    /// Step-after: a horizontal line to the next point's x, then a vertical
    /// line to its y.
    StepAfter,
}

/// d3-shape `natural.js`'s per-axis control points for a natural cubic
/// spline through `x` (one coordinate of every point; `x.len() >= 3`):
/// the tridiagonal system of the "second derivative continuous, zero at the
/// ends" conditions, solved with the Thomas algorithm. Returns `(a, b)`: the
/// first and second Bezier control coordinate of each of the `n - 1`
/// segments.
fn natural_control_points(x: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let n = x.len() - 1;
    let mut a = vec![0.0; n];
    let mut b = vec![0.0; n];
    let mut r = vec![0.0; n];
    a[0] = 0.0;
    b[0] = 2.0;
    r[0] = x[0] + 2.0 * x[1];
    for i in 1..n - 1 {
        a[i] = 1.0;
        b[i] = 4.0;
        r[i] = 4.0 * x[i] + 2.0 * x[i + 1];
    }
    a[n - 1] = 2.0;
    b[n - 1] = 7.0;
    r[n - 1] = 8.0 * x[n - 1] + x[n];
    for i in 1..n {
        let m = a[i] / b[i - 1];
        b[i] -= m;
        r[i] -= m * r[i - 1];
    }
    a[n - 1] = r[n - 1] / b[n - 1];
    for i in (0..n - 1).rev() {
        a[i] = (r[i] - a[i + 1]) / b[i];
    }
    b[n - 1] = (x[n] + a[n - 1]) / 2.0;
    for i in 0..n - 1 {
        b[i] = 2.0 * x[i + 1] - a[i + 1];
    }
    (a, b)
}

/// A straight or cubic edge connecting one on-curve anchor point to the
/// next (nearer-to-previous control point first, for `Cubic`).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Edge {
    /// A straight line from the previous anchor to this one.
    Line,
    /// A cubic Bezier from the previous anchor to this one.
    Cubic(f64, f64, f64, f64),
}

/// A curve as an explicit sequence of on-curve anchor points plus the
/// [`Edge`] connecting each consecutive pair (`edges.len() == anchors.len()
/// - 1`) -- the intermediate representation [`line_path`]/[`area_path`]/
/// [`area_between_path`] all build on.
///
/// This exists specifically so [`Self::reversed`] is correct for *every*
/// [`Curve`] variant, which [`area_between_path`]'s curved-baseline bottom
/// edge needs: retracing an already-built curve backward, not recomputing
/// a curve algorithm on reversed input. The latter happens to be
/// numerically correct for `Linear` and `Monotone` (Steffen's tangent
/// formula is symmetric under point-order reversal, confirmed by direct
/// computation while building this module) but is *wrong* for `Step`: its
/// "after" corner is a real anchor point, and naively recomputing "step
/// after" on reversed input silently produces a "step before" shape
/// instead of the same curve traced backward. Representing the corner as
/// its own anchor (connected by two `Line` edges) sidesteps that
/// distinction entirely -- reversing a sequence of anchors and edges is
/// unconditionally correct, regardless of what the edges mean visually.
struct Path {
    anchors: Vec<(f64, f64)>,
    edges: Vec<Edge>,
}

impl Path {
    /// Build a [`Path`] tracing `points` (at least 2) with `curve`'s
    /// interpolation.
    fn build(points: &[(f64, f64)], curve: Curve) -> Self {
        match curve {
            Curve::Linear => Path {
                anchors: points.to_vec(),
                edges: vec![Edge::Line; points.len().saturating_sub(1)],
            },
            Curve::Step | Curve::StepBefore | Curve::StepAfter => {
                // d3's `curveStep` family: the riser sits at `t` of the way
                // from one point to the next (0.5 midpoint, 0 before, 1
                // after). Every corner is its own anchor joined by straight
                // edges, so reversing the path retraces the same shape.
                let t = match curve {
                    Curve::StepBefore => 0.0,
                    Curve::StepAfter => 1.0,
                    _ => 0.5,
                };
                let mut anchors = vec![points[0]];
                for w in points.windows(2) {
                    let (x0, y0) = w[0];
                    let (x1, y1) = w[1];
                    if t <= 0.0 {
                        // Riser first, at the previous x, then the run.
                        anchors.push((x0, y1));
                        anchors.push((x1, y1));
                    } else {
                        let xm = x0 * (1.0 - t) + x1 * t;
                        anchors.push((xm, y0));
                        anchors.push((xm, y1));
                    }
                }
                // The midpoint step ends with a half-width run to the last
                // point (d3's `lineEnd`); the other two already end on it.
                if t > 0.0 && t < 1.0 {
                    anchors.push(*points.last().expect("at least 2 points"));
                }
                anchors.dedup();
                let edges = vec![Edge::Line; anchors.len().saturating_sub(1)];
                Path { anchors, edges }
            }
            Curve::Natural if points.len() > 2 => {
                let xs: Vec<f64> = points.iter().map(|p| p.0).collect();
                let ys: Vec<f64> = points.iter().map(|p| p.1).collect();
                let (ax, bx) = natural_control_points(&xs);
                let (ay, by) = natural_control_points(&ys);
                let edges = (0..points.len() - 1)
                    .map(|i| Edge::Cubic(ax[i], ay[i], bx[i], by[i]))
                    .collect();
                Path {
                    anchors: points.to_vec(),
                    edges,
                }
            }
            // d3 draws a two-point natural curve as a straight line.
            Curve::Natural => Path {
                anchors: points.to_vec(),
                edges: vec![Edge::Line; points.len().saturating_sub(1)],
            },
            Curve::Monotone => {
                let tangents = monotone_tangents(points);
                let mut edges = Vec::with_capacity(points.len() - 1);
                for i in 0..points.len() - 1 {
                    let (x0, y0) = points[i];
                    let (x1, y1) = points[i + 1];
                    let dx = (x1 - x0) / 3.0;
                    edges.push(Edge::Cubic(
                        x0 + dx,
                        y0 + dx * tangents[i],
                        x1 - dx,
                        y1 - dx * tangents[i + 1],
                    ));
                }
                Path {
                    anchors: points.to_vec(),
                    edges,
                }
            }
        }
    }

    /// The same curve, traced from its last anchor back to its first.
    fn reversed(&self) -> Self {
        let anchors: Vec<_> = self.anchors.iter().rev().copied().collect();
        let edges: Vec<_> = self
            .edges
            .iter()
            .rev()
            .map(|e| match e {
                Edge::Line => Edge::Line,
                Edge::Cubic(x1, y1, x2, y2) => Edge::Cubic(*x2, *y2, *x1, *y1),
            })
            .collect();
        Path { anchors, edges }
    }

    fn to_svg(&self) -> String {
        let fmt = super::scale::fmt_num;
        let Some((x0, y0)) = self.anchors.first() else {
            return String::new();
        };
        let mut out = format!("M{} {}", fmt(*x0), fmt(*y0));
        for (i, edge) in self.edges.iter().enumerate() {
            let (x, y) = self.anchors[i + 1];
            match edge {
                Edge::Line => out.push_str(&format!(" L{} {}", fmt(x), fmt(y))),
                Edge::Cubic(x1, y1, x2, y2) => out.push_str(&format!(
                    " C{} {} {} {} {} {}",
                    fmt(*x1),
                    fmt(*y1),
                    fmt(*x2),
                    fmt(*y2),
                    fmt(x),
                    fmt(y)
                )),
            }
        }
        out
    }

    fn first_x(&self) -> f64 {
        self.anchors.first().map_or(0.0, |(x, _)| *x)
    }

    fn last_x(&self) -> f64 {
        self.anchors.last().map_or(0.0, |(x, _)| *x)
    }
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
        return format!("M{} {}", super::scale::fmt_num(x), super::scale::fmt_num(y));
    }
    Path::build(points, curve).to_svg()
}

/// Build a closed SVG path `d` string for an area: the same top edge as
/// [`line_path`], then straight down to `baseline_y` under the last point,
/// straight back to `baseline_y` under the first point, and closed (`Z`).
/// For a *stacked* area (whose bottom edge is itself a curve -- the
/// previous series' cumulative top, not a flat line), use
/// [`area_between_path`] instead.
pub fn area_path(points: &[(f64, f64)], baseline_y: f64, curve: Curve) -> String {
    let fmt = super::scale::fmt_num;
    if points.is_empty() {
        return String::new();
    }
    if points.len() == 1 {
        let (x, y) = points[0];
        return format!(
            "M{x} {y} L{x} {b} Z",
            x = fmt(x),
            y = fmt(y),
            b = fmt(baseline_y)
        );
    }
    let path = Path::build(points, curve);
    let top = path.to_svg();
    format!(
        "{top} L{lx} {b} L{fx} {b} Z",
        lx = fmt(path.last_x()),
        b = fmt(baseline_y),
        fx = fmt(path.first_x())
    )
}

/// Build a closed SVG path `d` string for a *stacked* area: `top` (the
/// series' own values) drawn forward with `curve`'s interpolation, then
/// `bottom` (the previous series' cumulative top -- [`super::stack::stack`]'s
/// per-row `y0`) retraced backward with the same interpolation, closed.
/// `top` and `bottom` should share x positions pairwise (as
/// [`super::stack::stack`]'s output, read at the same x positions, does);
/// a length mismatch is handled defensively by using the shorter of the
/// two rather than panicking.
///
/// ```
/// use dioxus_primitives::chart::{area_between_path, Curve};
///
/// let top = [(0.0, 10.0), (10.0, 20.0)];
/// let bottom = [(0.0, 5.0), (10.0, 8.0)];
/// assert_eq!(
///     area_between_path(&top, &bottom, Curve::Linear),
///     "M0 10 L10 20 L10 8 L0 5 Z"
/// );
/// ```
pub fn area_between_path(top: &[(f64, f64)], bottom: &[(f64, f64)], curve: Curve) -> String {
    let fmt = super::scale::fmt_num;
    let n = top.len().min(bottom.len());
    if n == 0 {
        return String::new();
    }
    if n == 1 {
        let (x, y1) = top[0];
        let (_, y0) = bottom[0];
        return format!(
            "M{x} {y1} L{x} {y0} Z",
            x = fmt(x),
            y1 = fmt(y1),
            y0 = fmt(y0)
        );
    }
    let top_svg = Path::build(&top[..n], curve).to_svg();
    let bottom_svg = Path::build(&bottom[..n], curve).reversed().to_svg();
    // `bottom_svg` starts with its own "M x y" -- continue the same path
    // instead of starting a new subpath.
    let bottom_svg = bottom_svg
        .strip_prefix('M')
        .map(|rest| format!("L{rest}"))
        .unwrap_or(bottom_svg);
    format!("{top_svg} {bottom_svg} Z")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_line_path_is_straight_segments() {
        let path = line_path(&[(0.0, 0.0), (10.0, 10.0), (20.0, 0.0)], Curve::Linear);
        assert_eq!(path, "M0 0 L10 10 L20 0");
    }

    #[test]
    fn step_after_line_path_steps_at_the_next_point() {
        let path = line_path(&[(0.0, 0.0), (10.0, 10.0), (20.0, 0.0)], Curve::StepAfter);
        assert_eq!(path, "M0 0 L10 0 L10 10 L20 10 L20 0");
    }

    #[test]
    fn monotone_line_path_reduces_to_a_straight_line_when_collinear() {
        // Collinear points (constant slope) is the base case every
        // monotone-cubic implementation must reproduce exactly: every
        // tangent equals the shared slope, so the Bezier control points
        // land exactly back on the line.
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
    fn monotone_tangents_use_the_steffen_three_way_min_at_the_ends() {
        // Pinned regression for the Steffen end-condition (`slope2`) fix:
        // unequal x-spacing (h0=1, h1=2) makes the end tangent genuinely
        // different from "just the outer secant", which a naive
        // Fritsch-Carlson port (`m[0] = secant[0]`) would get wrong.
        // Hand-derived (see this lane's commit message for the arithmetic)
        // and cross-checked against `d3-shape`'s own `monotone.js` (module
        // doc) fed the same three points.
        let tangents = monotone_tangents(&[(0.0, 0.0), (1.0, 1.0), (3.0, 2.0)]);
        assert_eq!(tangents.len(), 3);
        assert!(
            (tangents[0] - 1.0833333333333333).abs() < 1e-12,
            "got {:?}",
            tangents[0]
        );
        assert!(
            (tangents[1] - 0.8333333333333333).abs() < 1e-12,
            "got {:?}",
            tangents[1]
        );
        assert!(
            (tangents[2] - 0.3333333333333333).abs() < 1e-12,
            "got {:?}",
            tangents[2]
        );
    }

    #[test]
    fn monotone_path_matches_the_pinned_steffen_derivation() {
        let path = line_path(&[(0.0, 0.0), (1.0, 1.0), (3.0, 2.0)], Curve::Monotone);
        assert_eq!(
            path,
            "M0 0 C0.333 0.361 0.667 0.722 1 1 C1.667 1.556 2.333 1.778 3 2"
        );
    }

    #[test]
    fn monotone_path_single_point_is_a_bare_move() {
        assert_eq!(line_path(&[(5.0, 5.0)], Curve::Monotone), "M5 5");
        assert_eq!(line_path(&[], Curve::Monotone), "");
    }

    #[test]
    fn monotone_two_points_is_equivalent_to_a_straight_line() {
        // d3-shape special-cases n==2 to a plain `lineTo`; this crate
        // emits a Bezier whose control points sit exactly on the line
        // instead (see `monotone_tangents`' doc) -- geometrically
        // identical, so the numbers on the line must match a plain
        // `Curve::Linear` path between the same two points.
        let monotone = line_path(&[(0.0, 0.0), (10.0, 4.0)], Curve::Monotone);
        assert_eq!(monotone, "M0 0 C3.333 1.333 6.667 2.667 10 4");
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

    // -- area_between_path / Path::reversed ------------------------------------------------------

    #[test]
    fn area_between_path_linear_closes_top_forward_bottom_backward() {
        let top = [(0.0, 10.0), (10.0, 20.0)];
        let bottom = [(0.0, 5.0), (10.0, 8.0)];
        assert_eq!(
            area_between_path(&top, &bottom, Curve::Linear),
            "M0 10 L10 20 L10 8 L0 5 Z"
        );
    }

    #[test]
    fn area_between_path_step_retraces_the_after_corner_correctly() {
        // The case `Path`'s anchor-based reversal exists for: naively
        // recomputing "step after" on reversed input would silently
        // produce a "step before" shape instead of this same curve traced
        // backward.
        let top = [(0.0, 10.0), (10.0, 20.0)];
        let bottom = [(0.0, 5.0), (10.0, 8.0)];
        // Top (step-after, forward): M0 10 L10 10 L10 20.
        // Bottom (step-after, forward) would be M0 5 L10 5 L10 8; traced
        // BACKWARD that's M10 8 L10 5 L0 5 -- the corner (10, 5) stays a
        // corner, it does not move to (0, 8).
        assert_eq!(
            area_between_path(&top, &bottom, Curve::StepAfter),
            "M0 10 L10 10 L10 20 L10 8 L10 5 L0 5 Z"
        );
    }

    #[test]
    fn area_between_path_monotone_matches_reversed_tangents() {
        // Cross-check against the pinned Steffen derivation
        // (`monotone_path_matches_the_pinned_steffen_derivation`): the
        // bottom edge here is exactly that same 3-point curve, so its
        // retraced-backward form must be that same path's segments in
        // reverse, control points swapped -- not a fresh (and, per this
        // module's doc, provably identical, but worth pinning) tangent
        // computation on the reversed points.
        let top = [(0.0, 10.0), (1.0, 10.0), (3.0, 10.0)]; // flat -- isolates the bottom edge
        let bottom = [(0.0, 0.0), (1.0, 1.0), (3.0, 2.0)];
        let path = area_between_path(&top, &bottom, Curve::Monotone);
        // Flat top: every tangent is 0, so its Bezier control points sit
        // at the same y as the anchors (still "C" commands, not "L" --
        // Monotone always emits cubics, even for a visually straight run).
        assert!(path.starts_with("M0 10 C0.333 10 0.667 10 1 10 C1.667 10 2.333 10 3 10 "));
        assert!(path.ends_with(" Z"));
        // The bottom edge, forward, is
        // "M0 0 C0.333 0.361 0.667 0.722 1 1 C1.667 1.556 2.333 1.778 3 2"
        // (the pinned derivation) -- traced backward that's the same two
        // Beziers, segment order AND each one's own control points
        // reversed, ending back at (0, 0).
        assert!(path.contains("L3 2 C2.333 1.778 1.667 1.556 1 1 C0.667 0.722 0.333 0.361 0 0"));
    }

    #[test]
    fn area_between_path_single_point_is_a_closed_sliver() {
        assert_eq!(
            area_between_path(&[(5.0, 10.0)], &[(5.0, 2.0)], Curve::Linear),
            "M5 10 L5 2 Z"
        );
    }

    #[test]
    fn area_between_path_empty_is_empty() {
        assert_eq!(area_between_path(&[], &[], Curve::Linear), "");
    }

    #[test]
    fn area_between_path_mismatched_lengths_uses_the_shorter() {
        let top = [(0.0, 10.0), (10.0, 20.0), (20.0, 30.0)];
        let bottom = [(0.0, 5.0), (10.0, 8.0)];
        assert_eq!(
            area_between_path(&top, &bottom, Curve::Linear),
            "M0 10 L10 20 L10 8 L0 5 Z"
        );
    }

    #[test]
    fn sign_treats_negative_zero_as_non_negative() {
        // The one behavior this crate's `sign` deliberately does NOT match
        // `f64::signum` on -- see this function's own doc.
        assert_eq!(sign(-0.0), 1.0);
        assert_eq!(sign(0.0), 1.0);
        assert_eq!(sign(-1.0), -1.0);
        assert_eq!(sign(1.0), 1.0);
    }

    // -- shadcn parity: the rendered `d` of shadcn's own demos (Recharts 3,
    // probed on ui.shadcn.com; area/line default data on a 369x208 chart,
    // plot x 12..357, y 0..178, y domain 0..320) ----------------------------

    /// The six default-data points exactly where shadcn's chart puts them.
    fn shadcn_default_points() -> Vec<(f64, f64)> {
        [186.0, 305.0, 237.0, 73.0, 209.0, 214.0]
            .iter()
            .enumerate()
            .map(|(i, v)| (12.0 + 69.0 * i as f64, 178.0 - v / 320.0 * 178.0))
            .collect()
    }

    /// Every number in an SVG path string, in order.
    fn numbers(d: &str) -> Vec<f64> {
        d.split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
            .filter(|t| !t.is_empty())
            .map(|t| t.parse().unwrap())
            .collect()
    }

    fn assert_close(ours: &str, theirs: &str) {
        let (a, b) = (numbers(ours), numbers(theirs));
        assert_eq!(a.len(), b.len(), "\n ours {ours}\n shadcn {theirs}");
        for (x, y) in a.iter().zip(&b) {
            assert!(
                (x - y).abs() < 0.002,
                "{x} vs {y}\n ours {ours}\n shadcn {theirs}"
            );
        }
    }

    #[test]
    fn natural_matches_shadcn_chart_line_default() {
        let d = line_path(&shadcn_default_points(), Curve::Natural);
        assert_close(
            &d,
            "M12,74.538C35,45.641,58,16.744,81,8.344C104,-0.057,127,12.039,150,46.169\
             C173,80.299,196,136.463,219,137.394C242,138.324,265,84.021,288,61.744\
             C311,39.466,334,49.214,357,58.963",
        );
    }

    #[test]
    fn step_is_the_midpoint_step_of_shadcn_chart_line_step() {
        let d = line_path(&shadcn_default_points(), Curve::Step);
        assert_close(
            &d,
            "M12,74.538L46.5,74.538L46.5,8.344L115.5,8.344L115.5,46.169L184.5,46.169\
             L184.5,137.394L253.5,137.394L253.5,61.744L322.5,61.744L322.5,58.963L357,58.963",
        );
    }

    #[test]
    fn linear_matches_shadcn_chart_line_linear() {
        let d = line_path(&shadcn_default_points(), Curve::Linear);
        assert_close(
            &d,
            "M12,74.538L81,8.344L150,46.169L219,137.394L288,61.744L357,58.963",
        );
    }

    #[test]
    fn monotone_matches_shadcn_chart_line_multiple() {
        // line-multiple's desktop series (Recharts `type="monotone"`).
        let d = line_path(&shadcn_default_points(), Curve::Monotone);
        let theirs = "M12,74.538C35,41.441,58,8.344,81,8.344";
        assert!(
            numbers(&d)[..8]
                .iter()
                .zip(numbers(theirs))
                .all(|(a, b)| (a - b).abs() < 0.002),
            "{d}"
        );
    }

    #[test]
    fn step_before_puts_the_riser_at_the_previous_point() {
        let path = line_path(&[(0.0, 0.0), (10.0, 10.0), (20.0, 0.0)], Curve::StepBefore);
        assert_eq!(path, "M0 0 L0 10 L10 10 L10 0 L20 0");
    }

    #[test]
    fn natural_with_two_points_is_a_straight_line() {
        assert_eq!(
            line_path(&[(0.0, 0.0), (10.0, 4.0)], Curve::Natural),
            "M0 0 L10 4"
        );
    }

    #[test]
    fn natural_area_between_retraces_the_bottom_curve() {
        // A natural spline through reversed points is the same curve, so the
        // retraced bottom edge must equal the forward path backwards.
        let bottom = shadcn_default_points();
        let fwd = line_path(&bottom, Curve::Natural);
        let rev: Vec<(f64, f64)> = bottom.iter().rev().copied().collect();
        let back = line_path(&rev, Curve::Natural);
        let (a, b) = (
            numbers(&Path::build(&bottom, Curve::Natural).reversed().to_svg()),
            numbers(&back),
        );
        assert_eq!(a.len(), b.len(), "{fwd}");
        assert!(a.iter().zip(&b).all(|(x, y)| (x - y).abs() < 1e-6));
    }
}
