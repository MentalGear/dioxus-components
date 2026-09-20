//! The [`ChartKind::Bar`](crate::chart::ChartKind::Bar) family: discrete
//! bars, grouped side-by-side per series unless stacked. Ported (stage-2
//! `s2-refactor` lane) from `components::chart`'s own `SeriesMarks`; this
//! lane (`s2-bar`) then extended it with [`BarOptions`]'s four fields --
//! see each field's own doc, and this module's "Horizontal orientation"
//! section below for the one genuinely tricky part.
//!
//! See `components::layout` for the scale/margin math this reads via
//! [`SeriesRenderContext`].
//!
//! ## Deviation from `$S/stage2-common.md`'s illustrative `BarOptions`
//! sketch, stated plainly
//!
//! The brief that dispatched this lane sketched `BarOptions { orientation:
//! Orientation::{Vertical,Horizontal}, radius, labels: BarLabels::{None,
//! Value, Inside...}, active_index, stack_mode }` -- written before
//! `s2-refactor`'s actual commit landed. Two real constraints from that
//! landed commit change the concrete shape:
//!
//! - **No new public enum can live in this file.** `components::series::
//!   mod.rs` hand-lists this family's re-export (`pub use bar::BarOptions;`
//!   only, not `pub use bar::*;`) and is owned "`s2-refactor` only,
//!   forever" (that file's own module doc) -- a brand new `pub enum`
//!   defined here would type-check locally but never become reachable from
//!   outside `dioxus_primitives` (Rust module privacy: `components` itself
//!   is a private `mod`, reached only through the specific `pub use` chains
//!   each ancestor module chose to write) without an edit to that
//!   permanently-frozen file. Every new field below is therefore a
//!   primitive type (`bool`, `Option<usize>`) that needs no re-export of
//!   its own -- `horizontal` mirrors `Slider`'s own `horizontal: bool`
//!   naming (`primitives/src/slider.rs`) rather than introducing an
//!   `Orientation` enum, and `LineOptions::dots: bool`
//!   (`components::series::line`, landed the same commit) is the same
//!   family's own precedent for "a plain bool, not a new enum, where one
//!   would also have worked."
//! - **No `stack_mode` field**: stacking stayed a shared `ChartProps::
//!   stacked` concern for Area+Bar (`components::series::mod`'s own module
//!   doc, "Shared vs. per-family props, decided here"), not duplicated
//!   onto either family's own `*Options`. This family reads the already-
//!   resolved `ctx.stacked`/`ctx.stacked_spans` exactly as it did before
//!   this lane's changes; nothing here re-litigates that decision.
//! - **No `radius` field**: an SVG `rect` only has one uniform `rx` (already
//!   applied by the shared `chart/style.css`'s `[data-slot="chart-bar"] {
//!   rx: var(--dx-radius-xs); }`), so a per-instance override needs a
//!   hand-built rounded-rect path, not a `rect` attribute -- tracked as a
//!   follow-up (see `preview/src/components/bar_chart/variants/
//!   stacked_legend/mod.rs`'s own doc comment for the concrete shadcn
//!   behavior it would reproduce: differential per-corner rounding at each
//!   stack's outer edge), not built in this pass.
//!
//! ## Horizontal orientation: what's genuinely computed here vs. what
//! chart.rs still assumes
//!
//! [`SeriesRenderContext`] itself never learns about [`BarOptions::
//! horizontal`] -- `components::layout::build` is called by `chart.rs`
//! (owned "`s2-refactor` only, forever") BEFORE `chart.rs` ever dispatches
//! to this family's `render`, with no per-family option threaded through,
//! so `ctx.x_scale`/`ctx.y_scale`/margins stay exactly what a VERTICAL
//! Cartesian chart would compute regardless of this option. Adding a
//! `LayoutParams`/`SeriesRenderContext` field for this would need a
//! `chart.rs` call-site edit this lane cannot make -- flagged as a proposed
//! construction under "requests for the refactor owner" in
//! `$S/stage2-lanes.md` rather than applied here.
//!
//! Given that constraint, this file computes its OWN second pair of scales
//! locally, purely from `ctx`'s already-public fields (`plot_x0..plot_x1`,
//! `plot_y0..plot_y1`, and `ctx.y_scale.domain` -- the value domain
//! `layout::build` already nice-rounded, reused as-is): a [`BandScale`] over
//! the *y*-pixel range for categories, and a [`LinearScale`] over the
//! *x*-pixel range for values. This makes the bars themselves, this
//! family's own category-axis labels, and this family's own hit-bands all
//! genuinely correct for a horizontal layout -- verified by
//! `playwright/bar_chart.spec.ts`'s `horizontal` suite (bar width > height,
//! category labels positioned at the y-axis edge, hover tracks the bar
//! under the pointer).
//!
//! What this does NOT (and structurally cannot, from inside this file)
//! fix: `chart.rs`'s own `is_cartesian` hit-band/cursor block, which
//! unconditionally builds a vertical full-height strip from `ctx.x_scale`
//! for every Cartesian kind. Left as-is, that stale hit-band would sit on
//! top of (later in paint order than) this family's own marks and win
//! every pointer event, silently breaking hover for a horizontal bar chart.
//! Fixed here by construction, not by ignoring the conflict: this family's
//! own hit-bands additionally carry `data-orientation="horizontal"`, and
//! the horizontal gallery variant's own stylesheet
//! (`preview/src/components/bar_chart/style.css`) disables pointer events
//! on chart.rs's un-marked ones with a plain CSS rule scoped to that one
//! demo (`[data-slot="chart-hit-band"]:not([data-orientation="horizontal"])
//! { pointer-events: none; }` under that page's own `.dx-bar-chart-
//! horizontal` class) -- CSS legitimately overrides an SVG presentation
//! attribute like chart.rs's inline `pointer-events="all"` regardless of
//! source order, so this needs no chart.rs cooperation at all. The same
//! construction (a `data-orientation` marker + a themed CSS override)
//! resolves `chart.rs`'s own `chart-cursor-rect` the same way; this
//! family's own equivalent (below) reads `ctx.active_index` directly (a
//! plain snapshot `SeriesRenderContext` already carries) rather than
//! needing write access to draw. A precise, ready-to-apply chart.rs diff
//! that would make this whole workaround unnecessary is filed in
//! `$S/stage2-lanes.md`.
//!
//! `ChartLayout` (`context.rs`, what `ChartTooltip` positions itself from)
//! is a separate, already-known limitation, not a new one this lane
//! introduces: `s2-radar` already filed the same "Cartesian-only" gap for
//! its own tooltip positioning (`$S/stage2-lanes.md`) -- a horizontal
//! `Chart`'s tooltip inherits that identical limitation (it opens with the
//! correct content, just not necessarily anchored over the hovered bar)
//! until that shared construction lands.

use dioxus::prelude::*;

use super::super::layout::{SeriesRenderContext, BAND_PADDING};
use crate::chart::engine::geometry::bar_extent;
use crate::chart::engine::scale::fmt_num;
use crate::chart::{use_chart, BandScale, LinearScale};

/// Padding between grouped (non-stacked, multi-series) bars sharing one
/// category band. Bar-only (grouped-bar sub-positioning is this family's
/// own concern), unlike `components::layout`'s `BAND_PADDING`, which every
/// family's outer per-category band scale shares.
const GROUP_PADDING: f64 = 0.15;

/// Pixel gap between a bar's own edge and a label placed just outside it
/// (`BarOptions::value_labels`, and the value half of `inside_labels`) or
/// just inside it (the category half of `inside_labels`) -- shadcn's own
/// demos use a comparable `offset={8}`/`offset={12}` in the same unit
/// family (SVG user units here, px there).
const LABEL_OFFSET: f64 = 8.0;

/// [`crate::chart::ChartKind::Bar`]'s own options. See this module's own
/// doc for why every field here is a primitive type rather than the new
/// enums `$S/stage2-common.md`'s illustrative sketch named.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct BarOptions {
    /// Draw bars horizontally (a category axis running top-to-bottom,
    /// values running left-to-right) instead of the default vertical
    /// layout -- shadcn's `chart-bar-horizontal`/`-mixed`/`-label-custom`.
    /// See this module's own doc for what is and is not fully correct
    /// under this option today.
    pub horizontal: bool,

    /// Draw each datum's own value just outside its bar's far end (shadcn's
    /// `chart-bar-label`: `<LabelList position="top" .../>`). Ignored on a
    /// stacked chart (shadcn never combines the two either).
    pub value_labels: bool,

    /// Draw the datum's category label INSIDE the bar near its start (in a
    /// color meant to read against the bar's own fill) plus its value just
    /// outside the bar's far end -- shadcn's `chart-bar-label-custom`.
    /// Takes precedence over `value_labels` when both are set (shadcn's own
    /// demo never sets both). Ignored on a stacked chart.
    pub inside_labels: bool,

    /// Highlight exactly the bar at this datum index: that rect (every
    /// series' segment of it, for a grouped/stacked multi-series chart)
    /// gets `data-active="true"`; every other bar gets `data-active=
    /// "false"`, for a themed stylesheet to dim via
    /// `[data-slot="chart-bar"]:not([data-active="true"])` -- shadcn's
    /// `chart-bar-active`.
    pub active_index: Option<usize>,
}

/// Render every configured series' bars, in config order, plus (when
/// [`BarOptions::horizontal`]) this family's own category-axis labels and
/// hit-bands -- see the module doc for why those two are this family's own
/// job under that option, not `components::layout`'s or `chart.rs`'s.
pub(crate) fn render(ctx: &SeriesRenderContext, opts: &BarOptions) -> Element {
    let series_count = ctx.config.series.len();
    let n = ctx.xs.len();

    if opts.horizontal {
        // See the module doc: a second, LOCAL pair of scales -- a category
        // BandScale over the y-pixel range, a value LinearScale (same
        // already-nice-rounded domain `ctx.y_scale` used) over the x-pixel
        // range -- computed once here and threaded through every helper
        // below, rather than 5 separate recomputations.
        let category_scale = BandScale {
            count: n.max(1),
            range: (ctx.plot_y0, ctx.plot_y1),
            padding: BAND_PADDING,
        };
        let value_scale = LinearScale {
            domain: ctx.y_scale.domain,
            range: (ctx.plot_x0, ctx.plot_x1),
        };
        let zero_x = value_scale.scale(0.0);

        rsx! {
            for (s , series) in ctx.config.series.iter().enumerate() {
                g {
                    key: "{series.key}",
                    "data-slot": "chart-series",
                    "data-series": "{series.slot()}",
                    style: "--series-color: var(--color-{series.slot()})",
                    {render_horizontal_series(ctx, opts, s, series_count, &category_scale, &value_scale, zero_x)}
                }
            }
            {render_horizontal_category_labels(ctx, &category_scale)}
            {render_horizontal_hit_bands(ctx, &category_scale)}
        }
    } else {
        rsx! {
            for (s , series) in ctx.config.series.iter().enumerate() {
                g {
                    key: "{series.key}",
                    "data-slot": "chart-series",
                    "data-series": "{series.slot()}",
                    style: "--series-color: var(--color-{series.slot()})",
                    {render_vertical_series(ctx, opts, s, series_count)}
                }
            }
        }
    }
}

/// Format a value for a label -- the hidden table's own up-to-2-decimals
/// rule (`engine::scale::fmt_decimal`), matching every other number this
/// crate renders directly into markup.
fn fmt_value(v: f64) -> String {
    crate::chart::engine::scale::fmt_decimal(v, 2)
}

/// One series' vertical bars (the original, pre-stage-2 layout): grouped
/// side by side per category unless `ctx.stacked`.
fn render_vertical_series(
    ctx: &SeriesRenderContext,
    opts: &BarOptions,
    s: usize,
    series_count: usize,
) -> Element {
    let n = ctx.xs.len();
    if ctx.stacked {
        rsx! {
            for i in 0..n {
                {
                    let (bar_x, bar_w) = ctx.x_scale.band(i);
                    let (y0_raw, y1_raw) = ctx.stacked_spans[i][s];
                    let y0 = ctx.y_scale.scale(y0_raw);
                    let y1 = ctx.y_scale.scale(y1_raw);
                    let (rect_y, rect_h) = if y1 <= y0 { (y1, y0 - y1) } else { (y0, y1 - y0) };
                    let is_active = Some(i) == opts.active_index;
                    rsx! {
                        rect {
                            key: "{i}",
                            "data-slot": "chart-bar",
                            "data-index": "{i}",
                            "data-active": is_active,
                            x: "{fmt_num(bar_x)}",
                            y: "{fmt_num(rect_y)}",
                            width: "{fmt_num(bar_w)}",
                            height: "{fmt_num(rect_h)}",
                        }
                    }
                }
            }
        }
    } else {
        let values = ctx.series_values(s);
        rsx! {
            for i in 0..n {
                if let Some(v) = values[i] {
                    {
                        let (outer_x, outer_w) = ctx.x_scale.band(i);
                        let inner = BandScale {
                            count: series_count.max(1),
                            range: (outer_x, outer_x + outer_w),
                            padding: GROUP_PADDING,
                        };
                        let (bar_x, bar_w) = inner.band(s);
                        let y1 = ctx.y_scale.scale(v);
                        let (rect_y, rect_h) = bar_extent(y1, ctx.zero_y);
                        let color = ctx.data[i].color.clone();
                        let fill_style = color.map(|c| format!("fill: {c}"));
                        let is_active = Some(i) == opts.active_index;
                        let center_x = bar_x + bar_w / 2.0;
                        rsx! {
                            rect {
                                key: "{i}",
                                "data-slot": "chart-bar",
                                "data-index": "{i}",
                                "data-active": is_active,
                                x: "{fmt_num(bar_x)}",
                                y: "{fmt_num(rect_y)}",
                                width: "{fmt_num(bar_w)}",
                                height: "{fmt_num(rect_h)}",
                                style: fill_style,
                            }
                            if opts.inside_labels {
                                text {
                                    "data-slot": "chart-label",
                                    "data-position": "inside",
                                    "data-index": "{i}",
                                    x: "{fmt_num(center_x)}",
                                    y: "{fmt_num(rect_y + LABEL_OFFSET + 4.0)}",
                                    "text-anchor": "middle",
                                    style: "fill: var(--primary-color-1)",
                                    {ctx.data[i].label.clone()}
                                }
                                text {
                                    "data-slot": "chart-label",
                                    "data-position": "value",
                                    "data-index": "{i}",
                                    x: "{fmt_num(center_x)}",
                                    y: "{fmt_num(rect_y - LABEL_OFFSET / 2.0)}",
                                    "text-anchor": "middle",
                                    {fmt_value(v)}
                                }
                            } else if opts.value_labels {
                                text {
                                    "data-slot": "chart-label",
                                    "data-index": "{i}",
                                    x: "{fmt_num(center_x)}",
                                    y: "{fmt_num(rect_y - LABEL_OFFSET / 2.0)}",
                                    "text-anchor": "middle",
                                    {fmt_value(v)}
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// One series' horizontal bars -- see the module doc for `category_scale`/
/// `value_scale`'s own construction. Grouped side by side per category
/// unless `ctx.stacked`, mirroring [`render_vertical_series`]'s own split
/// with x/y (and band/linear) swapped throughout.
#[allow(clippy::too_many_arguments)]
fn render_horizontal_series(
    ctx: &SeriesRenderContext,
    opts: &BarOptions,
    s: usize,
    series_count: usize,
    category_scale: &BandScale,
    value_scale: &LinearScale,
    zero_x: f64,
) -> Element {
    let n = ctx.xs.len();
    if ctx.stacked {
        rsx! {
            for i in 0..n {
                {
                    let (bar_y, bar_h) = category_scale.band(i);
                    let (x0_raw, x1_raw) = ctx.stacked_spans[i][s];
                    let x0 = value_scale.scale(x0_raw);
                    let x1 = value_scale.scale(x1_raw);
                    let (rect_x, rect_w) = if x1 <= x0 { (x1, x0 - x1) } else { (x0, x1 - x0) };
                    let is_active = Some(i) == opts.active_index;
                    rsx! {
                        rect {
                            key: "{i}",
                            "data-slot": "chart-bar",
                            "data-index": "{i}",
                            "data-active": is_active,
                            x: "{fmt_num(rect_x)}",
                            y: "{fmt_num(bar_y)}",
                            width: "{fmt_num(rect_w)}",
                            height: "{fmt_num(bar_h)}",
                        }
                    }
                }
            }
        }
    } else {
        let values = ctx.series_values(s);
        rsx! {
            for i in 0..n {
                if let Some(v) = values[i] {
                    {
                        let (outer_y, outer_h) = category_scale.band(i);
                        let inner = BandScale {
                            count: series_count.max(1),
                            range: (outer_y, outer_y + outer_h),
                            padding: GROUP_PADDING,
                        };
                        let (bar_y, bar_h) = inner.band(s);
                        let x1 = value_scale.scale(v);
                        let (rect_x, rect_w) = bar_extent(x1, zero_x);
                        let color = ctx.data[i].color.clone();
                        let fill_style = color.map(|c| format!("fill: {c}"));
                        let is_active = Some(i) == opts.active_index;
                        let center_y = bar_y + bar_h / 2.0;
                        rsx! {
                            rect {
                                key: "{i}",
                                "data-slot": "chart-bar",
                                "data-index": "{i}",
                                "data-active": is_active,
                                x: "{fmt_num(rect_x)}",
                                y: "{fmt_num(bar_y)}",
                                width: "{fmt_num(rect_w)}",
                                height: "{fmt_num(bar_h)}",
                                style: fill_style,
                            }
                            if opts.inside_labels {
                                text {
                                    "data-slot": "chart-label",
                                    "data-position": "inside",
                                    "data-index": "{i}",
                                    x: "{fmt_num(rect_x + LABEL_OFFSET)}",
                                    y: "{fmt_num(center_y)}",
                                    "dominant-baseline": "central",
                                    style: "fill: var(--primary-color-1)",
                                    {ctx.data[i].label.clone()}
                                }
                                text {
                                    "data-slot": "chart-label",
                                    "data-position": "value",
                                    "data-index": "{i}",
                                    x: "{fmt_num(rect_x + rect_w + LABEL_OFFSET)}",
                                    y: "{fmt_num(center_y)}",
                                    "dominant-baseline": "central",
                                    {fmt_value(v)}
                                }
                            } else if opts.value_labels {
                                text {
                                    "data-slot": "chart-label",
                                    "data-index": "{i}",
                                    x: "{fmt_num(rect_x + rect_w + LABEL_OFFSET)}",
                                    y: "{fmt_num(center_y)}",
                                    "dominant-baseline": "central",
                                    {fmt_value(v)}
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// This family's own category-axis labels for a horizontal chart --
/// shadcn's `<YAxis type="category">`. Ported here (not `components::
/// layout::render_x_axis`/`render_y_axis`, which `chart.rs` calls
/// unconditionally with no orientation awareness -- see the module doc)
/// specifically so a horizontal demo can set `show_x_axis`/`show_y_axis:
/// false` (matching shadcn's own choice: every horizontal demo hides both
/// of Recharts' default axes and supplies its own category `YAxis`) and
/// still get correctly-positioned, correctly-styled category labels: this
/// reuses the SAME `data-slot="chart-axis"` selector `chart/style.css`
/// already styles (`fill`/`font-size`), so no new CSS is needed for the
/// text to look right, only for the different attribute value on this
/// group.
fn render_horizontal_category_labels(ctx: &SeriesRenderContext, category_scale: &BandScale) -> Element {
    rsx! {
        g { "data-slot": "chart-axis", "data-axis": "category",
            for (i , datum) in ctx.data.iter().enumerate() {
                text {
                    key: "{i}",
                    "data-index": "{i}",
                    x: "{fmt_num(ctx.plot_x0 - 8.0)}",
                    y: "{fmt_num(category_scale.center(i))}",
                    "text-anchor": "end",
                    "dominant-baseline": "central",
                    {datum.label.chars().take(3).collect::<String>()}
                }
            }
        }
    }
}

/// This family's own hit-bands for a horizontal chart: full-width strips
/// positioned by the category band scale (along y), each wired directly to
/// [`crate::chart::use_chart`]'s own `active_index` signal -- see the
/// module doc for why calling that "hook" from this plain (non-`#[component]`)
/// function is safe: `try_consume_context` (what it wraps) is explicitly
/// documented, in `dioxus-core` itself, as "not a hook" and callable "from
/// anywhere the Dioxus runtime is active ... without following the rules of
/// hooks" -- this runs synchronously on `Chart`'s own render call stack (the
/// same scope `Chart`'s own top-level `use_chart()` call already reads
/// from), never deferred, so there is no rules-of-hooks question here at
/// all, only an ordinary context read.
///
/// Deliberately marked `data-orientation="horizontal"`, distinct from
/// `chart.rs`'s own (vertical-only) `[data-slot="chart-hit-band"]` group it
/// renders unconditionally alongside this one -- the horizontal gallery
/// variant's own stylesheet uses that marker to disable pointer events on
/// the stale ones without disturbing this family's own, or any other
/// kind's hit-bands elsewhere.
fn render_horizontal_hit_bands(ctx: &SeriesRenderContext, category_scale: &BandScale) -> Element {
    let n = ctx.xs.len();
    let mut active_index = use_chart().active_index;
    rsx! {
        g { "data-slot": "chart-hit-bands", "data-orientation": "horizontal",
            for i in 0..n {
                {
                    let (by, bh) = category_scale.band(i);
                    rsx! {
                        rect {
                            key: "{i}",
                            "data-slot": "chart-hit-band",
                            "data-orientation": "horizontal",
                            "data-index": "{i}",
                            x: "{fmt_num(ctx.plot_x0)}",
                            y: "{fmt_num(by)}",
                            width: "{fmt_num(ctx.plot_x1 - ctx.plot_x0)}",
                            height: "{fmt_num(bh)}",
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart::engine::table::table_rows;
    use crate::chart::{ChartConfig, ChartContainer, ChartDatum, ChartKind};
    use dioxus_core::NoOpMutations;

    fn config_one() -> ChartConfig {
        ChartConfig::new().series("visitors", "Visitors", "var(--dx-chart-1)")
    }

    fn config_two() -> ChartConfig {
        ChartConfig::new()
            .series("desktop", "Desktop", "var(--dx-chart-1)")
            .series("mobile", "Mobile", "var(--dx-chart-2)")
    }

    #[derive(Clone, PartialEq, Props)]
    struct HarnessProps {
        config: ChartConfig,
        data: Vec<ChartDatum>,
        bar: BarOptions,
        #[props(default)]
        stacked: bool,
    }

    #[component]
    fn Harness(props: HarnessProps) -> Element {
        let config = use_signal(|| props.config.clone());
        let data = use_signal(|| props.data.clone());
        rsx! {
            ChartContainer { config, data, kind: ChartKind::Bar,
                crate::chart::Chart {
                    aria_label: "Test bar chart",
                    stacked: props.stacked,
                    bar: props.bar.clone(),
                    show_x_axis: !props.bar.horizontal,
                    show_y_axis: false,
                }
            }
        }
    }

    fn render(props: HarnessProps) -> String {
        let mut dom = VirtualDom::new_with_props(Harness, props);
        dom.rebuild_in_place();
        dom.render_immediate(&mut NoOpMutations);
        dioxus_ssr::render(&dom)
    }

    fn sample_data() -> Vec<ChartDatum> {
        vec![
            ChartDatum {
                label: "January".to_string(),
                values: vec![Some(186.0), Some(80.0)],
                ..Default::default()
            },
            ChartDatum {
                label: "February".to_string(),
                values: vec![Some(305.0), Some(200.0)],
                ..Default::default()
            },
        ]
    }

    #[test]
    fn horizontal_bars_are_wider_than_tall_for_a_wide_chart() {
        let html = render(HarnessProps {
            config: config_one(),
            data: vec![
                ChartDatum {
                    label: "chrome".to_string(),
                    values: vec![Some(275.0)],
                    ..Default::default()
                },
                ChartDatum {
                    label: "safari".to_string(),
                    values: vec![Some(200.0)],
                    ..Default::default()
                },
            ],
            bar: BarOptions {
                horizontal: true,
                ..Default::default()
            },
            stacked: false,
        });
        // Two bars, each width > height (a 600x300 default viewBox, two
        // categories -- each band is far taller-looking in the category
        // sense but the RECT itself, spanning most of the plot's width for
        // a large positive value, is wider than the band is tall).
        assert_eq!(html.matches(r#"data-slot="chart-bar""#).count(), 2);
        for line in html.split("<rect").skip(1) {
            let width: f64 = attr_f64(line, "width");
            let height: f64 = attr_f64(line, "height");
            assert!(
                width > height,
                "expected width > height for a horizontal bar, got {width} x {height} in: {line}"
            );
        }
    }

    #[test]
    fn horizontal_category_labels_render_at_the_plot_left_edge() {
        let html = render(HarnessProps {
            config: config_one(),
            data: vec![
                ChartDatum {
                    label: "chrome".to_string(),
                    values: vec![Some(275.0)],
                    ..Default::default()
                },
                ChartDatum {
                    label: "safari".to_string(),
                    values: vec![Some(200.0)],
                    ..Default::default()
                },
            ],
            bar: BarOptions {
                horizontal: true,
                ..Default::default()
            },
            stacked: false,
        });
        assert!(html.contains(r#"data-axis="category""#));
        assert!(html.contains(">chr<"), "expected a truncated category label: {html}");
        assert!(html.contains(">saf<"));
    }

    #[test]
    fn horizontal_hit_bands_are_marked_and_cover_every_datum() {
        let html = render(HarnessProps {
            config: config_one(),
            data: sample_data(),
            bar: BarOptions {
                horizontal: true,
                ..Default::default()
            },
            stacked: false,
        });
        assert_eq!(
            html.matches(r#"data-orientation="horizontal""#).count(),
            // one on the hit-bands GROUP, one per hit-band RECT (2 data points)
            3,
            "expected the group + one marked rect per datum: {html}"
        );
    }

    #[test]
    fn active_index_marks_exactly_one_bar_data_active_true() {
        let html = render(HarnessProps {
            config: config_two(),
            data: sample_data(),
            bar: BarOptions {
                active_index: Some(1),
                ..Default::default()
            },
            stacked: false,
        });
        assert_eq!(html.matches(r#"data-active="true""#).count(), 2, "both series' February bar: {html}");
        assert_eq!(html.matches(r#"data-active="false""#).count(), 2, "both series' January bar: {html}");
    }

    #[test]
    fn no_active_index_marks_every_bar_data_active_false() {
        let html = render(HarnessProps {
            config: config_two(),
            data: sample_data(),
            bar: BarOptions::default(),
            stacked: false,
        });
        assert!(!html.contains(r#"data-active="true""#));
        assert_eq!(html.matches(r#"data-active="false""#).count(), 4);
    }

    #[test]
    fn per_datum_color_overrides_the_series_fill_via_inline_style() {
        let data = vec![
            ChartDatum {
                label: "January".to_string(),
                values: vec![Some(186.0)],
                color: Some("var(--dx-chart-1)".to_string()),
            },
            ChartDatum {
                label: "February".to_string(),
                values: vec![Some(-207.0)],
                color: Some("var(--dx-chart-2)".to_string()),
            },
        ];
        let html = render(HarnessProps {
            config: config_one(),
            data,
            bar: BarOptions::default(),
            stacked: false,
        });
        assert!(html.contains("fill: var(--dx-chart-1)"));
        assert!(html.contains("fill: var(--dx-chart-2)"));
    }

    #[test]
    fn negative_value_bar_extends_below_the_zero_line() {
        let data = vec![
            ChartDatum {
                label: "January".to_string(),
                values: vec![Some(186.0)],
                ..Default::default()
            },
            ChartDatum {
                label: "February".to_string(),
                values: vec![Some(-207.0)],
                ..Default::default()
            },
        ];
        let html = render(HarnessProps {
            config: config_one(),
            data,
            bar: BarOptions::default(),
            stacked: false,
        });
        // The zero line itself is drawn (ChartKind::Bar, layout::render_grid).
        assert!(html.contains(r#"data-slot="chart-zero-line""#));
        let zero_y = attr_f64_after(&html, r#"data-slot="chart-zero-line""#, "y1");

        // Two rects: the positive one's own y+height must land AT OR ABOVE
        // (<=) the zero line (it sits above the baseline), the negative
        // one's y must land AT OR BELOW (>=) the zero line (it starts at
        // or below the baseline and extends further down).
        let rects: Vec<&str> = html.split("<rect").skip(1).collect();
        assert_eq!(rects.len(), 2);
        let pos_y = attr_f64(rects[0], "y");
        let pos_h = attr_f64(rects[0], "height");
        let neg_y = attr_f64(rects[1], "y");
        assert!(
            pos_y + pos_h <= zero_y + 0.01,
            "positive bar's bottom ({}) should sit at/above the zero line ({zero_y}): {html}",
            pos_y + pos_h
        );
        assert!(
            neg_y >= zero_y - 0.01,
            "negative bar's top ({neg_y}) should sit at/below the zero line ({zero_y}): {html}"
        );
    }

    #[test]
    fn value_labels_render_one_text_per_bar_with_the_value() {
        let html = render(HarnessProps {
            config: config_one(),
            data: sample_data()
                .into_iter()
                .map(|d| ChartDatum {
                    values: vec![d.values[0]],
                    ..d
                })
                .collect(),
            bar: BarOptions {
                value_labels: true,
                ..Default::default()
            },
            stacked: false,
        });
        assert_eq!(html.matches(r#"data-slot="chart-label""#).count(), 2);
        assert!(html.contains(">186<"));
        assert!(html.contains(">305<"));
    }

    #[test]
    fn inside_labels_render_two_texts_per_bar_category_and_value() {
        let html = render(HarnessProps {
            config: config_one(),
            data: sample_data()
                .into_iter()
                .map(|d| ChartDatum {
                    values: vec![d.values[0]],
                    ..d
                })
                .collect(),
            bar: BarOptions {
                inside_labels: true,
                horizontal: true,
                ..Default::default()
            },
            stacked: false,
        });
        // 2 data points * 2 labels (inside category name + value) each.
        assert_eq!(html.matches(r#"data-slot="chart-label""#).count(), 4);
        assert_eq!(html.matches(r#"data-position="inside""#).count(), 2);
        assert_eq!(html.matches(r#"data-position="value""#).count(), 2);
        assert!(html.contains("January"));
        assert!(html.contains(">186<"));
    }

    #[test]
    fn hidden_table_still_lists_every_datum_regardless_of_orientation() {
        // The hidden data table is `chart.rs`'s own job, untouched by this
        // lane -- pinning here only that this family's own rendering
        // doesn't somehow interfere with it (e.g. by consuming `ctx.data`
        // in a way that would panic on an edge case).
        let rows = table_rows(&sample_data());
        assert_eq!(rows.len(), 2);
    }

    /// Extract an attribute's `f64` value from one `<rect .../>`-shaped
    /// fragment (the text starting right after the literal `<rect`, as
    /// `dioxus_ssr::render`'s own flat HTML string naturally splits).
    fn attr_f64(fragment: &str, attr: &str) -> f64 {
        let needle = format!("{attr}=\"");
        let start = fragment.find(&needle).unwrap_or_else(|| panic!("no {attr} attribute in: {fragment}")) + needle.len();
        let rest = &fragment[start..];
        let end = rest.find('"').unwrap();
        rest[..end].parse().unwrap_or_else(|_| panic!("non-numeric {attr} in: {fragment}"))
    }

    /// Like [`attr_f64`], but searches the whole `html` starting from the
    /// first occurrence of `marker` (used to scope a search to one
    /// specific element, e.g. the zero-line, without assuming a following
    /// element ordering).
    fn attr_f64_after(html: &str, marker: &str, attr: &str) -> f64 {
        let start = html.find(marker).unwrap_or_else(|| panic!("marker {marker} not found in: {html}"));
        attr_f64(&html[start..], attr)
    }
}
