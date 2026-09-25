//! The [`ChartKind::Pie`](crate::chart::ChartKind::Pie) family: one ring of
//! proportional wedges. Geometry: [`super::super::super::engine::polar`]
//! (`arc_path`/`pie_layout`/`centroid` -- see that module's own doc for the
//! d3-shape provenance).
//!
//! ## Data model: one value per datum, not per series
//!
//! Every other family (Area/Bar/Line) draws one mark **per configured
//! series** per datum. A pie draws the opposite shape: one slice **per
//! datum** (category), sized by that datum's own `values[0]` -- matching
//! shadcn's own `<Pie data={chartData} dataKey="visitors" />`, which reads
//! `visitors` off each *row*, not off a second, per-row-varying series.
//! Concretely: a plain (non-stacked) pie's [`crate::chart::ChartConfig`] is
//! just **one** series (whatever key/label the caller likes -- shadcn's own
//! demos use `"visitors"`), and every [`crate::chart::ChartDatum::color`]
//! (not [`crate::chart::ChartSeries::color`]) is what colors that datum's
//! own slice -- falling back to `--dx-chart-<1..8>` by position when a
//! datum sets no color of its own, cycling if there are more than 8.
//!
//! **Stacked pie is the one case with more than one series**: shadcn's
//! `chart-pie-stacked` draws two concentric rings (`desktop`/`mobile`),
//! each its own full pie over the *same* categories, colored by category
//! either way (both rings share one 5-color month palette in the demo).
//! This module detects that shape from `ctx.config.series.len() > 1` and
//! radius-divides `[inner_radius, outer_radius]` into that many
//! concentric bands (a [`BandScale`], series `0` = innermost, matching
//! shadcn's own `outerRadius=60` (first/inner) vs `innerRadius=70,
//! outerRadius=90` (second/outer)), reading each ring's own values via
//! [`SeriesRenderContext::series_values`].
//!
//! ## Known cross-family limitation (not this file's to fix)
//!
//! `ChartTooltip`/`ChartLegend` (`components::{tooltip,legend}`, s2-tooltip-
//! owned) both iterate `ctx.config.series` -- correct for every Cartesian
//! family (one row/swatch per series IS the right shape there), but wrong
//! for a single-series pie: shadcn's `chart-pie-legend` shows one swatch
//! **per slice** (per datum), which today's `ChartLegend` cannot produce
//! (it would render exactly one swatch, labeled by the pie's one series).
//! Radar independently hit the analogous `ChartTooltip` positioning gap
//! (`$S/stage2-lanes.md`, s2-radar's own entries) -- two occurrences of
//! "Cartesian-only assumption baked into the shared tooltip/legend", which
//! is this repo's own trigger for treating it as a class needing a
//! construction, not two one-off patches (`CLAUDE.md`). Flagged in this
//! lane's own ledger entry for whoever owns that generalization; this
//! file still renders `ChartTooltip`/`ChartLegend` in the variants that
//! port a shadcn demo using them, since the content they DO show (however
//! incomplete) is still correct and forward-compatible with that fix.

use dioxus::prelude::*;

use super::super::layout::SeriesRenderContext;
use crate::chart::context::use_chart;
use crate::chart::engine::polar::{arc_path, centroid, pie_layout, PieArc};
use crate::chart::engine::scale::{fmt_decimal, fmt_num};
use crate::chart::{BandScale, ChartDatum};

/// Fraction of the SVG's own half-min(width, height) used as a pie's
/// outer radius -- leaves a small inset so a slice's stroke, and any
/// grown/active outer radius, never clips against the `viewBox` edge.
const OUTER_RADIUS_FRACTION: f64 = 0.9;
/// Padding (as a fraction of one ring's own step) between two concentric
/// rings of a stacked pie -- same role as `components::layout`'s
/// `BAND_PADDING`, just for a radial band scale instead of a horizontal
/// one.
const RING_PADDING: f64 = 0.08;

/// How [`render`] draws per-slice text -- see the module doc for why
/// every variant is positioned at the slice's own [`centroid`] rather
/// than shadcn/Recharts' default outside-the-ring-with-a-leader-line
/// placement (a documented simplification: this crate's polar engine
/// deliverable is `centroid` for labels, not leader-line routing).
#[derive(Clone, Debug, PartialEq, Default)]
pub enum PieLabels {
    /// No per-slice labels (the default -- matches most of shadcn's own
    /// pie demos).
    #[default]
    None,
    /// Each slice's own value, formatted like the hidden table's cells.
    /// Matches shadcn's `chart-pie-label`.
    Value,
    /// Each slice's share of the total, as a percentage (one decimal
    /// place, e.g. `"29.7%"`).
    Percent,
    /// Caller-supplied text, one entry per datum (by position). Covers
    /// both shadcn's `chart-pie-label-list` (pass each category's own
    /// name) and `chart-pie-label-custom` (pass any other formatted
    /// string) as the same mechanism -- the caller decides the text, this
    /// crate only decides where it goes. Shorter than `data.len()`:
    /// a missing entry renders no label for that slice (handled
    /// defensively, not a panic).
    List(Vec<String>),
}

/// [`crate::chart::ChartKind::Pie`]'s own options.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct PieOptions {
    /// `0.0` draws a plain pie; greater than `0.0` draws a donut with a
    /// hole of this radius (logical SVG units, clamped to never exceed
    /// the computed outer radius).
    pub inner_radius: f64,
    /// The full angular gap between adjacent slices, in radians --
    /// forwarded to [`pie_layout`]/[`arc_path`] unchanged.
    pub pad_angle: f64,
    /// Rounds each slice's corners by this radius (logical SVG units) --
    /// see `engine::polar::arc_path`'s own doc for this crate's
    /// simplified (documented) construction.
    pub corner_radius: f64,
    /// Per-slice text -- see [`PieLabels`].
    pub labels: PieLabels,
    /// Force a specific slice "active" (grown outer radius, `data-active`
    /// attribute) regardless of hover -- e.g. shadcn's
    /// `chart-pie-donut-active` (a fixed index) and `chart-pie-interactive`
    /// (driven by a `Select`, reactively re-passed here by the caller).
    /// `None` (the default) leaves "active" purely hover-driven (this
    /// family's own `onpointerenter`, same [`crate::chart::ChartContext::
    /// active_index`] every other family's hit-bands write).
    pub active_index: Option<usize>,
    /// Text centered in a donut's hole: `(primary, secondary)`, e.g.
    /// `("275", "Visitors")` -- matches shadcn's `chart-pie-donut-text`.
    /// Meaningful only when `inner_radius > 0.0`.
    pub center_text: Option<(String, String)>,
}

/// Render one ring (plain pie) or several concentric rings (stacked pie,
/// `ctx.config.series.len() > 1` -- see the module doc), plus the
/// optional donut center text.
pub(crate) fn render(ctx: &SeriesRenderContext, opts: &PieOptions) -> Element {
    let chart_ctx = use_chart();
    let active_index_signal = chart_ctx.active_index;

    let cx = ctx.width / 2.0;
    let cy = ctx.height / 2.0;
    let max_radius = (ctx.width.min(ctx.height) / 2.0) * OUTER_RADIUS_FRACTION;
    let outer_radius = max_radius;
    let inner_radius = opts
        .inner_radius
        .max(0.0)
        .min((outer_radius - 1.0).max(0.0));

    let hover_active = ctx.active_index;
    let active = opts.active_index.or(hover_active);

    let n_series = ctx.config.series.len();
    let stacked_rings = n_series > 1;

    let translate = format!("translate({}, {})", fmt_num(cx), fmt_num(cy));

    rsx! {
        g { "data-slot": "chart-series",
            g { transform: "{translate}",
                if stacked_rings {
                    {render_rings(ctx, opts, inner_radius, outer_radius, n_series, active, active_index_signal)}
                } else {
                    {render_single_ring(ctx, opts, inner_radius, outer_radius, active, active_index_signal)}
                }
            }
            if let Some((primary, secondary)) = &opts.center_text {
                text {
                    "data-slot": "chart-pie-center-text",
                    x: "{fmt_num(cx)}",
                    y: "{fmt_num(cy)}",
                    text_anchor: "middle",
                    tspan { x: "{fmt_num(cx)}", dy: "-0.1em", "{primary}" }
                    tspan { x: "{fmt_num(cx)}", dy: "1.4em", "{secondary}" }
                }
            }
        }
    }
}

/// The plain (single-series) pie: one slice per datum, colored per-datum
/// -- see the module doc.
fn render_single_ring(
    ctx: &SeriesRenderContext,
    opts: &PieOptions,
    inner_radius: f64,
    outer_radius: f64,
    active: Option<usize>,
    active_index_signal: Signal<Option<usize>>,
) -> Element {
    let values: Vec<f64> = ctx
        .data
        .iter()
        .map(|d| d.values.first().copied().flatten().unwrap_or(0.0))
        .collect();
    let total: f64 = values.iter().sum();
    let arcs = pie_layout(
        &values,
        opts.pad_angle,
        0.0,
        crate::chart::engine::polar::PieSort::None,
    );

    rsx! {
        for (i , arc) in arcs.iter().enumerate() {
            {render_slice(SliceParams {
                inner_radius,
                outer_radius,
                arc,
                index: i,
                series_slot: None,
                datum: ctx.data.get(i),
                total,
                is_active: active == Some(i),
                labels: &opts.labels,
                corner_radius: opts.corner_radius,
                active_index_signal,
            })}
        }
    }
}

/// A stacked pie: `ctx.config.series.len()` concentric rings, series `0`
/// innermost -- see the module doc.
fn render_rings(
    ctx: &SeriesRenderContext,
    opts: &PieOptions,
    inner_radius: f64,
    outer_radius: f64,
    n_series: usize,
    active: Option<usize>,
    active_index_signal: Signal<Option<usize>>,
) -> Element {
    let band = BandScale {
        count: n_series,
        range: (inner_radius, outer_radius),
        padding: RING_PADDING,
    };

    rsx! {
        for (s , series) in ctx.config.series.iter().enumerate() {
            {
                let (ring_inner, ring_width) = band.band(s);
                let ring_outer = ring_inner + ring_width;
                let values: Vec<f64> = ctx
                    .series_values(s)
                    .into_iter()
                    .map(|v| v.unwrap_or(0.0))
                    .collect();
                let total: f64 = values.iter().sum();
                let arcs = pie_layout(&values, opts.pad_angle, 0.0, crate::chart::engine::polar::PieSort::None);
                let slot = series.slot();
                rsx! {
                    g { key: "{series.key}", "data-slot": "chart-series", "data-series": "{slot}",
                        for (i , arc) in arcs.iter().enumerate() {
                            {render_slice(SliceParams {
                                inner_radius: ring_inner,
                                outer_radius: ring_outer,
                                arc,
                                index: i,
                                series_slot: Some(slot.clone()),
                                datum: ctx.data.get(i),
                                total,
                                is_active: active == Some(i),
                                labels: &opts.labels,
                                corner_radius: opts.corner_radius,
                                active_index_signal,
                            })}
                        }
                    }
                }
            }
        }
    }
}

/// Bundled args for [`render_slice`] -- enough fields (radii, the arc
/// itself, its index/series/datum identity, label config, the shared
/// active-index signal) that a positional parameter list would fail
/// clippy's `too_many_arguments`, same rationale as `components::layout::
/// LayoutParams`.
struct SliceParams<'a> {
    inner_radius: f64,
    outer_radius: f64,
    arc: &'a PieArc,
    index: usize,
    series_slot: Option<String>,
    datum: Option<&'a ChartDatum>,
    total: f64,
    is_active: bool,
    labels: &'a PieLabels,
    corner_radius: f64,
    active_index_signal: Signal<Option<usize>>,
}

/// One slice: the `path[data-slot="chart-arc"]` itself, plus its label
/// (if [`PieLabels`] calls for one at this index). The path's own
/// `d` is computed at this slice's *resting* outer radius always (SSR-
/// stable regardless of hover) -- growing on `data-active` is this
/// family's own themed CSS's job (a `transform: scale(...)` from the
/// ring's own center, which this element's parent `<g>` already
/// translated to -- see [`render`]'s own doc), not a second, larger
/// `arc_path` computed here.
fn render_slice(mut p: SliceParams<'_>) -> Element {
    let mut active_index_signal = p.active_index_signal;
    let index = p.index;
    let d = arc_path(
        p.inner_radius,
        p.outer_radius,
        p.arc.start_angle,
        p.arc.end_angle,
        p.arc.pad_angle,
        p.corner_radius,
    );
    let color = p
        .datum
        .and_then(|d| d.color.clone())
        .unwrap_or_else(|| format!("var(--dx-chart-{})", (p.index % 8) + 1));
    let label_text = label_text(&p);
    let (label_x, label_y) = centroid(
        p.inner_radius,
        p.outer_radius,
        p.arc.start_angle,
        p.arc.end_angle,
    );

    rsx! {
        path {
            key: "{index}",
            "data-slot": "chart-arc",
            "data-index": "{index}",
            "data-series": p.series_slot.take(),
            "data-active": p.is_active.then_some("true"),
            "data-start-angle": "{fmt_num(p.arc.start_angle)}",
            "data-end-angle": "{fmt_num(p.arc.end_angle)}",
            d: "{d}",
            fill: "{color}",
            onpointerenter: move |_| active_index_signal.set(Some(index)),
        }
        if let Some(text_content) = label_text {
            text {
                "data-slot": "chart-arc-label",
                x: "{fmt_num(label_x)}",
                y: "{fmt_num(label_y)}",
                text_anchor: "middle",
                "{text_content}"
            }
        }
    }
}

fn label_text(p: &SliceParams<'_>) -> Option<String> {
    match p.labels {
        PieLabels::None => None,
        PieLabels::Value => Some(fmt_decimal(p.arc.value, 2)),
        PieLabels::Percent => {
            if p.total > 0.0 {
                Some(format!("{:.1}%", p.arc.value / p.total * 100.0))
            } else {
                Some("0%".to_string())
            }
        }
        PieLabels::List(items) => items.get(p.index).filter(|s| !s.is_empty()).cloned(),
    }
}
