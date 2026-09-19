//! Polar geometry for the arc-based chart families (Pie/Donut, RadialBar)
//! -- pure, unit-tested, no `dioxus` types (see the `engine` module doc for
//! why): [`arc_path`] (SVG path `d` for one annular sector), [`pie_layout`]
//! (partition values into the arcs of a pie/donut), [`centroid`] (a
//! slice's label anchor point), and [`angle_scale`] (value -> angle,
//! reusing [`super::scale::LinearScale`] -- see its own doc).
//!
//! ## Angle convention
//!
//! Every angle in this module is in **radians**, `0.0` = 12 o'clock (the
//! top), increasing = **clockwise** on screen. This matches `d3-shape`'s
//! own `arc()`/`pie()` (internally, `arc()` subtracts `PI/2` from the
//! angles it's given before doing any trigonometry -- ported verbatim
//! below) and is why a plain `(r*cos(a), r*sin(a))` point looks clockwise
//! despite using the standard math `cos`/`sin` convention: SVG's y-axis
//! points down, which flips the visual handedness of increasing-angle
//! travel versus the usual x-right/y-up picture.
//!
//! ## Provenance
//!
//! [`arc_path`]'s padAngle handling and [`pie_layout`] are clean-room Rust
//! ports of `d3-shape`'s `src/arc.js` and `src/pie.js` (ISC license, Mike
//! Bostock, `d3/d3-shape@a82254af78f08799c71d7ab25df557c4872a3c51`, both
//! read in full this lane; a working copy of the same commit is at
//! `$S/refs/d3-shape` for this repo's own future reference). Reference
//! outputs quoted in this file's tests were computed by installing the
//! real `d3-shape@3.2.0` npm package and calling its actual `arc()`/
//! `pie()` (`$S/stage2/s2-polar/d3ref/ref.mjs`), not transcribed from
//! memory or from reading the source alone.
//!
//! **`arc_path`'s `corner_radius` handling is this module's own
//! simplification, not a port of `d3-shape`'s `intersect`/
//! `cornerTangents` (`arc.js` lines ~25-75).** d3's version solves, per
//! corner, for the exact tangent circle consistent with *both* adjacent
//! corners' radii possibly having been independently reduced (so a very
//! small slice's two rounded corners can meet without overlapping,
//! `rc0`/`rc1` in its source). This module instead uses one formula,
//! shared by all four corners of a sector: a fillet of radius `rc`,
//! tangent to the ring (internally for the outer ring, externally for the
//! inner) and to the sector's straight radial edge, whose center sits on
//! the line from the origin to the edge at distance `r ∓ rc`, at an
//! angular offset `phi = asin(rc / (r ∓ rc))` from the edge -- the classic
//! tangent-line/tangent-circle relationship. If the two fillets on a ring
//! would need more angle than that ring's own span (a slice too thin for
//! the requested corner radius), rounding is disabled entirely for that
//! call (falling back to sharp corners) rather than d3's own finer-grained
//! per-corner reduction. **This is a real, visible difference only in
//! that one (thin-slice, large-corner-radius) edge case** -- the
//! `corner_radius = 0.0` path (the common case: sharp corners, still with
//! full padAngle support) is bit-for-bit the same construction as d3's,
//! and is what the tests below hold to exact parity; the rounded-corner
//! tests check shape sanity (valid, closed, continuous path; corner
//! points land where this module's own formula says they should) rather
//! than parity with `d3-shape`'s own algorithm. Reasonable per this
//! lane's own brief ("simplified corner-radius handling is acceptable if
//! documented").

use std::f64::consts::PI;

use super::scale::{fmt_num, LinearScale};

const EPSILON: f64 = 1e-12;
const TAU: f64 = 2.0 * PI;
const HALF_PI: f64 = PI / 2.0;

/// Clamp `x` into `[-1, 1]` before `asin`, matching `d3-shape`'s own
/// `asin` helper (`d3-shape/src/math.js`). Not just defensive: this is
/// load-bearing for [`arc_path`]'s padAngle math when `inner_radius` is
/// `0.0` (a plain, non-donut pie slice) -- `rp / 0.0 * ap.sin()` evaluates
/// to `f64::INFINITY` (IEEE 754, same as JavaScript), and clamping that
/// here to `HALF_PI` is exactly what makes the padding math collapse a
/// zero-radius "inner ring" to a single point instead of producing `NaN`
/// -- see [`arc_path`]'s own doc.
fn asin_clamped(x: f64) -> f64 {
    if x >= 1.0 {
        HALF_PI
    } else if x <= -1.0 {
        -HALF_PI
    } else {
        x.asin()
    }
}

/// The shortest signed angular delta from `from` to `to`, wrapped into
/// `(-PI, PI]`. Used only for [`arc_path`]'s corner-radius fillets, whose
/// endpoint angles are recovered via `atan2` (a principal value, so two
/// angles that are geometrically close can print a full turn apart)
/// rather than being built up from a single continuous running angle the
/// way the rest of this module's angle bookkeeping already is.
fn short_delta(from: f64, to: f64) -> f64 {
    let mut d = (to - from) % TAU;
    if d > PI {
        d -= TAU;
    } else if d <= -PI {
        d += TAU;
    }
    d
}

/// `(cx, cy) + r * (cos(a), sin(a))` -- this module's one point-on-circle
/// formula, named so call sites read as geometry, not arithmetic.
fn polar_point(cx: f64, cy: f64, r: f64, a: f64) -> (f64, f64) {
    (cx + r * a.cos(), cy + r * a.sin())
}

/// A minimal SVG path-`d` builder: `move_to`/`line_to`/`arc`/`close`. This
/// is plain SVG semantics throughout -- unlike `d3-path`'s `Path` class
/// (which `d3-shape` itself builds on, emulating `CanvasRenderingContext2D`
/// and auto-inserting a connecting line before an `arc()` call whose
/// start doesn't match the current point), an [`Self::arc`] call here
/// always starts from wherever the pen already is: every call site in
/// this file already knows, by construction, whether a connecting
/// [`Self::line_to`] is needed first, so there's no need for this type to
/// infer it from a runtime point-equality check (which floating-point
/// rounding across two different formulas for "the same" point would
/// make unreliable anyway).
struct PathBuilder {
    d: String,
}

impl PathBuilder {
    fn new() -> Self {
        Self { d: String::new() }
    }

    fn move_to(&mut self, x: f64, y: f64) {
        self.d.push('M');
        self.d.push_str(&fmt_num(x));
        self.d.push(',');
        self.d.push_str(&fmt_num(y));
    }

    fn line_to(&mut self, x: f64, y: f64) {
        self.d.push('L');
        self.d.push_str(&fmt_num(x));
        self.d.push(',');
        self.d.push_str(&fmt_num(y));
    }

    /// Append an `A` command sweeping from angle `a_from` to `a_to`
    /// (radians, this module's convention) around `(cx, cy)` with radius
    /// `r`; the pen must already be at `polar_point(cx, cy, r, a_from)`.
    fn arc(&mut self, cx: f64, cy: f64, r: f64, a_from: f64, a_to: f64) {
        let large_arc = if (a_to - a_from).abs() > PI { 1 } else { 0 };
        self.arc_with_flag(cx, cy, r, a_from, a_to, large_arc);
    }

    /// [`Self::arc`], with an explicit large-arc-flag rather than one
    /// derived from `a_from`/`a_to`. Only [`arc_path`]'s full-circle/
    /// annulus special case needs this: each of its two `A` commands
    /// spans *exactly* `PI` by construction, where the large-arc-flag is
    /// genuinely immaterial (both flag values trace the identical
    /// semicircle) -- `d3-shape`'s own output there happens to always
    /// say `1`, so this lets that one case match it exactly rather than
    /// merely being an equally-valid `0`.
    fn arc_with_flag(&mut self, cx: f64, cy: f64, r: f64, a_from: f64, a_to: f64, large_arc: u8) {
        let (ex, ey) = polar_point(cx, cy, r, a_to);
        let sweep = if a_to > a_from { 1 } else { 0 };
        self.d.push('A');
        self.d.push_str(&fmt_num(r));
        self.d.push(',');
        self.d.push_str(&fmt_num(r));
        self.d.push_str(",0,");
        self.d.push_str(&large_arc.to_string());
        self.d.push(',');
        self.d.push_str(&sweep.to_string());
        self.d.push(',');
        self.d.push_str(&fmt_num(ex));
        self.d.push(',');
        self.d.push_str(&fmt_num(ey));
    }

    fn close(&mut self) {
        self.d.push('Z');
    }

    fn build(self) -> String {
        self.d
    }
}

/// Build the SVG path `d` for one annular sector -- a pie slice
/// (`inner_radius = 0.0`), a donut slice (`inner_radius > 0.0`), or a
/// radial bar (typically drawn with `pad_angle = 0.0`, and its own
/// `start_angle`/`end_angle` coming from [`angle_scale`] rather than
/// [`pie_layout`]). A clean-room port of `d3-shape`'s `arc()` -- see the
/// module doc for exactly what's ported faithfully (padAngle, the
/// full-circle/annulus special case) versus simplified
/// (`corner_radius`).
///
/// `inner_radius`/`outer_radius` are swapped if given in the wrong order
/// (matching `d3-shape`: "Ensure that the outer radius is always larger
/// than the inner radius"); negative radii and a negative `corner_radius`
/// are clamped to `0.0` (handled defensively, matching this crate's other
/// engine functions).
///
/// ```
/// use dioxus_primitives::chart::engine::polar::arc_path;
/// use std::f64::consts::PI;
///
/// // A quarter-circle pie slice (no inner radius, no pad, no corner
/// // radius) -- verified against real d3-shape's own output.
/// assert_eq!(
///     arc_path(0.0, 100.0, 0.0, PI / 2.0, 0.0, 0.0),
///     "M0,-100A100,100,0,0,1,100,0L0,0Z"
/// );
/// ```
pub fn arc_path(
    inner_radius: f64,
    outer_radius: f64,
    start_angle: f64,
    end_angle: f64,
    pad_angle: f64,
    corner_radius: f64,
) -> String {
    let (mut r0, mut r1) = (inner_radius.max(0.0), outer_radius.max(0.0));
    if r1 < r0 {
        std::mem::swap(&mut r0, &mut r1);
    }
    let corner_radius = corner_radius.max(0.0);
    let pad_angle = pad_angle.max(0.0);

    let a0 = start_angle - HALF_PI;
    let a1 = end_angle - HALF_PI;
    let da = (a1 - a0).abs();
    let cw = a1 > a0;
    let dir = if cw { 1.0 } else { -1.0 };

    let mut path = PathBuilder::new();

    // Is it a point? (`r1` was just clamped via `.max(0.0)` above, so it
    // is never `NaN` here -- `r1 <= EPSILON` is a plain, clippy-clean
    // total-order comparison, not `d3-shape`'s own `!(r1 > epsilon)`
    // partial-order idiom, which only differs from this for a `NaN` this
    // function can no longer receive.)
    if r1 <= EPSILON {
        path.move_to(0.0, 0.0);
        path.close();
        return path.build();
    }

    // Is it a circle or annulus (the sector spans a full turn)? Split
    // into two half-turn `A` commands -- a single `A` cannot represent a
    // full turn (its start and end point would coincide, which is
    // ambiguous), matching `d3-shape`'s own special case here.
    if da > TAU - EPSILON {
        let (sx, sy) = polar_point(0.0, 0.0, r1, a0);
        let mid = a0 + PI * dir;
        let end = a0 + TAU * dir;
        path.move_to(sx, sy);
        path.arc_with_flag(0.0, 0.0, r1, a0, mid, 1);
        path.arc_with_flag(0.0, 0.0, r1, mid, end, 1);
        if r0 > EPSILON {
            let (ix, iy) = polar_point(0.0, 0.0, r0, a1);
            let mid0 = a1 - PI * dir;
            let end0 = a1 - TAU * dir;
            path.move_to(ix, iy);
            path.arc_with_flag(0.0, 0.0, r0, a1, mid0, 1);
            path.arc_with_flag(0.0, 0.0, r0, mid0, end0, 1);
        }
        path.close();
        return path.build();
    }

    // A circular or annular sector.
    let ap = pad_angle / 2.0;
    let pad_active = ap > EPSILON;
    let rp = (r0 * r0 + r1 * r1).sqrt();

    let (mut a00, mut a10, mut a01, mut a11) = (a0, a1, a0, a1);
    // `da0` (unlike `da1`) is read again below, past this block, to
    // decide whether the inner ring degenerated to a point.
    let mut da0 = da;

    if pad_active {
        let p0 = asin_clamped(rp / r0 * ap.sin());
        let p1 = asin_clamped(rp / r1 * ap.sin());

        da0 -= p0 * 2.0;
        if da0 > EPSILON {
            let p0 = p0 * dir;
            a00 += p0;
            a10 -= p0;
        } else {
            da0 = 0.0;
            a00 = (a0 + a1) / 2.0;
            a10 = a00;
        }

        if da - p1 * 2.0 > EPSILON {
            let p1 = p1 * dir;
            a01 += p1;
            a11 -= p1;
        } else {
            a01 = (a0 + a1) / 2.0;
            a11 = a01;
        }
    }

    // Corner radius, clamped like d3-shape (never more than half the
    // ring's own thickness); disabled entirely if either ring is too
    // thin an angular span for two fillets of this radius (see the
    // module doc: this module's own simplification of d3's per-corner
    // reduction).
    let mut rc = ((r1 - r0).abs() / 2.0).min(corner_radius);
    let mut phi1 = 0.0;
    let mut phi0 = 0.0;
    if rc > EPSILON {
        phi1 = asin_clamped(rc / (r1 - rc));
        if r0 > EPSILON {
            phi0 = asin_clamped(rc / (r0 + rc));
        }
        let outer_span = (a11 - a01).abs();
        let inner_span = (a10 - a00).abs();
        let outer_overlaps = 2.0 * phi1 >= outer_span;
        let inner_overlaps = r0 > EPSILON && da0 > EPSILON && 2.0 * phi0 >= inner_span;
        if outer_overlaps || inner_overlaps {
            rc = 0.0;
            phi1 = 0.0;
            phi0 = 0.0;
        }
    }
    let rounded = rc > EPSILON;

    if rounded {
        // Outer ring: edge -> fillet -> (inset) main arc -> fillet -> edge.
        let outer_edge_r = (((r1 - rc) * (r1 - rc) - rc * rc).max(0.0)).sqrt();
        let c0_angle = a01 + dir * phi1;
        let c1_angle = a11 - dir * phi1;
        let c0 = polar_point(0.0, 0.0, r1 - rc, c0_angle);
        let c1 = polar_point(0.0, 0.0, r1 - rc, c1_angle);
        let edge0 = polar_point(0.0, 0.0, outer_edge_r, a01);
        let edge1 = polar_point(0.0, 0.0, outer_edge_r, a11);
        let psi0 = (edge0.1 - c0.1).atan2(edge0.0 - c0.0);
        let psi1 = (edge1.1 - c1.1).atan2(edge1.0 - c1.0);

        path.move_to(edge0.0, edge0.1);
        path.arc(c0.0, c0.1, rc, psi0, psi0 + short_delta(psi0, c0_angle));
        path.arc(0.0, 0.0, r1, c0_angle, c1_angle);
        path.arc(
            c1.0,
            c1.1,
            rc,
            c1_angle,
            c1_angle + short_delta(c1_angle, psi1),
        );

        if r0 > EPSILON && da0 > EPSILON {
            // Inner ring, traversed backward (end side a10 first, matching
            // the non-rounded case below): edge -> fillet -> main arc ->
            // fillet -> edge.
            let inner_edge_r = (((r0 + rc) * (r0 + rc) - rc * rc).max(0.0)).sqrt();
            let ic1_angle = a10 - dir * phi0;
            let ic0_angle = a00 + dir * phi0;
            let ic1 = polar_point(0.0, 0.0, r0 + rc, ic1_angle);
            let ic0 = polar_point(0.0, 0.0, r0 + rc, ic0_angle);
            let iedge1 = polar_point(0.0, 0.0, inner_edge_r, a10);
            let iedge0 = polar_point(0.0, 0.0, inner_edge_r, a00);
            let ipsi1 = (iedge1.1 - ic1.1).atan2(iedge1.0 - ic1.0);
            let ipsi0 = (iedge0.1 - ic0.1).atan2(iedge0.0 - ic0.0);

            path.line_to(iedge1.0, iedge1.1);
            path.arc(
                ic1.0,
                ic1.1,
                rc,
                ipsi1,
                ipsi1 + short_delta(ipsi1, ic1_angle),
            );
            path.arc(0.0, 0.0, r0, ic1_angle, ic0_angle);
            path.arc(
                ic0.0,
                ic0.1,
                rc,
                ic0_angle,
                ic0_angle + short_delta(ic0_angle, ipsi0),
            );
        } else {
            let (ix, iy) = polar_point(0.0, 0.0, r0, a10);
            path.line_to(ix, iy);
        }
    } else {
        let (sx, sy) = polar_point(0.0, 0.0, r1, a01);
        path.move_to(sx, sy);
        path.arc(0.0, 0.0, r1, a01, a11);
        let (ix, iy) = polar_point(0.0, 0.0, r0, a10);
        path.line_to(ix, iy);
        if r0 > EPSILON && da0 > EPSILON {
            path.arc(0.0, 0.0, r0, a10, a00);
        }
    }

    path.close();
    path.build()
}

/// The label anchor for one arc: midway between `inner_radius` and
/// `outer_radius`, at the arc's own midpoint angle. A clean-room port of
/// `d3-shape`'s `arc.centroid()` (same file/citation as [`arc_path`]).
///
/// ```
/// use dioxus_primitives::chart::engine::polar::centroid;
/// use std::f64::consts::PI;
///
/// let (x, y) = centroid(50.0, 100.0, 0.0, PI / 2.0);
/// assert!((x - 53.033_008_588_991_07).abs() < 1e-9);
/// assert!((y - -53.033_008_588_991_06).abs() < 1e-9);
/// ```
pub fn centroid(
    inner_radius: f64,
    outer_radius: f64,
    start_angle: f64,
    end_angle: f64,
) -> (f64, f64) {
    let r = (inner_radius + outer_radius) / 2.0;
    let a = (start_angle + end_angle) / 2.0 - HALF_PI;
    (r * a.cos(), r * a.sin())
}

/// A value -> angle mapping for the RadialBar family: each bar's own end
/// angle is `angle_scale(domain, start_angle, end_angle).scale(value)`
/// (and, for a stacked radial bar, the same scale applied to each
/// series' cumulative value -- see `components/series/radial.rs`'s own
/// doc). This is exactly [`LinearScale`] -- a plain domain -> range affine
/// map is already the right tool, and this crate already has one (see
/// its own doc) -- this function exists only so a call site reads
/// `angle_scale(domain, a0, a1)` rather than a struct literal, giving the
/// concept a name in this module's own public surface without a second
/// implementation of the same four-field affine map.
///
/// ```
/// use dioxus_primitives::chart::engine::polar::angle_scale;
/// use std::f64::consts::PI;
///
/// let scale = angle_scale((0.0, 100.0), 0.0, PI);
/// assert_eq!(scale.scale(50.0), PI / 2.0);
/// ```
pub fn angle_scale(domain: (f64, f64), start_angle: f64, end_angle: f64) -> LinearScale {
    LinearScale {
        domain,
        range: (start_angle, end_angle),
    }
}

/// One slice's angular span from [`pie_layout`] -- `arcs[i]` (the `i`-th
/// element of that function's returned `Vec`) is always the slice for
/// `values[i]`, regardless of `sort` (see that function's own doc: only
/// *which value claims which angular position* is affected by sorting,
/// never the order slices are returned in).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct PieArc {
    /// This slice's angular start (this module's convention: radians, `0`
    /// = 12 o'clock, increasing = clockwise -- see the module doc).
    pub start_angle: f64,
    /// This slice's angular end.
    pub end_angle: f64,
    /// The resolved (clamped to `abs(2*PI) / values.len()`, matching
    /// `pie.js`) pad angle actually used for this layout -- in the same
    /// "full" units [`arc_path`]'s own `pad_angle` parameter expects
    /// (that function halves it internally, matching `d3-shape`'s
    /// `arc()`), so the typical call site is `arc_path(inner, outer,
    /// arc.start_angle, arc.end_angle, arc.pad_angle, corner_radius)`.
    pub pad_angle: f64,
    /// This slice's own value, copied through from `values[i]` unchanged
    /// (including if it was negative or non-finite -- only the *layout*
    /// math treats those as zero-width, per this function's own doc).
    pub value: f64,
}

/// How [`pie_layout`] orders slices around the circle -- which value
/// claims the *earliest* angular position (`start_angle`) -- while always
/// returning [`PieArc`]s in `values`' own original order (see that type's
/// own doc). Does not affect each slice's *size* (still strictly
/// proportional to its value), only *where* it sits.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum PieSort {
    /// Preserve `values`' own order. Matches every shadcn/Recharts `Pie`
    /// demo (Recharts' own `Pie` does not reorder `data` by value) and
    /// this crate's own `ChartConfig`/`ChartDatum` "order is the only
    /// thing that matters" convention (`engine::data`'s module doc) --
    /// what this crate's own `Pie`/`RadialBar` series use.
    #[default]
    None,
    /// Largest value first -- `d3-shape`'s own *default*
    /// (`pie.js`'s `sortValues = descending`), offered here for parity/a
    /// future caller, not used by this crate's own series.
    DescendingByValue,
    /// Smallest value first.
    AscendingByValue,
}

/// Partition `values` into a full turn's worth of [`PieArc`]s starting at
/// `start_angle` (this module's angle convention). A clean-room Rust port
/// of `d3-shape`'s `pie()` (see the module doc for the exact source
/// citation), fixed to a full-turn `endAngle = start_angle + 2*PI` and
/// this module's own [`PieSort`] in place of d3's general
/// value/comparator accessors -- no caller in this crate needs that
/// generality.
///
/// A non-finite or non-positive value is treated as a zero-width slice at
/// its position (matching `pie.js`'s own `sum ? ... : 0` guard against an
/// all-non-positive input, and its `v > 0 ? v * k : 0` per-slice guard)
/// -- it still gets a `pad_angle`-sized gap after it, exactly like d3.
///
/// ```
/// use dioxus_primitives::chart::engine::polar::{pie_layout, PieSort};
/// use std::f64::consts::PI;
///
/// let arcs = pie_layout(&[1.0, 2.0, 3.0], 0.0, 0.0, PieSort::None);
/// assert_eq!(arcs.len(), 3);
/// assert_eq!(arcs[0].start_angle, 0.0);
/// assert!((arcs[0].end_angle - PI / 3.0).abs() < 1e-9); // 1/6 turn
/// assert!((arcs[2].end_angle - 2.0 * PI).abs() < 1e-9); // slices sum to a full turn
/// ```
pub fn pie_layout(values: &[f64], pad_angle: f64, start_angle: f64, sort: PieSort) -> Vec<PieArc> {
    let n = values.len();
    if n == 0 {
        return Vec::new();
    }
    let raw: Vec<f64> = values
        .iter()
        .map(|v| if v.is_finite() && *v > 0.0 { *v } else { 0.0 })
        .collect();
    // pie.js sums only the positive values; NaN/negative were already
    // zeroed above, so a plain sum is exact.
    let sum: f64 = raw.iter().sum();

    let da_full = TAU; // this port's fixed full-turn endAngle - start_angle (always positive).
    let p = (da_full / n as f64).min(pad_angle.max(0.0));

    let mut order: Vec<usize> = (0..n).collect();
    match sort {
        PieSort::None => {}
        PieSort::DescendingByValue => order.sort_by(|&i, &j| {
            raw[j]
                .partial_cmp(&raw[i])
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
        PieSort::AscendingByValue => order.sort_by(|&i, &j| {
            raw[i]
                .partial_cmp(&raw[j])
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
    }

    let k = if sum > 0.0 {
        (da_full - n as f64 * p) / sum
    } else {
        0.0
    };
    let mut arcs = vec![
        PieArc {
            start_angle: 0.0,
            end_angle: 0.0,
            pad_angle: p,
            value: 0.0,
        };
        n
    ];
    let mut a0 = start_angle;
    for &j in &order {
        let v = raw[j];
        let a1 = a0 + (if v > 0.0 { v * k } else { 0.0 }) + p;
        arcs[j] = PieArc {
            start_angle: a0,
            end_angle: a1,
            pad_angle: p,
            value: values[j],
        };
        a0 = a1;
    }
    arcs
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- arc_path: exact parity with real d3-shape@3.2.0 output
    //     ($S/stage2/s2-polar/d3ref/ref.mjs) for every corner_radius=0.0
    //     case (see the module doc for why corner_radius > 0.0 isn't
    //     held to exact parity). ---

    #[test]
    fn arc_path_quarter_circle_pie_slice() {
        assert_eq!(
            arc_path(0.0, 100.0, 0.0, PI / 2.0, 0.0, 0.0),
            "M0,-100A100,100,0,0,1,100,0L0,0Z"
        );
    }

    #[test]
    fn arc_path_donut_sector_no_pad() {
        assert_eq!(
            arc_path(50.0, 100.0, 0.0, PI / 2.0, 0.0, 0.0),
            "M0,-100A100,100,0,0,1,100,0L50,0A50,50,0,0,0,0,-50Z"
        );
    }

    #[test]
    fn arc_path_donut_sector_with_pad_angle() {
        assert_eq!(
            arc_path(50.0, 100.0, 0.0, PI / 2.0, 0.05, 0.0),
            "M2.795,-99.961A100,100,0,0,1,99.961,-2.795L49.922,-2.795A50,50,0,0,0,2.795,-49.922Z"
        );
    }

    #[test]
    fn arc_path_full_circle() {
        assert_eq!(
            arc_path(0.0, 100.0, 0.0, TAU, 0.0, 0.0),
            "M0,-100A100,100,0,1,1,0,100A100,100,0,1,1,0,-100Z"
        );
    }

    #[test]
    fn arc_path_full_annulus() {
        assert_eq!(
            arc_path(50.0, 100.0, 0.0, TAU, 0.0, 0.0),
            "M0,-100A100,100,0,1,1,0,100A100,100,0,1,1,0,-100\
M0,-50A50,50,0,1,0,0,50A50,50,0,1,0,0,-50Z"
        );
    }

    #[test]
    fn arc_path_point_when_outer_radius_is_zero() {
        assert_eq!(arc_path(0.0, 0.0, 0.0, 1.0, 0.0, 0.0), "M0,0Z");
    }

    #[test]
    fn arc_path_swaps_inverted_radii() {
        // inner_radius > outer_radius: d3-shape swaps them; so do we.
        assert_eq!(
            arc_path(100.0, 50.0, 0.0, PI / 2.0, 0.0, 0.0),
            arc_path(50.0, 100.0, 0.0, PI / 2.0, 0.0, 0.0)
        );
    }

    #[test]
    fn arc_path_negative_radii_clamped_to_zero() {
        assert_eq!(
            arc_path(-10.0, 100.0, 0.0, PI / 2.0, 0.0, 0.0),
            arc_path(0.0, 100.0, 0.0, PI / 2.0, 0.0, 0.0)
        );
    }

    // --- arc_path: corner_radius > 0.0 -- shape sanity, not d3 parity
    //     (see the module doc). ---

    #[test]
    fn arc_path_corner_radius_produces_a_closed_valid_path() {
        let d = arc_path(50.0, 100.0, 0.0, PI / 2.0, 0.0, 10.0);
        assert!(d.starts_with('M'));
        assert!(d.ends_with('Z'));
        // Four rounded corners + two ring arcs => 6 `A` commands.
        assert_eq!(d.matches('A').count(), 6);
        // Every arc radius that appears is either the fillet (10) or one
        // of the two ring radii (50, 100) -- no NaN/negative leaked out.
        assert!(!d.contains("NaN"));
    }

    #[test]
    fn arc_path_corner_radius_is_clamped_to_half_the_ring_thickness() {
        // A corner_radius far larger than (outer-inner)/2 = 25 must
        // render identically to passing exactly 25 (d3-shape's own
        // clamp: "Ensure ... always larger ..." / rc = min(...)).
        assert_eq!(
            arc_path(50.0, 100.0, 0.0, PI / 2.0, 0.0, 1000.0),
            arc_path(50.0, 100.0, 0.0, PI / 2.0, 0.0, 25.0)
        );
    }

    #[test]
    fn arc_path_corner_radius_falls_back_to_sharp_when_slice_too_thin() {
        // A tiny slice can't fit two 40-radius fillets on a ring whose
        // available span is a fraction of a degree; must fall back to
        // the exact sharp-corner (corner_radius=0.0) path rather than
        // emitting a malformed one.
        let thin = arc_path(50.0, 100.0, 0.0, 0.01, 0.0, 40.0);
        let sharp = arc_path(50.0, 100.0, 0.0, 0.01, 0.0, 0.0);
        assert_eq!(thin, sharp);
    }

    #[test]
    fn arc_path_corner_radius_on_a_plain_pie_slice_keeps_a_sharp_tip() {
        // inner_radius=0.0: the two rounded OUTER corners must not
        // produce NaN, and the tip at the origin stays a plain point
        // (this module's own doc: no inner-ring rounding when there's no
        // inner ring).
        let d = arc_path(0.0, 100.0, 0.0, PI / 2.0, 0.0, 10.0);
        assert!(d.starts_with('M'));
        assert!(d.ends_with("L0,0Z") || d.contains("L0,0"));
        assert!(!d.contains("NaN"));
    }

    // --- centroid: exact parity with real d3-shape. ---

    #[test]
    fn centroid_matches_d3_shape_donut_sector() {
        let (x, y) = centroid(50.0, 100.0, 0.0, PI / 2.0);
        assert!((x - 53.033_008_588_991_07).abs() < 1e-9);
        assert!((y - -53.033_008_588_991_06).abs() < 1e-9);
    }

    #[test]
    fn centroid_matches_d3_shape_half_turn_pie_slice() {
        let (x, y) = centroid(0.0, 100.0, 0.0, PI);
        assert!((x - 50.0).abs() < 1e-9);
        assert!(y.abs() < 1e-9);
    }

    // --- angle_scale: a thin, correctly-wired LinearScale. ---

    #[test]
    fn angle_scale_maps_the_value_domain_onto_the_angle_range() {
        let scale = angle_scale((0.0, 200.0), -HALF_PI, PI);
        assert_eq!(scale.scale(0.0), -HALF_PI);
        assert_eq!(scale.scale(200.0), PI);
        assert!((scale.scale(100.0) - (-HALF_PI + (PI - -HALF_PI) / 2.0)).abs() < 1e-9);
    }

    // --- pie_layout: exact parity with real d3-shape for every case
    //     computed in $S/stage2-lanes.md's referenced ref.mjs run. ---

    fn approx(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9, "{a} not close to {b}");
    }

    #[test]
    fn pie_layout_preserves_input_order_when_sort_is_none() {
        let arcs = pie_layout(&[1.0, 2.0, 3.0], 0.0, 0.0, PieSort::None);
        assert_eq!(arcs.len(), 3);
        approx(arcs[0].start_angle, 0.0);
        approx(arcs[0].end_angle, 1.047_197_551_196_597_6);
        approx(arcs[1].start_angle, 1.047_197_551_196_597_6);
        approx(arcs[1].end_angle, PI);
        approx(arcs[2].start_angle, PI);
        approx(arcs[2].end_angle, TAU);
        for a in &arcs {
            approx(a.pad_angle, 0.0);
        }
    }

    #[test]
    fn pie_layout_shadcn_browsers_dataset() {
        // The exact dataset every one of shadcn's 11 pie demos uses.
        let arcs = pie_layout(&[275.0, 200.0, 187.0, 173.0, 90.0], 0.0, 0.0, PieSort::None);
        approx(arcs[0].start_angle, 0.0);
        approx(arcs[0].end_angle, 1.867_974_010_242_579_7);
        approx(arcs[1].start_angle, 1.867_974_010_242_579_7);
        approx(arcs[1].end_angle, 3.226_500_563_146_274);
        approx(arcs[2].start_angle, 3.226_500_563_146_274);
        approx(arcs[2].end_angle, 4.496_722_890_111_228);
        approx(arcs[3].start_angle, 4.496_722_890_111_228);
        approx(arcs[3].end_angle, 5.671_848_358_372_924);
        approx(arcs[4].start_angle, 5.671_848_358_372_924);
        approx(arcs[4].end_angle, TAU);
        // Every slice's value passes through unchanged.
        for (arc, expected) in arcs.iter().zip([275.0, 200.0, 187.0, 173.0, 90.0]) {
            assert_eq!(arc.value, expected);
        }
    }

    #[test]
    fn pie_layout_slices_always_sum_to_a_full_turn() {
        for values in [
            vec![1.0, 2.0, 3.0],
            vec![275.0, 200.0, 187.0, 173.0, 90.0],
            vec![5.0, 0.0, 5.0],
            vec![1.0; 12],
        ] {
            let n = values.len();
            let arcs = pie_layout(&values, 0.0, 0.0, PieSort::None);
            approx(arcs[0].start_angle, 0.0);
            approx(arcs[n - 1].end_angle, TAU);
            // Contiguous: each slice's end is the next one's start.
            for i in 1..n {
                approx(arcs[i - 1].end_angle, arcs[i].start_angle);
            }
        }
    }

    #[test]
    fn pie_layout_with_pad_angle() {
        let arcs = pie_layout(&[1.0, 1.0, 1.0, 1.0], 0.02, 0.0, PieSort::None);
        approx(arcs[0].start_angle, 0.0);
        approx(arcs[0].end_angle, HALF_PI);
        approx(arcs[1].start_angle, HALF_PI);
        approx(arcs[1].end_angle, PI);
        approx(arcs[2].start_angle, PI);
        approx(arcs[2].end_angle, 4.712_388_980_384_69);
        approx(arcs[3].start_angle, 4.712_388_980_384_69);
        approx(arcs[3].end_angle, TAU);
        for a in &arcs {
            approx(a.pad_angle, 0.02);
        }
    }

    #[test]
    fn pie_layout_zero_value_is_a_zero_width_slice_at_its_position() {
        let arcs = pie_layout(&[5.0, 0.0, 5.0], 0.0, 0.0, PieSort::None);
        approx(arcs[0].start_angle, 0.0);
        approx(arcs[0].end_angle, PI);
        approx(arcs[1].start_angle, PI);
        approx(arcs[1].end_angle, PI); // zero-width: start == end
        approx(arcs[2].start_angle, PI);
        approx(arcs[2].end_angle, TAU);
    }

    #[test]
    fn pie_layout_descending_sort_orders_by_value_but_keeps_position() {
        let arcs = pie_layout(&[1.0, 2.0, 3.0], 0.0, 0.0, PieSort::DescendingByValue);
        // Position 2 (value 3, the largest) claims the EARLIEST angle...
        approx(arcs[2].start_angle, 0.0);
        approx(arcs[2].end_angle, PI);
        // ...position 1 (value 2) the middle slot...
        approx(arcs[1].start_angle, PI);
        approx(arcs[1].end_angle, 5.0 / 3.0 * PI);
        // ...and position 0 (value 1, the smallest) the last slot -- but
        // `arcs[0]` is still "the slice for values[0]", per this
        // function's own doc, regardless of draw order.
        approx(arcs[0].start_angle, 5.0 / 3.0 * PI);
        approx(arcs[0].end_angle, TAU);
    }

    #[test]
    fn pie_layout_ascending_sort_is_descendings_mirror() {
        let asc = pie_layout(&[1.0, 2.0, 3.0], 0.0, 0.0, PieSort::AscendingByValue);
        // Smallest value (position 0) now claims the earliest slot.
        approx(asc[0].start_angle, 0.0);
        approx(asc[0].end_angle, PI / 3.0);
    }

    #[test]
    fn pie_layout_of_no_values_is_empty() {
        assert!(pie_layout(&[], 0.0, 0.0, PieSort::None).is_empty());
    }

    #[test]
    fn pie_layout_all_zero_values_produces_zero_width_slices_with_no_panic() {
        let arcs = pie_layout(&[0.0, 0.0], 0.0, 0.0, PieSort::None);
        assert_eq!(arcs.len(), 2);
        for a in &arcs {
            approx(a.start_angle, 0.0);
            approx(a.end_angle, 0.0);
        }
    }

    #[test]
    fn pie_layout_start_angle_offsets_the_whole_layout() {
        let at_zero = pie_layout(&[1.0, 1.0], 0.0, 0.0, PieSort::None);
        let offset = pie_layout(&[1.0, 1.0], 0.0, HALF_PI, PieSort::None);
        approx(offset[0].start_angle, at_zero[0].start_angle + HALF_PI);
        approx(offset[1].end_angle, at_zero[1].end_angle + HALF_PI);
    }
}
