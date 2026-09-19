//! Linear and band scales, "nice" tick generation, and numeric formatting
//! -- pure, unit-tested, no `dioxus` types (see the `engine` module doc for
//! why).
//!
//! [`LinearScale::ticks`]/[`LinearScale::nice`]/[`nice_domain`] generate
//! "round" numbers using the classic 1-2-5 step series (every step is
//! `{1, 2, 5, 10} * 10^k`) -- the construction described in Paul Heckbert's
//! "Nice Numbers for Graph Labels" (*Graphics Gems*, 1990). The exact
//! arithmetic below (the `tick_spec` "multiply above one, divide below
//! one" split in particular, which exists purely to get IEEE-754
//! correctly-rounded results for a sub-`1.0` step -- `3.0 * 0.2` and
//! `3.0 / 5.0` are not the same double, and only the latter is the nearest
//! double to the true `0.6`) was cross-checked line-by-line against
//! `d3-array`'s implementation of the same published algorithm
//! (`ticks.js`, ISC license, `d3/d3-array@be0ae0d2ea` -- consulted for
//! verification of this well-known numerical technique, not copied: no
//! line of this file is a transcription of that one). `LinearScale::nice`
//! additionally mirrors `d3-scale`'s `continuous.js` `nice(count)` in
//! iterating `tick_increment` to a fixed point (bounded to a small,
//! generous max-iterations constant) rather than nice-rounding once, which
//! matters when the first pass's rounding would itself deserve a different
//! step size.

/// Threshold constants for [`tick_spec`]'s magnitude-factor selection:
/// the residual `step / 10^floor(log10(step))` always lands in `[1, 10)`,
/// and is rounded to the nearest of `{1, 2, 5, 10}` using these geometric-
/// mean thresholds (`sqrt(2)`, `sqrt(10)`, `sqrt(50)`). Computed via
/// `.sqrt()` (`f64::sqrt` is not yet usable in a `const` context on this
/// crate's MSRV) rather than a rounded decimal literal, which
/// `clippy::approx_constant` correctly flags as looking like a failed
/// attempt at `std::f64::consts::SQRT_2`.
fn tick_factor_thresholds() -> (f64, f64, f64) {
    (2f64.sqrt(), 10f64.sqrt(), 50f64.sqrt())
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

    /// Generate "nice" tick values spanning [`Self::domain`], stepping by
    /// `{1, 2, 5, 10} * 10^k` -- see the module doc. `count` is a target,
    /// not an exact count: d3's own `ticks` has the same property (the
    /// returned length depends on how evenly the domain divides by the
    /// chosen step).
    pub fn ticks(&self, count: usize) -> Vec<f64> {
        generate_ticks(self.domain.0, self.domain.1, count)
    }

    /// Extend [`Self::domain`] outward to a "nice" round boundary on each
    /// end (mirrors d3-scale's `.nice()`, including its fixed-point
    /// iteration -- see the module doc), targeting `DEFAULT_NICE_TICKS`
    /// steps. A zero-width or non-finite domain is
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
        let (mut lo, mut hi) = if ascending { (d0, d1) } else { (d1, d0) };

        let mut prev_step: Option<f64> = None;
        for _ in 0..MAX_NICE_ITERATIONS {
            let step = tick_increment(lo, hi, DEFAULT_NICE_TICKS);
            if prev_step == Some(step) {
                break;
            }
            if step > 0.0 {
                lo = (lo / step).floor() * step;
                hi = (hi / step).ceil() * step;
            } else if step < 0.0 {
                lo = (lo * step).ceil() / step;
                hi = (hi * step).floor() / step;
            } else {
                break;
            }
            prev_step = Some(step);
        }

        let domain = if ascending { (lo, hi) } else { (hi, lo) };
        Self { domain, ..self }
    }
}

/// The tick target [`LinearScale::nice`] and [`nice_domain`] both nice-round
/// against -- matches d3-scale's own default `.nice()` (no explicit count)
/// behavior. A chart's actually-*rendered* tick count (`Chart`'s
/// `y_tick_count` prop, default `5`) is a separate, smaller request against
/// the already-niced domain -- the two can differ, the same way d3's
/// `.nice()` and a later `.ticks(n)` call are independent.
const DEFAULT_NICE_TICKS: usize = 10;

/// Upper bound on [`LinearScale::nice`]'s fixed-point iteration. d3-scale's
/// own loop is bounded the same way (`continuous.js`, `maxIter = 10`);
/// every domain this crate's tests or the chart's own auto-domain
/// (`nice_domain`) can produce converges in 1-2 passes, so this bound is
/// generous headroom against an unforeseen non-terminating case, not a
/// number tuned to any specific input.
const MAX_NICE_ITERATIONS: usize = 10;

/// A band scale: divides `range` into `count` equal-width bands with even
/// padding between and around them, for a categorical axis (one band per
/// data point). Mirrors d3's `scaleBand` with `paddingInner == paddingOuter
/// == padding` (this crate's single `padding` field) and no rounding/align
/// -- this chart draws in SVG viewBox units, not integer pixels, so the
/// sub-pixel alignment d3's rounding solves for doesn't apply here.
///
/// Nesting a second `BandScale` inside one of the outer scale's own bands
/// (`range` set to that band's `(start, start + width)`) is how a grouped
/// (non-stacked) multi-series bar chart positions each series' bar within
/// its shared category band -- no separate "sub-band" helper is needed for
/// that; it composes directly.
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

/// Mirrors d3-array's `tickSpec` (see the module doc for the citation and
/// why the multiply/divide split exists). Returns `(i1, i2, inc)`: the
/// ticks are `i1*inc, (i1+1)*inc, ..., i2*inc` when `inc > 0`, or
/// `i1/-inc, ..., i2/-inc` when `inc < 0`. Requires `start <= stop` and
/// `count >= 1`; callers needing a descending domain sort first (see
/// [`generate_ticks`]), mirroring d3's own `ticks()`/`tickIncrement()`
/// wrapper split.
fn tick_spec(start: f64, stop: f64, count: usize) -> (i64, i64, f64) {
    let count_f = count as f64;
    let step = (stop - start) / count_f;
    let power = step.log10().floor();
    let error = step / 10f64.powf(power);
    let (e2, e5, e10) = tick_factor_thresholds();
    let factor = if error >= e10 {
        10.0
    } else if error >= e5 {
        5.0
    } else if error >= e2 {
        2.0
    } else {
        1.0
    };

    let (mut i1, mut i2, mut inc);
    if power < 0.0 {
        inc = 10f64.powf(-power) / factor;
        i1 = (start * inc).round() as i64;
        i2 = (stop * inc).round() as i64;
        if (i1 as f64) / inc < start {
            i1 += 1;
        }
        if (i2 as f64) / inc > stop {
            i2 -= 1;
        }
        inc = -inc;
    } else {
        inc = 10f64.powf(power) * factor;
        i1 = (start / inc).round() as i64;
        i2 = (stop / inc).round() as i64;
        if (i1 as f64) * inc < start {
            i1 += 1;
        }
        if (i2 as f64) * inc > stop {
            i2 -= 1;
        }
    }

    // d3's own rescue for a too-small count that produced an empty range:
    // only reachable here when `count == 1` (the only integer satisfying
    // `0.5 <= count < 2`), so it's spelled as that direct check rather than
    // the open-ended float comparison the JS source uses.
    if i2 < i1 && count == 1 {
        return tick_spec(start, stop, 2);
    }

    (i1, i2, inc)
}

/// Mirrors d3-array's `tickIncrement`: the signed step [`tick_spec`] would
/// use for `(start, stop, count)` (negative for a sub-`1.0` step -- see
/// [`tick_spec`]'s doc). Requires `start <= stop`, matching `tick_spec`.
fn tick_increment(start: f64, stop: f64, count: usize) -> f64 {
    tick_spec(start, stop, count).2
}

/// Generate ticks between `start` and `stop` (either order) via
/// [`tick_spec`], inclusive of an endpoint that lands exactly on a step
/// multiple. `start == stop` returns that single value (a flat domain
/// still gets one tick, e.g. for an all-equal-value chart); `count == 0`
/// returns no ticks at all (checked first, matching d3's own `ticks()`
/// precedence between the two).
fn generate_ticks(start: f64, stop: f64, count: usize) -> Vec<f64> {
    if count == 0 {
        return Vec::new();
    }
    if start == stop {
        return vec![start];
    }

    let reverse = stop < start;
    let (i1, i2, inc) = if reverse {
        tick_spec(stop, start, count)
    } else {
        tick_spec(start, stop, count)
    };
    if i2 < i1 {
        return Vec::new();
    }

    let n = (i2 - i1 + 1) as usize;
    let mut out = Vec::with_capacity(n);
    let divide = inc < 0.0;
    for i in 0..n {
        let k = if reverse {
            i2 - i as i64
        } else {
            i1 + i as i64
        };
        out.push(if divide {
            k as f64 / -inc
        } else {
            k as f64 * inc
        });
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

/// Format a number to at most `decimals` places: trailing zeros and a
/// trailing `.` trimmed, and `-0` normalized to `0` (a value that rounds to
/// exactly zero from the negative side, e.g. a scale of a tiny negative
/// input, must not render the confusing, byte-different `-0`).
pub(super) fn fmt_decimal(v: f64, decimals: u8) -> String {
    if !v.is_finite() {
        return "0".to_string();
    }
    let scale = 10f64.powi(decimals as i32);
    let rounded = (v * scale).round() / scale;
    let rounded = if rounded == 0.0 { 0.0 } else { rounded }; // normalize -0.0 -> 0.0
    let s = format!("{rounded:.*}", decimals as usize);
    let s = s.trim_end_matches('0');
    let s = s.trim_end_matches('.');
    if s.is_empty() || s == "-" {
        "0".to_string()
    } else {
        s.to_string()
    }
}

/// Format a coordinate for an SVG attribute: at most 3 decimal places (per
/// `$S/chart-api.md`'s numeric-formatting rule). See [`fmt_decimal`].
pub(super) fn fmt_num(v: f64) -> String {
    fmt_decimal(v, 3)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Assert two `f64` slices are equal to within float rounding noise.
    /// Only [`LinearScale::scale`]'s plain affine map (unrelated to the
    /// "nice" tick arithmetic, which is deliberately exact -- see the
    /// module doc) still needs this: `1.1 * 200.0` is not bit-exact in
    /// IEEE 754, and there's no reason to special-case that away.
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

        // The canonical d3-array example: domain (0,1), count 5 -> step
        // 0.2. This is now bit-exact (not just close), which is the whole
        // point of `tick_spec`'s divide-below-one branch -- see module doc.
        let unit = LinearScale {
            domain: (0.0, 1.0),
            range: (0.0, 1.0),
        };
        assert_eq!(unit.ticks(5), vec![0.0, 0.2, 0.4, 0.6, 0.8, 1.0]);
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
    fn ticks_of_zero_count_is_empty_even_for_a_flat_domain() {
        // Matches d3's own precedence: the `count > 0` guard runs before
        // the `start === stop` check.
        let s = LinearScale {
            domain: (7.0, 7.0),
            range: (0.0, 1.0),
        };
        assert_eq!(s.ticks(0), Vec::<f64>::new());
    }

    #[test]
    fn ticks_rescues_a_too_small_count() {
        // count == 1's first attempt asks for a step of the whole span
        // (0.8), which rounds both endpoints *inward* past each other
        // (i2 < i1) and would return no ticks at all -- d3's rescue
        // re-tries once at count*2 rather than giving up empty.
        let s = LinearScale {
            domain: (0.1, 0.9),
            range: (0.0, 1.0),
        };
        assert_eq!(s.ticks(1), vec![0.5]);
    }

    #[test]
    fn nice_extends_domain_to_round_bounds() {
        let s = LinearScale {
            domain: (0.32, 98.6),
            range: (0.0, 1.0),
        }
        .nice();
        assert_eq!(s.domain, (0.0, 100.0));

        // Already-nice bounds are left alone (the fixed-point loop
        // converges on the first comparison).
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

    #[test]
    fn nice_handles_a_descending_domain() {
        let s = LinearScale {
            domain: (98.6, 0.32),
            range: (0.0, 1.0),
        }
        .nice();
        assert_eq!(s.domain, (100.0, 0.0));
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

    #[test]
    fn nested_band_scale_positions_grouped_series_within_one_band() {
        // The grouped-bar composition the module doc describes: an outer
        // per-datum band, then an inner per-series band nested inside it.
        let outer = BandScale {
            count: 2,
            range: (0.0, 200.0),
            padding: 0.0,
        };
        let (start, width) = outer.band(0);
        assert_eq!((start, width), (0.0, 100.0));
        let inner = BandScale {
            count: 2,
            range: (start, start + width),
            padding: 0.1,
        };
        let (b0, w0) = inner.band(0);
        let (b1, _) = inner.band(1);
        assert!(b0 < b1, "the two series bars don't overlap");
        assert!(
            b1 + inner.band(1).1 <= 100.0 + 1e-9,
            "stays inside the outer band"
        );
        assert!(w0 > 0.0);
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

    // -- fmt_num / fmt_decimal ------------------------------------------------------

    #[test]
    fn fmt_num_trims_to_at_most_3_decimals() {
        assert_eq!(fmt_num(1.0), "1");
        assert_eq!(fmt_num(1.5), "1.5");
        assert_eq!(fmt_num(1.23456), "1.235");
        assert_eq!(fmt_num(-0.0), "0");
        assert_eq!(fmt_num(0.00049), "0");
        assert_eq!(fmt_num(-2.0), "-2");
    }

    #[test]
    fn fmt_decimal_supports_other_precisions() {
        assert_eq!(fmt_decimal(1.256, 2), "1.26");
        assert_eq!(fmt_decimal(3.0, 2), "3");
        assert_eq!(fmt_decimal(f64::NAN, 2), "0");
        assert_eq!(fmt_decimal(f64::INFINITY, 2), "0");
        // A well-known IEEE 754 caveat, pinned rather than papered over:
        // 1.005 has no exact binary representation and stores as very
        // slightly *below* 1.005, so it rounds down like 1.004999... would
        // -- not the "1.01" naive decimal intuition suggests.
        assert_eq!(fmt_decimal(1.005, 2), "1");
    }
}
