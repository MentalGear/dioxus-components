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
    /// Monotone cubic Hermite interpolation (Steffen 1990) -- smooth, and
    /// guaranteed to never overshoot past a data point's own value.
    Monotone,
    /// Step-after: a horizontal line to the next point's x, then a
    /// vertical line to its y.
    Step,
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
            Curve::Step => {
                let mut anchors = vec![points[0]];
                let mut edges = Vec::new();
                for w in points.windows(2) {
                    let (_, y0) = w[0];
                    let (x1, y1) = w[1];
                    anchors.push((x1, y0));
                    edges.push(Edge::Line);
                    anchors.push((x1, y1));
                    edges.push(Edge::Line);
                }
                Path { anchors, edges }
            }
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
        let path = line_path(&[(0.0, 0.0), (10.0, 10.0), (20.0, 0.0)], Curve::Step);
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
            area_between_path(&top, &bottom, Curve::Step),
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
}
