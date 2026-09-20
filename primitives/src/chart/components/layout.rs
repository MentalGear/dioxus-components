//! Shared Cartesian plot geometry: the margin constants, the scale
//! construction, and the grid/axis rendering `components::chart` used to do
//! inline, plus [`SeriesRenderContext`] -- the read-only bundle every
//! series family's own `render` (`components::series::{area,bar,line,
//! pie,radar,radial}`) receives. Factored out of `chart.rs` in the stage-2
//! chart round's `s2-refactor` lane specifically so this geometry has one
//! home shared by every family, instead of six copies drifting apart.
//!
//! Only [`ChartKind::is_cartesian`] kinds (Area/Bar/Line) actually use the
//! grid/axis renderers here -- see `components::chart`'s own module doc for
//! why the polar/radial stub kinds skip them entirely rather than drawing a
//! Cartesian grid behind a shape that isn't Cartesian.

use dioxus::prelude::*;

use crate::chart::engine::scale::{fmt_decimal, fmt_num};
use crate::chart::{
    stack_with_mode, BandScale, ChartConfig, ChartDatum, ChartKind, Curve, LinearScale, StackMode,
};
use crate::direction::Direction;

/// Fixed MVP layout constants (logical SVG units, scaled visually by CSS --
/// see `components::chart`'s own module doc). Not configurable yet: this
/// repo's other multi-part primitives (e.g. `Resizable`) don't expose
/// pixel-tuning props either, preferring a themed wrapper's CSS for that;
/// the same holds here once a need for it is demonstrated.
const MARGIN_TOP: f64 = 8.0;
const MARGIN_RIGHT: f64 = 8.0;
const MARGIN_BOTTOM_WITH_AXIS: f64 = 24.0;
const MARGIN_BOTTOM_BARE: f64 = 8.0;
const MARGIN_LEFT_WITH_AXIS: f64 = 40.0;
const MARGIN_LEFT_BARE: f64 = 8.0;
/// Fraction of one category's step left as a gap around/between its bars
/// (or, for Line/Area, simply how far a hit band's edge sits from its
/// neighbor's -- the point positions themselves are the band *centers*
/// either way, so this only visibly matters for `Bar`).
///
/// `pub(crate)`, not private: `components::series::bar` (s2-bar-owned, same
/// as this file) reuses this exact value when it builds its own *second*
/// band scale for a horizontal bar's category axis (this module's own
/// `x_scale`/`y_scale` stay Cartesian-only -- see that file's module doc for
/// why horizontal orientation is computed locally there rather than
/// threaded through `LayoutParams`/`SeriesRenderContext`), so the two
/// orientations' category spacing matches exactly rather than drifting via
/// two independently-tuned literals.
pub(crate) const BAND_PADDING: f64 = 0.2;

/// The inputs [`build`] needs to compute a [`SeriesRenderContext`] --
/// bundled into one struct rather than a long parameter list (clippy's
/// `too_many_arguments`, and simple readability: this is every one of
/// `Chart`'s own props/resolved-state values that geometry depends on).
pub(crate) struct LayoutParams<'a> {
    pub width: f64,
    pub height: f64,
    pub show_x_axis: bool,
    pub show_y_axis: bool,
    pub y_tick_count: usize,
    pub kind: ChartKind,
    /// Already resolved by the caller to this render's *effective* stacking
    /// (e.g. `props.stacked && matches!(kind, Area | Bar)`) -- this module
    /// applies no kind-based gating of its own.
    pub stacked: bool,
    /// Which [`stack_with_mode`] mode `stacked`'s spans use -- ignored
    /// entirely when `stacked` is `false`. Defaults to
    /// [`StackMode::Normal`] (`#[derive(Default)]` on `StackMode` itself),
    /// so every existing call site that doesn't set this field keeps
    /// today's exact behavior. Stage-2 chart round, §4(c) of the handoff:
    /// before this field existed, `AreaOptions::stack_mode ==
    /// StackMode::Expand` was read by `series::area::render` alone, which
    /// recomputed its own local percent spans + a local `(0.0,
    /// 1.0)`-domain scale for the marks -- correct for the marks
    /// themselves, but `Chart`'s shared grid lines/y-axis ticks/tooltip
    /// vertical anchor still read the raw, non-percent `y_scale` this
    /// struct computes, and visually disagreed with a percent-stacked
    /// chart. Once a caller threads its own `AreaOptions`/`BarOptions`
    /// `stack_mode` through to this field, `y_scale`/`stacked_spans` here
    /// reflect the same mode the marks use, and the family's own `render`
    /// can read them instead of recomputing a local copy.
    pub stack_mode: StackMode,
    pub curve: Curve,
    pub dir: Direction,
    pub active_index: Option<usize>,
    pub config: &'a ChartConfig,
    pub data: &'a [ChartDatum],
}

/// Shared read-only context every series family's `render` needs: the
/// scales and plot geometry [`build`] computed once for this render, plus
/// the chart's own data/config/active index. Lives in the Dioxus-facing
/// `components` layer (unlike `engine`, nothing here is bound by that
/// module's upstreaming seam -- see its doc) even though none of today's
/// fields actually name a `dioxus` type.
///
/// Deliberately holds owned `config`/`data` (cheap: a chart's series/datum
/// counts are always small) rather than borrowing, so a family's `render`
/// signature stays a plain `fn(&SeriesRenderContext, &XOptions) -> Element`
/// with no lifetime parameter to thread through `rsx!`'s own closures.
pub(crate) struct SeriesRenderContext {
    // `kind`/`dir`/`active_index`/`width`/`height` are read by no family's
    // `render` today (Area/Bar/Line need none of them; the three stub
    // families ignore `ctx` entirely) -- `#[allow(dead_code)]` because
    // `pub(crate)` visibility lets rustc see that whole-crate truth and
    // warn on it, unlike a fully `pub` struct's fields (unprovably unused,
    // so exempt by default). Kept anyway: `$S/stage2-common.md`'s own spec
    // for this struct names exactly these fields (plus the ones already
    // read below), reserved for a family that needs them -- e.g. a future
    // dimmed-when-inactive mark (`active_index`), an RTL-mirrored polar
    // layout (`dir`), or an arc/polygon geometry computed from the SVG's
    // own center (`width`/`height`) -- not yet, but by construction rather
    // than by re-adding them under time pressure later.
    #[allow(dead_code)]
    pub kind: ChartKind,
    pub config: ChartConfig,
    pub data: Vec<ChartDatum>,
    #[allow(dead_code)]
    pub dir: Direction,
    /// The currently hovered/keyboard-focused datum index, read-only here
    /// -- only `Chart`'s own hit-bands (still rendered by `chart.rs`
    /// itself, not any family) write it.
    #[allow(dead_code)]
    pub active_index: Option<usize>,
    #[allow(dead_code)]
    pub width: f64,
    #[allow(dead_code)]
    pub height: f64,
    pub plot_x0: f64,
    pub plot_x1: f64,
    pub plot_y0: f64,
    pub plot_y1: f64,
    pub x_scale: BandScale,
    pub y_scale: LinearScale,
    pub y_ticks: Vec<f64>,
    /// `y_scale.scale(0.0)` -- every non-stacked Area/Bar baseline.
    pub zero_y: f64,
    /// Each datum's x position (band centers), aligned to `data`.
    pub xs: Vec<f64>,
    pub curve: Curve,
    pub stacked: bool,
    /// [`stack()`]'s output, one row per datum, empty when not stacking.
    pub stacked_spans: Vec<Vec<(f64, f64)>>,
}

impl SeriesRenderContext {
    /// Series `s`'s raw value for every datum, `None` when a datum has no
    /// entry for it at all (a shorter `values` list than the config's
    /// series count -- defensive, never panics). Shared by every
    /// non-stacked family's `render` (`series::{area,bar,line}`) --
    /// pure data extraction, not per-family drawing logic, so unlike the
    /// per-series marks loop itself (deliberately NOT factored out here --
    /// each family's loop body differs enough that sharing it would cost
    /// more clarity than it saves), this one small helper is worth sharing.
    pub(crate) fn series_values(&self, s: usize) -> Vec<Option<f64>> {
        self.data
            .iter()
            .map(|d| d.values.get(s).copied().flatten())
            .collect()
    }
}

/// Compute this render's [`SeriesRenderContext`]: margins (from
/// `show_x_axis`/`show_y_axis`), the x [`BandScale`]/y [`LinearScale`] (via
/// [`crate::chart::nice_domain`]), y ticks, and (when `stacked`) the
/// [`stack()`] spans every stacked Area/Bar mark reads. Ported unchanged
/// from `Chart`'s own body (pre-stage-2-refactor) -- see
/// `components::chart`'s tests for the behavior this must keep producing
/// bit-for-bit.
pub(crate) fn build(p: LayoutParams<'_>) -> SeriesRenderContext {
    let margin_left = if p.show_y_axis {
        MARGIN_LEFT_WITH_AXIS
    } else {
        MARGIN_LEFT_BARE
    };
    let margin_bottom = if p.show_x_axis {
        MARGIN_BOTTOM_WITH_AXIS
    } else {
        MARGIN_BOTTOM_BARE
    };
    let plot_x0 = margin_left;
    let plot_x1 = p.width - MARGIN_RIGHT;
    let plot_y0 = MARGIN_TOP;
    let plot_y1 = p.height - margin_bottom;

    let n = p.data.len();
    let x_scale = BandScale {
        count: n.max(1),
        range: (plot_x0, plot_x1),
        padding: BAND_PADDING,
    };
    let xs: Vec<f64> = (0..n).map(|i| x_scale.center(i)).collect();

    let (y_min, y_max) = y_extent(p.config, p.data, p.stacked, p.stack_mode);
    let y_domain = crate::chart::nice_domain(y_min, y_max);
    let y_scale = LinearScale {
        domain: y_domain,
        range: (plot_y1, plot_y0),
    };
    let y_ticks = y_scale.ticks(p.y_tick_count);
    let zero_y = y_scale.scale(0.0);

    let stacked_spans: Vec<Vec<(f64, f64)>> = if p.stacked {
        let rows: Vec<Vec<Option<f64>>> = p.data.iter().map(|d| d.values.clone()).collect();
        stack_with_mode(&rows, p.stack_mode)
    } else {
        Vec::new()
    };

    SeriesRenderContext {
        kind: p.kind,
        config: p.config.clone(),
        data: p.data.to_vec(),
        dir: p.dir,
        active_index: p.active_index,
        width: p.width,
        height: p.height,
        plot_x0,
        plot_x1,
        plot_y0,
        plot_y1,
        x_scale,
        y_scale,
        y_ticks,
        zero_y,
        xs,
        curve: p.curve,
        stacked: p.stacked,
        stacked_spans,
    }
}

/// The y-domain input before [`crate::chart::nice_domain`]: the min/max
/// across every configured series' values, or (for `stacked`) across
/// [`stack_with_mode`]'s own per-row spans -- a stacked chart's axis must
/// span the *cumulative* (or, under [`StackMode::Expand`], the normalized
/// 0..1) totals, not each series' own raw values.
fn y_extent(
    config: &ChartConfig,
    data: &[ChartDatum],
    stacked: bool,
    stack_mode: StackMode,
) -> (f64, f64) {
    let mut lo = 0.0f64;
    let mut hi = 0.0f64;
    if stacked {
        let rows: Vec<Vec<Option<f64>>> = data.iter().map(|d| d.values.clone()).collect();
        for row in stack_with_mode(&rows, stack_mode) {
            for (y0, y1) in row {
                lo = lo.min(y0).min(y1);
                hi = hi.max(y0).max(y1);
            }
        }
    } else {
        for datum in data {
            for v in datum.values.iter().take(config.series.len()).flatten() {
                lo = lo.min(*v);
                hi = hi.max(*v);
            }
        }
    }
    (lo, hi)
}

/// Render `g[data-slot="chart-grid"]`: one horizontal line per y tick, plus
/// (`ChartKind::Bar` only) an explicit `line[data-slot="chart-zero-line"]`
/// at the baseline.
///
/// The zero line is gated on `ctx.kind == ChartKind::Bar` specifically,
/// not drawn for every kind that happens to share this grid renderer
/// (Area/Line too): [`crate::chart::nice_domain`] always includes `0.0` in
/// the y domain, so `ctx.zero_y` is always a real, valid pixel position
/// regardless of kind, but a explicit baseline line is only useful where a
/// mark's own visual weight actually starts *from* zero (a bar) --
/// `$S/stage2-common.md`'s own brief for this lane names the `negative`
/// bar-chart variant's "zero line drawn" requirement specifically. Gating
/// here, on a field every kind's `render_grid` call already receives,
/// keeps this change from altering Area/Line's rendered output at all
/// (verified: `ctx.kind` is a plain match, not a new parameter neither
/// `s2-area` nor `s2-line`'s own call sites need to know about), rather
/// than risking a shared-infrastructure behavior change those lanes did
/// not ask for and have not verified against.
pub(crate) fn render_grid(ctx: &SeriesRenderContext) -> Element {
    rsx! {
        g { "data-slot": "chart-grid",
            for y_tick in ctx.y_ticks.iter().copied() {
                line {
                    key: "{y_tick}",
                    x1: "{fmt_num(ctx.plot_x0)}",
                    x2: "{fmt_num(ctx.plot_x1)}",
                    y1: "{fmt_num(ctx.y_scale.scale(y_tick))}",
                    y2: "{fmt_num(ctx.y_scale.scale(y_tick))}",
                }
            }
            if matches!(ctx.kind, ChartKind::Bar) {
                line {
                    "data-slot": "chart-zero-line",
                    x1: "{fmt_num(ctx.plot_x0)}",
                    x2: "{fmt_num(ctx.plot_x1)}",
                    y1: "{fmt_num(ctx.zero_y)}",
                    y2: "{fmt_num(ctx.zero_y)}",
                }
            }
        }
    }
}

/// Render `g[data-slot="chart-axis"][data-axis="x"]`: one tick label per
/// datum whose index survives [`x_tick_step`]'s thinning.
pub(crate) fn render_x_axis(
    ctx: &SeriesRenderContext,
    x_tick_format: &Option<Callback<String, String>>,
    max_x_ticks: usize,
) -> Element {
    let tick_step = x_tick_step(ctx.data.len(), max_x_ticks);
    rsx! {
        g { "data-slot": "chart-axis", "data-axis": "x",
            for (i , datum) in ctx.data.iter().enumerate() {
                if i % tick_step == 0 {
                    text {
                        key: "{i}",
                        "data-index": "{i}",
                        x: "{fmt_num(ctx.x_scale.center(i))}",
                        y: "{fmt_num(ctx.plot_y1 + 16.0)}",
                        {format_x_tick(&datum.label, x_tick_format)}
                    }
                }
            }
        }
    }
}

/// Render `g[data-slot="chart-axis"][data-axis="y"]`: one tick label per y
/// tick.
pub(crate) fn render_y_axis(ctx: &SeriesRenderContext) -> Element {
    rsx! {
        g { "data-slot": "chart-axis", "data-axis": "y",
            for y_tick in ctx.y_ticks.iter().copied() {
                text {
                    key: "{y_tick}",
                    x: "{fmt_num(ctx.plot_x0 - 8.0)}",
                    y: "{fmt_num(ctx.y_scale.scale(y_tick))}",
                    {fmt_decimal(y_tick, 2)}
                }
            }
        }
    }
}

/// Format an x-axis category label: the caller's own formatter if given,
/// else its first 3 characters (shadcn's own demo convention).
fn format_x_tick(label: &str, format: &Option<Callback<String, String>>) -> String {
    match format {
        Some(cb) => cb.call(label.to_string()),
        None => label.chars().take(3).collect(),
    }
}

/// The x-axis tick-label stride (see `ChartProps::max_x_ticks`): label
/// datum `i` only when `i % x_tick_step(..) == 0`, so at most `max_x_ticks`
/// labels are drawn regardless of `n`, always including the first datum
/// (`i == 0`). MVP count-based thinning -- a pure function so the
/// "at most `max_x_ticks` labels" guarantee is unit-testable independent of
/// any SSR render.
fn x_tick_step(n: usize, max_x_ticks: usize) -> usize {
    if n == 0 {
        return 1;
    }
    n.div_ceil(max_x_ticks.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn x_tick_step_keeps_the_rendered_count_at_or_under_max_x_ticks() {
        // The stride itself, and the classic pagination identity it relies
        // on (`ceil(n / ceil(n / max)) <= max` for positive integers): the
        // rendered tick count is `ceil(n / x_tick_step(n, max))`, so no
        // input can ever render more than `max_x_ticks` labels.
        for n in [0usize, 1, 2, 11, 12, 13, 29, 90, 91, 1000] {
            for max in [1usize, 3, 5, 12, 50] {
                let step = x_tick_step(n, max);
                assert!(step >= 1, "step must be >= 1 for n={n} max={max}");
                let rendered = if n == 0 { 0 } else { n.div_ceil(step) };
                assert!(
                    rendered <= max,
                    "n={n} max={max} step={step} rendered={rendered} exceeds max_x_ticks"
                );
            }
        }
        // Concrete cases named in the API doc/commit message.
        assert_eq!(x_tick_step(2, 12), 1);
        assert_eq!(x_tick_step(90, 12), 8);
        assert_eq!(x_tick_step(0, 12), 1);
        assert_eq!(x_tick_step(10, 0), 10);
    }

    #[test]
    fn format_x_tick_defaults_to_first_three_characters() {
        // `Some(callback)` is exercised by `Chart`'s own SSR tests (e.g.
        // `x_axis_labels_default_to_first_three_characters`), not here:
        // `Callback::new` requires a live Dioxus runtime (`Runtime::
        // current()`), which a plain `#[test]` fn -- no `VirtualDom` --
        // does not provide.
        assert_eq!(format_x_tick("January", &None), "Jan");
        assert_eq!(format_x_tick("Hi", &None), "Hi");
    }
}
