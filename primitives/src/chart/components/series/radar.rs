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
//! 1. **Tooltip position.** `components::tooltip::ChartTooltip` positions
//!    itself from `crate::chart::context::ChartLayout`'s `BandScale`/
//!    `LinearScale` pair, which cannot express a radar vertex's position
//!    (a sinusoidal, not affine, function of category index). This file
//!    does not write to that layout signal, so `ChartTooltip` opens with
//!    the correct label/values but falls back to its own `(50%, 50%)`
//!    default position until a family-agnostic anchor representation
//!    lands there.
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
use crate::chart::context::use_chart;
use crate::chart::engine::radar::{
    category_angles, grid_polygon_path, point_radial, radar_series_path, radial_scale, sector_path,
};
use crate::chart::engine::scale::fmt_num;
use crate::chart::nice_domain;

/// Which grid rings/spokes a [`RadarOptions`]-configured radar chart draws
/// -- shadcn's `PolarGrid` `gridType`/`radialLines`/`className` combinations,
/// one variant per distinct combination its own demos actually use. Default:
/// [`RadarGrid::Polygon`] (shadcn's own `<PolarGrid />` default: polygon
/// rings + spokes, no fill).
#[derive(Clone, PartialEq, Debug, Default)]
pub enum RadarGrid {
    /// Straight-sided polygon rings at each magnitude tick, plus a spoke
    /// per category. shadcn's `chart-radar-default`/`-dots`/`-multiple`/
    /// `-legend`/`-icons`/`-label-custom`/`-lines-only`/`-radius`.
    #[default]
    Polygon,
    /// Circular rings instead of polygon rings, still with spokes.
    /// shadcn's `chart-radar-grid-circle`.
    Circle,
    /// Circular rings + spokes, with every ring's own interior tinted using
    /// the first configured series' color (`PolarGrid`'s
    /// `className="fill-(--color-desktop) opacity-20"`, `gridType="circle"`).
    /// shadcn's `chart-radar-grid-circle-fill`.
    CircleFill,
    /// Circular rings, no spokes (`radialLines={false}`). shadcn's
    /// `chart-radar-grid-circle-no-lines`.
    CircleNoLines,
    /// Polygon rings + spokes, tinted the same way as [`Self::CircleFill`]
    /// (`PolarGrid`'s default `gridType="polygon"` with the same
    /// `className`). shadcn's `chart-radar-grid-fill`.
    Fill,
    /// No grid at all (no `<PolarGrid />` rendered). shadcn's
    /// `chart-radar-grid-none`.
    None,
    /// Caller-specified ring values (in the chart's own data domain, not
    /// pixels -- mapped through the same radial scale as every series) and
    /// whether spokes are drawn, in place of the default nice-tick ring
    /// set -- shadcn's `chart-radar-grid-custom`
    /// (`<PolarGrid radialLines={false} polarRadius={[90]} />`).
    Custom {
        /// Ring values, in the chart's own value domain.
        values: Vec<f64>,
        /// Whether to draw a spoke per category.
        spokes: bool,
    },
}

/// Configuration for [`crate::chart::ChartKind::Radar`].
#[derive(Clone, PartialEq, Debug)]
pub struct RadarOptions {
    /// Which grid rings/spokes to draw.
    pub grid: RadarGrid,
    /// Fill opacity for every series' own closed polygon
    /// (`path[data-slot="chart-radar-area"]`) -- there is no separate
    /// fill/stroke path pair the way Area has; the same path is both
    /// filled (this value, as a direct `fill-opacity` SVG presentation
    /// attribute -- a per-chart *configurable* value, unlike Area's fixed
    /// CSS `.4`, so it cannot live in the themed stylesheet) and stroked.
    /// shadcn's own `fillOpacity={0.6}`.
    pub fill_opacity: f64,
    /// Draw a `circle[data-slot="chart-dot"]` at every defined vertex.
    /// shadcn's `dot={{ r: 4, fillOpacity: 1 }}`.
    pub dots: bool,
    /// Force [`Self::fill_opacity`] to `0.0` regardless of its own value,
    /// and document the intent -- shadcn's `chart-radar-lines-only`
    /// (`fillOpacity={0}`, an explicit `stroke`).
    pub lines_only: bool,
    /// The outermost ring's radius, as a fraction `(0.0, 1.0]` of the
    /// available half-extent (`min(width, height) / 2`, minus rim-label
    /// margin when [`Self::axis_labels`] is set).
    pub outer_radius: f64,
    /// Draw each category's label around the rim
    /// (`g[data-slot="chart-axis"][data-axis="angle"]`) -- shadcn's
    /// `<PolarAngleAxis dataKey="month" />`.
    pub axis_labels: bool,
}

impl Default for RadarOptions {
    fn default() -> Self {
        Self {
            grid: RadarGrid::default(),
            fill_opacity: 0.6,
            dots: false,
            lines_only: false,
            outer_radius: 0.8,
            axis_labels: true,
        }
    }
}

/// The five ring values every non-[`RadarGrid::Custom`] grid variant draws
/// rings at: [`crate::chart::LinearScale::ticks`]'s "nice" values over the
/// chart's own value domain (`scale.domain`), matching this crate's
/// Cartesian y-axis's own `y_tick_count` default of 5
/// (`components::chart::ChartProps::y_tick_count`) -- a radar's magnitude
/// axis is the same kind of scale, just drawn as rings instead of
/// horizontal lines.
fn nice_ring_values(scale: &crate::chart::LinearScale) -> Vec<f64> {
    scale.ticks(5)
}

/// Render every configured series' closed polygon, this family's own grid/
/// spokes/rim labels, and the angular hit-sectors that drive hover -- see
/// the module doc for why all of this (not just the per-series marks) is
/// this file's own responsibility.
pub(crate) fn render(ctx: &SeriesRenderContext, opts: &RadarOptions) -> Element {
    // See the module doc's "Hover" section for why a second `use_chart()`
    // call here (not `ctx.active_index`) is correct and safe.
    let mut active_index = use_chart().active_index;

    let n = ctx.data.len();
    let cx = ctx.width / 2.0;
    let cy = ctx.height / 2.0;
    let half_extent = ctx.width.min(ctx.height) / 2.0;
    // Leave room for the rim category labels outside the outermost ring.
    let label_margin = if opts.axis_labels { 24.0 } else { 0.0 };
    let outer_radius =
        ((half_extent - label_margin).max(0.0) * opts.outer_radius.clamp(0.0, 1.0)).max(1.0);

    let angles = category_angles(n);
    let angle_step = if n > 0 {
        std::f64::consts::TAU / n as f64
    } else {
        0.0
    };

    // The value domain across every series' every value -- always includes
    // 0.0 (`nice_domain`), same reasoning as a bar/area chart's y-axis: a
    // radar's radius must never float away from zero.
    let (lo, hi) = ctx
        .data
        .iter()
        .flat_map(|d| d.values.iter().flatten().copied())
        .fold((0.0f64, 0.0f64), |(lo, hi), v| (lo.min(v), hi.max(v)));
    let domain = nice_domain(lo, hi);
    let scale = radial_scale(domain, outer_radius);

    let fill_opacity = if opts.lines_only {
        0.0
    } else {
        opts.fill_opacity
    };

    // Grid ring radii (pixels) + spokes/shape/fill flags, per `opts.grid`.
    let (ring_values, draw_spokes, ring_is_circle, ring_fill): (Vec<f64>, bool, bool, bool) =
        match &opts.grid {
            RadarGrid::None => (Vec::new(), false, false, false),
            RadarGrid::Polygon => (nice_ring_values(&scale), true, false, false),
            RadarGrid::Circle => (nice_ring_values(&scale), true, true, false),
            RadarGrid::CircleFill => (nice_ring_values(&scale), true, true, true),
            RadarGrid::CircleNoLines => (nice_ring_values(&scale), false, true, false),
            RadarGrid::Fill => (nice_ring_values(&scale), true, false, true),
            RadarGrid::Custom { values, spokes } => (values.clone(), *spokes, false, false),
        };
    let ring_radii: Vec<f64> = ring_values.iter().map(|v| scale.scale(*v)).collect();
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
                            let (x, y) = point_radial(angle, outer_radius + 12.0);
                            rsx! {
                                text {
                                    key: "{i}",
                                    "data-index": "{i}",
                                    x: "{fmt_num(x)}",
                                    y: "{fmt_num(y)}",
                                    "text-anchor": "middle",
                                    "{datum.label}"
                                }
                            }
                        }
                    }
                }
            }

            for (s , series) in ctx.config.series.iter().enumerate() {
                {
                    let values: Vec<Option<f64>> = ctx.series_values(s);
                    let d = radar_series_path(&values, &scale);
                    rsx! {
                        g {
                            key: "{series.key}",
                            "data-series": "{series.slot()}",
                            style: "--series-color: var(--color-{series.slot()})",
                            path {
                                "data-slot": "chart-radar-area",
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
                                                    r: "3",
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

            // The whole hover mechanism -- one angular wedge per category,
            // no coordinate math (`$S/chart-api.md`'s rule for the
            // Cartesian hit-bands, reused here for a wedge instead of a
            // band; see `engine::radar::sector_path`'s own doc). Drawn
            // last (on top) so a wedge always wins the pointer over any
            // mark beneath it, same DOM-order convention as the Cartesian
            // families' own `chart-hit-bands` group.
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
                                onpointerenter: move |_| active_index.set(Some(i)),
                            }
                        }
                    }
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
                values: vec![50.0],
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
            fill_opacity: 0.6, // deliberately non-zero -- lines_only must still win
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
}
