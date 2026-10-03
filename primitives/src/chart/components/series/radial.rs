//! The [`ChartKind::RadialBar`](crate::chart::ChartKind::RadialBar) family:
//! concentric proportional arcs, one ring per datum (the default), or every
//! configured series stacked into ONE ring ([`RadialOptions::stacked`]) --
//! shadcn/ui's `chart-radial-*` demos, which are Recharts `RadialBarChart`s.
//!
//! ## Recharts semantics
//!
//! Every option means what the same-named Recharts prop means, so a shadcn
//! demo's numbers are copied verbatim:
//! - **angles** are Recharts degrees: `0` = three o'clock, counter-clockwise
//!   positive (`start_angle: -90.0, end_angle: 380.0` is shadcn's
//!   `chart-radial-label`); default `0 -> 360`;
//! - **value to sweep**: a bar for `value` runs from `start_angle` to
//!   `start_angle + value / domain_max * (end_angle - start_angle)`, with
//!   `domain_max` the largest value in the data (Recharts' angle axis domain
//!   `[0, dataMax]`, not "nice"-rounded) -- so a one-datum chart's bar
//!   always fills the whole `end_angle` sweep;
//! - **radii** ([`Radius`]): `[inner_radius, outer_radius]` is split into
//!   one equal category band per datum, datum `0` innermost, and each bar is
//!   inset 10% of its band on both sides with its thickness truncated to
//!   whole pixels -- Recharts' `barCategoryGap` and `getBarPosition`
//!   (`engine::polar::radial_bar_radii`);
//! - [`RadialOptions::background`] is `<RadialBar background>`: a muted
//!   track at each bar's radii over the full `start..end` sweep, and
//!   [`RadialOptions::grid`] is `<PolarGrid>` ([`RadialGrid`]).
//!
//! ## Stacked: proportions of the stack total (a deliberate difference)
//!
//! A stacked chart's segments split the sweep in proportion to the stack
//! total: segment `s` covers its own share of `sum(values)`, so the stack
//! fills `start..end` exactly. Recharts instead keeps the angle domain at
//! the largest *single* value and clamps the overflowing stack at
//! `end_angle` (shadcn's `chart-radial-stacked` therefore draws mobile 570
//! over 45% of the half turn and desktop 1260 over the other 55%, where the
//! true shares are 31% / 69%). That is a scale bug, not a design, so this
//! crate does not reproduce it. Segments stack in series declaration order
//! (Recharts: element order), the first from `start_angle`.
//!
//! ## Load animation
//!
//! The bars (not the background tracks or grid) sit under the same sweep
//! mask [`super::pie`] uses (`pie::sweep_mask`), swept from `start_angle`
//! to `end_angle` once the chart's svg is `data-animate="true"`.

use dioxus::prelude::*;

use super::super::center_text::{self, CenterLayout};
use super::super::fragment::safe_fragment_id;
use super::super::layout::SeriesRenderContext;
use super::super::pointer::Sector;
use super::pie::{frame, slice_color, sweep_mask};
use crate::chart::context::use_chart;
use crate::chart::engine::polar::{
    angle_scale, arc_centerline_path, arc_path, centroid, radial_band_center, radial_bar_radii,
    recharts_angle, to_recharts_angle, PolarFrame, RadialGrid, Radius,
};
use crate::chart::engine::scale::{fmt_decimal, fmt_num};
use crate::chart::{ChartDatum, LinearScale, PieLabels};

/// Recharts' `insideStart` label offset along the arc, degrees.
const LABEL_OFFSET_DEG: f64 = 5.0;
/// How far a label's path runs around the ring (Recharts: 359 degrees).
const LABEL_PATH_DEG: f64 = 359.0;

/// [`crate::chart::ChartKind::RadialBar`]'s own options -- Recharts
/// `RadialBarChart`/`RadialBar` props (see the module doc).
#[derive(Clone, PartialEq, Debug)]
pub struct RadialOptions {
    /// `innerRadius`. Default `Px(0)` (Recharts' default).
    pub inner_radius: Radius,
    /// `outerRadius`. Default [`Radius::DEFAULT_OUTER`] (`80%`).
    pub outer_radius: Radius,
    /// `startAngle`, Recharts degrees (`0` = three o'clock). Default `0`.
    pub start_angle: f64,
    /// `endAngle`, Recharts degrees; may exceed a full turn (shadcn's
    /// `chart-radial-label` sweeps `-90 -> 380`). Default `360`.
    pub end_angle: f64,
    /// `cornerRadius`, px: rounds both ends of every bar (clamped to half
    /// the bar's thickness).
    pub corner_radius: f64,
    /// `<RadialBar background>`: a muted full-sweep track behind each bar.
    pub background: bool,
    /// `<PolarGrid>` -- see [`RadialGrid`].
    pub grid: RadialGrid,
    /// Per-ring text, set along the ring from just inside its start
    /// (Recharts' `<LabelList position="insideStart">`) -- see
    /// [`PieLabels`] for the text kinds (`Outside` reads as `Value` here).
    pub labels: PieLabels,
    /// `false` (the default): one ring per datum. `true`: every configured
    /// series' value for the FIRST datum stacks into one ring
    /// (`chart-radial-stacked`'s `stackId`).
    pub stacked: bool,
    /// The angle axis' domain max: the value that sweeps the whole
    /// `start..end`. `None` (the default): the largest value in the data,
    /// or the stack total when [`Self::stacked`].
    pub domain_max: Option<f64>,
    /// Text centered in the hole: `(primary, secondary)`, e.g. `("200",
    /// "Visitors")` -- shadcn's `chart-radial-text`.
    pub center_text: Option<(String, String)>,
    /// Lay [`Self::center_text`] out for a half-turn gauge: both lines in
    /// the upper half (`chart-radial-stacked`: primary 16px above the
    /// centre, caption 4px below it) instead of centred on the centre.
    pub center_text_raised: bool,
}

impl Default for RadialOptions {
    fn default() -> Self {
        Self {
            inner_radius: Radius::Px(0.0),
            outer_radius: Radius::DEFAULT_OUTER,
            start_angle: 0.0,
            end_angle: 360.0,
            corner_radius: 0.0,
            background: false,
            grid: RadialGrid::None,
            labels: PieLabels::None,
            stacked: false,
            domain_max: None,
            center_text: None,
            center_text_raised: false,
        }
    }
}

/// One drawn bar (a ring, or one stacked segment), engine angles.
#[derive(Clone, Debug, PartialEq)]
struct Bar {
    /// The datum it belongs to (`0` for every stacked segment).
    index: usize,
    /// The series slot, for a stacked segment.
    series: Option<String>,
    value: f64,
    r0: f64,
    r1: f64,
    a0: f64,
    a1: f64,
    color: String,
}

/// The chart's resolved geometry: frame, radii, the full sweep and bars.
struct Layout {
    frame: PolarFrame,
    inner: f64,
    outer: f64,
    /// The full sweep (engine angles): where every track runs.
    sweep: (f64, f64),
    /// The angle axis' domain max.
    max: f64,
    /// Category band count (one per datum; one when stacked).
    bands: usize,
    bars: Vec<Bar>,
}

fn layout(ctx: &SeriesRenderContext, opts: &RadialOptions) -> Layout {
    let frame = frame(ctx);
    let outer = opts.outer_radius.resolve(frame.max_radius).max(0.0);
    let inner = opts
        .inner_radius
        .resolve(frame.max_radius)
        .clamp(0.0, outer);
    let (a0, a1) = (
        recharts_angle(opts.start_angle),
        recharts_angle(opts.end_angle),
    );
    let value_of = |v: Option<f64>| v.filter(|v| v.is_finite()).unwrap_or(0.0).max(0.0);

    if opts.stacked {
        let datum = ctx.data.first();
        let values: Vec<f64> = (0..ctx.config.series.len())
            .map(|s| value_of(datum.and_then(|d| d.values.get(s).copied().flatten())))
            .collect();
        let max = opts
            .domain_max
            .unwrap_or_else(|| values.iter().sum())
            .max(f64::EPSILON);
        let scale = angle_scale((0.0, max), a0, a1);
        let (r0, r1) = radial_bar_radii(inner, outer, 1, 0);
        let mut cumulative = 0.0;
        let bars = ctx
            .config
            .series
            .iter()
            .zip(values)
            .map(|(series, value)| {
                let start = scale.scale(cumulative);
                cumulative += value;
                Bar {
                    index: 0,
                    series: Some(series.slot()),
                    value,
                    r0,
                    r1,
                    a0: start,
                    a1: scale.scale(cumulative),
                    color: format!("var(--color-{})", series.slot()),
                }
            })
            .collect();
        return Layout {
            frame,
            inner,
            outer,
            sweep: (a0, a1),
            max,
            bands: 1,
            bars,
        };
    }

    let n = ctx.data.len();
    let values: Vec<f64> = ctx
        .data
        .iter()
        .map(|d| value_of(d.values.first().copied().flatten()))
        .collect();
    let max = opts
        .domain_max
        .unwrap_or_else(|| values.iter().copied().fold(0.0, f64::max))
        .max(f64::EPSILON);
    let scale = angle_scale((0.0, max), a0, a1);
    let bars = values
        .iter()
        .enumerate()
        .map(|(i, &value)| {
            let (r0, r1) = radial_bar_radii(inner, outer, n, i);
            Bar {
                index: i,
                series: None,
                value,
                r0,
                r1,
                a0,
                a1: scale.scale(value),
                color: slice_color(ctx.data.get(i), i),
            }
        })
        .collect();
    Layout {
        frame,
        inner,
        outer,
        sweep: (a0, a1),
        max,
        bands: n.max(1),
        bars,
    }
}

/// Each datum's keyboard tooltip anchor, in px: the centroid of its bar
/// (the stacked layout has one datum, anchored mid-stack). A pointer hover
/// follows the pointer instead (`Follow::Pointer`).
pub(crate) fn anchors(ctx: &SeriesRenderContext, opts: &RadialOptions) -> Vec<(f64, f64)> {
    let l = layout(ctx, opts);
    let (cx, cy) = (l.frame.cx, l.frame.cy);
    if opts.stacked {
        let (Some(first), Some(last)) = (l.bars.first(), l.bars.last()) else {
            return Vec::new();
        };
        let (x, y) = centroid(first.r0, first.r1, first.a0, last.a1);
        return vec![(cx + x, cy + y)];
    }
    l.bars
        .iter()
        .map(|b| {
            let (x, y) = centroid(b.r0, b.r1, b.a0, b.a1);
            (cx + x, cy + y)
        })
        .collect()
}

/// The hit regions the pointer is resolved against (see
/// `components::pointer`): each bar, or -- stacked -- the whole stack. The
/// empty remainder of a ring (and the gaps between rings) is outside every
/// sector, so the tooltip shows only over a bar, as in shadcn.
pub(crate) fn sectors(ctx: &SeriesRenderContext, opts: &RadialOptions) -> Vec<Sector> {
    let l = layout(ctx, opts);
    if opts.stacked {
        let (Some(first), Some(last)) = (l.bars.first(), l.bars.last()) else {
            return Vec::new();
        };
        return vec![Sector {
            index: 0,
            r0: first.r0,
            r1: first.r1,
            a0: first.a0,
            a1: last.a1,
        }];
    }
    l.bars
        .iter()
        .map(|b| Sector {
            index: b.index,
            r0: b.r0,
            r1: b.r1,
            a0: b.a0,
            a1: b.a1,
        })
        .collect()
}

/// Render the grid, the background tracks, the bars (under the load
/// animation's sweep mask), their labels, and the center text.
pub(crate) fn render(ctx: &SeriesRenderContext, opts: &RadialOptions) -> Element {
    let chart = use_chart();
    let chart_id = (chart.id)();
    let l = layout(ctx, opts);
    let (cx, cy) = (l.frame.cx, l.frame.cy);
    let translate = format!("translate({}, {})", fmt_num(cx), fmt_num(cy));
    let mask_id = safe_fragment_id(&format!("{chart_id}-radial-sweep"));
    let extent = ctx.width.max(ctx.height);
    let (a0, a1) = l.sweep;

    // Background tracks: one per ring (stacked: the one ring).
    let tracks: Vec<(f64, f64)> = if opts.stacked {
        l.bars.first().map(|b| (b.r0, b.r1)).into_iter().collect()
    } else {
        l.bars.iter().map(|b| (b.r0, b.r1)).collect()
    };

    let labels: Vec<ArcLabel> = l
        .bars
        .iter()
        .enumerate()
        .filter_map(|(k, bar)| {
            let datum = ctx.data.get(bar.index);
            let total = if opts.stacked {
                l.bars.iter().map(|b| b.value).sum()
            } else {
                l.max
            };
            let text = label_text(&opts.labels, k, datum, bar.value, total)?;
            Some(ArcLabel {
                id: format!("{chart_id}-radial-label-{k}"),
                text,
                radius: (bar.r0 + bar.r1) / 2.0,
                start: bar.a0,
                end: bar.a1,
            })
        })
        .collect();
    let hover = ctx.active_index;
    let layout_kind = if opts.center_text_raised {
        CenterLayout::HalfGauge
    } else {
        CenterLayout::Gauge
    };

    rsx! {
        g { "data-slot": "chart-series",
            g { transform: "{translate}",
                {render_grid(&opts.grid, &l)}
                if opts.background {
                    for (i , (r0 , r1)) in tracks.iter().copied().enumerate() {
                        path {
                            key: "{i}",
                            "data-slot": "chart-radial-background",
                            "data-start-angle": "{fmt_num(to_recharts_angle(a0))}",
                            "data-end-angle": "{fmt_num(to_recharts_angle(a1))}",
                            "data-inner-radius": "{fmt_num(r0)}",
                            "data-outer-radius": "{fmt_num(r1)}",
                            d: "{arc_path(r0, r1, a0, a1, 0.0, opts.corner_radius)}",
                        }
                    }
                }
                defs { {sweep_mask(&mask_id, a0, a1, extent)} }
                g { "mask": "url(#{mask_id})",
                    for (k , bar) in l.bars.iter().enumerate() {
                        path {
                            key: "{k}",
                            "data-slot": "chart-arc",
                            "data-index": "{bar.index}",
                            "data-series": bar.series.clone(),
                            "data-active": (hover == Some(bar.index)).then_some("true"),
                            "data-start-angle": "{fmt_num(to_recharts_angle(bar.a0))}",
                            "data-end-angle": "{fmt_num(to_recharts_angle(bar.a1))}",
                            "data-inner-radius": "{fmt_num(bar.r0)}",
                            "data-outer-radius": "{fmt_num(bar.r1)}",
                            d: "{arc_path(bar.r0, bar.r1, bar.a0, bar.a1, 0.0, opts.corner_radius)}",
                            fill: "{bar.color}",
                        }
                    }
                }
                if !labels.is_empty() {
                    g { "data-slot": "chart-arc-labels",
                        for label in labels.iter() {
                            {label.render()}
                        }
                    }
                }
            }
            if let Some((primary, secondary)) = &opts.center_text {
                {center_text::render(cx, cy, l.inner, primary, secondary, layout_kind)}
            }
        }
    }
}

/// The `<PolarGrid>` -- see [`RadialGrid`]. Drawn first, under everything.
fn render_grid(grid: &RadialGrid, l: &Layout) -> Element {
    match grid {
        RadialGrid::None => rsx! {},
        RadialGrid::Circles { radial_lines } => {
            let circles: Vec<f64> = (0..l.bands)
                .map(|i| radial_band_center(l.inner, l.outer, l.bands, i))
                .collect();
            let scale = angle_scale((0.0, l.max), l.sweep.0, l.sweep.1);
            let spokes: Vec<f64> = if *radial_lines {
                LinearScale {
                    domain: (0.0, l.max),
                    range: (0.0, 1.0),
                }
                .ticks(10)
                .into_iter()
                .map(|t| scale.scale(t))
                .collect()
            } else {
                Vec::new()
            };
            let point = |r: f64, a: f64| centroid(r, r, a, a);
            rsx! {
                g { "data-slot": "chart-grid",
                    for (i , r) in circles.iter().enumerate() {
                        circle {
                            key: "c{i}",
                            "data-slot": "chart-grid-ring",
                            cx: "0",
                            cy: "0",
                            r: "{fmt_num(*r)}",
                        }
                    }
                    for (i , a) in spokes.iter().enumerate() {
                        {
                            let (x1, y1) = point(l.inner, *a);
                            let (x2, y2) = point(l.outer, *a);
                            rsx! {
                                line {
                                    key: "s{i}",
                                    "data-slot": "chart-grid-spoke",
                                    x1: "{fmt_num(x1)}",
                                    y1: "{fmt_num(y1)}",
                                    x2: "{fmt_num(x2)}",
                                    y2: "{fmt_num(y2)}",
                                }
                            }
                        }
                    }
                }
            }
        }
        RadialGrid::Annulus { outer, inner } => {
            let d = arc_path(*inner, *outer, 0.0, std::f64::consts::TAU, 0.0, 0.0);
            rsx! {
                g { "data-slot": "chart-grid",
                    path {
                        "data-slot": "chart-radial-annulus",
                        "data-inner-radius": "{fmt_num(*inner)}",
                        "data-outer-radius": "{fmt_num(*outer)}",
                        d: "{d}",
                    }
                }
            }
        }
    }
}

/// One bar's label, set along the ring's centerline -- see [`RadialOptions::labels`].
struct ArcLabel {
    /// Unique (per chart) id for the centerline `<path>` the text follows.
    id: String,
    text: String,
    /// The bar's centerline radius.
    radius: f64,
    /// The bar's start and end, engine angles.
    start: f64,
    end: f64,
}

impl ArcLabel {
    /// Recharts' `insideStart` radial label: a `<textPath>` along a 359
    /// degree arc of the bar's centerline that starts 5 degrees past the
    /// bar's start and runs the bar's own way round, the text starting at
    /// the path's start and centred on the centerline.
    ///
    /// `direction="ltr"` on the `<text>` is load-bearing: under an `rtl`
    /// ancestor (`dir="rtl"`, inherited by SVG text) `text-anchor: start`
    /// means the *right* end, so the glyphs would lay out backwards off the
    /// start of the path and vanish.
    fn render(&self) -> Element {
        let id = safe_fragment_id(&self.id);
        let href = format!("#{id}");
        let text = self.text.clone();
        rsx! {
            defs {
                path { id: "{id}", d: "{self.centerline()}", fill: "none" }
            }
            text {
                "data-slot": "chart-arc-label",
                "data-position": "inside-start",
                direction: "ltr",
                "dominant-baseline": "central",
                textPath { "href": "{href}", "{text}" }
            }
        }
    }

    /// The label's centerline path `d`.
    fn centerline(&self) -> String {
        let sign = if self.end < self.start { -1.0 } else { 1.0 };
        let from = self.start + sign * LABEL_OFFSET_DEG.to_radians();
        arc_centerline_path(self.radius, from, from + sign * LABEL_PATH_DEG.to_radians())
    }
}

/// The text for bar `index` under `labels` (`Outside` reads as `Value`).
fn label_text(
    labels: &PieLabels,
    index: usize,
    datum: Option<&ChartDatum>,
    value: f64,
    total: f64,
) -> Option<String> {
    let _ = datum;
    match labels {
        PieLabels::None => None,
        PieLabels::Value | PieLabels::Outside { .. } => Some(fmt_decimal(value, 2)),
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart::components::chart::Chart;
    use crate::chart::{ChartConfig, ChartContainer, ChartKind};
    use dioxus_core::NoOpMutations;

    fn two_series() -> ChartConfig {
        ChartConfig::new()
            .series("mobile", "Mobile", "var(--dx-chart-2)")
            .series("desktop", "Desktop", "var(--dx-chart-1)")
    }

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

    fn one_series() -> ChartConfig {
        ChartConfig::new().series("visitors", "Visitors", "var(--dx-chart-1)")
    }

    fn render(config: ChartConfig, data: Vec<ChartDatum>, opts: RadialOptions) -> String {
        render_dir(config, data, opts, None)
    }

    fn render_dir(
        config: ChartConfig,
        data: Vec<ChartDatum>,
        opts: RadialOptions,
        dir: Option<crate::direction::Direction>,
    ) -> String {
        #[derive(Clone, PartialEq, Props)]
        struct HarnessProps {
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
                ChartContainer { config, data, kind: ChartKind::RadialBar,
                    Chart {
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
    fn defaults_are_recharts_defaults() {
        let opts = RadialOptions::default();
        assert_eq!(opts.inner_radius, Radius::Px(0.0));
        assert_eq!(opts.outer_radius, Radius::Percent(80.0));
        assert_eq!((opts.start_angle, opts.end_angle), (0.0, 360.0));
        assert!(!opts.stacked && !opts.background);
        assert_eq!(opts.grid, RadialGrid::None);
    }

    #[test]
    fn rings_start_at_three_oclock_and_sweep_counter_clockwise() {
        let opts = RadialOptions {
            inner_radius: Radius::Px(30.0),
            outer_radius: Radius::Px(110.0),
            ..Default::default()
        };
        let html = render(one_series(), browsers(), opts);
        let starts = attrs(&html, "chart-arc", "data-start-angle");
        let ends = attrs(&html, "chart-arc", "data-end-angle");
        assert_eq!(starts, vec![0.0; 5], "{html}");
        // 200 / 275 * 360 = 261.8 (measured on chart-radial-simple).
        assert!((ends[0] - 360.0).abs() < 1e-3, "{ends:?}");
        assert!((ends[1] - 261.818).abs() < 1e-2, "{ends:?}");
        let inner = attrs(&html, "chart-arc", "data-inner-radius");
        let outer = attrs(&html, "chart-arc", "data-outer-radius");
        assert_eq!(inner[0], 31.6);
        assert_eq!(outer[0], 43.6);
        assert_eq!(inner[4], 95.6);
    }

    #[test]
    fn stacked_segments_split_the_sweep_by_the_stack_total_in_series_order() {
        let data = vec![ChartDatum {
            label: "january".to_string(),
            values: vec![Some(570.0), Some(1260.0)],
            ..Default::default()
        }];
        let opts = RadialOptions {
            stacked: true,
            end_angle: 180.0,
            inner_radius: Radius::Px(80.0),
            outer_radius: Radius::Px(110.0),
            ..Default::default()
        };
        let html = render(two_series(), data, opts);
        let starts = attrs(&html, "chart-arc", "data-start-angle");
        let ends = attrs(&html, "chart-arc", "data-end-angle");
        assert_eq!(starts.len(), 2);
        // mobile first: 570 / 1830 * 180 = 56.066.
        assert!(html.find(r#"data-series="mobile""#) < html.find(r#"data-series="desktop""#));
        assert_eq!(starts[0], 0.0);
        assert!((ends[0] - 56.066).abs() < 1e-2, "{ends:?}");
        assert_eq!(starts[1], ends[0]);
        assert!((ends[1] - 180.0).abs() < 1e-3, "{ends:?}");
        assert_eq!(attrs(&html, "chart-arc", "data-inner-radius")[0], 83.0);
    }

    #[test]
    fn background_draws_a_full_sweep_track_per_ring() {
        let opts = RadialOptions {
            background: true,
            ..Default::default()
        };
        let html = render(one_series(), browsers(), opts);
        assert_eq!(
            html.matches(r#"data-slot="chart-radial-background""#)
                .count(),
            5
        );
    }

    #[test]
    fn circle_grid_draws_band_circles_and_value_spokes() {
        let opts = RadialOptions {
            inner_radius: Radius::Px(30.0),
            outer_radius: Radius::Px(100.0),
            grid: RadialGrid::Circles { radial_lines: true },
            ..Default::default()
        };
        let html = render(one_series(), browsers(), opts);
        assert_eq!(
            attrs(&html, "chart-grid-ring", "r"),
            vec![37.0, 51.0, 65.0, 79.0, 93.0]
        );
        // Ticks every 20 over [0, 275]: 14 spokes.
        assert_eq!(html.matches(r#"data-slot="chart-grid-spoke""#).count(), 14);
        assert!(!html.contains("chart-radial-background"));
    }

    #[test]
    fn annulus_grid_is_one_full_ring() {
        let opts = RadialOptions {
            grid: RadialGrid::Annulus {
                outer: 90.0,
                inner: 80.0,
            },
            ..Default::default()
        };
        let html = render(one_series(), browsers(), opts);
        assert_eq!(
            html.matches(r#"data-slot="chart-radial-annulus""#).count(),
            1
        );
    }

    #[test]
    fn labels_follow_each_bar_from_five_degrees_past_its_start() {
        let opts = RadialOptions {
            start_angle: -90.0,
            end_angle: 380.0,
            labels: PieLabels::List(vec!["Chrome".into(), "Safari".into()]),
            ..Default::default()
        };
        let html = render(one_series(), browsers(), opts);
        assert_eq!(html.matches("<textPath").count(), 2, "{html}");
        assert!(html.contains(r#"data-position="inside-start""#));
        let label = ArcLabel {
            id: "x".into(),
            text: "a".into(),
            radius: 100.0,
            start: recharts_angle(-90.0),
            end: recharts_angle(200.0),
        };
        // Recharts -85 degrees: x = 100 cos(-85deg), y = -100 sin(-85deg).
        let d = label.centerline();
        assert!(d.starts_with("M8.716 99.619"), "{d}");
        // Counter-clockwise (sweep flag 0), like the bar.
        assert!(d.contains(" 0 0 0 "), "{d}");
    }

    #[test]
    fn labels_stay_left_to_right_under_an_rtl_page() {
        use crate::direction::Direction;
        let opts = RadialOptions {
            labels: PieLabels::Value,
            ..Default::default()
        };
        let rtl = render_dir(one_series(), browsers(), opts, Some(Direction::Rtl));
        let labels = rtl.matches(r#"data-slot="chart-arc-label""#).count();
        assert_eq!(labels, 5, "{rtl}");
        assert_eq!(
            rtl.matches(r#"data-position="inside-start" direction="ltr""#)
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
        };
        let html = dioxus_ssr::render_element(label.render());
        assert!(!html.contains("chart%"), "{html}");
        assert!(
            html.contains(r##"href="#chart_25_1-radial-label-0""##),
            "{html}"
        );
    }

    #[test]
    fn center_text_layouts() {
        let opts = RadialOptions {
            inner_radius: Radius::Px(80.0),
            center_text: Some(("1,830".to_string(), "Visitors".to_string())),
            center_text_raised: true,
            ..Default::default()
        };
        let html = render(one_series(), browsers(), opts);
        assert!(html.contains(r#"data-layout="half-gauge""#), "{html}");
        assert!(html.contains("1,830"));
    }

    #[test]
    fn corner_radius_adds_fillet_arc_commands() {
        let sharp = render(one_series(), browsers(), RadialOptions::default());
        let rounded = render(
            one_series(),
            browsers(),
            RadialOptions {
                corner_radius: 8.0,
                ..Default::default()
            },
        );
        assert!(rounded.matches('A').count() > sharp.matches('A').count());
    }

    #[test]
    fn does_not_panic_with_empty_data() {
        let html = render(one_series(), Vec::new(), RadialOptions::default());
        assert_eq!(html.matches(r#"data-slot="chart-arc""#).count(), 0);
        let html = render(
            two_series(),
            Vec::new(),
            RadialOptions {
                stacked: true,
                ..Default::default()
            },
        );
        assert!(!html.contains("NaN"), "{html}");
    }
}
