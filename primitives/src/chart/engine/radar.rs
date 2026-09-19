//! Pure geometry for a radar (spider/polar) chart: category angles, a
//! radial magnitude scale, closed polygon paths for each series and for
//! the grid rings, and angular hit-sector paths for hover -- pure,
//! unit-tested, no `dioxus` types (see the `engine` module doc for why).
//!
//! # Angle convention
//!
//! Every angle in this module is in radians, `0.0` at 12 o'clock (the
//! first category sits at the top), increasing **clockwise** -- matching
//! both this module's own cited sources:
//!
//! - d3-shape's `pointRadial` (`src/pointRadial.js`, ISC,
//!   `d3/d3-shape@a82254af78` (2023-10-06), read in full):
//!   `(y * cos(x - PI/2), y * sin(x - PI/2))`. Expanding the angle
//!   subtraction (`cos(a - PI/2) = sin(a)`, `sin(a - PI/2) = -cos(a)`)
//!   gives `(y * sin(x), -y * cos(x))` -- exactly [`point_radial`]'s
//!   formula below, independently re-derived from that trig identity, not
//!   transcribed.
//! - Recharts' own `RadarChart` (`recharts/recharts` commit
//!   `86ad3632ff3f83a742001ae6fc079dd27961a2a4`, MIT,
//!   `src/chart/RadarChart.tsx`'s `defaultProps: { startAngle: 90,
//!   endAngle: -270 }`, combined with `src/util/PolarUtils.ts`'s
//!   `polarToCartesian`): substituting `angle_recharts = 90 -
//!   degrees(this_module's_angle)` into that function's `x = cx +
//!   cos(-RADIAN * angle) * r, y = cy + sin(-RADIAN * angle) * r` and
//!   simplifying algebraically lands on the exact same `(r * sin(a), -r *
//!   cos(a))` offset -- confirming this is not merely d3's abstract
//!   convention but shadcn/Recharts' own actual radar orientation (first
//!   category at top, clockwise), the thing this port needs to match for
//!   visual parity with the demos in `$S/refs/ui/.../charts/chart-radar-*.tsx`.
//!
//! # Scope: `Curve::Linear` only, by construction of the chart family being ported
//!
//! `$S/chart-api.md`'s [`super::curve::Curve`] enum (`Linear`/`Monotone`/
//! `Step`) is a Cartesian *open-line* concept: d3-shape's `curveRadial`
//! (`src/curve/radial.js`) generalizes it to a closed radial curve by
//! wrapping any of its Cartesian curve generators, and `lineRadial`/
//! `areaRadial` (`src/lineRadial.js`/`src/areaRadial.js`, both read in
//! full) are thin wrappers applying that same generalization to `line`/
//! `area`. This module ports only the straight-line (`curveLinearClosed`-
//! equivalent) case, **not** a general closed-curve interpolator for
//! every [`super::curve::Curve`] variant -- a periodic monotone spline
//! closing back on itself is a materially different (and harder) problem
//! than an open one (the tangent at the seam must be consistent with both
//! neighbors, not one-sided), and is out of scope for this MVP.
//!
//! This narrowing costs nothing against the actual port target: Recharts'
//! own `<Radar>` component (`src/polar/Radar.tsx`, read in full) has **no
//! curve/interpolation prop at all** -- it always renders through
//! `Polygon` (`src/shape/Polygon.tsx`), a straight-sided closed shape, and
//! not one of the fourteen shadcn radar demos in
//! `$S/refs/ui/.../charts/chart-radar-*.tsx` passes anything resembling a
//! smoothing option. `Curve::Linear` is not a narrowing of the port
//! target's own behavior; it is the whole of it.
//!
//! # Modules this file composes, not duplicates
//!
//! The radial *magnitude* scale is [`super::scale::LinearScale`] itself
//! (see [`radial_scale`]'s doc) -- no new scale type, mirroring
//! [`super::scale::BandScale`]'s own "nesting composes directly" precedent
//! for grouped bars. `super::scale::fmt_num` formats every coordinate
//! this module emits, same as [`super::curve`]'s path builders.

use super::scale::{fmt_num, LinearScale};

/// The `count` evenly-spaced category angles for a radar chart with
/// `count` axes, in this module's own convention (radians, `0.0` at 12
/// o'clock, increasing clockwise -- see the module doc). Mirrors how
/// Recharts' angle axis divides a categorical domain: `count` points
/// spaced `360 / count` degrees apart spanning the full circle, the first
/// one at the start angle (here, always the top) -- confirmed against
/// `RadarChart`'s own `startAngle: 90, endAngle: -270` defaults (a full
/// 360° span), see the module doc's citation.
///
/// `count == 0` returns an empty vec (never panics, matching this crate's
/// other scale constructors' "degenerate input, no crash" convention --
/// e.g. [`super::scale::BandScale::band`]'s `count == 0` case).
///
/// ```
/// use dioxus_primitives::chart::engine::radar::category_angles;
/// use std::f64::consts::PI;
///
/// let angles = category_angles(4);
/// assert_eq!(angles.len(), 4);
/// assert_eq!(angles[0], 0.0); // top
/// assert!((angles[1] - PI / 2.0).abs() < 1e-12); // right (quarter turn clockwise)
/// assert!((angles[2] - PI).abs() < 1e-12); // bottom
/// ```
pub fn category_angles(count: usize) -> Vec<f64> {
    if count == 0 {
        return Vec::new();
    }
    let step = std::f64::consts::TAU / count as f64;
    (0..count).map(|i| i as f64 * step).collect()
}

/// Map one `(angle, radius)` polar coordinate (this module's own
/// convention -- see the module doc) to a Cartesian `(x, y)` **offset from
/// an implicit origin**. Callers add their own center point (`(cx + x, cy
/// + y)`) -- this function stays center-agnostic, mirroring d3-shape's own
/// `pointRadial` (`src/pointRadial.js`, cited in the module doc), which
/// likewise returns an offset a caller's own translated drawing context
/// positions. A negative `radius` reflects through the origin (never
/// special-cased away -- same "don't clamp, let the caller decide"
/// stance as [`super::scale::LinearScale::scale`]'s own doc).
///
/// ```
/// use dioxus_primitives::chart::engine::radar::point_radial;
///
/// let (x, y) = point_radial(0.0, 10.0);
/// assert_eq!((x, y), (0.0, -10.0)); // straight up
/// ```
pub fn point_radial(angle: f64, radius: f64) -> (f64, f64) {
    (radius * angle.sin(), -radius * angle.cos())
}

/// The radial (magnitude) scale for a radar chart: reuses
/// [`LinearScale`] verbatim, mapping `domain` (the series' value range,
/// typically [`super::scale::nice_domain`]'s output so it always includes
/// `0.0` -- a radar's radius must never float away from zero, same
/// reasoning as a bar/area chart's y-axis) onto `[0, outer_radius]`. No
/// dedicated scale type exists for this: a polar magnitude scale is
/// mathematically just a linear scale whose range happens to be a radius
/// instead of a pixel offset, mirroring [`super::scale::BandScale`]'s own
/// "nesting composes directly, no bespoke helper" precedent (see that
/// type's module doc).
///
/// ```
/// use dioxus_primitives::chart::engine::radar::radial_scale;
///
/// let scale = radial_scale((0.0, 300.0), 100.0);
/// assert_eq!(scale.scale(0.0), 0.0);
/// assert_eq!(scale.scale(300.0), 100.0);
/// assert_eq!(scale.scale(150.0), 50.0);
/// ```
pub fn radial_scale(domain: (f64, f64), outer_radius: f64) -> LinearScale {
    LinearScale {
        domain,
        range: (0.0, outer_radius),
    }
}

/// Build a closed SVG path `d` string through `points` (Cartesian `(x,
/// y)`, e.g. from [`point_radial`] plus a caller's own center offset), in
/// order, connected by straight lines (`Curve::Linear` -- see the module
/// doc for why a radar polygon never generalizes past this) and closed
/// back to the first point.
///
/// A single point renders a zero-length `M` (matching
/// [`super::curve::line_path`]'s own single-point convention); empty
/// input renders nothing. Two points still close (a back-and-forth
/// sliver), matching this crate's "never panic on too little input"
/// stance throughout `engine` (e.g. [`super::curve::area_path`]'s own
/// single-point case).
///
/// ```
/// use dioxus_primitives::chart::engine::radar::radar_polygon_path;
///
/// let square = [(0.0, -10.0), (10.0, 0.0), (0.0, 10.0), (-10.0, 0.0)];
/// assert_eq!(radar_polygon_path(&square), "M0 -10 L10 0 L0 10 L-10 0 Z");
/// ```
pub fn radar_polygon_path(points: &[(f64, f64)]) -> String {
    let Some((x0, y0)) = points.first() else {
        return String::new();
    };
    if points.len() == 1 {
        return format!("M{} {}", fmt_num(*x0), fmt_num(*y0));
    }
    let mut out = format!("M{} {}", fmt_num(*x0), fmt_num(*y0));
    for (x, y) in &points[1..] {
        out.push_str(&format!(" L{} {}", fmt_num(*x), fmt_num(*y)));
    }
    out.push_str(" Z");
    out
}

/// Build one series' closed polygon path directly from its per-category
/// `values` (aligned to [`category_angles`]' output, `None` = no data for
/// that category) and a [`radial_scale`]-shaped [`LinearScale`] mapping
/// value to radius.
///
/// **Missing-value handling, stated plainly (no shadcn/Recharts precedent
/// to follow -- their demos never have a `None` in a radar series):** a
/// `None` category is dropped from the polygon entirely (its vertex is
/// skipped, connecting its two neighbors directly), not drawn as a
/// zero-radius vertex -- collapsing a vertex to the center would silently
/// invent a data point ("this category is exactly zero") that isn't in
/// the data, which is a worse misrepresentation than a slightly
/// short-cut edge. This mirrors [`super::geometry::plot_runs`]'s own
/// "never fabricate a value" stance, adapted for a single closed ring
/// instead of possibly-multiple open runs (a radar chart has no
/// established "broken ring" visual the way a Cartesian line has a gap,
/// so this crate does not invent one). A row with fewer than one defined
/// value renders nothing.
///
/// ```
/// use dioxus_primitives::chart::engine::radar::{radar_series_path, radial_scale};
///
/// let scale = radial_scale((0.0, 100.0), 10.0);
/// let values = [Some(100.0), Some(100.0), None, Some(100.0)];
/// // The `None` third vertex is skipped -- a triangle, not a 4-gon with a
/// // fabricated zero-radius corner.
/// assert_eq!(
///     radar_series_path(&values, &scale),
///     "M0 -10 L10 0 L-10 0 Z"
/// );
/// ```
pub fn radar_series_path(values: &[Option<f64>], scale: &LinearScale) -> String {
    let n = values.len();
    let angles = category_angles(n);
    let points: Vec<(f64, f64)> = values
        .iter()
        .zip(angles.iter())
        .filter_map(|(v, angle)| v.map(|v| point_radial(*angle, scale.scale(v))))
        .collect();
    radar_polygon_path(&points)
}

/// Build a closed grid-ring polygon at a single `radius`, through the same
/// `angles` every series' vertices use (so the grid's corners always line
/// up with the categories, matching `PolarGrid`'s default `gridType="polygon"`
/// behavior). A thin composition of [`point_radial`] + [`radar_polygon_path`],
/// kept as its own function (rather than inlined at every call site) so it
/// has its own name and its own test.
///
/// ```
/// use dioxus_primitives::chart::engine::radar::{category_angles, grid_polygon_path};
///
/// let angles = category_angles(4);
/// assert_eq!(grid_polygon_path(&angles, 10.0), "M0 -10 L10 0 L0 10 L-10 0 Z");
/// ```
pub fn grid_polygon_path(angles: &[f64], radius: f64) -> String {
    let points: Vec<(f64, f64)> = angles.iter().map(|&a| point_radial(a, radius)).collect();
    radar_polygon_path(&points)
}

/// Build an SVG path `d` string for one angular hit-sector (a pie-slice-
/// shaped wedge from the center to `outer_radius`, spanning
/// `[start_angle, end_angle]` in this module's own clockwise convention)
/// -- the shape a pointer anywhere "in category `i`'s wedge" hits, with no
/// coordinate math needed at hover time: the wedge's own SVG boundary does
/// the work natively, the same "the hit target's own shape carries the
/// geometry" principle `$S/chart-api.md`'s Cartesian hit-bands use (a
/// `rect`'s bounds there; an `A`-command wedge here, since a radar's
/// per-category hover region is angular, not a vertical band).
///
/// Requires `end_angle >= start_angle` and `end_angle - start_angle <=
/// PI` (true for any real radar with 2 or more categories -- the widest
/// possible per-category sector, `n == 2`, spans exactly `PI`, i.e. a
/// half circle) -- the large-arc-flag is always `0` under that
/// constraint, so this function does not need to compute it dynamically.
/// A wider span is defensively clamped to `PI` (drawing a half-circle
/// wedge rather than emitting a self-overlapping arc) rather than
/// panicking; a radar chart with fewer than 2 categories is a degenerate
/// input no caller should construct, not a case worth a fully general
/// (2-arc) full-circle formula.
///
/// ```
/// use dioxus_primitives::chart::engine::radar::sector_path;
/// use std::f64::consts::PI;
///
/// // A quarter-circle wedge from straight up to straight right.
/// let d = sector_path(-PI / 4.0, PI / 4.0, 10.0);
/// assert!(d.starts_with("M0 0 L"));
/// assert!(d.contains("A10 10 0 0 1"));
/// assert!(d.ends_with(" Z"));
/// ```
pub fn sector_path(start_angle: f64, end_angle: f64, outer_radius: f64) -> String {
    let span = (end_angle - start_angle).clamp(0.0, std::f64::consts::PI);
    let end_angle = start_angle + span;
    let (x0, y0) = point_radial(start_angle, outer_radius);
    let (x1, y1) = point_radial(end_angle, outer_radius);
    format!(
        "M0 0 L{} {} A{} {} 0 0 1 {} {} Z",
        fmt_num(x0),
        fmt_num(y0),
        fmt_num(outer_radius),
        fmt_num(outer_radius),
        fmt_num(x1),
        fmt_num(y1),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-9,
            "{actual} not close to {expected}"
        );
    }

    // -- category_angles -------------------------------------------------

    #[test]
    fn category_angles_of_zero_is_empty() {
        assert!(category_angles(0).is_empty());
    }

    #[test]
    fn category_angles_of_one_is_a_single_top_angle() {
        assert_eq!(category_angles(1), vec![0.0]);
    }

    #[test]
    fn category_angles_evenly_divide_the_circle_clockwise_from_the_top() {
        let angles = category_angles(4);
        assert_eq!(angles.len(), 4);
        assert_eq!(angles[0], 0.0);
        assert_close(angles[1], PI / 2.0);
        assert_close(angles[2], PI);
        assert_close(angles[3], 3.0 * PI / 2.0);
    }

    #[test]
    fn category_angles_of_six_matches_shadcns_own_month_count() {
        // The shadcn radar demos all use 6 months -- pin the step this
        // crate's gallery variants will actually render at.
        let angles = category_angles(6);
        assert_eq!(angles.len(), 6);
        for (i, angle) in angles.iter().enumerate() {
            assert_close(*angle, i as f64 * PI / 3.0);
        }
    }

    // -- point_radial ------------------------------------------------------

    #[test]
    fn point_radial_top_is_straight_up() {
        assert_eq!(point_radial(0.0, 10.0), (0.0, -10.0));
    }

    #[test]
    fn point_radial_quarter_turns_land_on_the_axes() {
        let (x, y) = point_radial(PI / 2.0, 10.0);
        assert_close(x, 10.0); // right
        assert_close(y, 0.0);

        let (x, y) = point_radial(PI, 10.0);
        assert_close(x, 0.0); // bottom
        assert_close(y, 10.0);

        let (x, y) = point_radial(3.0 * PI / 2.0, 10.0);
        assert_close(x, -10.0); // left
        assert_close(y, 0.0);
    }

    #[test]
    fn point_radial_zero_radius_is_the_origin() {
        assert_eq!(point_radial(1.2345, 0.0), (0.0, -0.0));
    }

    // -- radial_scale ------------------------------------------------------

    #[test]
    fn radial_scale_maps_the_value_domain_onto_0_to_outer_radius() {
        let scale = radial_scale((0.0, 300.0), 100.0);
        assert_eq!(scale.range, (0.0, 100.0));
        assert_eq!(scale.scale(0.0), 0.0);
        assert_eq!(scale.scale(300.0), 100.0);
        assert_eq!(scale.scale(150.0), 50.0);
    }

    // -- radar_polygon_path --------------------------------------------------

    #[test]
    fn radar_polygon_path_closes_a_square() {
        let square = [(0.0, -10.0), (10.0, 0.0), (0.0, 10.0), (-10.0, 0.0)];
        assert_eq!(radar_polygon_path(&square), "M0 -10 L10 0 L0 10 L-10 0 Z");
    }

    #[test]
    fn radar_polygon_path_single_point_is_a_bare_move() {
        assert_eq!(radar_polygon_path(&[(5.0, 5.0)]), "M5 5");
    }

    #[test]
    fn radar_polygon_path_empty_is_empty() {
        assert_eq!(radar_polygon_path(&[]), "");
    }

    #[test]
    fn radar_polygon_path_two_points_closes_a_sliver() {
        assert_eq!(
            radar_polygon_path(&[(0.0, 0.0), (10.0, 0.0)]),
            "M0 0 L10 0 Z"
        );
    }

    // -- radar_series_path --------------------------------------------------

    #[test]
    fn radar_series_path_builds_a_polygon_at_each_categorys_angle() {
        let scale = radial_scale((0.0, 100.0), 10.0);
        let values = [Some(100.0), Some(100.0), Some(100.0), Some(100.0)];
        // Every value is the max -- a perfect square touching outer_radius
        // at all four category angles.
        assert_eq!(
            radar_series_path(&values, &scale),
            "M0 -10 L10 0 L0 10 L-10 0 Z"
        );
    }

    #[test]
    fn radar_series_path_skips_a_none_category_without_fabricating_a_vertex() {
        let scale = radial_scale((0.0, 100.0), 10.0);
        let values = [Some(100.0), Some(100.0), None, Some(100.0)];
        assert_eq!(
            radar_series_path(&values, &scale),
            "M0 -10 L10 0 L-10 0 Z",
            "the None (bottom) category must be skipped, not drawn as a zero-radius vertex"
        );
    }

    #[test]
    fn radar_series_path_all_none_is_empty() {
        let scale = radial_scale((0.0, 100.0), 10.0);
        assert_eq!(radar_series_path(&[None, None, None], &scale), "");
    }

    #[test]
    fn radar_series_path_of_no_categories_is_empty() {
        let scale = radial_scale((0.0, 100.0), 10.0);
        assert_eq!(radar_series_path(&[], &scale), "");
    }

    // -- grid_polygon_path --------------------------------------------------

    #[test]
    fn grid_polygon_path_matches_series_geometry_at_the_same_radius() {
        let angles = category_angles(4);
        assert_eq!(
            grid_polygon_path(&angles, 10.0),
            "M0 -10 L10 0 L0 10 L-10 0 Z"
        );
    }

    #[test]
    fn grid_polygon_path_of_no_angles_is_empty() {
        assert_eq!(grid_polygon_path(&[], 10.0), "");
    }

    // -- sector_path --------------------------------------------------------

    #[test]
    fn sector_path_starts_and_ends_at_the_center_and_closes() {
        let d = sector_path(-PI / 4.0, PI / 4.0, 10.0);
        assert!(d.starts_with("M0 0 "), "got {d}");
        assert!(d.ends_with(" Z"), "got {d}");
    }

    #[test]
    fn sector_path_spans_the_requested_angle_via_the_arc_endpoints() {
        // A quarter-circle wedge from straight up (-PI/4 + PI/4 = 0, top)
        // is not what's tested here directly; instead pin the two arc
        // endpoints for a wedge centered on the top category, spanning a
        // known angle, against `point_radial` itself -- the two must
        // agree, since `sector_path` is built directly on it.
        let half = PI / 6.0;
        let d = sector_path(-half, half, 10.0);
        let (x0, y0) = point_radial(-half, 10.0);
        let (x1, y1) = point_radial(half, 10.0);
        assert!(
            d.contains(&format!("L{} {}", fmt_num(x0), fmt_num(y0))),
            "got {d}"
        );
        assert!(
            d.contains(&format!("{} {} Z", fmt_num(x1), fmt_num(y1))),
            "got {d}"
        );
    }

    #[test]
    fn sector_path_uses_a_zero_large_arc_flag() {
        // Every real per-category sector (n >= 2) spans at most PI, so the
        // large-arc-flag is always 0 -- pin that it's hardcoded, not
        // computed (and thus can never accidentally flip to 1).
        let d = sector_path(0.0, PI / 2.0, 10.0);
        assert!(d.contains(" 0 0 1 "), "expected a zero large-arc-flag: {d}");
    }

    #[test]
    fn sector_path_clamps_an_overwide_span_instead_of_panicking() {
        // Defensive: a caller passing more than PI (shouldn't happen for
        // any real n >= 2 radar) gets a clamped half-circle wedge, not a
        // panic or a self-overlapping arc.
        let d = sector_path(0.0, 3.0 * PI, 10.0);
        let (x1, y1) = point_radial(PI, 10.0);
        assert!(
            d.contains(&format!("{} {} Z", fmt_num(x1), fmt_num(y1))),
            "got {d}"
        );
    }

    #[test]
    fn sector_path_rejects_a_reversed_span_by_clamping_to_zero_width() {
        let d = sector_path(PI / 2.0, 0.0, 10.0);
        // Clamped to span 0.0 -- both arc endpoints coincide at start_angle.
        let (x, y) = point_radial(PI / 2.0, 10.0);
        assert!(
            d.contains(&format!("L{} {}", fmt_num(x), fmt_num(y))),
            "got {d}"
        );
    }
}
