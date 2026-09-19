//! The [`ChartKind::Area`](crate::chart::ChartKind::Area) family: a filled
//! area under a (possibly curved) line, optionally stacked. Ported
//! unchanged (stage-2 `s2-refactor` lane) from `components::chart`'s own
//! `SeriesMarks` -- see `components::layout` for the scale/margin math this
//! reads via `SeriesRenderContext`, and `components::series`'s own module
//! doc for the `stacked`/`curve`-stay-on-`ChartProps` decision.
//!
//! ## Stage-2 `s2-area` additions: [`AreaOptions`]
//!
//! Four fields, each self-contained in this file (no edit to any other
//! stage-2 lane's owned file -- `components::layout`/`engine::geometry` --
//! was needed for any of them; see each field's own doc for how):
//! - [`AreaOptions::fill_opacity`] -- an inline `style="fill-opacity:..."`
//!   on the area path itself, which (ordinary CSS cascade: inline beats a
//!   stylesheet class selector) overrides the themed package's own
//!   `fill-opacity: 0.4` rule (`preview/src/components/chart/style.css`)
//!   with no edit to that file needed.
//! - [`AreaOptions::gradient`] -- a per-series `<linearGradient>` `defs`
//!   inside that series' own `g[data-series]` group, id'd
//!   `"{chart_id}-gradient-{series_slot}"`. `chart_id` comes from
//!   [`crate::chart::use_chart`] (`ChartContext::id`) called directly in
//!   this file's own `render`, not a new `SeriesRenderContext` field --
//!   `use_chart` is `try_consume_context`-based, not hook-list-based (see
//!   its own doc), so calling it here, one plain-function frame below
//!   `Chart`'s own render body (which already called it once), is exactly
//!   as sound as calling it from `Chart` itself, and needs no new field on
//!   a struct owned by another lane. SSR-stable: derived only from `id`
//!   (already SSR-stable, `ChartContainer`'s own `use_id_or`/
//!   `use_unique_id`) and each series' own already-sanitized `slot()` --
//!   nothing randomized, nothing client-only.
//! - [`AreaOptions::connect_nulls`] -- pre-compacts this series' `(x,
//!   value)` pairs to only the defined ones *before* calling
//!   [`plot_runs`], rather than changing what counts as a gap inside that
//!   shared (`engine::geometry`, `s2-bar`-owned) function itself: with no
//!   `None` left in the input, `plot_runs` already returns one unbroken
//!   run spanning every defined point, exactly the "skip it, connect
//!   across it" behavior this prop asks for, via a function this file
//!   doesn't need to touch.
//! - [`AreaOptions::stack_mode`] -- percent ("100%"/"expand") stacking.
//!   **Known, stated limitation** (not fixable from this file alone,
//!   filed to the layout owner -- see `$S/stage2-lanes.md`'s "requests for
//!   the refactor owner" section, s2-area's entry): `components::layout::
//!   build` computes `SeriesRenderContext::y_scale`/`zero_y`/
//!   `stacked_spans` once, chart-wide, from the plain (`StackMode::Normal`)
//!   [`crate::chart::stack()`] -- it has no notion of a per-family stack
//!   mode, and `layout.rs` is `s2-bar`-owned, not this file. So when
//!   [`StackMode::Expand`] is set, this file computes its OWN, locally
//!   scoped percent spans (via [`stack_with_mode`]) and its OWN local
//!   `LinearScale` with domain `(0.0, 1.0)` for the area/line marks
//!   themselves (which is what this stage's own acceptance test checks:
//!   the topmost series' area reaches the plot's top) -- but `Chart`'s
//!   shared grid lines, y-axis tick *values*, and the tooltip's vertical
//!   anchor (all computed from `ctx.y_scale`/`ctx.zero_y`, upstream of this
//!   function, in `chart.rs`/`layout.rs`) still reflect the *raw*
//!   (non-percent) domain and so will not visually agree with a percent-
//!   stacked chart's own marks until `layout::build` becomes
//!   `StackMode`-aware. The `stacked_expand` demo variant works around
//!   this by turning its own grid off (`show_grid: false`) rather than
//!   ship a visibly-misleading reference line.

use dioxus::prelude::*;

use super::super::layout::SeriesRenderContext;
use crate::chart::context::use_chart;
use crate::chart::engine::curve::{area_between_path, area_path, line_path};
use crate::chart::engine::geometry::plot_runs;
use crate::chart::engine::scale::{fmt_num, LinearScale};
use crate::chart::engine::stack::{stack_with_mode, StackMode};

/// [`crate::chart::ChartKind::Area`]'s own options. See this module's own
/// doc for how each field is implemented without touching any other
/// stage-2 lane's file.
#[derive(Clone, PartialEq, Debug)]
pub struct AreaOptions {
    /// Fill each series' area with a top-to-bottom `<linearGradient>`
    /// (opaque near the line, fading toward the baseline, matching
    /// shadcn's own `chart-area-gradient`/`chart-area-interactive` demos'
    /// `stopOpacity` `0.8`/`0.1` pair) instead of a flat
    /// `var(--series-color)` fill.
    pub gradient: bool,
    /// Fill opacity for every series' area path (the line stroke itself is
    /// always fully opaque). Matches shadcn's own demos' own literal
    /// `fillOpacity={0.4}`.
    pub fill_opacity: f64,
    /// Skip a `None` value instead of breaking the drawn area/line there --
    /// shadcn/Recharts' own `connectNulls`. Non-stacked marks only (a
    /// stacked area already draws one unbroken shape across every datum
    /// unconditionally -- see [`crate::chart::stack()`]'s own doc on `None`
    /// as a zero-height span, not a break).
    pub connect_nulls: bool,
    /// Percent ("100%"/"expand") stacking -- see this module's own doc for
    /// the one part of this that a caller should know is not (yet) fully
    /// self-contained (grid/axis/tooltip still reflect the raw domain).
    pub stack_mode: StackMode,
}

impl Default for AreaOptions {
    fn default() -> Self {
        Self {
            gradient: false,
            fill_opacity: 0.4,
            connect_nulls: false,
            stack_mode: StackMode::default(),
        }
    }
}

/// Render every configured series' area + top-line mark, in config order.
pub(crate) fn render(ctx: &SeriesRenderContext, opts: &AreaOptions) -> Element {
    // `use_chart()` is safe to call again here (one plain-function frame
    // below `Chart`'s own render body, which already called it) -- see
    // this module's own doc. Only actually read when `opts.gradient` is
    // set; computing it unconditionally is cheap (a `Memo` read) and
    // avoids threading an `Option<Memo<String>>` through `render_one`.
    let chart_id = use_chart().id;

    rsx! {
        for (s , series) in ctx.config.series.iter().enumerate() {
            {
                let slot = series.slot();
                let gradient_id = format!("{}-gradient-{slot}", chart_id());
                let fill = if opts.gradient {
                    format!("url(#{gradient_id})")
                } else {
                    "var(--series-color)".to_string()
                };
                rsx! {
                    g {
                        key: "{series.key}",
                        "data-slot": "chart-series",
                        "data-series": "{slot}",
                        style: "--series-color: var(--color-{slot})",

                        if opts.gradient {
                            defs {
                                linearGradient {
                                    id: "{gradient_id}",
                                    "x1": "0",
                                    "y1": "0",
                                    "x2": "0",
                                    "y2": "1",
                                    stop { "offset": "5%", "stop-color": "var(--series-color)", "stop-opacity": "0.8" }
                                    stop { "offset": "95%", "stop-color": "var(--series-color)", "stop-opacity": "0.1" }
                                }
                            }
                        }
                        {render_one(ctx, s, opts, &fill)}
                    }
                }
            }
        }
    }
}

fn render_one(ctx: &SeriesRenderContext, s: usize, opts: &AreaOptions, fill: &str) -> Element {
    let fill_opacity = fmt_num(opts.fill_opacity);
    if ctx.stacked {
        let n = ctx.xs.len();
        // Each arm's own `.collect::<Vec<_>>()` turbofish (not an outer
        // `(Vec<(f64, f64)>, Vec<(f64, f64)>)` annotation on the `let`
        // itself, which trips clippy's `type_complexity` lint) pins the
        // target collection type at the point of collection.
        let (top, bottom) = match opts.stack_mode {
            StackMode::Normal => (
                (0..n)
                    .map(|i| (ctx.xs[i], ctx.y_scale.scale(ctx.stacked_spans[i][s].1)))
                    .collect::<Vec<_>>(),
                (0..n)
                    .map(|i| (ctx.xs[i], ctx.y_scale.scale(ctx.stacked_spans[i][s].0)))
                    .collect::<Vec<_>>(),
            ),
            // See this module's own doc: `ctx.y_scale`/`ctx.stacked_spans`
            // were computed chart-wide from `StackMode::Normal` (
            // `components::layout::build`, `s2-bar`-owned) -- Expand
            // recomputes its own percent spans and its own local (0.0,
            // 1.0)-domain scale here instead of reading either.
            StackMode::Expand => {
                let rows: Vec<Vec<Option<f64>>> =
                    ctx.data.iter().map(|d| d.values.clone()).collect();
                let spans = stack_with_mode(&rows, StackMode::Expand);
                let percent_scale = LinearScale {
                    domain: (0.0, 1.0),
                    range: (ctx.plot_y1, ctx.plot_y0),
                };
                (
                    (0..n)
                        .map(|i| (ctx.xs[i], percent_scale.scale(spans[i][s].1)))
                        .collect::<Vec<_>>(),
                    (0..n)
                        .map(|i| (ctx.xs[i], percent_scale.scale(spans[i][s].0)))
                        .collect::<Vec<_>>(),
                )
            }
        };
        let area_d = area_between_path(&top, &bottom, ctx.curve);
        let line_d = line_path(&top, ctx.curve);
        rsx! {
            path {
                "data-slot": "chart-area",
                d: "{area_d}",
                fill: "{fill}",
                style: "fill-opacity: {fill_opacity}",
            }
            path { "data-slot": "chart-line", d: "{line_d}", fill: "none" }
        }
    } else {
        let values = ctx.series_values(s);
        let (xs, values): (Vec<f64>, Vec<Option<f64>>) = if opts.connect_nulls {
            connect_nulls(&ctx.xs, &values)
        } else {
            (ctx.xs.clone(), values)
        };
        rsx! {
            for run in plot_runs(&xs, &values) {
                {
                    let scaled: Vec<(f64, f64)> =
                        run.iter().map(|(x, v)| (*x, ctx.y_scale.scale(*v))).collect();
                    let area_d = area_path(&scaled, ctx.zero_y, ctx.curve);
                    let line_d = line_path(&scaled, ctx.curve);
                    rsx! {
                        path {
                            "data-slot": "chart-area",
                            d: "{area_d}",
                            fill: "{fill}",
                            style: "fill-opacity: {fill_opacity}",
                        }
                        path { "data-slot": "chart-line", d: "{line_d}", fill: "none" }
                    }
                }
            }
        }
    }
}

/// [`AreaOptions::connect_nulls`]'s own pre-pass: drop every `None` entry
/// (and its paired x position) instead of leaving it for [`plot_runs`] to
/// break the run at. `xs`/`values` are positional and same-length (the
/// caller's own `ctx.xs`/`ctx.series_values(s)`), so a simple parallel
/// filter is enough -- no index bookkeeping to get wrong.
///
/// ```
/// use dioxus_primitives::chart::engine::geometry::plot_runs;
/// # // connect_nulls itself is private; this doctest documents plot_runs'
/// # // own behavior on already-compacted input, which is what makes the
/// # // compaction approach correct: a fully-defined input is always one run.
/// let xs = vec![0.0, 2.0, 3.0];
/// let values = vec![Some(1.0), Some(2.0), Some(3.0)];
/// assert_eq!(plot_runs(&xs, &values).len(), 1);
/// ```
fn connect_nulls(xs: &[f64], values: &[Option<f64>]) -> (Vec<f64>, Vec<Option<f64>>) {
    let mut out_xs = Vec::with_capacity(xs.len());
    let mut out_values = Vec::with_capacity(values.len());
    for (x, v) in xs.iter().zip(values.iter()) {
        if let Some(v) = v {
            out_xs.push(*x);
            out_values.push(Some(*v));
        }
    }
    (out_xs, out_values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn area_options_default_matches_shadcns_own_demo_convention() {
        let opts = AreaOptions::default();
        assert!(!opts.gradient);
        assert_eq!(opts.fill_opacity, 0.4);
        assert!(!opts.connect_nulls);
        assert_eq!(opts.stack_mode, StackMode::Normal);
    }

    #[test]
    fn connect_nulls_drops_gaps_and_keeps_pairs_aligned() {
        let xs = vec![0.0, 1.0, 2.0, 3.0];
        let values = vec![Some(10.0), None, Some(30.0), None];
        let (out_xs, out_values) = connect_nulls(&xs, &values);
        assert_eq!(out_xs, vec![0.0, 2.0]);
        assert_eq!(out_values, vec![Some(10.0), Some(30.0)]);
    }

    #[test]
    fn connect_nulls_of_no_gaps_is_unchanged() {
        let xs = vec![0.0, 1.0];
        let values = vec![Some(1.0), Some(2.0)];
        let (out_xs, out_values) = connect_nulls(&xs, &values);
        assert_eq!(out_xs, xs);
        assert_eq!(out_values, values);
    }

    #[test]
    fn connect_nulls_all_gaps_is_empty() {
        let (out_xs, out_values) = connect_nulls(&[0.0, 1.0], &[None, None]);
        assert!(out_xs.is_empty());
        assert!(out_values.is_empty());
    }
}
