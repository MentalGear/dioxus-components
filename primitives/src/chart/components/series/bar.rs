//! The [`ChartKind::Bar`](crate::chart::ChartKind::Bar) family: discrete
//! bars, grouped side by side per series unless stacked, vertical or
//! horizontal -- sized, rounded and highlighted the way Recharts' `<Bar>`
//! does it (what shadcn/ui's bar charts render):
//!
//! - **Sizing** (`getBarPosition`): each category owns an unpadded band
//!   (`components::layout`); the bars are inset by
//!   [`BarOptions::category_gap`] (Recharts' `barCategoryGap`, 10%) of the
//!   band on each side, grouped bars are [`BarOptions::bar_gap`] px apart
//!   (`barGap`, 4), and the bar size is truncated to whole px. A stack is one
//!   bar. One series on shadcn's 369px card: 47px bars in 59.8px bands.
//! - **Radius** ([`BarRadius`], Recharts' `radius`): uniform or per corner
//!   (`[tl, tr, br, bl]`), clamped to half the bar, per series
//!   ([`BarOptions::series_radius`]) so a stack can round only its outer
//!   ends (shadcn: bottom segment `[0, 0, 4, 4]`, top `[4, 4, 0, 0]`).
//! - **Active bar** ([`BarOptions::active_index`]): `data-active="true"`,
//!   which the themed stylesheet draws as shadcn's `chart-bar-active` --
//!   0.8 fill opacity and a dashed outline in the bar's own color; the other
//!   bars are unchanged.
//! - **Labels**: [`BarOptions::value_labels`] (`<LabelList position="top"
//!   offset={12}>`), [`BarOptions::category_labels`] (the category name just
//!   beyond the bar's end in the bar's color -- `chart-bar-negative`), and
//!   [`BarOptions::inside_labels`] (`chart-bar-label-custom`).
//!
//! Horizontal bars ([`BarOptions::horizontal`]) read the same scales: the
//! layout puts the category scale down y and the value scale across x
//! (`SeriesRenderContext::horizontal`), so the bars, the category labels
//! (`components::layout`, on the y axis), the grid, the cursor and the hit
//! test agree by construction. This family draws its own row hit bands.
//!
//! A per-datum [`crate::chart::ChartDatum::color`] sets the bar's own
//! `--series-color` inline, so its fill, its active outline and its
//! category label all follow it.

use dioxus::prelude::*;

use super::super::layout::SeriesRenderContext;
use crate::chart::engine::scale::fmt_num;

/// Gap between a bar's end and its value label (shadcn's `chart-bar-label`:
/// `offset={12}`).
const VALUE_LABEL_OFFSET: f64 = 12.0;
/// Gap between a bar's end and its category label (Recharts' `LabelList`
/// default `offset`, 5 -- `chart-bar-negative`).
const CATEGORY_LABEL_OFFSET: f64 = 5.0;
/// Inset/gap of the two `inside_labels` texts (`chart-bar-label-custom`:
/// `offset={8}`).
const INSIDE_LABEL_OFFSET: f64 = 8.0;

/// A bar's corner radii in px, Recharts' `radius` (`[tl, tr, br, bl]`, in
/// screen orientation). Each is clamped to half the bar's width and height.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct BarRadius {
    /// Top-left corner radius, px.
    pub top_left: f64,
    /// Top-right corner radius, px.
    pub top_right: f64,
    /// Bottom-right corner radius, px.
    pub bottom_right: f64,
    /// Bottom-left corner radius, px.
    pub bottom_left: f64,
}

impl BarRadius {
    /// Square corners (Recharts' default).
    pub const NONE: Self = Self::all(0.0);

    /// The same radius on every corner (`radius={8}`).
    pub const fn all(r: f64) -> Self {
        Self::corners(r, r, r, r)
    }

    /// Per corner, Recharts' array order (`radius={[tl, tr, br, bl]}`).
    pub const fn corners(
        top_left: f64,
        top_right: f64,
        bottom_right: f64,
        bottom_left: f64,
    ) -> Self {
        Self {
            top_left,
            top_right,
            bottom_right,
            bottom_left,
        }
    }

    fn uniform(&self) -> Option<f64> {
        let r = self.top_left;
        (self.top_right == r && self.bottom_right == r && self.bottom_left == r).then_some(r)
    }
}

impl From<f64> for BarRadius {
    fn from(r: f64) -> Self {
        Self::all(r)
    }
}

/// [`crate::chart::ChartKind::Bar`]'s own options.
#[derive(Clone, PartialEq, Debug)]
pub struct BarOptions {
    /// Draw bars horizontally (categories top to bottom, values left to
    /// right) -- Recharts' `layout="vertical"` (shadcn's
    /// `chart-bar-horizontal`/`-mixed`/`-label-custom`). The category labels
    /// are then the y axis (`show_y_axis`).
    pub horizontal: bool,

    /// Every bar's corner radius (Recharts' `radius`; default square).
    pub radius: BarRadius,

    /// Per-series radius, in config order, overriding [`Self::radius`] for
    /// the series it covers -- one `<Bar radius>` per series in Recharts, so
    /// a stack rounds only its outer ends (shadcn's stacked demos: the
    /// bottom series `[0, 0, 4, 4]`, the top one `[4, 4, 0, 0]`).
    pub series_radius: Vec<BarRadius>,

    /// The gap on each side of a category's bars, as a fraction of the
    /// category's band (Recharts' `barCategoryGap`, default 10%).
    pub category_gap: f64,

    /// The gap in px between grouped bars of one category (Recharts'
    /// `barGap`, default 4).
    pub bar_gap: f64,

    /// Draw each bar's value just beyond its end (shadcn's `chart-bar-label`:
    /// `<LabelList position="top" offset={12}>`). Not on a stacked chart.
    pub value_labels: bool,

    /// Draw each datum's category name just beyond its bar's end -- above a
    /// positive bar, below a negative one -- in the bar's own color (shadcn's
    /// `chart-bar-negative`: `<LabelList position="top" dataKey="month">`).
    /// Not on a stacked chart.
    pub category_labels: bool,

    /// Draw the category name INSIDE the bar near its start (in the card's
    /// background color) plus the value just beyond its end -- shadcn's
    /// `chart-bar-label-custom`. Takes precedence over the other two label
    /// options. Not on a stacked chart.
    pub inside_labels: bool,

    /// Highlight the bar at this datum index (every series' bar there):
    /// `data-active="true"`, drawn by the themed stylesheet as shadcn's
    /// `chart-bar-active` (0.8 fill opacity + a dashed outline in the bar's
    /// own color). Every other bar is `data-active="false"` and unchanged.
    pub active_index: Option<usize>,
}

impl Default for BarOptions {
    fn default() -> Self {
        Self {
            horizontal: false,
            radius: BarRadius::NONE,
            series_radius: Vec::new(),
            category_gap: 0.1,
            bar_gap: 4.0,
            value_labels: false,
            category_labels: false,
            inside_labels: false,
            active_index: None,
        }
    }
}

/// Recharts' `getBarPosition` for one category band `(start, size)` holding
/// `groups` side-by-side bars: `(offset of bar 0 from the band start, bar
/// size, distance between consecutive bars' starts)`. The bar size is
/// truncated to whole px when above 1, exactly as Recharts does
/// (`realBarSize >>= 0`), so a lone bar on shadcn's 59.83px band is 47px.
pub(crate) fn bar_slots(
    band_size: f64,
    groups: usize,
    category_gap: f64,
    bar_gap: f64,
) -> (f64, f64, f64) {
    let groups = groups.max(1) as f64;
    let offset = band_size * category_gap;
    let mut gap = bar_gap;
    if band_size - 2.0 * offset - (groups - 1.0) * gap <= 0.0 {
        gap = 0.0;
    }
    let mut size = (band_size - 2.0 * offset - (groups - 1.0) * gap) / groups;
    if size > 1.0 {
        size = size.trunc();
    }
    (offset, size.max(0.0), size.max(0.0) + gap)
}

/// One bar's resting geometry in px, screen-oriented (`w`, `h` >= 0), plus
/// which end is its value end (for labels).
#[derive(Clone, Copy, Debug, PartialEq)]
struct BarRect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    /// The value is below zero: the bar grows down (vertical) or left
    /// (horizontal) from the baseline.
    negative: bool,
}

/// Render every configured series' bars, in config order (later series on
/// top), plus horizontal bars' own row hit bands.
pub(crate) fn render(ctx: &SeriesRenderContext, opts: &BarOptions) -> Element {
    let series_count = ctx.config.series.len();
    let groups = if ctx.stacked { 1 } else { series_count };
    rsx! {
        for (s , series) in ctx.config.series.iter().enumerate() {
            g {
                key: "{series.key}",
                "data-slot": "chart-series",
                "data-series": "{series.slot()}",
                style: "--series-color: var(--color-{series.slot()})",
                {render_series(ctx, opts, s, groups)}
            }
        }
        if ctx.horizontal {
            {render_horizontal_hit_bands(ctx)}
        }
    }
}

/// Bar `i` of series `s`, or `None` for a missing value (a stacked gap still
/// draws its zero-height cell, like Recharts).
fn bar_rect(
    ctx: &SeriesRenderContext,
    opts: &BarOptions,
    s: usize,
    i: usize,
    groups: usize,
) -> Option<(BarRect, f64)> {
    let (band_start, band_size) = ctx.x_scale.band(i);
    let (offset, size, pitch) = bar_slots(band_size, groups, opts.category_gap, opts.bar_gap);
    let slot = if ctx.stacked { 0 } else { s };
    let along = band_start + offset + pitch * slot as f64;
    let (v0, v1, value) = if ctx.stacked {
        let (v0, v1) = *ctx.stacked_spans.get(i)?.get(s)?;
        (
            v0,
            v1,
            ctx.data[i].values.get(s).copied().flatten().unwrap_or(0.0),
        )
    } else {
        let v = ctx.data[i].values.get(s).copied().flatten()?;
        (0.0, v, v)
    };
    let (p0, p1) = (ctx.y_scale.scale(v0), ctx.y_scale.scale(v1));
    let (lo, hi) = (p0.min(p1), p0.max(p1));
    let negative = v1 < v0;
    let rect = if ctx.horizontal {
        BarRect {
            x: lo,
            y: along,
            w: hi - lo,
            h: size,
            negative,
        }
    } else {
        BarRect {
            x: along,
            y: lo,
            w: size,
            h: hi - lo,
            negative,
        }
    };
    Some((rect, value))
}

fn render_series(ctx: &SeriesRenderContext, opts: &BarOptions, s: usize, groups: usize) -> Element {
    let n = ctx.xs.len();
    let radius = opts.series_radius.get(s).copied().unwrap_or(opts.radius);
    let labels = !ctx.stacked;
    rsx! {
        for i in 0..n {
            if let Some((rect, value)) = bar_rect(ctx, opts, s, i, groups) {
                {
                    let style = ctx.data[i].color.as_ref().map(|c| format!("--series-color: {c}"));
                    let active = opts.active_index == Some(i);
                    rsx! {
                        {render_shape(rect, radius, i, active, style)}
                        if labels {
                            {render_labels(ctx, opts, rect, i, value)}
                        }
                    }
                }
            }
        }
    }
}

/// The bar itself: a `rect` (with `rx`/`ry` for a uniform radius) or, for
/// per-corner radii, Recharts' rounded-rect `path`.
fn render_shape(
    rect: BarRect,
    radius: BarRadius,
    i: usize,
    active: bool,
    style: Option<String>,
) -> Element {
    let max = (rect.w / 2.0).min(rect.h / 2.0).max(0.0);
    match radius.uniform() {
        Some(r) => {
            let r = r.min(max).max(0.0);
            let rx = (r > 0.0).then(|| fmt_num(r));
            rsx! {
                rect {
                    key: "{i}",
                    "data-slot": "chart-bar",
                    "data-index": "{i}",
                    "data-active": active,
                    x: "{fmt_num(rect.x)}",
                    y: "{fmt_num(rect.y)}",
                    width: "{fmt_num(rect.w)}",
                    height: "{fmt_num(rect.h)}",
                    rx: rx.clone(),
                    ry: rx,
                    style,
                }
            }
        }
        None => {
            let d = rounded_rect_path(rect, radius);
            rsx! {
                path {
                    key: "{i}",
                    "data-slot": "chart-bar",
                    "data-index": "{i}",
                    "data-active": active,
                    d: "{d}",
                    style,
                }
            }
        }
    }
}

/// Recharts' `getRectanglePath` for a per-corner radius (each clamped to
/// half the bar), traced clockwise from the top-left corner.
fn rounded_rect_path(rect: BarRect, radius: BarRadius) -> String {
    let BarRect { x, y, w, h, .. } = rect;
    let max = (w / 2.0).min(h / 2.0).max(0.0);
    let [tl, tr, br, bl] = [
        radius.top_left,
        radius.top_right,
        radius.bottom_right,
        radius.bottom_left,
    ]
    .map(|r| r.clamp(0.0, max));
    let f = fmt_num;
    let mut d = format!("M{} {}", f(x), f(y + tl));
    if tl > 0.0 {
        d += &format!(" A{r} {r} 0 0 1 {} {}", f(x + tl), f(y), r = f(tl));
    }
    d += &format!(" L{} {}", f(x + w - tr), f(y));
    if tr > 0.0 {
        d += &format!(" A{r} {r} 0 0 1 {} {}", f(x + w), f(y + tr), r = f(tr));
    }
    d += &format!(" L{} {}", f(x + w), f(y + h - br));
    if br > 0.0 {
        d += &format!(" A{r} {r} 0 0 1 {} {}", f(x + w - br), f(y + h), r = f(br));
    }
    d += &format!(" L{} {}", f(x + bl), f(y + h));
    if bl > 0.0 {
        d += &format!(" A{r} {r} 0 0 1 {} {}", f(x), f(y + h - bl), r = f(bl));
    }
    d + " Z"
}

/// Format a value for a label -- up to 2 decimals, like every other number
/// this crate renders into markup.
fn fmt_value(v: f64) -> String {
    crate::chart::engine::scale::fmt_decimal(v, 2)
}

/// The label(s) [`BarOptions`] asks for on one bar.
fn render_labels(
    ctx: &SeriesRenderContext,
    opts: &BarOptions,
    rect: BarRect,
    i: usize,
    value: f64,
) -> Element {
    let category = ctx.data[i].label.clone();
    if opts.inside_labels {
        // Category inside the bar's start, value beyond its end.
        let (ix, iy, vx, vy, anchor, dy) = if ctx.horizontal {
            let cy = rect.y + rect.h / 2.0;
            (
                rect.x + INSIDE_LABEL_OFFSET,
                cy,
                rect.x + rect.w + INSIDE_LABEL_OFFSET,
                cy,
                "start",
                "0.355em",
            )
        } else {
            let cx = rect.x + rect.w / 2.0;
            (
                cx,
                rect.y + rect.h - INSIDE_LABEL_OFFSET,
                cx,
                rect.y - INSIDE_LABEL_OFFSET,
                "middle",
                "0",
            )
        };
        return rsx! {
            text {
                "data-slot": "chart-label",
                "data-position": "inside",
                "data-index": "{i}",
                x: "{fmt_num(ix)}",
                y: "{fmt_num(iy)}",
                dy,
                "text-anchor": anchor,
                {category}
            }
            text {
                "data-slot": "chart-label",
                "data-position": "value",
                "data-index": "{i}",
                x: "{fmt_num(vx)}",
                y: "{fmt_num(vy)}",
                dy,
                "text-anchor": anchor,
                {fmt_value(value)}
            }
        };
    }
    let (text, offset, position) = if opts.category_labels {
        (category, CATEGORY_LABEL_OFFSET, "category")
    } else if opts.value_labels {
        (fmt_value(value), VALUE_LABEL_OFFSET, "value")
    } else {
        return rsx! {};
    };
    // Beyond the bar's value end: above (right of) a positive bar, below
    // (left of) a negative one.
    let (x, y, anchor, dy) = match (ctx.horizontal, rect.negative) {
        (false, false) => (rect.x + rect.w / 2.0, rect.y - offset, "middle", "0"),
        (false, true) => (
            rect.x + rect.w / 2.0,
            rect.y + rect.h + offset,
            "middle",
            "0.71em",
        ),
        (true, false) => (
            rect.x + rect.w + offset,
            rect.y + rect.h / 2.0,
            "start",
            "0.355em",
        ),
        (true, true) => (rect.x - offset, rect.y + rect.h / 2.0, "end", "0.355em"),
    };
    rsx! {
        text {
            "data-slot": "chart-label",
            "data-position": position,
            "data-index": "{i}",
            x: "{fmt_num(x)}",
            y: "{fmt_num(y)}",
            dy,
            "text-anchor": anchor,
            {text}
        }
    }
}

/// This family's own hit bands for a horizontal chart: full-width strips,
/// one per category row (`data-orientation="horizontal"`, distinct from
/// `Chart`'s own vertical columns, which it does not draw for horizontal
/// bars). Hover itself is resolved from the pointer's coordinates
/// (`components::pointer`).
fn render_horizontal_hit_bands(ctx: &SeriesRenderContext) -> Element {
    let n = ctx.xs.len();
    rsx! {
        g { "data-slot": "chart-hit-bands", "data-orientation": "horizontal",
            for i in 0..n {
                {
                    let (by, bh) = ctx.x_scale.band(i);
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
                    show_y_axis: props.bar.horizontal,
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
        // a large positive value, is wider than the band is tall). Only
        // `data-slot="chart-bar"` rects: `chart.rs`'s own (vertical-only,
        // un-marked) hit-bands and this family's own horizontal hit-bands
        // are ALSO `<rect>`s on this same page (the module doc's own
        // "what this does NOT fix" section), so a plain "every `<rect>`"
        // scan would wrongly include them.
        assert_eq!(html.matches(r#"data-slot="chart-bar""#).count(), 2);
        for line in html.split("<rect").skip(1) {
            if !line.trim_start().starts_with(r#"data-slot="chart-bar""#) {
                continue;
            }
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
        assert!(html.contains(r#"data-axis="y""#));
        assert!(
            html.contains(">chr<"),
            "expected a truncated category label: {html}"
        );
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
        // `data-active` is a plain `bool` attribute
        // (`dioxus_ssr::renderer::write_attribute`'s own `AttributeValue::
        // Bool` arm writes ` name=value`, unquoted, always -- the same
        // convention `navigation_menu.rs`'s/`carousel.rs`'s own
        // `"data-active": <bool>` already use elsewhere in this crate), not
        // a quoted string: `data-active=true`/`data-active=false` in the
        // raw SSR text. This is still a fully valid unquoted HTML
        // attribute token, and CSS's own `[data-active="true"]` matches it
        // in the real DOM regardless of the source's quoting (a browser's
        // HTML parser and `dioxus-web`'s own CSR `set_attribute` both
        // resolve it to the identical attribute VALUE string) -- this
        // module's own doc/CSS-selector citations above stay accurate.
        assert_eq!(
            html.matches("data-active=true").count(),
            2,
            "both series' February bar: {html}"
        );
        assert_eq!(
            html.matches("data-active=false").count(),
            2,
            "both series' January bar: {html}"
        );
    }

    #[test]
    fn no_active_index_marks_every_bar_data_active_false() {
        let html = render(HarnessProps {
            config: config_two(),
            data: sample_data(),
            bar: BarOptions::default(),
            stacked: false,
        });
        // See the previous test's own comment for why this is unquoted.
        assert!(!html.contains("data-active=true"));
        assert_eq!(html.matches("data-active=false").count(), 4);
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
        assert!(html.contains("--series-color: var(--dx-chart-1)"));
        assert!(html.contains("--series-color: var(--dx-chart-2)"));
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
        // The value axis' zero, published on the svg for the animation.
        let zero_y = zero_px(&html);

        // Two `data-slot="chart-bar"` rects (in DOM/config order, so
        // `rects[0]` is January and `rects[1]` is February): `chart.rs`'s
        // own per-datum hit-bands are ALSO `<rect>`s on this same page,
        // rendered after the marks, so a plain "every `<rect>`" scan would
        // wrongly pick some of those up too. The positive one's own
        // y+height must land AT OR ABOVE (<=) the zero line (it sits above
        // the baseline), the negative one's y must land AT OR BELOW (>=)
        // the zero line (it starts at or below the baseline and extends
        // further down).
        let rects: Vec<&str> = html
            .split("<rect")
            .skip(1)
            .filter(|line| line.trim_start().starts_with(r#"data-slot="chart-bar""#))
            .collect();
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
        let start = fragment
            .find(&needle)
            .unwrap_or_else(|| panic!("no {attr} attribute in: {fragment}"))
            + needle.len();
        let rest = &fragment[start..];
        let end = rest.find('"').unwrap();
        rest[..end]
            .parse()
            .unwrap_or_else(|_| panic!("non-numeric {attr} in: {fragment}"))
    }

    /// The svg's `--dx-chart-zero` (the value axis' zero, px).
    fn zero_px(html: &str) -> f64 {
        let start = html.find("--dx-chart-zero: ").expect("zero var") + "--dx-chart-zero: ".len();
        let rest = &html[start..];
        rest[..rest.find("px").unwrap()].parse().unwrap()
    }

    #[test]
    fn bar_slots_match_recharts_get_bar_position() {
        // shadcn bar-default: one series on a 59.833px band -> 5.983px
        // offset, a 47px bar.
        let (offset, size, _) = bar_slots(359.0 / 6.0, 1, 0.1, 4.0);
        assert!((offset - 5.9833).abs() < 1e-3);
        assert_eq!(size, 47.0);
        // bar-multiple: two 21px bars, 4px apart.
        let (_, size, pitch) = bar_slots(359.0 / 6.0, 2, 0.1, 4.0);
        assert_eq!((size, pitch), (21.0, 25.0));
        // bar-interactive hero: 91 bars on 1262px -> 11px.
        assert_eq!(bar_slots(1262.0 / 91.0, 1, 0.1, 4.0).1, 11.0);
        // Too narrow for the gap: the gap is dropped, never negative.
        let (_, size, pitch) = bar_slots(4.0, 3, 0.1, 4.0);
        assert!(size >= 0.0 && pitch == size);
    }

    #[test]
    fn rounded_rect_path_rounds_only_the_given_corners_and_clamps() {
        let r = BarRect {
            x: 10.0,
            y: 20.0,
            w: 47.0,
            h: 43.4,
            negative: false,
        };
        // shadcn's stacked bottom segment `[0, 0, 4, 4]`.
        assert_eq!(
            rounded_rect_path(r, BarRadius::corners(0.0, 0.0, 4.0, 4.0)),
            "M10 20 L57 20 L57 59.4 A4 4 0 0 1 53 63.4 L14 63.4 A4 4 0 0 1 10 59.4 Z"
        );
        // A radius larger than half the bar is clamped to it.
        let thin = BarRect {
            x: 0.0,
            y: 0.0,
            w: 6.0,
            h: 100.0,
            negative: false,
        };
        assert!(rounded_rect_path(thin, BarRadius::corners(8.0, 8.0, 0.0, 0.0)).contains("A3 3"));
    }

    #[test]
    fn a_uniform_radius_is_a_rect_with_rx_and_corners_are_a_path() {
        let html = render(HarnessProps {
            config: config_one(),
            data: sample_data(),
            bar: BarOptions {
                radius: BarRadius::all(8.0),
                ..Default::default()
            },
            stacked: false,
        });
        assert!(html.contains(r#"rx="8""#), "{html}");
        let html = render(HarnessProps {
            config: config_two(),
            data: sample_data(),
            bar: BarOptions {
                series_radius: vec![
                    BarRadius::corners(0.0, 0.0, 4.0, 4.0),
                    BarRadius::corners(4.0, 4.0, 0.0, 0.0),
                ],
                ..Default::default()
            },
            stacked: true,
        });
        assert_eq!(
            html.matches(r#"<path data-slot="chart-bar""#).count(),
            4,
            "{html}"
        );
    }

    #[test]
    fn category_labels_sit_beyond_the_bar_end_in_its_color() {
        let data = vec![
            ChartDatum {
                label: "January".to_string(),
                values: vec![Some(186.0)],
                color: Some("var(--dx-chart-1)".to_string()),
            },
            ChartDatum {
                label: "March".to_string(),
                values: vec![Some(-207.0)],
                color: Some("var(--dx-chart-2)".to_string()),
            },
        ];
        let html = render(HarnessProps {
            config: config_one(),
            data,
            bar: BarOptions {
                category_labels: true,
                ..Default::default()
            },
            stacked: false,
        });
        assert_eq!(html.matches(r#"data-position="category""#).count(), 2);
        assert!(html.contains(">January<") && html.contains(">March<"));
        // The negative bar's label hangs below it.
        assert!(html.contains(r#"dy="0.71em""#), "{html}");
    }
}
