//! Defines the [`Chart`] component: the actual SVG drawing surface.

use dioxus::prelude::*;

use crate::chart::context::{use_chart, ChartLayout};
use crate::chart::engine::geometry::plot_runs;
use crate::chart::engine::scale::{fmt_decimal, fmt_num};
use crate::chart::engine::table::table_rows;
use crate::chart::{
    area_between_path, area_path, line_path, stack, BandScale, ChartConfig, ChartDatum, ChartKind,
    Curve, LinearScale,
};
use crate::direction::{use_direction, Direction, HorizontalNav};

/// Fixed MVP layout constants (logical SVG units, scaled visually by CSS --
/// see the module doc). Not configurable yet: this repo's other multi-part
/// primitives (e.g. `Resizable`) don't expose pixel-tuning props either,
/// preferring a themed wrapper's CSS for that; the same holds here once a
/// need for it is demonstrated.
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
const BAND_PADDING: f64 = 0.2;
/// Padding between grouped (non-stacked, multi-series) bars sharing one
/// category band.
const GROUP_PADDING: f64 = 0.15;

/// The props for the [`Chart`] component.
#[derive(Props, Clone, PartialEq)]
pub struct ChartProps {
    /// The chart's accessible name -- required, and should be non-empty:
    /// this is the only thing a screen reader announces for the compact
    /// SVG visual (`role="img"`), before a user ever reaches the hidden
    /// data table.
    pub aria_label: String,

    /// An optional longer description, rendered as the SVG's `<desc>`.
    #[props(default)]
    pub description: Option<String>,

    /// The `viewBox`'s logical width. CSS (the themed wrapper's
    /// `width: 100%; height: auto`) makes the rendered size responsive;
    /// this and [`Self::height`] only set the aspect ratio and the scale
    /// every other logical measurement (font sizes included, via the
    /// browser's SVG scaling) is relative to.
    #[props(default = 600.0)]
    pub width: f64,

    /// The `viewBox`'s logical height.
    #[props(default = 300.0)]
    pub height: f64,

    /// Stack each datum's series values instead of drawing them
    /// independently (Area/Bar only -- a "stacked line chart" isn't a
    /// standard construction, so this is silently ignored for
    /// [`ChartKind::Line`], matching shadcn/Recharts' own scope).
    #[props(default)]
    pub stacked: bool,

    /// The line/area interpolation.
    #[props(default = Curve::Monotone)]
    pub curve: Curve,

    /// Show horizontal gridlines at each y tick.
    #[props(default = true)]
    pub show_grid: bool,

    /// Show x-axis category labels.
    #[props(default = true)]
    pub show_x_axis: bool,

    /// Show y-axis value labels. Off by default, matching shadcn's own
    /// demos (the tooltip/hidden table carry exact values instead).
    #[props(default)]
    pub show_y_axis: bool,

    /// The x-axis category column's header: rendered as the hidden data
    /// table's corner `<th scope="col">` (top-left cell, above the row
    /// headers). Not part of the original `$S/chart-api.md` sketch, which
    /// left that cell empty (`th {}`) -- an empty `<th>` has no accessible
    /// name, which axe's `empty-table-header` rule (`best-practice` tag)
    /// correctly flags as a real defect on every scan, not a false
    /// positive: a screen-reader user browsing the table by column has no
    /// way to tell what the first column *is*. Defaults to `"Category"`,
    /// a neutral header that fits any chart's x-axis regardless of what
    /// the data actually represents (dates, labels, ...); callers with a
    /// more specific axis (e.g. `"Date"`) should override it.
    #[props(default = "Category".to_string())]
    pub x_label: String,

    /// Format an x-axis category label. Defaults to its first 3 characters
    /// (shadcn's own demo convention, e.g. `"January"` -> `"Jan"`).
    #[props(default)]
    pub x_tick_format: Option<Callback<String, String>>,

    /// The maximum number of x-axis tick *labels* to draw, regardless of
    /// how many data points the chart has. A dense chart (e.g. 90 daily
    /// data points) would otherwise draw one `<text>` per datum and
    /// overlap them into an unreadable smear; this labels only every
    /// `ceil(n / max_x_ticks)`-th datum (always including the first),
    /// leaving every hit band, mark and hidden-table row exactly as
    /// before -- this only thins the *visible tick labels*, never the
    /// underlying per-datum data or interactivity. MVP count-based
    /// thinning: it does not account for the actual rendered pixel width
    /// of a label (a genuinely crowded chart at a narrow viewport can
    /// still overlap short labels, or leave room for more than
    /// `max_x_ticks` long ones) -- the forks survey
    /// (`dev-docs/research/chart-forks-2026-09-19.md`, §6/§3c,
    /// `leptos-chartistry`'s `ticks/gen/aligned_floats.rs`) documents a
    /// width-aware alternative (derive the count from estimated label
    /// width vs. available pixel span) as the natural stage-2 upgrade;
    /// not built here since this MVP has no text-measurement facility and
    /// the fixed-count default already fixes the crowded 90-point demo.
    #[props(default = 12)]
    pub max_x_ticks: usize,

    /// Target number of y-axis ticks (see [`LinearScale::ticks`] -- the
    /// actual count can differ slightly, same as d3's own `ticks`).
    #[props(default = 5)]
    pub y_tick_count: usize,

    /// Show a dot at each defined data point ([`ChartKind::Line`] only).
    #[props(default)]
    pub show_dots: bool,

    /// Enable arrow-key/Home/End/Escape stepping of the active index on
    /// this chart's own focusable wrapper (see the module doc for why the
    /// wrapper, not `ChartContainer`, hosts this). Tier-3 opinion, cited to
    /// Recharts' `accessibilityLayer` -- additive to, never a replacement
    /// for, the hidden data table every chart always renders regardless of
    /// this prop.
    #[props(default = true)]
    pub keyboard: bool,

    /// The text direction for the keyboard layer's ArrowLeft/ArrowRight
    /// swap, matching every other direction-aware component in this crate
    /// (`Slider`, `Select`, ...): a local override that wins over the
    /// nearest [`crate::direction::DirectionProvider`], or LTR if neither
    /// is present. Not part of the original `$S/chart-api.md` sketch --
    /// added for consistency with this crate's own established contract
    /// ("every RTL-aware component accepts a local `dir` prop", per
    /// `direction.rs`'s own module doc) now that the keyboard layer makes
    /// this component direction-aware.
    #[props(default)]
    pub dir: Option<Direction>,

    /// Additional attributes to apply to the chart's own wrapper element
    /// (see the module doc: `Chart` renders one `div[data-slot="chart"]`
    /// around its `svg` and hidden `table`, and this is that div).
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
}

/// # Chart
///
/// Renders the chart's actual drawing surface: an accessible SVG (grid,
/// axes, one mark per configured series, the hover cursor, and the
/// invisible hit bands that are its *only* hover mechanism -- no
/// coordinate math, no layout reads, no `document::eval`) plus a real,
/// visually-hidden `<table>` mirroring the same data, both inside one
/// `div[data-slot="chart"]` wrapper. Must be rendered inside a
/// [`crate::chart::ChartContainer`].
///
/// ## Wrapper element, stated plainly (see the module doc for the reason)
///
/// This component's own top-level element is a `div`, not the `svg`
/// itself: [`ChartProps::keyboard`]'s `tabindex`/`role="group"`/
/// `aria-roledescription`/`aria-label` land on that wrapper `div`, and
/// [`ChartProps::attributes`] merges onto it too. The `svg` itself stays
/// non-focusable (`role="img"` only) and is a plain child, alongside the
/// hidden `table`.
///
/// ## A11y contract (`dev-docs/research/chart-2026-09-19.md` §2.5/§6.4)
///
/// 1. `svg[role="img"][aria-label]` + `<title>`/`<desc>` -- a single
///    indivisible graphic (WAI-ARIA Graphics Module 1.0's own definition
///    of `img`), not a claim of internal navigability.
/// 2. A real `<table>` mirroring the chart's data -- the actual
///    screen-reader "browse the data" mechanism, native and free, and the
///    reason this component never *requires* the keyboard layer below to
///    be usable.
/// 3. Optional (this prop, default on) sighted-keyboard stepping of the
///    active index -- tier-3 opinion, mirroring Recharts'
///    `accessibilityLayer` (`recharts/recharts` commit
///    `86ad3632ff3f83a742001ae6fc079dd27961a2a4`,
///    `src/state/keyboardEventsMiddleware.ts`): ArrowRight/ArrowLeft move
///    the active index by one (direction-aware via
///    [`crate::direction::use_direction`], matching that file's own
///    `selectChartDirection` RTL handling), Home/End jump to the first/
///    last datum, Escape clears it. Deliberately narrower than Recharts'
///    version: no `role="application"` (a heavy-handed escape hatch this
///    report's research recommends against -- it suppresses a screen
///    reader's own browse-mode navigation for the rest of the page, which
///    point 2 above makes unnecessary here anyway) and no "Enter pins the
///    tooltip open" behavior (not requested by `$S/chart-api.md`).
///
/// **Known Dioxus limitation (not a bug here, and not fixable from inside
/// this component):** `dioxus-html` 0.7.9 has no SVG-namespaced `<title>`
/// element, only the HTML one, so the `<title>` this component renders
/// inside its `<svg>` gets created without the SVG namespace when Dioxus
/// builds the DOM directly (confirmed by reading `dioxus-web`'s own
/// `create_template_node`: a `None` namespace calls plain
/// `document.createElement`, never `createElementNS`) -- e.g. on a
/// CSR-only dev-server page load. This does not affect this repo's actual
/// deployed site: it is SSR + hydrate, and a browser's own HTML parser
/// (per the HTML5 "foreign content" spec -- `title` is not in its
/// HTML-breakout list) correctly parses a `<title>` nested in `<svg>` in
/// the *source text* as an SVG title regardless of what Dioxus's own
/// per-element namespace table says, and hydration reuses that
/// already-correct node rather than recreating it. `aria-label` on the
/// `svg` (an attribute, not an element -- no namespace ambiguity) already
/// provides a working accessible name either way.
///
/// ## Styling
///
/// The [`Chart`] component defines the following data attributes you can
/// use to control styling (every element is otherwise unstyled --
/// `data-slot` is this crate's own convention for a themed wrapper to
/// select on):
/// - `data-slot="chart"`: the wrapper div.
/// - `data-slot="chart-svg"`: the SVG root.
/// - `data-slot="chart-grid"`/`"chart-axis"` (`data-axis="x"|"y"`).
/// - `data-slot="chart-series"[data-series=<key>]`: one per configured
///   series, wrapping that series' own `"chart-area"`/`"chart-line"`/
///   `"chart-bar"`/`"chart-dot"` marks (`data-index` on the per-datum ones).
/// - `data-slot="chart-cursor"` (`"chart-cursor-line"` for Area/Line,
///   `"chart-cursor-rect"` for Bar -- not named individually in the
///   original contract sketch, added here since the two need distinct
///   selectors) and `"chart-hit-band"[data-index]`.
/// - `data-slot="chart-data"`: the hidden data table.
#[component]
pub fn Chart(props: ChartProps) -> Element {
    let ctx = use_chart();
    let config = (ctx.config)();
    let data = (ctx.data)();
    let kind = (ctx.kind)();
    let direction = use_direction(props.dir);
    let mut active_index = ctx.active_index;
    let mut layout_signal = ctx.layout;

    // A "stacked line chart" isn't a standard construction -- see this
    // prop's own doc. Every computation below reads this, not the raw
    // prop, so the rule can't be forgotten in just one branch.
    let stacked = props.stacked && !matches!(kind, ChartKind::Line);

    let width = props.width;
    let height = props.height;
    let margin_left = if props.show_y_axis {
        MARGIN_LEFT_WITH_AXIS
    } else {
        MARGIN_LEFT_BARE
    };
    let margin_bottom = if props.show_x_axis {
        MARGIN_BOTTOM_WITH_AXIS
    } else {
        MARGIN_BOTTOM_BARE
    };
    let plot_x0 = margin_left;
    let plot_x1 = width - MARGIN_RIGHT;
    let plot_y0 = MARGIN_TOP;
    let plot_y1 = height - margin_bottom;

    let n = data.len();
    let x_scale = BandScale {
        count: n.max(1),
        range: (plot_x0, plot_x1),
        padding: BAND_PADDING,
    };
    let xs: Vec<f64> = (0..n).map(|i| x_scale.center(i)).collect();

    let (y_min, y_max) = y_extent(&config, &data, stacked);
    let y_domain = crate::chart::nice_domain(y_min, y_max);
    let y_scale = LinearScale {
        domain: y_domain,
        range: (plot_y1, plot_y0),
    };
    let y_ticks = y_scale.ticks(props.y_tick_count);
    let zero_y = y_scale.scale(0.0);

    let stacked_spans: Vec<Vec<(f64, f64)>> = if stacked {
        let rows: Vec<Vec<Option<f64>>> = data.iter().map(|d| d.values.clone()).collect();
        stack(&rows)
    } else {
        Vec::new()
    };

    // Share this render's layout for `ChartTooltip`'s benefit -- see
    // `ChartLayout`'s own doc for why a plain write here (not an effect)
    // is correct: it depends only on this component's own props, so
    // recomputing and re-setting every render is cheap and right, and
    // `Signal`'s equality check keeps it a no-op once stable.
    let top_value: Vec<f64> = (0..n)
        .map(|i| {
            let top = if stacked {
                stacked_spans[i]
                    .iter()
                    .map(|(_, y1)| *y1)
                    .fold(f64::NEG_INFINITY, f64::max)
            } else {
                data[i]
                    .values
                    .iter()
                    .take(config.series.len())
                    .flatten()
                    .copied()
                    .fold(f64::NEG_INFINITY, f64::max)
            };
            if top.is_finite() {
                top
            } else {
                0.0
            }
        })
        .collect();
    layout_signal.set(Some(ChartLayout {
        width,
        height,
        x_scale,
        y_scale,
        top_value,
    }));

    let rows = table_rows(&data);

    let wrapper_role = props.keyboard.then_some("group");
    let wrapper_roledescription = props.keyboard.then_some("chart");
    let wrapper_label = props.keyboard.then(|| props.aria_label.clone());
    let wrapper_tabindex = props.keyboard.then_some(0);

    rsx! {
        div {
            "data-slot": "chart",
            role: wrapper_role,
            "aria-roledescription": wrapper_roledescription,
            aria_label: wrapper_label,
            tabindex: wrapper_tabindex,
            dir: direction.as_str(),
            "data-direction": direction.as_str(),
            onkeydown: move |evt| {
                if !props.keyboard || n == 0 {
                    return;
                }
                let key = evt.key();
                if key == Key::Home {
                    active_index.set(Some(0));
                    evt.prevent_default();
                } else if key == Key::End {
                    active_index.set(Some(n - 1));
                    evt.prevent_default();
                } else if key == Key::Escape {
                    active_index.set(None);
                    evt.prevent_default();
                } else if let Some(nav) = direction.resolve_horizontal(&key) {
                    let current = active_index();
                    let next = match (nav, current) {
                        (HorizontalNav::Next, Some(i)) => (i + 1).min(n - 1),
                        (HorizontalNav::Next, None) => 0,
                        (HorizontalNav::Prev, Some(i)) => i.saturating_sub(1),
                        (HorizontalNav::Prev, None) => n - 1,
                    };
                    active_index.set(Some(next));
                    evt.prevent_default();
                }
            },
            ..props.attributes,

            svg {
                "data-slot": "chart-svg",
                role: "img",
                "aria-label": "{props.aria_label}",
                view_box: "0 0 {fmt_num(width)} {fmt_num(height)}",
                onpointerleave: move |_| active_index.set(None),

                title { "{props.aria_label}" }
                if let Some(description) = &props.description {
                    desc { "{description}" }
                }

                if props.show_grid {
                    g { "data-slot": "chart-grid",
                        for y_tick in y_ticks.iter().copied() {
                            line {
                                key: "{y_tick}",
                                x1: "{fmt_num(plot_x0)}",
                                x2: "{fmt_num(plot_x1)}",
                                y1: "{fmt_num(y_scale.scale(y_tick))}",
                                y2: "{fmt_num(y_scale.scale(y_tick))}",
                            }
                        }
                    }
                }

                if props.show_x_axis {
                    {
                        let tick_step = x_tick_step(n, props.max_x_ticks);
                        rsx! {
                            g { "data-slot": "chart-axis", "data-axis": "x",
                                for (i , datum) in data.iter().enumerate() {
                                    if i % tick_step == 0 {
                                        text {
                                            key: "{i}",
                                            "data-index": "{i}",
                                            x: "{fmt_num(x_scale.center(i))}",
                                            y: "{fmt_num(plot_y1 + 16.0)}",
                                            {format_x_tick(&datum.label, &props.x_tick_format)}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if props.show_y_axis {
                    g { "data-slot": "chart-axis", "data-axis": "y",
                        for y_tick in y_ticks.iter().copied() {
                            text {
                                key: "{y_tick}",
                                x: "{fmt_num(plot_x0 - 8.0)}",
                                y: "{fmt_num(y_scale.scale(y_tick))}",
                                {fmt_decimal(y_tick, 2)}
                            }
                        }
                    }
                }

                for (s , series) in config.series.iter().enumerate() {
                    g {
                        key: "{series.key}",
                        "data-slot": "chart-series",
                        "data-series": "{series.slot()}",
                        style: "--series-color: var(--color-{series.slot()})",

                        SeriesMarks {
                            kind,
                            curve: props.curve,
                            stacked,
                            show_dots: props.show_dots,
                            series_index: s,
                            values: series_values(&data, s),
                            xs: xs.clone(),
                            x_scale,
                            y_scale,
                            zero_y,
                            series_count: config.series.len(),
                            stacked_spans: stacked_spans.clone(),
                        }
                    }
                }

                g { "data-slot": "chart-cursor",
                    if let Some(i) = active_index() {
                        if i < n {
                            if matches!(kind, ChartKind::Bar) {
                                {
                                    let (bx, bw) = x_scale.band(i);
                                    rsx! {
                                        rect {
                                            "data-slot": "chart-cursor-rect",
                                            x: "{fmt_num(bx)}",
                                            y: "{fmt_num(plot_y0)}",
                                            width: "{fmt_num(bw)}",
                                            height: "{fmt_num(plot_y1 - plot_y0)}",
                                        }
                                    }
                                }
                            } else {
                                {
                                    let cx = x_scale.center(i);
                                    rsx! {
                                        line {
                                            "data-slot": "chart-cursor-line",
                                            x1: "{fmt_num(cx)}",
                                            x2: "{fmt_num(cx)}",
                                            y1: "{fmt_num(plot_y0)}",
                                            y2: "{fmt_num(plot_y1)}",
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                g { "data-slot": "chart-hit-bands",
                    for i in 0..n {
                        {
                            let (bx, bw) = x_scale.band(i);
                            rsx! {
                                rect {
                                    key: "{i}",
                                    "data-slot": "chart-hit-band",
                                    "data-index": "{i}",
                                    x: "{fmt_num(bx)}",
                                    y: "{fmt_num(plot_y0)}",
                                    width: "{fmt_num(bw)}",
                                    height: "{fmt_num(plot_y1 - plot_y0)}",
                                    fill: "transparent",
                                    "pointer-events": "all",
                                    onpointerenter: move |_| active_index.set(Some(i)),
                                }
                            }
                        }
                    }
                }
            }

            table { "data-slot": "chart-data",
                caption { "{props.aria_label}" }
                thead {
                    tr {
                        th { scope: "col", "{props.x_label}" }
                        for series in &config.series {
                            th { key: "{series.key}", "{series.label}" }
                        }
                    }
                }
                tbody {
                    for row in rows.iter() {
                        tr { key: "{row.label}",
                            th { scope: "row", "{row.label}" }
                            for (i , cell) in row.cells.iter().enumerate() {
                                td { key: "{i}", "{cell}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Props for the internal `SeriesMarks` helper component (one instance per
/// configured series -- see [`Chart`]). Not part of this crate's public
/// API.
#[derive(Props, Clone, PartialEq)]
struct SeriesMarksProps {
    kind: ChartKind,
    curve: Curve,
    stacked: bool,
    show_dots: bool,
    series_index: usize,
    /// This series' raw value per datum, `None` for a gap.
    values: Vec<Option<f64>>,
    /// Each datum's x position (band centers), aligned to `values`.
    xs: Vec<f64>,
    x_scale: BandScale,
    y_scale: LinearScale,
    /// `y_scale.scale(0.0)` -- every non-stacked Area/Bar baseline.
    zero_y: f64,
    /// Total configured series count (grouped Bar sub-positioning).
    series_count: usize,
    /// [`stack()`]'s output, one row per datum, empty when not stacking.
    stacked_spans: Vec<Vec<(f64, f64)>>,
}

/// Renders one series' own marks (inside its already-rendered
/// `g[data-series]` wrapper) for whichever [`ChartKind`] the chart draws.
/// Split out of [`Chart`] only to keep that component's own body from
/// growing a third level of nested nested-`for`/`match` -- this has no
/// context or state of its own, and is never used outside [`Chart`].
#[component]
fn SeriesMarks(props: SeriesMarksProps) -> Element {
    let n = props.xs.len();
    match props.kind {
        ChartKind::Line => rsx! {
            for run in plot_runs(&props.xs, &props.values) {
                {
                    let scaled: Vec<(f64, f64)> =
                        run.iter().map(|(x, v)| (*x, props.y_scale.scale(*v))).collect();
                    let d = line_path(&scaled, props.curve);
                    rsx! {
                        path { "data-slot": "chart-line", d: "{d}", fill: "none" }
                    }
                }
            }
            if props.show_dots {
                for i in 0..n {
                    if let Some(v) = props.values[i] {
                        circle {
                            key: "{i}",
                            "data-slot": "chart-dot",
                            "data-index": "{i}",
                            cx: "{fmt_num(props.xs[i])}",
                            cy: "{fmt_num(props.y_scale.scale(v))}",
                            r: "3",
                        }
                    }
                }
            }
        },
        ChartKind::Area if !props.stacked => rsx! {
            for run in plot_runs(&props.xs, &props.values) {
                {
                    let scaled: Vec<(f64, f64)> =
                        run.iter().map(|(x, v)| (*x, props.y_scale.scale(*v))).collect();
                    let area_d = area_path(&scaled, props.zero_y, props.curve);
                    let line_d = line_path(&scaled, props.curve);
                    rsx! {
                        path { "data-slot": "chart-area", d: "{area_d}" }
                        path { "data-slot": "chart-line", d: "{line_d}", fill: "none" }
                    }
                }
            }
        },
        ChartKind::Area => rsx! {
            {
                let top: Vec<(f64, f64)> = (0..n)
                    .map(|i| (props.xs[i], props.y_scale.scale(props.stacked_spans[i][props.series_index].1)))
                    .collect();
                let bottom: Vec<(f64, f64)> = (0..n)
                    .map(|i| (props.xs[i], props.y_scale.scale(props.stacked_spans[i][props.series_index].0)))
                    .collect();
                let area_d = area_between_path(&top, &bottom, props.curve);
                let line_d = line_path(&top, props.curve);
                rsx! {
                    path { "data-slot": "chart-area", d: "{area_d}" }
                    path { "data-slot": "chart-line", d: "{line_d}", fill: "none" }
                }
            }
        },
        ChartKind::Bar if !props.stacked => rsx! {
            for i in 0..n {
                if let Some(v) = props.values[i] {
                    {
                        let (outer_x, outer_w) = props.x_scale.band(i);
                        let inner = BandScale {
                            count: props.series_count.max(1),
                            range: (outer_x, outer_x + outer_w),
                            padding: GROUP_PADDING,
                        };
                        let (bar_x, bar_w) = inner.band(props.series_index);
                        let y1 = props.y_scale.scale(v);
                        let (rect_y, rect_h) = if y1 <= props.zero_y {
                            (y1, props.zero_y - y1)
                        } else {
                            (props.zero_y, y1 - props.zero_y)
                        };
                        rsx! {
                            rect {
                                key: "{i}",
                                "data-slot": "chart-bar",
                                "data-index": "{i}",
                                x: "{fmt_num(bar_x)}",
                                y: "{fmt_num(rect_y)}",
                                width: "{fmt_num(bar_w)}",
                                height: "{fmt_num(rect_h)}",
                            }
                        }
                    }
                }
            }
        },
        ChartKind::Bar => rsx! {
            for i in 0..n {
                {
                    let (bar_x, bar_w) = props.x_scale.band(i);
                    let (y0_raw, y1_raw) = props.stacked_spans[i][props.series_index];
                    let y0 = props.y_scale.scale(y0_raw);
                    let y1 = props.y_scale.scale(y1_raw);
                    let (rect_y, rect_h) = if y1 <= y0 { (y1, y0 - y1) } else { (y0, y1 - y0) };
                    rsx! {
                        rect {
                            key: "{i}",
                            "data-slot": "chart-bar",
                            "data-index": "{i}",
                            x: "{fmt_num(bar_x)}",
                            y: "{fmt_num(rect_y)}",
                            width: "{fmt_num(bar_w)}",
                            height: "{fmt_num(rect_h)}",
                        }
                    }
                }
            }
        },
    }
}

/// Extract series `s`'s raw value for every datum, `None` when a datum has
/// no entry for it at all (a shorter `values` list than the config's
/// series count -- defensive, never panics).
fn series_values(data: &[ChartDatum], s: usize) -> Vec<Option<f64>> {
    data.iter()
        .map(|d| d.values.get(s).copied().flatten())
        .collect()
}

/// The y-domain input before [`crate::chart::nice_domain`]: the min/max
/// across every configured series' values, or (for `stacked`) across
/// [`stack()`]'s own per-row spans -- a stacked chart's axis must span the
/// *cumulative* totals, not each series' own raw values.
fn y_extent(config: &ChartConfig, data: &[ChartDatum], stacked: bool) -> (f64, f64) {
    let mut lo = 0.0f64;
    let mut hi = 0.0f64;
    if stacked {
        let rows: Vec<Vec<Option<f64>>> = data.iter().map(|d| d.values.clone()).collect();
        for row in stack(&rows) {
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

/// Format an x-axis category label: the caller's own formatter if given,
/// else its first 3 characters (shadcn's own demo convention).
fn format_x_tick(label: &str, format: &Option<Callback<String, String>>) -> String {
    match format {
        Some(cb) => cb.call(label.to_string()),
        None => label.chars().take(3).collect(),
    }
}

/// The x-axis tick-label stride (see [`ChartProps::max_x_ticks`]): label
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
    use crate::chart::ChartContainer;
    use dioxus_core::NoOpMutations;

    fn sample_config() -> ChartConfig {
        ChartConfig::new()
            .series("desktop", "Desktop", "var(--dx-chart-1)")
            .series("mobile", "Mobile", "var(--dx-chart-2)")
    }

    fn sample_data() -> Vec<ChartDatum> {
        vec![
            ChartDatum {
                label: "January".to_string(),
                values: vec![Some(186.0), Some(80.0)],
            },
            ChartDatum {
                label: "February".to_string(),
                values: vec![Some(305.0), None],
            },
        ]
    }

    #[derive(Clone, PartialEq, Props)]
    struct HarnessProps {
        kind: ChartKind,
        #[props(default)]
        stacked: bool,
        #[props(default = true)]
        keyboard: bool,
    }

    #[component]
    fn Harness(props: HarnessProps) -> Element {
        let config = use_signal(sample_config);
        let data = use_signal(sample_data);
        rsx! {
            ChartContainer { config, data, kind: props.kind,
                Chart {
                    aria_label: "Visitors by month",
                    description: "A test chart",
                    stacked: props.stacked,
                    keyboard: props.keyboard,
                }
            }
        }
    }

    /// SSR-render a `ChartContainer`+`Chart` pair -- this crate's existing
    /// SSR-test pattern (`tooltip.rs`'s own tests: `rebuild_in_place` alone
    /// never runs a `use_effect`, but this component has none that change
    /// first-render output, so a plain rebuild is already the final tree;
    /// `render_immediate` is still called for parity with that precedent).
    fn render(kind: ChartKind, stacked: bool, keyboard: bool) -> String {
        let mut dom = VirtualDom::new_with_props(
            Harness,
            HarnessProps {
                kind,
                stacked,
                keyboard,
            },
        );
        dom.rebuild_in_place();
        dom.render_immediate(&mut NoOpMutations);
        dioxus_ssr::render(&dom)
    }

    #[test]
    fn svg_root_has_role_img_and_accessible_name() {
        let html = render(ChartKind::Line, false, true);
        assert!(html.contains(r#"data-slot="chart-svg""#));
        assert!(html.contains(r#"role="img""#));
        assert!(html.contains(r#"aria-label="Visitors by month""#));
        assert!(
            html.contains("<title>Visitors by month</title>"),
            "expected an SVG <title>: {html}"
        );
        assert!(
            html.contains("<desc>A test chart</desc>"),
            "expected an SVG <desc>: {html}"
        );
    }

    #[test]
    fn wrapper_div_carries_the_keyboard_attributes_when_enabled() {
        let html = render(ChartKind::Line, false, true);
        assert!(html.contains(r#"data-slot="chart""#));
        assert!(html.contains(r#"role="group""#));
        assert!(html.contains(r#"aria-roledescription="chart""#));
        assert!(html.contains(r#"aria-label="Visitors by month""#));
        // dioxus_ssr renders a numeric attribute value unquoted (valid
        // HTML5: an unquoted value is fine as long as it has no
        // whitespace/quotes/etc in it), unlike every string attribute
        // above -- confirmed empirically, not assumed.
        assert!(html.contains("tabindex=0"));
    }

    #[test]
    fn wrapper_div_omits_keyboard_attributes_when_disabled() {
        let html = render(ChartKind::Line, false, false);
        assert!(!html.contains("aria-roledescription"));
        assert!(!html.contains("tabindex"));
        // role="group" must be absent too -- only role="img" (the SVG's
        // own, unconditional role) should remain.
        assert!(!html.contains(r#"role="group""#));
    }

    #[test]
    fn line_kind_renders_one_chart_line_per_series() {
        let html = render(ChartKind::Line, false, true);
        assert_eq!(html.matches(r#"data-slot="chart-line""#).count(), 2);
        assert!(html.contains(r#"data-series="desktop""#));
        assert!(html.contains(r#"data-series="mobile""#));
        assert!(!html.contains(r#"data-slot="chart-bar""#));
        assert!(!html.contains(r#"data-slot="chart-area""#));
    }

    #[test]
    fn area_kind_renders_area_and_line_per_series_non_stacked() {
        let html = render(ChartKind::Area, false, true);
        assert_eq!(html.matches(r#"data-slot="chart-area""#).count(), 2);
        assert_eq!(html.matches(r#"data-slot="chart-line""#).count(), 2);
    }

    #[test]
    fn stacked_area_still_renders_one_area_and_line_per_series() {
        let html = render(ChartKind::Area, true, true);
        assert_eq!(html.matches(r#"data-slot="chart-area""#).count(), 2);
        assert_eq!(html.matches(r#"data-slot="chart-line""#).count(), 2);
    }

    #[test]
    fn bar_kind_renders_one_rect_per_datum_per_series_when_grouped() {
        let html = render(ChartKind::Bar, false, true);
        // 2 datums x 2 series, minus the one `None` cell (mobile/February)
        // which a non-stacked grouped bar omits entirely.
        assert_eq!(html.matches(r#"data-slot="chart-bar""#).count(), 3);
    }

    #[test]
    fn bar_kind_stacked_renders_a_rect_for_every_cell_including_gaps() {
        let html = render(ChartKind::Bar, true, true);
        // Stacked bars always render (a `None` becomes a zero-height,
        // still-present rect -- see `engine::stack`'s own doc).
        assert_eq!(html.matches(r#"data-slot="chart-bar""#).count(), 4);
    }

    #[test]
    fn hit_bands_cover_every_datum() {
        let html = render(ChartKind::Line, false, true);
        assert_eq!(html.matches(r#"data-slot="chart-hit-band""#).count(), 2);
        assert!(html.contains(r#"data-slot="chart-hit-bands""#));
        assert!(html.contains("pointer-events"));
    }

    #[test]
    fn cursor_group_is_present_but_empty_before_any_hover() {
        let html = render(ChartKind::Line, false, true);
        assert!(html.contains(r#"data-slot="chart-cursor""#));
        // Nothing has been hovered/focused yet (first render, no effects
        // change it) -- no cursor line/rect should exist.
        assert!(!html.contains("chart-cursor-line"));
        assert!(!html.contains("chart-cursor-rect"));
    }

    #[test]
    fn hidden_table_mirrors_the_data_with_gap_and_precision_rules() {
        let html = render(ChartKind::Bar, false, true);
        assert!(html.contains(r#"data-slot="chart-data""#));
        assert!(html.contains("<caption>Visitors by month</caption>"));
        assert!(html.contains("January"));
        assert!(html.contains("February"));
        assert!(html.contains("Desktop"));
        assert!(html.contains("Mobile"));
        assert!(html.contains("186"));
        assert!(html.contains("80"));
        assert!(html.contains("305"));
        assert!(html.contains('—'), "expected the None gap marker: {html}");
        assert!(html.contains(r#"scope="row""#));
    }

    /// Regression test for the empty corner `<th>` axe's `empty-table-header`
    /// rule (`best-practice` tag) flagged on every scan: the default
    /// `x_label` ("Category") must give that cell real, non-empty text
    /// instead of `th {}`.
    #[test]
    fn table_corner_header_has_a_default_accessible_label() {
        let html = render(ChartKind::Bar, false, true);
        assert!(
            html.contains(r#"<th scope="col">Category</th>"#),
            "expected the default x_label corner header, not an empty <th>: {html}"
        );
    }

    /// Props for a second, more configurable test harness (`x_label`/
    /// `max_x_ticks`/an arbitrary data length) -- kept separate from
    /// [`Harness`] above so every existing test's fixed 2-datum shape is
    /// untouched.
    #[derive(Clone, PartialEq, Props)]
    struct AxisHarnessProps {
        #[props(default = 2)]
        data_len: usize,
        #[props(default = 12)]
        max_x_ticks: usize,
        #[props(default = "Category".to_string())]
        x_label: String,
    }

    #[component]
    fn AxisHarness(props: AxisHarnessProps) -> Element {
        let config = use_signal(sample_config);
        let data_len = props.data_len;
        let data = use_signal(move || {
            (0..data_len)
                .map(|i| ChartDatum {
                    // First 3 chars unique per index (the default
                    // x_tick_format truncation) so rendered tick labels
                    // can be told apart in the thinning test below.
                    label: format!("D{i:02} full label"),
                    values: vec![Some(i as f64), Some((i * 2) as f64)],
                })
                .collect::<Vec<_>>()
        });
        rsx! {
            ChartContainer { config, data, kind: ChartKind::Line,
                Chart {
                    aria_label: "Visitors by month",
                    x_label: props.x_label.clone(),
                    max_x_ticks: props.max_x_ticks,
                }
            }
        }
    }

    fn render_axis(data_len: usize, max_x_ticks: usize, x_label: &str) -> String {
        let mut dom = VirtualDom::new_with_props(
            AxisHarness,
            AxisHarnessProps {
                data_len,
                max_x_ticks,
                x_label: x_label.to_string(),
            },
        );
        dom.rebuild_in_place();
        dom.render_immediate(&mut NoOpMutations);
        dioxus_ssr::render(&dom)
    }

    #[test]
    fn table_corner_header_can_be_overridden() {
        let html = render_axis(2, 12, "Date");
        assert!(
            html.contains(r#"<th scope="col">Date</th>"#),
            "expected the overridden corner header: {html}"
        );
    }

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
    fn x_axis_tick_labels_are_thinned_for_a_dense_chart() {
        let html = render_axis(90, 12, "Category");
        // Bound the search to the x-axis group's own children so a
        // coincidental substring match elsewhere (e.g. the hidden table,
        // which always mirrors every datum's full, untruncated label)
        // can't produce a false pass -- `data-axis="x"` is this group's
        // own attribute and appears nowhere else.
        let start = html
            .find(r#"data-axis="x""#)
            .expect("x-axis group should be present when show_x_axis is on");
        let after_open = &html[start..];
        let end = after_open
            .find("</g>")
            .expect("x-axis group should close with </g>");
        let axis_group = &after_open[..end];

        let tick_labels = axis_group.matches("<text ").count();
        assert!(
            tick_labels <= 12,
            "expected at most max_x_ticks=12 rendered tick labels for 90 data points, got {tick_labels}: {axis_group}"
        );
        // Matches x_tick_step(90, 12) == 8 exactly: ceil(90/8) == 12.
        assert_eq!(tick_labels, 12);
        // The first datum's label is always kept (default 3-char
        // truncation of "D00 full label" is "D00").
        assert!(
            axis_group.contains("D00"),
            "expected the first datum's tick label to survive thinning: {axis_group}"
        );
        // A thinned-away datum's label must NOT appear (index 1 falls
        // between kept indices 0 and 8).
        assert!(
            !axis_group.contains("D01"),
            "index 1 should have been thinned out: {axis_group}"
        );

        // Every hit band stays per-datum -- thinning only removes axis
        // *labels*, never data or interactivity.
        assert_eq!(html.matches(r#"data-slot="chart-hit-band""#).count(), 90);
    }

    #[test]
    fn container_emits_one_color_variable_per_series() {
        let html = render(ChartKind::Bar, false, true);
        assert!(html.contains("--color-desktop:var(--dx-chart-1)"));
        assert!(html.contains("--color-mobile:var(--dx-chart-2)"));
        assert!(html.contains(r#"data-kind="bar""#));
    }

    #[test]
    fn x_axis_labels_default_to_first_three_characters() {
        let html = render(ChartKind::Bar, false, true);
        assert!(html.contains(r#"data-axis="x""#));
        assert!(html.contains(">Jan<"));
        assert!(html.contains(">Feb<"));
    }

    #[test]
    fn y_axis_is_absent_by_default() {
        let html = render(ChartKind::Bar, false, true);
        assert!(!html.contains(r#"data-axis="y""#));
    }

    #[test]
    fn grid_lines_are_present_by_default() {
        let html = render(ChartKind::Bar, false, true);
        assert!(html.contains(r#"data-slot="chart-grid""#));
    }
}
