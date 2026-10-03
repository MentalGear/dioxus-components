//! The [`ChartKind::Radar`](crate::chart::ChartKind::Radar) family: a
//! closed polygon per series across N radial (per-datum) axes -- shadcn's
//! `RadarChart`/`Radar`/`PolarGrid`/`PolarAngleAxis`
//! (`$S/refs/ui/apps/v4/registry/new-york-v4/charts/chart-radar-*.tsx`,
//! all 14 demos), on top of [`crate::chart::engine::radar`]'s pure
//! geometry (angles, the radial scale, polygon/sector path builders -- see
//! that module's own doc for the d3-shape/Recharts citations).
//!
//! ## Why this file draws its own grid and hit-sectors, not `layout.rs`'s
//!
//! `ChartKind::is_cartesian()` is `false` for [`crate::chart::ChartKind::Radar`],
//! so `components::chart::Chart`'s own body renders none of the Cartesian
//! grid/axis/cursor/hit-bands for this kind (a rectangular grid behind a
//! circular shape would be actively misleading, not merely unfinished --
//! see that module's own doc). Every visual piece a radar chart needs
//! instead -- concentric/polygon grid rings, spokes, category labels
//! around the rim, and the angular hit-sectors that are this family's
//! *only* hover mechanism -- is therefore this file's own job, inside the
//! one `g[data-slot="chart-series"][data-kind="radar"]` group `Chart`
//! already wraps this function's return value in.
//!
//! ## Hover, and why this file calls [`use_chart`] a second time
//!
//! `SeriesRenderContext::active_index` is a plain, **read-only**
//! `Option<usize>` (`components::layout`'s own doc: "only `Chart`'s own
//! hit-bands ... write it") -- correct for the three Cartesian families,
//! whose hit-bands `chart.rs` itself still renders, but no help to a
//! family that must render its *own* hit-target shape (an angular wedge,
//! not a rectangular band). This file calls [`use_chart`] itself, exactly
//! as `components::tooltip`/`components::legend` already do, to reach the
//! same [`crate::chart::context::ChartContext::active_index`] `Signal`
//! `Chart` itself reads and writes -- a second, unconditional call to a
//! plain context-tree lookup (not a stateful, call-order-indexed hook like
//! `use_signal`), safe to call any number of times from anywhere still
//! executing inside `Chart`'s own render pass, which this function always
//! is (`Chart`'s body calls it directly and synchronously, never deferred).
//! No `SeriesRenderContext`/`components::layout` change needed for this.
//!
//! ## Known, flagged limitations (both orthogonal to this file, see
//! `$S/stage2-lanes.md`'s own `s2-radar` entries)
//!
//! 1. **Tooltip position.** Solved in the shared layer: this family
//!    publishes [`anchors`] (each spoke's vertex of the largest value, used
//!    when the keyboard opens the tooltip) and `Chart` registers it with
//!    `Follow::Pointer`, so on hover the tooltip follows the pointer, with
//!    the active spoke picked by this file's own wedge hit-targets.
//! 2. **Hidden data table.** `components::chart::Chart`'s own body still
//!    picks `engine::table::table_rows_single_series` (category + first
//!    series only) for every `!is_cartesian` kind unconditionally --
//!    correct forever for Pie/RadialBar (one value per category, by
//!    construction), but not for Radar, which genuinely overlays N series
//!    per category the same way Area/Bar/Line do (see this family's own
//!    multi-series gallery variants). A ledger request proposes decoupling
//!    that choice from `is_cartesian()`; until it lands, `ChartKind::Radar`
//!    renders the reduced table end-to-end even though this file's own
//!    geometry is already fully multi-series-correct.

use dioxus::prelude::*;

use super::super::layout::SeriesRenderContext;
use super::super::pointer::Sector;
use super::pie::frame;

use crate::chart::engine::polar::{recharts_angle, Radius};
use crate::chart::engine::radar::{
    angle_tick_anchor, category_angles, grid_polygon_path, point_radial, radar_series_path,
    radial_scale, sector_path, RadarTick,
};
use crate::chart::engine::scale::{fmt_decimal, fmt_num, nice_ticks};

/// Recharts' `PolarAngleAxis` `tickSize`: tick labels sit this far beyond
/// the outer radius.
const ANGLE_TICK_OFFSET: f64 = 8.0;
/// Recharts' `PolarRadiusAxis` default `tickCount`.
const RADIUS_TICK_COUNT: usize = 5;
/// shadcn's `dot={{ r: 4 }}`.
const DOT_RADIUS: f64 = 4.0;
/// shadcn's custom two-line tick lifts the top label this far
/// (`chart-radar-label-custom`: `y + (index === 0 ? -10 : 0)`).
const CUSTOM_TICK_TOP_LIFT: f64 = 10.0;

/// Which grid rings/spokes a [`RadarOptions`]-configured radar chart draws
/// -- shadcn's `PolarGrid` `gridType`/`radialLines`/`polarRadius`/
/// `className` combinations, one variant per combination its demos use.
/// Rings sit at the radius axis' ticks (Recharts: exactly five, `0`
/// included -- see [`RadarOptions`]). Default: [`RadarGrid::Polygon`].
#[derive(Clone, PartialEq, Debug, Default)]
pub enum RadarGrid {
    /// `<PolarGrid />`: polygon rings plus a spoke per category.
    #[default]
    Polygon,
    /// `<PolarGrid radialLines={false} />`: polygon rings, no spokes
    /// (`chart-radar-lines-only`).
    PolygonNoLines,
    /// `gridType="circle"`: circular rings, with spokes.
    Circle,
    /// Circular rings + spokes, every ring and spoke tinted with the first
    /// series' color at 20% opacity (`className="fill-(--color-desktop)
    /// opacity-20"`). `chart-radar-grid-circle-fill`.
    CircleFill,
    /// Circular rings, no spokes. `chart-radar-grid-circle-no-lines`.
    CircleNoLines,
    /// Polygon rings + spokes, tinted like [`Self::CircleFill`].
    /// `chart-radar-grid-fill`.
    Fill,
    /// No grid. `chart-radar-grid-none`.
    None,
    /// Polygon rings at caller-given radii **in px** (Recharts'
    /// `polarRadius`), with or without spokes -- `chart-radar-grid-custom`
    /// (`<PolarGrid radialLines={false} polarRadius={[90]} />`).
    Custom {
        /// Ring radii, px.
        polar_radius: Vec<f64>,
        /// Whether to draw a spoke per category.
        spokes: bool,
    },
}

/// Configuration for [`crate::chart::ChartKind::Radar`] -- Recharts
/// `RadarChart`/`Radar`/`PolarAngleAxis`/`PolarRadiusAxis` props.
///
/// The radius axis is Recharts' `PolarRadiusAxis`: domain `[0, auto]` with
/// exactly five "nice" ticks (`engine::scale::nice_ticks`; `0..320` step
/// 80 for shadcn's data), so a vertex's radius is `value / 320 * outer`.
#[derive(Clone, PartialEq, Debug)]
pub struct RadarOptions {
    /// Which grid rings/spokes to draw.
    pub grid: RadarGrid,
    /// Each series' `fillOpacity`, by configured-series position. A series
    /// without an entry is opaque (`1.0`), Recharts' default -- shadcn's
    /// two-series demos give only the first `<Radar>` `fillOpacity={0.6}`,
    /// so the second is drawn opaque on top: `vec![0.6]`.
    pub fill_opacity: Vec<f64>,
    /// Draw a dot (`r` 4) at every defined vertex -- shadcn's `dot={{ r: 4,
    /// fillOpacity: 1 }}`.
    pub dots: bool,
    /// Outline-only polygons: no fill, a 2px stroke in the series color --
    /// shadcn's `chart-radar-lines-only` (`fillOpacity={0}`,
    /// `strokeWidth={2}`). Without it a polygon has no stroke (Recharts'
    /// default).
    pub lines_only: bool,
    /// `outerRadius`. Default [`Radius::DEFAULT_OUTER`] (`80%` of the max
    /// radius: 96px in shadcn's 250px square).
    pub outer_radius: Radius,
    /// `<PolarAngleAxis dataKey=... />`: each category's label 8px beyond
    /// the rim, anchored away from the centre.
    pub axis_labels: bool,
    /// `<PolarRadiusAxis angle={..} />`: the radius axis' tick values drawn
    /// along the spoke at this angle (Recharts degrees, `0` = three
    /// o'clock), each rotated to read along it -- `chart-radar-radius`
    /// (`angle={60}`).
    pub radius_axis: Option<f64>,
    /// Replace the category labels with custom two-line ticks, one per
    /// category (`chart-radar-label-custom`) -- see [`RadarTick`].
    pub ticks: Option<Vec<RadarTick>>,
}

impl Default for RadarOptions {
    fn default() -> Self {
        Self {
            grid: RadarGrid::default(),
            fill_opacity: Vec::new(),
            dots: false,
            lines_only: false,
            outer_radius: Radius::DEFAULT_OUTER,
            axis_labels: true,
            radius_axis: None,
            ticks: None,
        }
    }
}

/// The radar's own plot geometry: its center, outermost ring radius, the
/// radius axis' ticks and the value -> radius scale. Shared by [`render`]
/// (the marks) and [`anchors`] (the keyboard tooltip anchors), so the two
/// cannot disagree.
struct RadarGeometry {
    cx: f64,
    cy: f64,
    outer_radius: f64,
    ticks: Vec<f64>,
    scale: crate::chart::LinearScale,
}

fn geometry(ctx: &SeriesRenderContext, opts: &RadarOptions) -> RadarGeometry {
    let frame = frame(ctx);
    let outer_radius = opts.outer_radius.resolve(frame.max_radius).max(1.0);
    let max = ctx
        .data
        .iter()
        .flat_map(|d| {
            d.values
                .iter()
                .take(ctx.config.series.len())
                .flatten()
                .copied()
        })
        .filter(|v| v.is_finite())
        .fold(0.0f64, f64::max);
    let ticks = nice_ticks(0.0, max, RADIUS_TICK_COUNT);
    let top = ticks.last().copied().filter(|t| *t > 0.0).unwrap_or(1.0);
    RadarGeometry {
        cx: frame.cx,
        cy: frame.cy,
        outer_radius,
        ticks,
        scale: radial_scale((0.0, top), outer_radius),
    }
}

/// Each category's keyboard tooltip anchor, in px: the vertex of the
/// category's largest value on its own spoke (the center for a category
/// with no values). A pointer hover instead follows the pointer
/// (`Follow::Pointer`), so these only place a tooltip the keyboard opened.
pub(crate) fn anchors(ctx: &SeriesRenderContext, opts: &RadarOptions) -> Vec<(f64, f64)> {
    let RadarGeometry { cx, cy, scale, .. } = geometry(ctx, opts);
    category_angles(ctx.data.len())
        .into_iter()
        .zip(ctx.data.iter())
        .map(|(angle, datum)| {
            let top = datum
                .values
                .iter()
                .take(ctx.config.series.len())
                .flatten()
                .copied()
                .fold(None, |m: Option<f64>, v| Some(m.map_or(v, |m| m.max(v))));
            let radius = top.map_or(0.0, |v| scale.scale(v));
            let (x, y) = point_radial(angle, radius);
            (cx + x, cy + y)
        })
        .collect()
}

/// The hit regions the pointer is resolved against (see
/// `components::pointer`): one wedge per category around its spoke, out to
/// the outermost ring -- the same wedges the (now decorative) hit-band
/// paths draw.
pub(crate) fn sectors(ctx: &SeriesRenderContext, opts: &RadarOptions) -> Vec<Sector> {
    let RadarGeometry { outer_radius, .. } = geometry(ctx, opts);
    let n = ctx.data.len();
    let step = if n > 0 {
        std::f64::consts::TAU / n as f64
    } else {
        0.0
    };
    category_angles(n)
        .into_iter()
        .enumerate()
        .map(|(index, angle)| Sector {
            index,
            r0: 0.0,
            r1: outer_radius,
            a0: angle - step / 2.0,
            a1: angle + step / 2.0,
        })
        .collect()
}

/// Render the grid, every configured series' closed polygon, the angle and
/// radius axes and the angular hit-sectors -- see the module doc for why
/// all of this (not just the per-series marks) is this file's job.
pub(crate) fn render(ctx: &SeriesRenderContext, opts: &RadarOptions) -> Element {
    let n = ctx.data.len();
    let RadarGeometry {
        cx,
        cy,
        outer_radius,
        ticks,
        scale,
    } = geometry(ctx, opts);

    let angles = category_angles(n);
    let angle_step = if n > 0 {
        std::f64::consts::TAU / n as f64
    } else {
        0.0
    };

    // Grid ring radii (px) + spokes/shape/fill flags, per `opts.grid`.
    let tick_radii: Vec<f64> = ticks.iter().map(|t| scale.scale(*t)).collect();
    let (ring_radii, draw_spokes, ring_is_circle, ring_fill): (Vec<f64>, bool, bool, bool) =
        match &opts.grid {
            RadarGrid::None => (Vec::new(), false, false, false),
            RadarGrid::Polygon => (tick_radii, true, false, false),
            RadarGrid::PolygonNoLines => (tick_radii, false, false, false),
            RadarGrid::Circle => (tick_radii, true, true, false),
            RadarGrid::CircleFill => (tick_radii, true, true, true),
            RadarGrid::CircleNoLines => (tick_radii, false, true, false),
            RadarGrid::Fill => (tick_radii, true, false, true),
            RadarGrid::Custom {
                polar_radius,
                spokes,
            } => (polar_radius.clone(), *spokes, false, false),
        };
    let first_series_color = ctx
        .config
        .series
        .first()
        .map(|s| format!("var(--color-{})", s.slot()))
        .unwrap_or_default();

    rsx! {
        g {
            "data-slot": "chart-series",
            "data-kind": "radar",
            transform: "translate({fmt_num(cx)}, {fmt_num(cy)})",

            if !matches!(opts.grid, RadarGrid::None) {
                g {
                    "data-slot": "chart-grid",
                    style: if ring_fill { "--grid-fill-color: {first_series_color}" },
                    for (i , r) in ring_radii.iter().copied().enumerate() {
                        if ring_is_circle {
                            circle {
                                key: "{i}",
                                "data-slot": "chart-grid-ring",
                                "data-fill": if ring_fill { "true" },
                                cx: "0",
                                cy: "0",
                                r: "{fmt_num(r)}",
                            }
                        } else {
                            path {
                                key: "{i}",
                                "data-slot": "chart-grid-ring",
                                "data-fill": if ring_fill { "true" },
                                "data-radius": "{fmt_num(r)}",
                                d: "{grid_polygon_path(&angles, r)}",
                            }
                        }
                    }
                    if draw_spokes {
                        for (i , angle) in angles.iter().copied().enumerate() {
                            {
                                let (x, y) = point_radial(angle, outer_radius);
                                rsx! {
                                    line {
                                        key: "{i}",
                                        "data-slot": "chart-grid-spoke",
                                        "data-fill": if ring_fill { "true" },
                                        x1: "0",
                                        y1: "0",
                                        x2: "{fmt_num(x)}",
                                        y2: "{fmt_num(y)}",
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if opts.axis_labels {
                g { "data-slot": "chart-axis", "data-axis": "angle",
                    for (i , (angle , datum)) in angles.iter().copied().zip(ctx.data.iter()).enumerate() {
                        {
                            let (x, y) = point_radial(angle, outer_radius + ANGLE_TICK_OFFSET);
                            let (anchor, dy) = angle_tick_anchor(angle);
                            match opts.ticks.as_ref().and_then(|t| t.get(i)) {
                                Some(tick) => render_custom_tick(i, x, y, anchor, tick),
                                None => rsx! {
                                    text {
                                        key: "{i}",
                                        "data-index": "{i}",
                                        x: "{fmt_num(x)}",
                                        y: "{fmt_num(y)}",
                                        "text-anchor": anchor,
                                        dy,
                                        "{datum.label}"
                                    }
                                },
                            }
                        }
                    }
                }
            }

            for (s , series) in ctx.config.series.iter().enumerate() {
                {
                    let values: Vec<Option<f64>> = ctx.series_values(s);
                    let d = radar_series_path(&values, &scale);
                    let fill_opacity = if opts.lines_only {
                        0.0
                    } else {
                        opts.fill_opacity.get(s).copied().unwrap_or(1.0)
                    };
                    rsx! {
                        g {
                            key: "{series.key}",
                            "data-series": "{series.slot()}",
                            style: "--series-color: var(--color-{series.slot()})",
                            path {
                                "data-slot": "chart-radar-area",
                                "data-lines-only": opts.lines_only.then_some("true"),
                                d: "{d}",
                                "fill-opacity": "{fmt_num(fill_opacity)}",
                            }
                            if opts.dots {
                                for (i , angle) in angles.iter().copied().enumerate() {
                                    if let Some(v) = values.get(i).copied().flatten() {
                                        {
                                            let (x, y) = point_radial(angle, scale.scale(v));
                                            rsx! {
                                                circle {
                                                    key: "{i}",
                                                    "data-slot": "chart-dot",
                                                    "data-index": "{i}",
                                                    cx: "{fmt_num(x)}",
                                                    cy: "{fmt_num(y)}",
                                                    r: "{fmt_num(DOT_RADIUS)}",
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if let Some(angle_deg) = opts.radius_axis {
                g { "data-slot": "chart-axis", "data-axis": "radius",
                    for (i , t) in ticks.iter().copied().enumerate() {
                        {
                            // Recharts angle -> this module's (radians, 0 at
                            // the top, clockwise), then rotate the text to
                            // read along the spoke (`rotate(90 - angle)`).
                            let a = recharts_angle(angle_deg);
                            let (x, y) = point_radial(a, scale.scale(t));
                            let rotate = format!(
                                "rotate({}, {}, {})",
                                fmt_num(90.0 - angle_deg),
                                fmt_num(x),
                                fmt_num(y)
                            );
                            rsx! {
                                text {
                                    key: "{i}",
                                    x: "{fmt_num(x)}",
                                    y: "{fmt_num(y)}",
                                    transform: "{rotate}",
                                    "text-anchor": "middle",
                                    "{fmt_decimal(t, 2)}"
                                }
                            }
                        }
                    }
                }
            }

            // The whole hover mechanism -- one angular wedge per category
            // (see `engine::radar::sector_path`'s own doc). Drawn last (on
            // top), same DOM-order convention as the Cartesian families'
            // own `chart-hit-bands` group.
            g { "data-slot": "chart-hit-bands",
                for (i , angle) in angles.iter().copied().enumerate() {
                    {
                        let d = sector_path(
                            angle - angle_step / 2.0,
                            angle + angle_step / 2.0,
                            outer_radius,
                        );
                        rsx! {
                            path {
                                key: "{i}",
                                "data-slot": "chart-hit-band",
                                "data-index": "{i}",
                                d: "{d}",
                                fill: "transparent",
                                "pointer-events": "all",
                            }
                        }
                    }
                }
            }
        }
    }
}

/// One custom two-line angle tick (`RadarOptions::ticks`), shadcn's
/// `chart-radar-label-custom` markup: the first line's runs as `<tspan>`s
/// (muted ones `data-muted`), the caption a muted `<tspan>` one line below,
/// and the top label lifted 10px so its two lines clear the grid.
fn render_custom_tick(i: usize, x: f64, y: f64, anchor: &'static str, tick: &RadarTick) -> Element {
    let y = if i == 0 && tick.caption.is_some() {
        y - CUSTOM_TICK_TOP_LIFT
    } else {
        y
    };
    rsx! {
        text {
            key: "{i}",
            "data-index": "{i}",
            "data-custom": "true",
            x: "{fmt_num(x)}",
            y: "{fmt_num(y)}",
            "text-anchor": anchor,
            for (k , (part , muted)) in tick.parts.iter().enumerate() {
                tspan { key: "{k}", "data-muted": muted.then_some("true"), "{part}" }
            }
            if let Some(caption) = &tick.caption {
                tspan {
                    "data-caption": "true",
                    "data-muted": "true",
                    x: "{fmt_num(x)}",
                    dy: "1rem",
                    "{caption}"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart::components::chart::Chart;
    use crate::chart::{ChartConfig, ChartContainer, ChartDatum, ChartKind};
    use dioxus_core::NoOpMutations;

    fn sample_config() -> ChartConfig {
        ChartConfig::new()
            .series("desktop", "Desktop", "var(--dx-chart-1)")
            .series("mobile", "Mobile", "var(--dx-chart-2)")
    }

    /// 4 categories -- enough to distinguish "N vertices" from a
    /// coincidental fixed number, without the 6-month gallery fixture's own
    /// data (covered by that gallery's own Playwright spec instead).
    fn sample_data() -> Vec<ChartDatum> {
        ["North", "East", "South", "West"]
            .iter()
            .enumerate()
            .map(|(i, label)| ChartDatum {
                label: label.to_string(),
                values: vec![Some(100.0 + i as f64), Some(50.0 + i as f64)],
                ..Default::default()
            })
            .collect()
    }

    #[derive(Clone, PartialEq, Props)]
    struct HarnessProps {
        #[props(default)]
        radar: RadarOptions,
    }

    #[component]
    fn Harness(props: HarnessProps) -> Element {
        let config = use_signal(sample_config);
        let data = use_signal(sample_data);
        rsx! {
            ChartContainer { config, data, kind: ChartKind::Radar,
                Chart {
                    aria_label: "Regional totals",
                    radar: props.radar.clone(),
                }
            }
        }
    }

    fn render(radar: RadarOptions) -> String {
        let mut dom = VirtualDom::new_with_props(Harness, HarnessProps { radar });
        dom.rebuild_in_place();
        dom.render_immediate(&mut NoOpMutations);
        dioxus_ssr::render(&dom)
    }

    #[test]
    fn one_radar_path_per_series() {
        let html = render(RadarOptions::default());
        assert_eq!(html.matches(r#"data-slot="chart-radar-area""#).count(), 2);
        assert!(html.contains(r#"data-series="desktop""#));
        assert!(html.contains(r#"data-series="mobile""#));
    }

    #[test]
    fn each_series_path_has_n_vertices() {
        let html = render(RadarOptions::default());
        // 4 categories -> "M" + 3 "L" commands per closed polygon.
        let start = html.find(r#"data-series="desktop""#).unwrap();
        let d_start = html[start..].find(" d=\"").unwrap() + start + 4;
        let d_end = html[d_start..].find('"').unwrap() + d_start;
        let d = &html[d_start..d_end];
        assert_eq!(d.matches('M').count() + d.matches('L').count(), 4, "d={d}");
        assert!(d.trim_end().ends_with('Z'), "d={d}");
    }

    #[test]
    fn default_grid_is_polygon_with_spokes() {
        let html = render(RadarOptions::default());
        assert!(html.contains(r#"data-slot="chart-grid""#));
        assert!(html.contains(r#"data-slot="chart-grid-ring""#));
        assert!(html.contains("<path"));
        assert!(html.contains(r#"data-slot="chart-grid-spoke""#));
    }

    #[test]
    fn circle_grid_renders_circle_rings() {
        let html = render(RadarOptions {
            grid: RadarGrid::Circle,
            ..Default::default()
        });
        assert!(html.contains("<circle"));
        assert!(html.contains(r#"data-slot="chart-grid-spoke""#));
    }

    #[test]
    fn circle_no_lines_omits_spokes() {
        let html = render(RadarOptions {
            grid: RadarGrid::CircleNoLines,
            ..Default::default()
        });
        assert!(html.contains("<circle"));
        assert!(!html.contains(r#"data-slot="chart-grid-spoke""#));
    }

    #[test]
    fn grid_none_omits_the_grid_group_entirely() {
        let html = render(RadarOptions {
            grid: RadarGrid::None,
            ..Default::default()
        });
        assert!(!html.contains(r#"data-slot="chart-grid""#));
    }

    #[test]
    fn circle_fill_and_fill_tint_the_ring_with_the_first_series_color() {
        let html = render(RadarOptions {
            grid: RadarGrid::CircleFill,
            ..Default::default()
        });
        assert!(html.contains(r#"data-fill="true""#));
        assert!(html.contains("--grid-fill-color"));
        assert!(html.contains("--color-desktop"));
    }

    #[test]
    fn custom_grid_draws_exactly_the_given_rings() {
        let html = render(RadarOptions {
            grid: RadarGrid::Custom {
                polar_radius: vec![50.0],
                spokes: false,
            },
            ..Default::default()
        });
        assert_eq!(html.matches(r#"data-slot="chart-grid-ring""#).count(), 1);
        assert!(!html.contains(r#"data-slot="chart-grid-spoke""#));
    }

    #[test]
    fn lines_only_forces_zero_fill_opacity() {
        let html = render(RadarOptions {
            lines_only: true,
            fill_opacity: vec![0.6], // deliberately non-zero -- lines_only must still win
            ..Default::default()
        });
        assert!(html.contains(r#"fill-opacity="0""#));
    }

    #[test]
    fn dots_renders_one_circle_per_category_per_series() {
        let html = render(RadarOptions {
            dots: true,
            ..Default::default()
        });
        // 4 categories x 2 series = 8 dots.
        assert_eq!(html.matches(r#"data-slot="chart-dot""#).count(), 8);
    }

    #[test]
    fn dots_off_by_default() {
        let html = render(RadarOptions::default());
        assert!(!html.contains(r#"data-slot="chart-dot""#));
    }

    #[test]
    fn axis_labels_render_one_full_text_label_per_category() {
        let html = render(RadarOptions::default());
        assert_eq!(html.matches(r#"data-slot="chart-axis""#).count(), 1);
        assert!(html.contains(">North<"));
        assert!(html.contains(">South<"));
    }

    #[test]
    fn axis_labels_can_be_disabled() {
        let html = render(RadarOptions {
            axis_labels: false,
            ..Default::default()
        });
        assert!(!html.contains(r#"data-axis="angle""#));
    }

    #[test]
    fn hit_sectors_cover_every_category() {
        let html = render(RadarOptions::default());
        assert_eq!(html.matches(r#"data-slot="chart-hit-band""#).count(), 4);
        assert!(html.contains("pointer-events"));
    }

    #[test]
    fn a_none_value_skips_its_vertex_without_a_panic() {
        #[component]
        fn GapHarness() -> Element {
            let config = use_signal(sample_config);
            let data = use_signal(|| {
                let mut data = sample_data();
                data[1].values[0] = None;
                data
            });
            rsx! {
                ChartContainer { config, data, kind: ChartKind::Radar,
                    Chart { aria_label: "Regional totals" }
                }
            }
        }
        let mut dom = VirtualDom::new(GapHarness);
        dom.rebuild_in_place();
        dom.render_immediate(&mut NoOpMutations);
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains(r#"data-slot="chart-radar-area""#));
    }

    /// This file's `use_chart()` call reaches the SAME `ChartContext`
    /// `ChartTooltip` reads -- proven the same way `tooltip.rs`'s own test
    /// module proves it for the Cartesian families' hit-bands
    /// (`ActiveIndexSetter`, an effect that sets `active_index` once):
    /// `rebuild_in_place` alone never runs a `use_effect`, so
    /// `render_immediate` flushes it before `dioxus_ssr::render`
    /// serializes the tree. If this file's `use_chart()` returned a
    /// *different* context instance than `ChartContainer` actually
    /// provided, `ChartTooltip` (which independently calls `use_chart()`
    /// itself) would never see `active_index` change here, and this test
    /// would fail. A real pointer event firing this file's own
    /// `onpointerenter` closure is exercised end-to-end by
    /// `playwright/radar_chart.spec.ts` instead (SSR cannot execute event
    /// handlers at all, so a unit test can only prove the *shared context*
    /// half of this file's hover claim, not the closure itself).
    #[test]
    fn use_chart_reaches_the_same_context_chart_tooltip_reads() {
        use crate::chart::components::tooltip::ChartTooltip;
        use crate::chart::context::use_chart;

        #[component]
        fn ActiveIndexSetter(index: Option<usize>) -> Element {
            let ctx = use_chart();
            let mut active_index = ctx.active_index;
            use_effect(move || {
                active_index.set(index);
            });
            rsx! {}
        }

        #[component]
        fn HoverHarness() -> Element {
            let config = use_signal(sample_config);
            let data = use_signal(sample_data);
            rsx! {
                ChartContainer { config, data, kind: ChartKind::Radar,
                    Chart { aria_label: "Regional totals" }
                    ActiveIndexSetter { index: Some(1) }
                    ChartTooltip {}
                }
            }
        }

        let mut dom = VirtualDom::new(HoverHarness);
        dom.rebuild_in_place();
        dom.render_immediate(&mut NoOpMutations);
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains(r#"data-state="open""#), "{html}");
        assert!(
            html.contains("East"),
            "expected the active (index 1) category's label: {html}"
        );
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
    fn circle_rings_sit_at_five_recharts_ticks() {
        // max 103 -> nice ticks 0, 30, 60, 90, 120: five rings incl. r = 0.
        let html = render(RadarOptions {
            grid: RadarGrid::Circle,
            ..Default::default()
        });
        let radii = attrs(&html, "chart-grid-ring", "r");
        assert_eq!(radii.len(), 5, "{html}");
        assert_eq!(radii[0], 0.0);
        let outer = radii[4];
        assert!((radii[2] - outer / 2.0).abs() < 1e-3, "{radii:?}");
    }

    #[test]
    fn series_fill_opacity_is_per_series_and_defaults_to_opaque() {
        let html = render(RadarOptions {
            fill_opacity: vec![0.6],
            ..Default::default()
        });
        assert!(html.contains(r#"fill-opacity="0.6""#), "{html}");
        assert!(html.contains(r#"fill-opacity="1""#), "{html}");
    }

    #[test]
    fn angle_labels_are_anchored_away_from_the_centre() {
        let html = render(RadarOptions::default());
        // North (top) middle, East start, South middle, West end.
        assert!(html.contains(r#"text-anchor="start""#), "{html}");
        assert!(html.contains(r#"text-anchor="end""#), "{html}");
        assert!(html.contains(r#"dy="0.71em""#), "{html}");
    }

    #[test]
    fn radius_axis_draws_the_five_tick_values_along_its_angle() {
        let html = render(RadarOptions {
            radius_axis: Some(60.0),
            ..Default::default()
        });
        assert!(html.contains(r#"data-axis="radius""#), "{html}");
        assert!(html.contains("rotate(30, "), "{html}");
        assert!(html.contains(">120<"), "{html}");
    }

    #[test]
    fn custom_ticks_replace_the_category_labels() {
        let ticks = (0..4)
            .map(|i| RadarTick {
                parts: vec![(format!("{i}"), false), ("/".into(), true)],
                caption: Some(format!("M{i}")),
            })
            .collect();
        let html = render(RadarOptions {
            ticks: Some(ticks),
            ..Default::default()
        });
        assert_eq!(html.matches(r#"data-custom="true""#).count(), 4, "{html}");
        assert!(html.contains(">M2<"), "{html}");
        let axis = &html[html.find(r#"data-axis="angle""#).unwrap()..];
        let axis = &axis[..axis.find("</g>").unwrap()];
        assert!(!axis.contains("North"), "{axis}");
    }

    #[test]
    fn dots_have_recharts_radius_four() {
        let html = render(RadarOptions {
            dots: true,
            ..Default::default()
        });
        assert!(html.contains(r#"r="4""#), "{html}");
    }
}
