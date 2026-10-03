//! The [`ChartKind::RadialBar`](crate::chart::ChartKind::RadialBar) family:
//! concentric proportional arcs, one ring per datum (the default), or every
//! configured series stacked cumulatively into ONE ring
//! ([`RadialOptions::stacked`]) -- shadcn/ui's own two spellings of the
//! same idea (`chart-radial-simple.tsx`'s one-`<RadialBar>`-per-row default
//! versus `chart-radial-stacked.tsx`'s explicit `stackId`).
//!
//! Shares [`crate::chart::engine::polar`] with [`super::pie`] (same
//! `arc_path`/`centroid` mark geometry, same [`crate::chart::PieLabels`]
//! per-arc label enum) -- the two families differ only in what an arc's
//! angular span *means* (Pie: a category's share of one ring's whole;
//! RadialBar: a category's own value against a shared domain, laid out via
//! [`angle_scale`]) and in how many rings a chart draws when unstacked
//! (Pie: always one, unless multiple series are configured; RadialBar:
//! always one ring PER DATUM, since each row is its own bar-wrapped-into-
//! a-circle by definition).

use dioxus::prelude::*;

use super::super::center_text;
use super::super::fragment::safe_fragment_id;
use super::super::layout::SeriesRenderContext;
use super::super::pointer::Sector;
use crate::chart::context::use_chart;
use crate::chart::engine::polar::{angle_scale, arc_path, centroid};
use crate::chart::engine::scale::{fmt_decimal, fmt_num};
use crate::chart::{BandScale, ChartDatum, PieLabels};

/// Radial padding (as a fraction of one ring's own step) between two
/// concentric, unstacked rings -- same role as [`super::pie`]'s own
/// `RING_PADDING`.
const RING_PADDING: f64 = 0.15;
/// Same fallback fraction [`super::pie`] uses when a caller leaves
/// [`RadialOptions::outer_radius`] at its default (`0.0`, meaning "size me
/// from the chart's own dimensions").
const OUTER_RADIUS_FRACTION: f64 = 0.9;

/// [`crate::chart::ChartKind::RadialBar`]'s own options.
#[derive(Clone, PartialEq, Debug)]
pub struct RadialOptions {
    /// The innermost ring's own inner radius (logical SVG units). `0.0`
    /// (the default) starts the innermost ring right at the chart's own
    /// center -- a real RadialBar chart almost always wants some positive
    /// value here (a bare center point makes the innermost ring's own
    /// share of a small value visually indistinguishable from a much
    /// larger one), left to the caller to set explicitly per shadcn's own
    /// `chart-radial-*.tsx` fixtures, which all do.
    pub inner_radius: f64,
    /// The outermost ring's own outer radius (logical SVG units). `0.0`
    /// (the default) auto-sizes it from the chart's own `width`/`height`
    /// (`(min(width, height) / 2) * 0.9`, the same fraction
    /// [`super::pie`] uses for its own auto-computed outer radius) --
    /// unlike Pie, this crate exposes RadialBar's outer radius directly
    /// (matching `RadialBarChart`'s own explicit prop upstream), so a
    /// caller who wants an exact size can set one.
    pub outer_radius: f64,
    /// Where the angular domain's `0.0` end maps to, in radians. `0.0` is
    /// twelve o'clock and angles increase clockwise (this crate's engine
    /// convention, `engine::polar`'s own doc, the same as `d3-shape`'s
    /// `arc()`); shadcn's own demos commonly start elsewhere (e.g. `-PI / 2`
    /// is nine o'clock here) -- passed straight to [`angle_scale`]'s own
    /// `range`.
    pub start_angle: f64,
    /// Where the angular domain's max end maps to, in radians -- may
    /// exceed `start_angle + TAU` (e.g. shadcn's own 380° sweep on some
    /// demos: slightly more than a full turn, so the first and last bars'
    /// own ends don't visually touch at 0/360).
    pub end_angle: f64,
    /// Rounds each ring's own two ends by this radius (logical SVG units)
    /// -- forwarded to [`arc_path`] unchanged, same convention
    /// [`super::pie::PieOptions::corner_radius`] uses.
    pub corner_radius: f64,
    /// Draw a muted full-sweep background track
    /// (`data-slot="chart-polar-grid"`) behind every ring's own value arc
    /// -- shadcn's own `<RadialBar background />` prop (unstacked) or a
    /// separate `<PolarGrid />` element (stacked); this crate unifies both
    /// spellings as one boolean, since the visual result -- a muted track
    /// at each ring's own inner/outer radius, drawn before its value arc
    /// -- is identical either way.
    pub grid: bool,
    /// Per-ring text -- see [`PieLabels`] (shared with [`super::pie`]:
    /// same enum, same "shorter than the ring count skips the rest" `List`
    /// behavior). Unlike Pie's centroid placement, a ring's text is set
    /// *along* the ring (an SVG `<textPath>` on the ring's own centerline)
    /// starting just inside the arc's start -- shadcn's `insideStart` -- so
    /// concentric rings' labels never collide the way centroid labels did
    /// (every ring's centroid sits at a different angle but, for similar
    /// values, nearly the same bearing). Text longer than the arc is
    /// clipped at the arc's end.
    pub labels: PieLabels,
    /// `false` (the default): one ring per datum, each an independent
    /// share of `ctx.data`'s own max value (`chart-radial-simple.tsx`'s
    /// own shape). `true`: every configured series' value for the FIRST
    /// datum stacks cumulatively into one ring instead
    /// (`chart-radial-stacked.tsx`) -- meaningful only when at least two
    /// series are configured; with one, stacking one value alone is the
    /// same ring `false` would draw for that one datum.
    pub stacked: bool,
    /// Text centered in the hole: `(primary, secondary)`, e.g. `("1,999",
    /// "Visitors")` -- matches shadcn's `chart-radial-text.tsx`. Meaningful
    /// only when `inner_radius > 0.0`; not itself part of shadcn's own
    /// `RadialBar`-specific prop surface (its own `<PolarRadiusAxis>`
    /// `Label` render-prop plays this role upstream), but the exact same
    /// mechanism [`super::pie::PieOptions::center_text`] already gives
    /// Pie, reused here rather than invented twice.
    pub center_text: Option<(String, String)>,
}

impl Default for RadialOptions {
    fn default() -> Self {
        Self {
            inner_radius: 0.0,
            outer_radius: 0.0,
            start_angle: 0.0,
            end_angle: std::f64::consts::TAU,
            corner_radius: 0.0,
            grid: false,
            labels: PieLabels::None,
            stacked: false,
            center_text: None,
        }
    }
}

/// A radial chart's own plot geometry: center, innermost ring's inner radius
/// and outermost ring's outer radius. Shared by [`render`] and [`anchors`].
fn geometry(ctx: &SeriesRenderContext, opts: &RadialOptions) -> (f64, f64, f64, f64) {
    let cx = ctx.width / 2.0;
    let cy = ctx.height / 2.0;
    let auto_outer = (ctx.width.min(ctx.height) / 2.0) * OUTER_RADIUS_FRACTION;
    let outer_radius = if opts.outer_radius > 0.0 {
        opts.outer_radius
    } else {
        auto_outer
    };
    let inner_radius = opts
        .inner_radius
        .max(0.0)
        .min((outer_radius - 1.0).max(0.0));
    (cx, cy, inner_radius, outer_radius)
}

/// Each datum's keyboard tooltip anchor, in the chart's logical SVG units:
/// the centroid of its ring's value arc (the stacked layout has one ring,
/// so one anchor, at the middle of the whole sweep). A pointer hover
/// instead follows the pointer (`Follow::Pointer`): a ring can sweep most
/// of a turn, so its centroid may sit on the far side of the chart from the
/// cursor, where a tooltip would read as unrelated to what is hovered.
pub(crate) fn anchors(ctx: &SeriesRenderContext, opts: &RadialOptions) -> Vec<(f64, f64)> {
    let (cx, cy, inner_radius, outer_radius) = geometry(ctx, opts);
    if opts.stacked {
        let (x, y) = centroid(inner_radius, outer_radius, opts.start_angle, opts.end_angle);
        return vec![(cx + x, cy + y)];
    }
    let n = ctx.data.len().max(1);
    let band = BandScale {
        count: n,
        range: (inner_radius, outer_radius),
        padding: RING_PADDING,
    };
    let values: Vec<f64> = ctx
        .data
        .iter()
        .map(|d| d.values.first().copied().flatten().unwrap_or(0.0))
        .collect();
    let max_value = values
        .iter()
        .copied()
        .fold(0.0_f64, f64::max)
        .max(f64::EPSILON);
    let scale = angle_scale((0.0, max_value), opts.start_angle, opts.end_angle);
    values
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let (ring_inner, ring_width) = band.band(i);
            let (x, y) = centroid(
                ring_inner,
                ring_inner + ring_width,
                opts.start_angle,
                scale.scale(*v),
            );
            (cx + x, cy + y)
        })
        .collect()
}

/// The hit regions the pointer is resolved against (see
/// `components::pointer`): each ring's own value arc, or -- stacked -- the
/// one ring's whole sweep. The empty angular remainder of a ring (and the
/// gaps between rings) is outside every sector, so the tooltip shows only
/// over a bar, as in shadcn.
pub(crate) fn sectors(ctx: &SeriesRenderContext, opts: &RadialOptions) -> Vec<Sector> {
    let (_, _, inner_radius, outer_radius) = geometry(ctx, opts);
    if opts.stacked {
        return vec![Sector {
            index: 0,
            r0: inner_radius,
            r1: outer_radius,
            a0: opts.start_angle,
            a1: opts.end_angle,
        }];
    }
    let n = ctx.data.len().max(1);
    let band = BandScale {
        count: n,
        range: (inner_radius, outer_radius),
        padding: RING_PADDING,
    };
    let values: Vec<f64> = ctx
        .data
        .iter()
        .map(|d| d.values.first().copied().flatten().unwrap_or(0.0))
        .collect();
    let max_value = values
        .iter()
        .copied()
        .fold(0.0_f64, f64::max)
        .max(f64::EPSILON);
    let scale = angle_scale((0.0, max_value), opts.start_angle, opts.end_angle);
    values
        .iter()
        .enumerate()
        .map(|(index, v)| {
            let (ring_inner, ring_width) = band.band(index);
            Sector {
                index,
                r0: ring_inner,
                r1: ring_inner + ring_width,
                a0: opts.start_angle,
                a1: scale.scale(*v),
            }
        })
        .collect()
}

/// Render one ring per datum (the default), or every configured series'
/// value for the first datum stacked into one ring
/// ([`RadialOptions::stacked`]), plus the optional center text.
pub(crate) fn render(ctx: &SeriesRenderContext, opts: &RadialOptions) -> Element {
    let chart = use_chart();
    let chart_id = (chart.id)();
    let hover_active = ctx.active_index;

    let (cx, cy, inner_radius, outer_radius) = geometry(ctx, opts);

    let translate = format!("translate({}, {})", fmt_num(cx), fmt_num(cy));

    rsx! {
        g { "data-slot": "chart-series",
            g { transform: "{translate}",
                if opts.stacked {
                    {render_stacked_ring(ctx, opts, &chart_id, inner_radius, outer_radius, hover_active)}
                } else {
                    {render_rings(ctx, opts, &chart_id, inner_radius, outer_radius, hover_active)}
                }
            }
            if let Some((primary, secondary)) = &opts.center_text {
                {center_text::render(cx, cy, inner_radius, primary, secondary)}
            }
        }
    }
}

/// One ring per datum -- `RadialOptions::stacked == false`, this family's
/// own default. Each ring is its own concentric band
/// ([`BandScale`] over `inner_radius..outer_radius`, one band per datum,
/// innermost = index `0`); its own value maps to an angular span via
/// [`angle_scale`] over `(0.0, max_value)`, so the largest datum's own
/// ring always reaches `end_angle` exactly and every other ring's own
/// share is proportionally shorter.
fn render_rings(
    ctx: &SeriesRenderContext,
    opts: &RadialOptions,
    chart_id: &str,
    inner_radius: f64,
    outer_radius: f64,
    hover_active: Option<usize>,
) -> Element {
    let n = ctx.data.len().max(1);
    let band = BandScale {
        count: n,
        range: (inner_radius, outer_radius),
        padding: RING_PADDING,
    };
    let values: Vec<f64> = ctx
        .data
        .iter()
        .map(|d| d.values.first().copied().flatten().unwrap_or(0.0))
        .collect();
    let max_value = values
        .iter()
        .copied()
        .fold(0.0_f64, f64::max)
        .max(f64::EPSILON);
    let scale = angle_scale((0.0, max_value), opts.start_angle, opts.end_angle);

    rsx! {
        for (i , datum) in ctx.data.iter().enumerate() {
            {
                let (ring_inner, ring_width) = band.band(i);
                let ring_outer = ring_inner + ring_width;
                let value = values[i];
                let start = opts.start_angle;
                let end = scale.scale(value);
                let color = datum
                    .color
                    .clone()
                    .unwrap_or_else(|| format!("var(--dx-chart-{})", (i % 8) + 1));
                let is_active = hover_active == Some(i);
                let label_text = label_text(&opts.labels, i, datum, value, max_value);
                let label = label_text.map(|text| ArcLabel {
                    id: format!("{}-radial-label-{i}", chart_id),
                    text,
                    radius: (ring_inner + ring_outer) / 2.0,
                    thickness: ring_width,
                    start,
                    end,
                    text_scale: ctx.text_scale,
                });
                rsx! {
                    g { key: "{i}",
                        if opts.grid {
                            path {
                                "data-slot": "chart-polar-grid",
                                d: "{arc_path(ring_inner, ring_outer, opts.start_angle, opts.end_angle, 0.0, 0.0)}",
                            }
                        }
                        path {
                            "data-slot": "chart-arc",
                            "data-index": "{i}",
                            "data-active": is_active.then_some("true"),
                            "data-start-angle": "{fmt_num(start)}",
                            "data-end-angle": "{fmt_num(end)}",
                            d: "{arc_path(ring_inner, ring_outer, start, end, 0.0, opts.corner_radius)}",
                            fill: "{color}",
                        }
                        if let Some(label) = label {
                            {label.render()}
                        }
                    }
                }
            }
        }
    }
}

/// Every configured series' own value for the FIRST datum, stacked
/// cumulatively into one ring -- `RadialOptions::stacked == true`
/// (`chart-radial-stacked.tsx`). Series `0`'s own share starts at
/// `start_angle`; series `1` continues exactly where series `0` ended, and
/// so on -- contiguous by construction (each span's own start is the
/// previous span's own end), never independently positioned.
fn render_stacked_ring(
    ctx: &SeriesRenderContext,
    opts: &RadialOptions,
    chart_id: &str,
    inner_radius: f64,
    outer_radius: f64,
    hover_active: Option<usize>,
) -> Element {
    let datum = ctx.data.first();
    let n_series = ctx.config.series.len().max(1);
    let values: Vec<f64> = (0..n_series)
        .map(|s| {
            datum
                .and_then(|d| d.values.get(s).copied().flatten())
                .unwrap_or(0.0)
        })
        .collect();
    let total: f64 = values.iter().sum::<f64>().max(f64::EPSILON);
    let span = opts.end_angle - opts.start_angle;

    let mut cursor = opts.start_angle;
    let spans: Vec<(f64, f64)> = values
        .iter()
        .map(|v| {
            let start = cursor;
            let end = cursor + span * (v / total);
            cursor = end;
            (start, end)
        })
        .collect();

    let is_active = hover_active == Some(0);

    rsx! {
        if opts.grid {
            path {
                "data-slot": "chart-polar-grid",
                d: "{arc_path(inner_radius, outer_radius, opts.start_angle, opts.end_angle, 0.0, 0.0)}",
            }
        }
        for (s , series) in ctx.config.series.iter().enumerate() {
            {
                let (start, end) = spans.get(s).copied().unwrap_or((opts.start_angle, opts.start_angle));
                let slot = series.slot();
                let color = format!("var(--color-{slot})");
                let value = values.get(s).copied().unwrap_or(0.0);
                let label_text = label_text(&opts.labels, s, datum.unwrap_or(&EMPTY_DATUM), value, total);
                let label = label_text.map(|text| ArcLabel {
                    id: format!("{}-radial-label-{s}", chart_id),
                    text,
                    radius: (inner_radius + outer_radius) / 2.0,
                    thickness: outer_radius - inner_radius,
                    start,
                    end,
                    text_scale: ctx.text_scale,
                });
                rsx! {
                    path {
                        key: "{series.key}",
                        "data-slot": "chart-arc",
                        "data-index": "0",
                        "data-series": "{slot}",
                        "data-active": is_active.then_some("true"),
                        "data-start-angle": "{fmt_num(start)}",
                        "data-end-angle": "{fmt_num(end)}",
                        d: "{arc_path(inner_radius, outer_radius, start, end, 0.0, opts.corner_radius)}",
                        fill: "{color}",
                    }
                    if let Some(label) = label {
                        {label.render()}
                    }
                }
            }
        }
    }
}

/// How far (arc length, logical SVG units, at text scale 1) an arc's label
/// starts inside the arc's own start edge.
const LABEL_INSET: f64 = 8.0;
/// The angular length (radians, ~0.95 turn) of every label's centerline path,
/// whatever the arc's own sweep -- see [`ArcLabel::centerline`]. Just under a
/// full turn so the path never closes onto its own start.
const LABEL_SPAN: f64 = std::f64::consts::TAU * 0.95;
/// The largest label font size, as a fraction of the ring's radial
/// thickness: cap height is ~0.7em, so 0.85em of text fits a ring band with a
/// little air on each side.
const LABEL_MAX_FONT_FRACTION: f64 = 0.85;
/// A label's own half-height as a fraction of its font size: the offset
/// (`dy`) that centers the glyphs on the ring's centerline rather than
/// resting their baseline on it.
const LABEL_HALF_HEIGHT_EM: f64 = 0.35;

/// One ring/segment's label, set along the arc's own centerline starting
/// just inside its start (shadcn's `insideStart`) -- see
/// [`RadialOptions::labels`].
struct ArcLabel {
    /// Unique (per chart) id for the centerline `<path>` the text follows.
    id: String,
    text: String,
    /// The ring's centerline radius.
    radius: f64,
    /// The ring's radial thickness (logical units): the label's font size is
    /// capped to a fraction of it ([`LABEL_MAX_FONT_FRACTION`]) so text on a
    /// thin ring (many rings, or a narrow container) never spills across its
    /// neighbours.
    thickness: f64,
    start: f64,
    end: f64,
    /// The chart's text compensation scale (`SeriesRenderContext::text_scale`):
    /// the label's own inset is a text-sized length, so it grows with it.
    text_scale: f64,
}

impl ArcLabel {
    /// The label's centerline path plus its `<textPath>` text. Reading
    /// direction is chosen so the text is upright: along the arc's own
    /// direction when it starts in the upper half of the chart, against it
    /// (anchored at the arc-start end) when it starts in the lower half --
    /// otherwise text there would render upside down.
    ///
    /// `direction="ltr"` on the `<text>` is load-bearing: under an `rtl`
    /// ancestor (`dir="rtl"`, inherited by SVG text) `text-anchor: start`
    /// means the *right* end, so the glyphs would lay out backwards off the
    /// start of the path and vanish. The label's own reading order is set
    /// by the path, never by the page direction.
    fn render(&self) -> Element {
        let (path_d, anchor, offset) = self.centerline();
        let id = safe_fragment_id(&self.id);
        let href = format!("#{id}");
        let text = self.text.clone();
        rsx! {
            defs {
                path { id: "{id}", d: "{path_d}", fill: "none" }
            }
            text {
                "data-slot": "chart-arc-label",
                direction: "ltr",
                style: "{self.font_size_style()}",
                text_anchor: "{anchor}",
                dy: "{LABEL_HALF_HEIGHT_EM}em",
                textPath { "href": "{href}", "startOffset": "{offset}", "{text}" }
            }
        }
    }

    /// The inline `font-size` for this label: the themed size (the
    /// stylesheet's `--dx-text-xs` x `--dx-chart-text-scale`, repeated here
    /// because an inline declaration replaces the stylesheet's) capped at
    /// [`LABEL_MAX_FONT_FRACTION`] of the ring's thickness. The cap is a
    /// logical-unit length (user units = `px` in SVG), so it scales with the
    /// drawing like the ring itself does.
    fn font_size_style(&self) -> String {
        let cap = (self.thickness * LABEL_MAX_FONT_FRACTION).max(0.0);
        format!(
            "font-size: min(calc(var(--dx-text-xs) * var(--dx-chart-text-scale, 1)), {}px)",
            fmt_num(cap)
        )
    }

    /// `(path d, text-anchor, startOffset)` for this label's centerline.
    ///
    /// The path always starts (just inside the arc start) at the arc's start
    /// and runs on for the fixed [`LABEL_SPAN`] -- never to the arc's own
    /// end. A zero-sweep (value `0`) ring would otherwise get a zero-length
    /// path and lose its label, and a short arc would clip its text to the
    /// arc's length; this way the text is always laid out in full and simply
    /// continues onto the (unfilled) track past a short arc.
    fn centerline(&self) -> (String, &'static str, &'static str) {
        let sweep = self.end - self.start;
        let sign = if sweep < 0.0 { -1.0 } else { 1.0 };
        let inset = (LABEL_INSET * self.text_scale / self.radius.max(1.0)).min(sweep.abs() / 2.0);
        let from = self.start + sign * inset;
        let far = from + sign * LABEL_SPAN;
        // Angle `0.0` is twelve o'clock, increasing clockwise: the upper
        // half is where clockwise travel reads left to right.
        let upper_half = from.cos() >= 0.0;
        let natural_is_clockwise = sign > 0.0;
        if natural_is_clockwise == upper_half {
            (centerline_path(self.radius, from, far), "start", "0%")
        } else {
            (centerline_path(self.radius, far, from), "end", "100%")
        }
    }
}

/// An SVG path `d` along the circle of `radius` (centered at the origin)
/// from angle `a0` to `a1` (same convention as [`arc_path`]: `0.0` is twelve
/// o'clock, increasing clockwise; `a1 < a0` runs counter-clockwise), split
/// into quarter-turn `A` segments so no single arc is ambiguous or
/// degenerate for a near-full or over-full sweep.
fn centerline_path(radius: f64, a0: f64, a1: f64) -> String {
    let point = |a: f64| (radius * a.sin(), -radius * a.cos());
    let delta = a1 - a0;
    let segments = ((delta.abs() / std::f64::consts::FRAC_PI_2).ceil() as usize).max(1);
    let sweep_flag = if delta >= 0.0 { 1 } else { 0 };
    let (x, y) = point(a0);
    let mut d = format!("M{} {}", fmt_num(x), fmt_num(y));
    for k in 1..=segments {
        let (x, y) = point(a0 + delta * k as f64 / segments as f64);
        d.push_str(&format!(
            "A{r} {r} 0 0 {sweep_flag} {} {}",
            fmt_num(x),
            fmt_num(y),
            r = fmt_num(radius)
        ));
    }
    d
}

/// A `ChartDatum::default()`-shaped placeholder, used only when a stacked
/// ring's own first datum is missing entirely (an empty `data` slice) --
/// `label_text`'s own `PieLabels::List` arm reads a series' index into it,
/// which an absent datum has no real value for anyway.
const EMPTY_DATUM: ChartDatum = ChartDatum {
    label: String::new(),
    values: Vec::new(),
    color: None,
};

/// Shared with [`super::pie`]'s own identically-shaped helper -- not
/// literally reused (each family's own `SliceParams`-equivalent bundles
/// different fields), but the same four `PieLabels` arms, the same
/// two-decimal `fmt_decimal` convention, and the same "index past the end
/// of `List`'s own `Vec` renders nothing" defensive default.
fn label_text(
    labels: &PieLabels,
    index: usize,
    datum: &ChartDatum,
    value: f64,
    total: f64,
) -> Option<String> {
    match labels {
        PieLabels::None => None,
        PieLabels::Value => Some(fmt_decimal(value, 2)),
        PieLabels::Percent => {
            let pct = if total > 0.0 {
                value / total * 100.0
            } else {
                0.0
            };
            Some(format!("{}%", fmt_decimal(pct, 1)))
        }
        PieLabels::List(items) => items.get(index).filter(|s| !s.is_empty()).cloned(),
    }
    .or_else(|| {
        // `datum` is otherwise unused when `labels` is `None`/`Value`/
        // `Percent` -- referencing it here (a no-op `None` fallback) keeps
        // every arm using the same function signature every call site
        // shares, rather than a datum-less signature for two arms and a
        // datum-needing one for `List` alone.
        let _ = datum;
        None
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart::components::chart::Chart;
    use crate::chart::{ChartConfig, ChartContainer, ChartKind};
    use dioxus_core::NoOpMutations;

    fn sample_config() -> ChartConfig {
        ChartConfig::new()
            .series("desktop", "Desktop", "var(--dx-chart-1)")
            .series("mobile", "Mobile", "var(--dx-chart-2)")
    }

    fn one_series_data() -> Vec<ChartDatum> {
        vec![
            ChartDatum {
                label: "Chrome".to_string(),
                values: vec![Some(275.0)],
                color: Some("var(--dx-chart-1)".to_string()),
            },
            ChartDatum {
                label: "Safari".to_string(),
                values: vec![Some(200.0)],
                color: Some("var(--dx-chart-2)".to_string()),
            },
            ChartDatum {
                label: "Firefox".to_string(),
                values: vec![Some(100.0)],
                color: Some("var(--dx-chart-3)".to_string()),
            },
        ]
    }

    fn one_series_config() -> ChartConfig {
        ChartConfig::new().series("visitors", "Visitors", "var(--dx-chart-1)")
    }

    fn render(
        kind: ChartKind,
        config: ChartConfig,
        data: Vec<ChartDatum>,
        opts: RadialOptions,
    ) -> String {
        render_dir(kind, config, data, opts, None)
    }

    fn render_dir(
        kind: ChartKind,
        config: ChartConfig,
        data: Vec<ChartDatum>,
        opts: RadialOptions,
        dir: Option<crate::direction::Direction>,
    ) -> String {
        #[derive(Clone, PartialEq, Props)]
        struct HarnessProps {
            kind: ChartKind,
            config: ChartConfig,
            data: Vec<ChartDatum>,
            opts: RadialOptions,
            dir: Option<crate::direction::Direction>,
        }
        #[component]
        fn Harness(props: HarnessProps) -> Element {
            let config = use_signal(|| props.config.clone());
            let data = use_signal(|| props.data.clone());
            rsx! {
                ChartContainer { config, data, kind: props.kind,
                    Chart {
                        width: 300.0,
                        height: 300.0,
                        aria_label: "Test",
                        radial: props.opts.clone(),
                        dir: props.dir,
                    }
                }
            }
        }
        let mut dom = VirtualDom::new_with_props(
            Harness,
            HarnessProps {
                kind,
                config,
                data,
                opts,
                dir,
            },
        );
        dom.rebuild_in_place();
        dom.render_immediate(&mut NoOpMutations);
        dioxus_ssr::render(&dom)
    }

    #[test]
    fn default_options_are_zero_hole_full_turn_unstacked() {
        let opts = RadialOptions::default();
        assert_eq!(opts.inner_radius, 0.0);
        assert_eq!(opts.outer_radius, 0.0);
        assert_eq!(opts.start_angle, 0.0);
        assert_eq!(opts.end_angle, std::f64::consts::TAU);
        assert!(!opts.stacked);
        assert!(!opts.grid);
        assert_eq!(opts.labels, PieLabels::None);
        assert!(opts.center_text.is_none());
    }

    #[test]
    fn unstacked_renders_one_ring_per_datum_each_its_own_data_index() {
        let html = render(
            ChartKind::RadialBar,
            one_series_config(),
            one_series_data(),
            RadialOptions::default(),
        );
        assert_eq!(html.matches(r#"data-slot="chart-arc""#).count(), 3);
        for i in 0..3 {
            assert!(html.contains(&format!(r#"data-index="{i}""#)), "{html}");
        }
    }

    /// Pulls one `f64`-valued attribute (e.g. `data-end-angle`) out of a
    /// single `<path ...>` tag slice -- forward-only (the attribute always
    /// appears somewhere AFTER the tag's own opening `<path`, never
    /// before), avoiding the backward-`rfind`-into-a-fixed-size-window
    /// bug an earlier version of these two tests had (it searched BEFORE
    /// a later attribute's own match, which only works when the wanted
    /// attribute happens to sort earlier in the tag than the anchor -- it
    /// doesn't here, `data-series` comes before `data-end-angle`).
    fn attr_f64(tag: &str, name: &str) -> f64 {
        let needle = format!(r#"{name}=""#);
        let start = tag
            .find(&needle)
            .unwrap_or_else(|| panic!("no {name} in {tag}"))
            + needle.len();
        tag[start..]
            .chars()
            .take_while(|c| *c != '"')
            .collect::<String>()
            .parse()
            .unwrap()
    }

    #[test]
    fn unstacked_largest_value_reaches_end_angle_exactly() {
        let html = render(
            ChartKind::RadialBar,
            one_series_config(),
            one_series_data(),
            RadialOptions::default(),
        );
        // Chrome (275.0) is the max of {275, 200, 100} -> its own
        // `data-end-angle` is exactly `end_angle` (TAU by default),
        // `fmt_num`'s own rounding aside (hence the coarser tolerance than
        // an exact-arithmetic assertion would use).
        let tag_start = html.find(r#"data-index="0""#).unwrap();
        let tag_end = html[tag_start..]
            .find("</path>")
            .map(|i| tag_start + i)
            .unwrap_or(html.len());
        let end = attr_f64(&html[tag_start..tag_end], "data-end-angle");
        assert!(
            (end - std::f64::consts::TAU).abs() < 1e-2,
            "expected TAU, got {end}: {html}"
        );
    }

    #[test]
    fn stacked_produces_contiguous_spans_summing_to_the_full_range() {
        let data = vec![ChartDatum {
            label: "January".to_string(),
            values: vec![Some(186.0), Some(80.0)],
            ..Default::default()
        }];
        let html = render(
            ChartKind::RadialBar,
            sample_config(),
            data,
            RadialOptions {
                stacked: true,
                ..Default::default()
            },
        );
        assert_eq!(html.matches(r#"data-slot="chart-arc""#).count(), 2);
        assert_eq!(html.matches(r#"data-index="0""#).count(), 2);
        assert!(html.contains(r#"data-series="desktop""#));
        assert!(html.contains(r#"data-series="mobile""#));

        // The two spans are contiguous: series 0's own end angle equals
        // series 1's own start angle. Each `<path>` is self-closing on one
        // line, so slicing from its own `data-series="..."` match to the
        // next `</path>`/`>` safely isolates just that one tag's own
        // attributes.
        let desktop_tag_start = html.find(r#"data-series="desktop""#).unwrap();
        let desktop_tag_end = html[desktop_tag_start..]
            .find('>')
            .map(|i| desktop_tag_start + i)
            .unwrap();
        let desktop_end = attr_f64(&html[desktop_tag_start..desktop_tag_end], "data-end-angle");

        let mobile_tag_start = html.find(r#"data-series="mobile""#).unwrap();
        let mobile_tag_end = html[mobile_tag_start..]
            .find('>')
            .map(|i| mobile_tag_start + i)
            .unwrap();
        let mobile_start = attr_f64(&html[mobile_tag_start..mobile_tag_end], "data-start-angle");

        assert!(
            (desktop_end - mobile_start).abs() < 1e-2,
            "expected contiguous spans: {html}"
        );
    }

    #[test]
    fn grid_renders_a_background_track_per_ring() {
        let opts = RadialOptions {
            grid: true,
            ..Default::default()
        };
        let html = render(
            ChartKind::RadialBar,
            one_series_config(),
            one_series_data(),
            opts,
        );
        assert_eq!(html.matches(r#"data-slot="chart-polar-grid""#).count(), 3);
    }

    #[test]
    fn stacked_grid_renders_exactly_one_background_track() {
        let data = vec![ChartDatum {
            label: "January".to_string(),
            values: vec![Some(186.0), Some(80.0)],
            ..Default::default()
        }];
        let opts = RadialOptions {
            stacked: true,
            grid: true,
            ..Default::default()
        };
        let html = render(ChartKind::RadialBar, sample_config(), data, opts);
        assert_eq!(html.matches(r#"data-slot="chart-polar-grid""#).count(), 1);
    }

    #[test]
    fn value_labels_render_one_per_ring() {
        let opts = RadialOptions {
            labels: PieLabels::Value,
            ..Default::default()
        };
        let html = render(
            ChartKind::RadialBar,
            one_series_config(),
            one_series_data(),
            opts,
        );
        assert_eq!(html.matches(r#"data-slot="chart-arc-label""#).count(), 3);
    }

    #[test]
    fn labels_follow_each_rings_centerline_from_the_arc_start() {
        let opts = RadialOptions {
            inner_radius: 30.0,
            labels: PieLabels::List(vec![
                "Chrome".to_string(),
                "Safari".to_string(),
                "Firefox".to_string(),
            ]),
            ..Default::default()
        };
        let html = render(
            ChartKind::RadialBar,
            one_series_config(),
            one_series_data(),
            opts,
        );
        // One centerline path and one textPath per ring, each text pointing
        // at its own ring's path (ids are unique per ring).
        assert_eq!(html.matches("<textPath").count(), 3, "{html}");
        for i in 0..3 {
            let id_marker = format!("-radial-label-{i}\"");
            assert!(html.contains(&id_marker), "ring {i}: {html}");
        }
        // Starts at the arc's start (twelve o'clock for the default
        // `start_angle`), reading clockwise from the left end: no centroid
        // `x`/`y`, and anchored at the start of the text.
        assert!(html.contains(r#"text-anchor="start""#), "{html}");
        assert!(!html.contains(r#"text-anchor="middle""#), "{html}");
        assert!(html.contains(r#"startOffset="0%""#), "{html}");
    }

    #[test]
    fn centerline_starts_just_past_the_arc_start_at_the_ring_radius() {
        let label = ArcLabel {
            id: "x".to_string(),
            text: "Chrome".to_string(),
            radius: 100.0,
            start: 0.0,
            end: std::f64::consts::PI,
            thickness: 40.0,
            text_scale: 1.0,
        };
        let (d, anchor, offset) = label.centerline();
        assert_eq!((anchor, offset), ("start", "0%"));
        // Twelve o'clock is (0, -r); the label starts `LABEL_INSET` (8)
        // along the arc, i.e. 0.08 rad clockwise: x = 100 sin(0.08) = 7.991.
        assert!(d.starts_with("M7.991 -99.68"), "{d}");
        // Clockwise (sweep flag 1), split into quarter-turn segments.
        assert!(d.contains("A100 100 0 0 1"), "{d}");
        // The fixed ~0.95-turn span, not the arc's own half-turn sweep.
        assert_eq!(d.matches('A').count(), 4, "{d}");
    }

    #[test]
    fn labels_on_a_lower_half_start_read_upright_by_reversing_direction() {
        use std::f64::consts::PI;
        // An arc starting at six o'clock: clockwise travel there runs right
        // to left (upside-down text), so the path is reversed (drawn from
        // the arc's end back to the label start) and the text is anchored at
        // the label start end of it.
        let label = ArcLabel {
            id: "x".to_string(),
            text: "Safari".to_string(),
            radius: 100.0,
            start: PI,
            end: PI * 1.5,
            thickness: 40.0,
            text_scale: 1.0,
        };
        let (d, anchor, offset) = label.centerline();
        assert_eq!((anchor, offset), ("end", "100%"));
        // Reversed => counter-clockwise (sweep flag 0), ending at the label
        // start just inside the arc start (six o'clock plus 0.08 rad:
        // x = -100 sin(0.08), y = 100 cos(0.08)).
        assert!(d.contains("A100 100 0 0 0"), "{d}");
        assert!(d.ends_with("-7.991 99.68"), "{d}");
    }

    #[test]
    fn centerline_handles_counter_clockwise_and_over_full_sweeps() {
        let full = centerline_path(50.0, 0.0, std::f64::consts::TAU * 1.05);
        assert_eq!(full.matches('A').count(), 5, "{full}");
        let ccw = centerline_path(50.0, 1.0, 0.0);
        assert!(ccw.contains("A50 50 0 0 0"), "{ccw}");
    }

    /// First and last points of a `centerline_path`-shaped `d`.
    fn path_ends(d: &str) -> ((f64, f64), (f64, f64)) {
        let nums = |t: &str| -> (f64, f64) {
            let v: Vec<f64> = t
                .split([' ', 'M', 'A'])
                .filter(|p| !p.is_empty())
                .map(|p| p.parse().unwrap())
                .collect();
            (v[v.len() - 2], v[v.len() - 1])
        };
        let first = d[..d[1..].find('A').unwrap() + 1].to_string();
        let last = d[d.rfind('A').unwrap()..].to_string();
        (nums(&first), nums(&last))
    }

    fn chord(d: &str) -> f64 {
        let ((x0, y0), (x1, y1)) = path_ends(d);
        ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt()
    }

    #[test]
    fn a_zero_sweep_arc_still_gets_a_label_path_with_length() {
        let label = ArcLabel {
            id: "x".to_string(),
            text: "0".to_string(),
            radius: 40.0,
            start: 0.0,
            end: 0.0,
            thickness: 40.0,
            text_scale: 1.0,
        };
        let (d, anchor, offset) = label.centerline();
        assert!(d.starts_with('M') && d.contains('A'), "{d}");
        assert!(!d.contains("NaN"), "{d}");
        // Chord of a 0.95-turn arc of radius 40: 2 * 40 * sin(0.95 pi) ~= 12.5.
        assert!(chord(&d) > 10.0, "zero-length label path: {d}");
        assert_eq!((anchor, offset), ("start", "0%"));
    }

    #[test]
    fn a_short_arc_label_path_is_not_clipped_to_the_arc() {
        let short = ArcLabel {
            id: "x".to_string(),
            text: "Chrome 275".to_string(),
            radius: 100.0,
            start: 0.0,
            end: 0.3,
            thickness: 40.0,
            text_scale: 1.0,
        };
        let long = ArcLabel {
            end: 5.0,
            id: "y".to_string(),
            text: "Chrome 275".to_string(),
            radius: 100.0,
            start: 0.0,
            thickness: 40.0,
            text_scale: 1.0,
        };
        // Same start, same fixed span: the path length does not depend on
        // the arc's end.
        assert_eq!(short.centerline().0.matches('A').count(), 4);
        assert_eq!(long.centerline().0.matches('A').count(), 4);
        assert!(chord(&short.centerline().0) > 25.0);
    }

    #[test]
    fn label_inset_grows_with_the_text_scale() {
        let at = |scale: f64| {
            ArcLabel {
                id: "x".to_string(),
                text: "a".to_string(),
                radius: 100.0,
                start: 0.0,
                end: PI_,
                thickness: 40.0,
                text_scale: scale,
            }
            .centerline()
            .0
        };
        const PI_: f64 = std::f64::consts::PI;
        // Inset 8 at s=1 (x = 100 sin 0.08 = 7.991), 16 at s=2 (sin 0.16).
        assert!(at(1.0).starts_with("M7.991 "), "{}", at(1.0));
        assert!(at(2.0).starts_with("M15.932 "), "{}", at(2.0));
    }

    #[test]
    fn labels_stay_left_to_right_under_an_rtl_page() {
        use crate::direction::Direction;
        let opts = RadialOptions {
            labels: PieLabels::Value,
            ..Default::default()
        };
        let rtl = render_dir(
            ChartKind::RadialBar,
            one_series_config(),
            one_series_data(),
            opts.clone(),
            Some(Direction::Rtl),
        );
        // Under `dir="rtl"` text-anchor `start` would otherwise mean the right
        // end and the glyphs would lay out backwards off the path start.
        assert!(rtl.contains(r#"dir="rtl""#), "{rtl}");
        let labels = rtl.matches(r#"data-slot="chart-arc-label""#).count();
        assert_eq!(labels, 3, "{rtl}");
        assert_eq!(
            rtl.matches(r#"data-slot="chart-arc-label" direction="ltr""#)
                .count(),
            labels,
            "every arc label must pin direction=ltr: {rtl}"
        );
    }

    #[test]
    fn a_percent_in_the_chart_id_never_reaches_the_href_fragment() {
        let label = ArcLabel {
            id: "chart%1-radial-label-0".to_string(),
            text: "x".to_string(),
            radius: 40.0,
            start: 0.0,
            end: 1.0,
            thickness: 40.0,
            text_scale: 1.0,
        };
        let html = dioxus_ssr::render_element(label.render());
        assert!(!html.contains("chart%"), "{html}");
        assert!(
            html.contains(r##"href="#chart_25_1-radial-label-0""##),
            "{html}"
        );
        assert!(html.contains(r#"id="chart_25_1-radial-label-0""#), "{html}");
    }

    #[test]
    fn list_labels_use_the_callers_own_text() {
        let opts = RadialOptions {
            labels: PieLabels::List(vec![
                "Chrome".to_string(),
                "Safari".to_string(),
                "Firefox".to_string(),
            ]),
            ..Default::default()
        };
        let html = render(
            ChartKind::RadialBar,
            one_series_config(),
            one_series_data(),
            opts,
        );
        assert!(html.contains(">Chrome<"));
        assert!(html.contains(">Safari<"));
        assert!(html.contains(">Firefox<"));
    }

    #[test]
    fn center_text_renders_both_lines() {
        let opts = RadialOptions {
            inner_radius: 30.0,
            center_text: Some(("1,999".to_string(), "Visitors".to_string())),
            ..Default::default()
        };
        let html = render(
            ChartKind::RadialBar,
            one_series_config(),
            one_series_data(),
            opts,
        );
        assert!(html.contains(r#"data-slot="chart-pie-center-text""#));
        assert!(html.contains("1,999"));
        assert!(html.contains("Visitors"));
    }

    #[test]
    fn corner_radius_adds_fillet_arc_commands() {
        let sharp = render(
            ChartKind::RadialBar,
            one_series_config(),
            one_series_data(),
            RadialOptions::default(),
        );
        let rounded = render(
            ChartKind::RadialBar,
            one_series_config(),
            one_series_data(),
            RadialOptions {
                corner_radius: 8.0,
                ..Default::default()
            },
        );
        let sharp_as = sharp.matches('A').count();
        let rounded_as = rounded.matches('A').count();
        assert!(
            rounded_as > sharp_as,
            "sharp={sharp_as} rounded={rounded_as}"
        );
    }

    #[test]
    fn does_not_panic_with_empty_data() {
        let html = render(
            ChartKind::RadialBar,
            one_series_config(),
            Vec::new(),
            RadialOptions::default(),
        );
        assert_eq!(html.matches(r#"data-slot="chart-arc""#).count(), 0);
    }

    #[test]
    fn label_font_size_is_capped_by_the_ring_thickness() {
        let mk = |thickness| ArcLabel {
            id: "x".to_string(),
            text: "Chrome".to_string(),
            radius: 100.0,
            start: 0.0,
            end: 1.0,
            thickness,
            text_scale: 1.0,
        };
        // 0.85 x 10 = 8.5 logical units: below the themed 12px.
        assert!(mk(10.0).font_size_style().ends_with(", 8.5px)"));
        assert!(mk(10.0).font_size_style().contains("var(--dx-text-xs)"));
        let html = dioxus_ssr::render_element(mk(10.0).render());
        assert!(html.contains("style=\"font-size: min("), "{html}");
        // A degenerate ring never yields a negative size.
        assert!(mk(-4.0).font_size_style().ends_with(", 0px)"));
    }
}
