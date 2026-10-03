//! The [`ChartKind::Pie`](crate::chart::ChartKind::Pie) family: one ring of
//! proportional wedges. Geometry: [`crate::chart::engine::polar`]
//! (`arc_path`/`pie_layout`/`centroid` -- see that module's own doc for the
//! d3-shape provenance).
//!
//! ## Recharts semantics
//!
//! Every option means what the same-named Recharts `<Pie>` prop means, so a
//! shadcn/ui demo's numbers are copied verbatim:
//! - angles are Recharts degrees (`0` = three o'clock, counter-clockwise);
//!   the default `0 -> 360` lays the first slice out counter-clockwise from
//!   three o'clock, as every shadcn pie does;
//! - radii are [`Radius`]es -- px, or a percentage of the max radius (half
//!   the chart box's shorter side minus Recharts' 5px margin); the default
//!   outer radius is `80%` (96px in shadcn's 250px square) and the default
//!   inner radius `0`;
//! - [`PieLabels::Outside`] is Recharts' `label` (the value, 20px outside
//!   the rim, anchored away from the centre, with a `labelLine` leader);
//!   the other label kinds are `<LabelList>` text at the slice centroid.
//!
//! ## Data model: one value per datum, not per series
//!
//! A pie draws one slice **per datum** (category), sized by that datum's
//! own `values[0]` -- shadcn's `<Pie data={chartData} dataKey="visitors" />`
//! reads `visitors` off each *row*. A plain pie's
//! [`crate::chart::ChartConfig`] is therefore just one series, and every
//! [`crate::chart::ChartDatum::color`] colors that datum's slice (falling
//! back to `--dx-chart-<1..8>` by position).
//!
//! **Several series draw several concentric pies** over the same categories
//! -- shadcn's `chart-pie-stacked` is two `<Pie>`s (`desktop` a full disc of
//! outer radius 60, `mobile` a ring 70..90). [`PieOptions::rings`] gives each
//! series its own `(inner, outer)` radii, exactly those two props; without
//! it the `[inner_radius, outer_radius]` range is split into equal bands,
//! series `0` innermost.
//!
//! ## Active slice
//!
//! [`PieOptions::active_index`] is Recharts' `shape` override shadcn uses
//! for `chart-pie-donut-active`: that slice is drawn 10px further out (its
//! hole unchanged), and with [`PieOptions::active_halo`] also gets the
//! separate ring 12..25px beyond the rim `chart-pie-interactive` draws. The
//! geometry is computed here, not scaled in CSS, so the hole stays put.
//! Hover only marks the hovered slice `data-active` for the tooltip;
//! Recharts' pie has no hover growth.
//!
//! ## Load animation
//!
//! The slices sit under an SVG `<mask>` whose single stroke sweeps from
//! `start_angle` to `end_angle` (`chart-sweep-mask`); the themed stylesheet
//! animates that stroke once the chart's svg carries `data-animate="true"`
//! (set after the first measured client render), so the pie sweeps in the
//! way Recharts' does. Without the attribute -- SSR, no JS, reduced motion
//! -- the mask is fully open.

use std::f64::consts::TAU;

use dioxus::prelude::*;

use super::super::center_text::{self, CenterLayout};
use super::super::fragment::safe_fragment_id;
use super::super::layout::SeriesRenderContext;
use super::super::pointer::Sector;

use crate::chart::context::use_chart;
use crate::chart::engine::polar::{
    arc_centerline_path, arc_path, centroid, pie_layout, recharts_angle, to_recharts_angle, PieArc,
    PieSort, PolarFrame, Radius,
};
use crate::chart::engine::scale::{fmt_decimal, fmt_num};
use crate::chart::{BandScale, ChartDatum};

/// Padding (as a fraction of one ring's own step) between two concentric
/// rings of a multi-series pie without explicit [`PieOptions::rings`].
const RING_PADDING: f64 = 0.08;
/// How much further out the forced-active slice is drawn (shadcn's
/// `outerRadius + 10`).
pub(crate) const ACTIVE_GROW: f64 = 10.0;
/// The active halo ring's radii beyond the rim (`chart-pie-interactive`'s
/// `innerRadius = outerRadius + 12`, `outerRadius = outerRadius + 25`).
const HALO: (f64, f64) = (12.0, 25.0);
/// Recharts' pie label offset beyond the rim (`offsetRadius = 20`).
const LABEL_OFFSET: f64 = 20.0;

/// How [`render`] draws per-slice text.
#[derive(Clone, Debug, PartialEq, Default)]
pub enum PieLabels {
    /// No per-slice labels (the default).
    #[default]
    None,
    /// Each slice's own value, at its centroid.
    Value,
    /// Each slice's share of the total, as a percentage (one decimal
    /// place, e.g. `"29.7%"`), at its centroid.
    Percent,
    /// Caller-supplied text, one entry per datum (by position), at the
    /// slice centroid -- shadcn's `<LabelList>` (`chart-pie-label-list`).
    /// A missing entry renders no label for that slice.
    List(Vec<String>),
    /// Recharts' own `<Pie label>`: the slice's value 20px beyond the rim
    /// at the slice's mid-angle, anchored `start` on the right half and
    /// `end` on the left, with a leader line from the rim (`labelLine`)
    /// unless `line` is `false` -- shadcn's `chart-pie-label` (`line:
    /// true`) and `chart-pie-label-custom` (`labelLine={false}`).
    Outside {
        /// Draw the leader line from the rim to the label.
        line: bool,
    },
}

/// [`crate::chart::ChartKind::Pie`]'s own options -- Recharts `<Pie>` props
/// (see the module doc).
#[derive(Clone, Debug, PartialEq)]
pub struct PieOptions {
    /// `innerRadius`: [`Radius::Px(0.0)`] (the default) draws a plain pie;
    /// more draws a donut.
    pub inner_radius: Radius,
    /// `outerRadius`: default [`Radius::DEFAULT_OUTER`] (`80%`).
    pub outer_radius: Radius,
    /// `startAngle`, Recharts degrees (`0` = three o'clock). Default `0`.
    pub start_angle: f64,
    /// `endAngle`, Recharts degrees. Default `360`: a full turn,
    /// counter-clockwise.
    pub end_angle: f64,
    /// `paddingAngle`, degrees: the gap between adjacent slices.
    pub padding_angle: f64,
    /// `cornerRadius`, px -- see `engine::polar::arc_path` for this crate's
    /// simplified (documented) corner construction.
    pub corner_radius: f64,
    /// Per-slice text -- see [`PieLabels`].
    pub labels: PieLabels,
    /// Draw this slice active: 10px further out, hole unchanged (shadcn's
    /// `chart-pie-donut-active` / `chart-pie-interactive` shape override).
    pub active_index: Option<usize>,
    /// With [`Self::active_index`], also draw the active slice's halo ring
    /// 12..25px beyond the rim (`chart-pie-interactive`).
    pub active_halo: bool,
    /// Text centered in a donut's hole: `(primary, secondary)`, e.g.
    /// `("1,125", "Visitors")` -- shadcn's `chart-pie-donut-text`.
    pub center_text: Option<(String, String)>,
    /// Per-series `(inner, outer)` radii for a multi-series pie, one
    /// concentric pie per configured series (shadcn's `chart-pie-stacked`:
    /// `[(Px(0), Px(60)), (Px(70), Px(90))]`). Empty (the default) splits
    /// `inner_radius..outer_radius` into equal bands instead.
    pub rings: Vec<(Radius, Radius)>,
}

impl Default for PieOptions {
    fn default() -> Self {
        Self {
            inner_radius: Radius::Px(0.0),
            outer_radius: Radius::DEFAULT_OUTER,
            start_angle: 0.0,
            end_angle: 360.0,
            padding_angle: 0.0,
            corner_radius: 0.0,
            labels: PieLabels::None,
            active_index: None,
            active_halo: false,
            center_text: None,
            rings: Vec::new(),
        }
    }
}

/// The chart's polar frame (Recharts' offset box: the svg minus `margin`).
pub(crate) fn frame(ctx: &SeriesRenderContext) -> PolarFrame {
    PolarFrame::from_box(ctx.plot_x0, ctx.plot_y0, ctx.plot_x1, ctx.plot_y1)
}

/// One concentric pie: its radii, its values (one per datum), and -- when
/// the chart has several -- the series it draws.
struct Ring {
    inner: f64,
    outer: f64,
    values: Vec<f64>,
    series: Option<(String, String)>,
}

/// Every ring this chart draws (one unless several series are configured).
fn rings(ctx: &SeriesRenderContext, opts: &PieOptions, frame: &PolarFrame) -> Vec<Ring> {
    let max = frame.max_radius;
    let outer = opts.outer_radius.resolve(max).max(0.0);
    let inner = opts.inner_radius.resolve(max).clamp(0.0, outer);
    let n_series = ctx.config.series.len();
    if n_series <= 1 {
        return vec![Ring {
            inner,
            outer,
            values: ctx
                .data
                .iter()
                .map(|d| d.values.first().copied().flatten().unwrap_or(0.0))
                .collect(),
            series: None,
        }];
    }
    let band = BandScale {
        count: n_series,
        range: (inner, outer),
        padding: RING_PADDING,
    };
    ctx.config
        .series
        .iter()
        .enumerate()
        .map(|(s, series)| {
            let (r0, r1) = match opts.rings.get(s) {
                Some((i, o)) => {
                    let o = o.resolve(max).max(0.0);
                    (i.resolve(max).clamp(0.0, o), o)
                }
                None => {
                    let (start, width) = band.band(s);
                    (start, start + width)
                }
            };
            Ring {
                inner: r0,
                outer: r1,
                values: ctx
                    .series_values(s)
                    .into_iter()
                    .map(|v| v.unwrap_or(0.0))
                    .collect(),
                series: Some((series.key.clone(), series.slot())),
            }
        })
        .collect()
}

/// The slices of one ring, in this module's engine angles.
fn layout(opts: &PieOptions, values: &[f64]) -> Vec<PieArc> {
    pie_layout(
        values,
        opts.padding_angle.max(0.0).to_radians(),
        recharts_angle(opts.start_angle),
        recharts_angle(opts.end_angle),
        PieSort::None,
    )
}

/// Each datum's tooltip anchor, in px: the centroid of its slice in the
/// first ring -- shadcn/Recharts anchor a pie tooltip there.
pub(crate) fn anchors(ctx: &SeriesRenderContext, opts: &PieOptions) -> Vec<(f64, f64)> {
    let frame = frame(ctx);
    let Some(ring) = rings(ctx, opts, &frame).into_iter().next() else {
        return Vec::new();
    };
    layout(opts, &ring.values)
        .iter()
        .map(|arc| {
            let (x, y) = centroid(ring.inner, ring.outer, arc.start_angle, arc.end_angle);
            (frame.cx + x, frame.cy + y)
        })
        .collect()
}

/// The hit regions the pointer is resolved against (see
/// `components::pointer`): one sector per slice per ring, over the resting
/// geometry -- contiguous in angle (the pad angle is ignored, so a pointer
/// in a hairline gap stays on the nearer slice instead of blinking the
/// tooltip off).
pub(crate) fn sectors(ctx: &SeriesRenderContext, opts: &PieOptions) -> Vec<Sector> {
    let frame = frame(ctx);
    rings(ctx, opts, &frame)
        .into_iter()
        .flat_map(|ring| {
            let (r0, r1) = (ring.inner, ring.outer);
            layout(opts, &ring.values)
                .into_iter()
                .enumerate()
                .map(move |(index, arc)| Sector {
                    index,
                    r0,
                    r1,
                    a0: arc.start_angle,
                    a1: arc.end_angle,
                })
        })
        .collect()
}

/// A slice's fill: the datum's own color, else the `--dx-chart-<1..8>`
/// token by position (cycling). One definition, shared with the tooltip so
/// a slice's swatch is always the color the slice is drawn in.
pub(crate) fn slice_color(datum: Option<&ChartDatum>, index: usize) -> String {
    datum
        .and_then(|d| d.color.clone())
        .unwrap_or_else(|| format!("var(--dx-chart-{})", (index % 8) + 1))
}

/// The load-animation mask shared by the arc families (see the module doc):
/// one stroke of width `extent` around a circle of radius `extent / 2`, from
/// `a0` to `a1` (engine angles, clamped to a turn), so a dash animated from
/// `0` to its full `pathLength` reveals a wedge sweeping from `a0`.
pub(crate) fn sweep_mask(id: &str, a0: f64, a1: f64, extent: f64) -> Element {
    let sweep = (a1 - a0).clamp(-TAU, TAU);
    let d = arc_centerline_path(extent / 2.0, a0, a0 + sweep);
    rsx! {
        mask {
            id: "{id}",
            "data-slot": "chart-sweep-mask",
            "maskUnits": "userSpaceOnUse",
            x: "{fmt_num(-extent)}",
            y: "{fmt_num(-extent)}",
            width: "{fmt_num(2.0 * extent)}",
            height: "{fmt_num(2.0 * extent)}",
            path {
                d: "{d}",
                fill: "none",
                stroke: "white",
                "stroke-width": "{fmt_num(extent)}",
                "pathLength": "1",
            }
        }
    }
}

/// Render every ring, the labels and the optional donut center text.
pub(crate) fn render(ctx: &SeriesRenderContext, opts: &PieOptions) -> Element {
    let chart = use_chart();
    let chart_id = (chart.id)();
    let frame = frame(ctx);
    let rings = rings(ctx, opts, &frame);
    let active = opts.active_index.or(ctx.active_index);
    let multi = rings.len() > 1;
    let hole = rings.first().map_or(0.0, |r| r.inner);

    let translate = format!("translate({}, {})", fmt_num(frame.cx), fmt_num(frame.cy));
    let mask_id = safe_fragment_id(&format!("{chart_id}-pie-sweep"));
    let extent = ctx.width.max(ctx.height);
    let (a0, a1) = (
        recharts_angle(opts.start_angle),
        recharts_angle(opts.end_angle),
    );

    // (ring, its slices, its `<g>` key, its `data-series` when several)
    let laid: Vec<(Ring, Vec<PieArc>, String, Option<String>)> = rings
        .into_iter()
        .map(|ring| {
            let arcs = layout(opts, &ring.values);
            let key = ring.series.as_ref().map_or(String::new(), |s| s.0.clone());
            let slot = ring.series.as_ref().filter(|_| multi).map(|s| s.1.clone());
            (ring, arcs, key, slot)
        })
        .collect();

    rsx! {
        g { "data-slot": "chart-series",
            g { transform: "{translate}",
                defs { {sweep_mask(&mask_id, a0, a1, extent)} }
                g { "mask": "url(#{mask_id})",
                    for (ring , arcs , key , slot) in laid.iter() {
                        g { key: "{key}", "data-series": slot.clone(),
                            for (i , arc) in arcs.iter().enumerate() {
                                {render_slice(SliceParams {
                                    ring,
                                    arc,
                                    index: i,
                                    datum: ctx.data.get(i),
                                    forced_active: opts.active_index == Some(i),
                                    halo: opts.active_halo,
                                    is_active: active == Some(i),
                                    corner_radius: opts.corner_radius,
                                })}
                            }
                        }
                    }
                }
                if !matches!(opts.labels, PieLabels::None) {
                    g { "data-slot": "chart-arc-labels",
                        for (ring , arcs , _ , _) in laid.iter() {
                            for (i , arc) in arcs.iter().enumerate() {
                                {render_label(ring, arc, i, ctx.data.get(i), &opts.labels)}
                            }
                        }
                    }
                }
            }
            if let Some((primary, secondary)) = &opts.center_text {
                {center_text::render(frame.cx, frame.cy, hole, primary, secondary, CenterLayout::Donut)}
            }
        }
    }
}

/// Bundled args for [`render_slice`].
struct SliceParams<'a> {
    ring: &'a Ring,
    arc: &'a PieArc,
    index: usize,
    datum: Option<&'a ChartDatum>,
    /// [`PieOptions::active_index`] names this slice: draw it grown.
    forced_active: bool,
    /// Draw the grown slice's halo ring too.
    halo: bool,
    /// Hovered or forced: `data-active` for the tooltip/legend link.
    is_active: bool,
    corner_radius: f64,
}

/// One slice: `path[data-slot="chart-arc"]`, and for the forced-active
/// slice its halo. `data-start-angle`/`data-end-angle` are Recharts
/// degrees and `data-inner-radius`/`data-outer-radius` px, as drawn.
fn render_slice(p: SliceParams<'_>) -> Element {
    let index = p.index;
    let inner = p.ring.inner;
    let outer = if p.forced_active {
        p.ring.outer + ACTIVE_GROW
    } else {
        p.ring.outer
    };
    let d = arc_path(
        inner,
        outer,
        p.arc.start_angle,
        p.arc.end_angle,
        p.arc.pad_angle,
        p.corner_radius,
    );
    let halo = (p.forced_active && p.halo).then(|| {
        arc_path(
            p.ring.outer + HALO.0,
            p.ring.outer + HALO.1,
            p.arc.start_angle,
            p.arc.end_angle,
            p.arc.pad_angle,
            p.corner_radius,
        )
    });
    let color = slice_color(p.datum, p.index);

    rsx! {
        path {
            key: "{index}",
            "data-slot": "chart-arc",
            "data-index": "{index}",
            "data-active": p.is_active.then_some("true"),
            "data-start-angle": "{fmt_num(to_recharts_angle(p.arc.start_angle))}",
            "data-end-angle": "{fmt_num(to_recharts_angle(p.arc.end_angle))}",
            "data-inner-radius": "{fmt_num(inner)}",
            "data-outer-radius": "{fmt_num(outer)}",
            d: "{d}",
            fill: "{color}",
        }
        if let Some(halo) = halo {
            path {
                key: "{index}-halo",
                "data-slot": "chart-arc-halo",
                "data-index": "{index}",
                d: "{halo}",
                fill: "{color}",
            }
        }
    }
}

/// One slice's label (if [`PieLabels`] calls for one at this index).
fn render_label(
    ring: &Ring,
    arc: &PieArc,
    index: usize,
    datum: Option<&ChartDatum>,
    labels: &PieLabels,
) -> Element {
    let total: f64 = ring.values.iter().filter(|v| **v > 0.0).sum();
    let text = match labels {
        PieLabels::None => None,
        PieLabels::Value | PieLabels::Outside { .. } => Some(fmt_decimal(arc.value, 2)),
        PieLabels::Percent => Some(if total > 0.0 {
            format!("{:.1}%", arc.value / total * 100.0)
        } else {
            "0%".to_string()
        }),
        PieLabels::List(items) => items.get(index).filter(|s| !s.is_empty()).cloned(),
    };
    let Some(text) = text else {
        return rsx! {};
    };
    if let PieLabels::Outside { line } = labels {
        let mid = (arc.start_angle + arc.end_angle) / 2.0;
        // `centroid` of a zero-width sector is the point at that radius.
        let rim = ring.outer;
        let (x0, y0) = centroid(rim, rim, mid, mid);
        let (x, y) = centroid(rim + LABEL_OFFSET, rim + LABEL_OFFSET, mid, mid);
        let anchor = if x > 1e-9 {
            "start"
        } else if x < -1e-9 {
            "end"
        } else {
            "middle"
        };
        let color = slice_color(datum, index);
        return rsx! {
            if *line {
                path {
                    "data-slot": "chart-arc-label-line",
                    d: "M{fmt_num(x0)},{fmt_num(y0)}L{fmt_num(x)},{fmt_num(y)}",
                    fill: "none",
                    stroke: "{color}",
                }
            }
            text {
                "data-slot": "chart-arc-label",
                "data-position": "outside",
                x: "{fmt_num(x)}",
                y: "{fmt_num(y)}",
                text_anchor: "{anchor}",
                "dominant-baseline": "middle",
                "{text}"
            }
        };
    }
    let (x, y) = centroid(ring.inner, ring.outer, arc.start_angle, arc.end_angle);
    rsx! {
        text {
            key: "{index}",
            "data-slot": "chart-arc-label",
            "data-position": "inside",
            x: "{fmt_num(x)}",
            y: "{fmt_num(y)}",
            text_anchor: "middle",
            "dominant-baseline": "central",
            "{text}"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart::components::chart::Chart;
    use crate::chart::{ChartConfig, ChartContainer, ChartKind};
    use dioxus_core::NoOpMutations;

    fn browsers() -> Vec<ChartDatum> {
        [275.0, 200.0, 187.0, 173.0, 90.0]
            .iter()
            .enumerate()
            .map(|(i, v)| ChartDatum {
                label: format!("B{i}"),
                values: vec![Some(*v)],
                ..Default::default()
            })
            .collect()
    }

    fn render(config: ChartConfig, data: Vec<ChartDatum>, pie: PieOptions) -> String {
        #[derive(Clone, PartialEq, Props)]
        struct HarnessProps {
            config: ChartConfig,
            data: Vec<ChartDatum>,
            pie: PieOptions,
        }
        #[component]
        fn Harness(props: HarnessProps) -> Element {
            let config = use_signal(|| props.config.clone());
            let data = use_signal(|| props.data.clone());
            rsx! {
                ChartContainer { config, data, kind: ChartKind::Pie,
                    Chart { aria_label: "Test", pie: props.pie.clone() }
                }
            }
        }
        let mut dom = VirtualDom::new_with_props(Harness, HarnessProps { config, data, pie });
        dom.rebuild_in_place();
        dom.render_immediate(&mut NoOpMutations);
        dioxus_ssr::render(&dom)
    }

    fn one() -> ChartConfig {
        ChartConfig::new().series("visitors", "Visitors", "var(--dx-chart-1)")
    }

    /// `name="value"` of every element carrying `slot`, in document order.
    fn attrs(html: &str, slot: &str, name: &str) -> Vec<f64> {
        let needle = format!(r#"data-slot="{slot}""#);
        html.match_indices(&needle)
            .filter_map(|(at, _)| {
                let tag_end = html[at..].find('>')? + at;
                let tag = &html[at..tag_end];
                let key = format!(r#"{name}=""#);
                let start = tag.find(&key)? + key.len();
                tag[start..].split('"').next()?.parse().ok()
            })
            .collect()
    }

    #[test]
    fn default_pie_runs_counter_clockwise_from_three_oclock() {
        let html = render(one(), browsers(), PieOptions::default());
        let starts = attrs(&html, "chart-arc", "data-start-angle");
        let ends = attrs(&html, "chart-arc", "data-end-angle");
        assert_eq!(starts.len(), 5, "{html}");
        assert!(starts[0].abs() < 1e-3, "{starts:?}");
        assert!((ends[0] - 107.0).abs() < 0.1, "{ends:?}");
        assert!((ends[4] - 360.0).abs() < 1e-3, "{ends:?}");
    }

    #[test]
    fn forced_active_slice_grows_its_rim_but_not_its_hole() {
        let opts = PieOptions {
            inner_radius: Radius::Px(60.0),
            active_index: Some(0),
            ..Default::default()
        };
        let html = render(one(), browsers(), opts);
        let outer = attrs(&html, "chart-arc", "data-outer-radius");
        let inner = attrs(&html, "chart-arc", "data-inner-radius");
        assert!(
            (outer[0] - outer[1] - ACTIVE_GROW).abs() < 1e-3,
            "{outer:?}"
        );
        assert_eq!(inner[0], 60.0);
        assert!(!html.contains("chart-arc-halo"), "{html}");
    }

    #[test]
    fn active_halo_draws_one_extra_ring() {
        let opts = PieOptions {
            inner_radius: Radius::Px(60.0),
            active_index: Some(2),
            active_halo: true,
            ..Default::default()
        };
        let html = render(one(), browsers(), opts);
        assert_eq!(html.matches(r#"data-slot="chart-arc-halo""#).count(), 1);
    }

    #[test]
    fn outside_labels_sit_beyond_the_rim_with_leader_lines() {
        let opts = PieOptions {
            labels: PieLabels::Outside { line: true },
            ..Default::default()
        };
        let html = render(one(), browsers(), opts);
        assert_eq!(html.matches(r#"data-position="outside""#).count(), 5);
        assert_eq!(
            html.matches(r#"data-slot="chart-arc-label-line""#).count(),
            5
        );
        assert!(html.contains(r#"text-anchor="start""#), "{html}");
        assert!(html.contains(r#"text-anchor="end""#), "{html}");
        assert!(html.contains(">275<"), "{html}");

        let no_line = render(
            one(),
            browsers(),
            PieOptions {
                labels: PieLabels::Outside { line: false },
                ..Default::default()
            },
        );
        assert!(!no_line.contains("chart-arc-label-line"), "{no_line}");
    }

    #[test]
    fn list_labels_sit_at_the_centroid() {
        let opts = PieOptions {
            labels: PieLabels::List(vec!["Chrome".into(), String::new()]),
            ..Default::default()
        };
        let html = render(one(), browsers(), opts);
        assert_eq!(html.matches(r#"data-position="inside""#).count(), 1);
        assert!(html.contains(">Chrome<"));
    }

    #[test]
    fn rings_give_each_series_its_own_radii() {
        let config = ChartConfig::new()
            .series("desktop", "Desktop", "var(--dx-chart-1)")
            .series("mobile", "Mobile", "var(--dx-chart-2)");
        let data: Vec<ChartDatum> = (0..5)
            .map(|i| ChartDatum {
                label: format!("M{i}"),
                values: vec![Some(10.0 + i as f64), Some(20.0)],
                ..Default::default()
            })
            .collect();
        let opts = PieOptions {
            rings: vec![
                (Radius::Px(0.0), Radius::Px(60.0)),
                (Radius::Px(70.0), Radius::Px(90.0)),
            ],
            ..Default::default()
        };
        let html = render(config, data, opts);
        let outer = attrs(&html, "chart-arc", "data-outer-radius");
        let inner = attrs(&html, "chart-arc", "data-inner-radius");
        assert_eq!(outer.len(), 10);
        assert!(outer[..5].iter().all(|r| *r == 60.0), "{outer:?}");
        assert!(inner[..5].iter().all(|r| *r == 0.0), "{inner:?}");
        assert!(outer[5..].iter().all(|r| *r == 90.0), "{outer:?}");
        assert!(inner[5..].iter().all(|r| *r == 70.0), "{inner:?}");
    }

    #[test]
    fn slices_sit_under_the_sweep_mask() {
        let html = render(one(), browsers(), PieOptions::default());
        assert!(html.contains(r#"data-slot="chart-sweep-mask""#), "{html}");
        assert!(html.contains(r#"mask="url(#"#), "{html}");
        assert!(html.contains(r#"pathLength="1""#), "{html}");
    }
}
