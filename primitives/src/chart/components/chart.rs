//! Defines the [`Chart`] component: the actual SVG drawing surface.
//!
//! ## Stage-2 chart round: an orchestrator over per-family renderers
//!
//! As of the stage-2 chart round's `s2-refactor` lane, this component no
//! longer draws any one family's marks itself. It:
//! 1. resolves this render's effective `stacked` flag and computes the
//!    shared plot geometry via `super::layout::build` (margins, scales,
//!    y ticks, stack spans -- see that module's own doc);
//! 2. dispatches on [`ChartKind`] to exactly one of
//!    [`super::series::area`]/[`super::series::bar`]/
//!    [`super::series::line`]/[`super::series::pie`]/
//!    [`super::series::radar`]/[`super::series::radial`]'s own `render` for
//!    the actual marks;
//! 3. for the three `ChartKind::is_cartesian` kinds only, additionally
//!    renders the grid, axes, hover cursor, and hit-bands (via
//!    `super::layout` for the first two, inline here for the last two --
//!    small enough, and specific enough to `Chart`'s own wrapper/state, that
//!    factoring them out bought nothing); the three polar/radial **stub**
//!    kinds get none of that (a Cartesian grid behind a shape that isn't
//!    Cartesian would be actively misleading, not merely unfinished) --
//!    just the dispatched placeholder mark group and a reduced data table
//!    (category + first configured series' value only, via
//!    [`crate::chart::engine::table::table_rows_single_series`]) until each
//!    stub's owning lane replaces it (see [`super::series`]'s own module
//!    doc for the ownership map).
//!
//! Every other behavior -- the wrapper `div`/keyboard layer, the SVG root,
//! the hidden data table's shape for the three real kinds -- is unchanged
//! from before this split; see this module's own tests, which this lane
//! kept green unmodified as the no-behavior-change proof (plus new tests
//! for the three stub kinds, which did not exist before).

use dioxus::prelude::*;

use super::series::{
    AreaOptions, BarOptions, LineOptions, PieOptions, RadarOptions, RadialOptions,
};
use super::{layout, series};
use crate::chart::context::{use_chart, ChartLayout};
use crate::chart::engine::scale::fmt_num;
use crate::chart::engine::table::{table_rows, table_rows_single_series};
use crate::chart::{ChartKind, Curve};
use crate::direction::{use_direction, Direction, HorizontalNav};

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
    /// independently. Only [`ChartKind::Area`] and [`ChartKind::Bar`] have
    /// a stacking construction -- silently ignored for every other kind,
    /// [`ChartKind::Line`] included (a "stacked line chart" isn't a
    /// standard construction, matching shadcn/Recharts' own scope). Kept
    /// on `ChartProps` itself, not duplicated onto `AreaOptions`/
    /// `BarOptions`, since it applies to both -- see
    /// [`super::series`]'s own module doc.
    #[props(default)]
    pub stacked: bool,

    /// The line/area interpolation. Applies to [`ChartKind::Area`] and
    /// [`ChartKind::Line`] (every kind with a drawn edge to interpolate);
    /// kept on `ChartProps` itself for the same reason as
    /// [`Self::stacked`] -- see [`super::series`]'s own module doc.
    #[props(default = Curve::Monotone)]
    pub curve: Curve,

    /// Show horizontal gridlines at each y tick. `ChartKind::is_cartesian`
    /// kinds only.
    #[props(default = true)]
    pub show_grid: bool,

    /// Show x-axis category labels. `ChartKind::is_cartesian` kinds only.
    #[props(default = true)]
    pub show_x_axis: bool,

    /// Show y-axis value labels. Off by default, matching shadcn's own
    /// demos (the tooltip/hidden table carry exact values instead).
    /// `ChartKind::is_cartesian` kinds only.
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
    /// `ChartKind::is_cartesian` kinds only.
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
    /// `ChartKind::is_cartesian` kinds only.
    #[props(default = 12)]
    pub max_x_ticks: usize,

    /// Target number of y-axis ticks (see [`crate::chart::LinearScale::ticks`]
    /// -- the actual count can differ slightly, same as d3's own `ticks`).
    /// `ChartKind::is_cartesian` kinds only.
    #[props(default = 5)]
    pub y_tick_count: usize,

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
    /// is present.
    #[props(default)]
    pub dir: Option<Direction>,

    /// [`ChartKind::Area`]'s own options.
    #[props(default)]
    pub area: AreaOptions,

    /// [`ChartKind::Bar`]'s own options.
    #[props(default)]
    pub bar: BarOptions,

    /// [`ChartKind::Line`]'s own options.
    #[props(default)]
    pub line: LineOptions,

    /// [`ChartKind::Pie`]'s own options. **Stub** -- see
    /// [`super::series::pie`].
    #[props(default)]
    pub pie: PieOptions,

    /// [`ChartKind::Radar`]'s own options. **Stub** -- see
    /// [`super::series::radar`].
    #[props(default)]
    pub radar: RadarOptions,

    /// [`ChartKind::RadialBar`]'s own options. **Stub** -- see
    /// [`super::series::radial`].
    #[props(default)]
    pub radial: RadialOptions,

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
/// coordinate math, no layout reads, no `document::eval` -- for the three
/// `ChartKind::is_cartesian` kinds; a single placeholder mark group for
/// the three polar/radial stub kinds, see the module doc) plus a real,
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
/// - `data-slot="chart-grid"`/`"chart-axis"` (`data-axis="x"|"y"`) --
///   `ChartKind::is_cartesian` kinds only.
/// - `data-slot="chart-series"[data-series=<key>]`: one per configured
///   series, wrapping that series' own `"chart-area"`/`"chart-line"`/
///   `"chart-bar"`/`"chart-dot"` marks (`data-index` on the per-datum ones)
///   -- `ChartKind::is_cartesian` kinds. The three polar/radial stub
///   kinds instead render one `data-slot="chart-series"[data-kind=<kind>]`
///   placeholder group (no `data-series`) -- see the module doc.
/// - `data-slot="chart-cursor"` (`"chart-cursor-line"` for Area/Line,
///   `"chart-cursor-rect"` for Bar) and `"chart-hit-band"[data-index]"` --
///   `ChartKind::is_cartesian` kinds only.
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

    // Only Area and Bar have a stacking construction -- see
    // `ChartProps::stacked`'s own doc. Every computation below reads this
    // resolved value, not the raw prop, so the rule can't be forgotten in
    // just one branch. (Pre-stage-2 this was a deny-list,
    // `!matches!(kind, Line)`, which -- now that `ChartKind` has three more
    // variants -- would have silently started "stacking" a brand new kind
    // unless every call site remembered to re-exclude it by hand. An
    // allow-list is the construction that can't do that: a kind added
    // later is un-stacked by default until someone deliberately opts it
    // in here.)
    let stacked = props.stacked && matches!(kind, ChartKind::Area | ChartKind::Bar);
    let is_cartesian = kind.is_cartesian();

    let n = data.len();
    let ctx_layout = layout::build(layout::LayoutParams {
        width: props.width,
        height: props.height,
        show_x_axis: props.show_x_axis,
        show_y_axis: props.show_y_axis,
        y_tick_count: props.y_tick_count,
        kind,
        stacked,
        curve: props.curve,
        dir: direction,
        active_index: active_index(),
        config: &config,
        data: &data,
    });

    // Share this render's layout for `ChartTooltip`'s benefit -- see
    // `ChartLayout`'s own doc for why a plain write here (not an effect)
    // is correct: it depends only on this component's own props, so
    // recomputing and re-setting every render is cheap and right, and
    // `Signal`'s equality check keeps it a no-op once stable.
    let top_value: Vec<f64> = (0..n)
        .map(|i| {
            let top = if stacked {
                ctx_layout.stacked_spans[i]
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
        width: props.width,
        height: props.height,
        x_scale: ctx_layout.x_scale,
        y_scale: ctx_layout.y_scale,
        top_value,
    }));

    let rows = if is_cartesian {
        table_rows(&data)
    } else {
        table_rows_single_series(&data)
    };

    let marks = match kind {
        ChartKind::Area => series::area::render(&ctx_layout, &props.area),
        ChartKind::Bar => series::bar::render(&ctx_layout, &props.bar),
        ChartKind::Line => series::line::render(&ctx_layout, &props.line),
        ChartKind::Pie => series::pie::render(&ctx_layout, &props.pie),
        ChartKind::Radar => series::radar::render(&ctx_layout, &props.radar),
        ChartKind::RadialBar => series::radial::render(&ctx_layout, &props.radial),
    };

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
                view_box: "0 0 {fmt_num(props.width)} {fmt_num(props.height)}",
                onpointerleave: move |_| active_index.set(None),

                title { "{props.aria_label}" }
                if let Some(description) = &props.description {
                    desc { "{description}" }
                }

                if is_cartesian {
                    if props.show_grid {
                        {layout::render_grid(&ctx_layout)}
                    }
                    if props.show_x_axis {
                        {layout::render_x_axis(&ctx_layout, &props.x_tick_format, props.max_x_ticks)}
                    }
                    if props.show_y_axis {
                        {layout::render_y_axis(&ctx_layout)}
                    }
                }

                {marks}

                if is_cartesian {
                    g { "data-slot": "chart-cursor",
                        if let Some(i) = active_index() {
                            if i < n {
                                if matches!(kind, ChartKind::Bar) {
                                    {
                                        let (bx, bw) = ctx_layout.x_scale.band(i);
                                        rsx! {
                                            rect {
                                                "data-slot": "chart-cursor-rect",
                                                x: "{fmt_num(bx)}",
                                                y: "{fmt_num(ctx_layout.plot_y0)}",
                                                width: "{fmt_num(bw)}",
                                                height: "{fmt_num(ctx_layout.plot_y1 - ctx_layout.plot_y0)}",
                                            }
                                        }
                                    }
                                } else {
                                    {
                                        let cx = ctx_layout.x_scale.center(i);
                                        rsx! {
                                            line {
                                                "data-slot": "chart-cursor-line",
                                                x1: "{fmt_num(cx)}",
                                                x2: "{fmt_num(cx)}",
                                                y1: "{fmt_num(ctx_layout.plot_y0)}",
                                                y2: "{fmt_num(ctx_layout.plot_y1)}",
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
                                let (bx, bw) = ctx_layout.x_scale.band(i);
                                rsx! {
                                    rect {
                                        key: "{i}",
                                        "data-slot": "chart-hit-band",
                                        "data-index": "{i}",
                                        x: "{fmt_num(bx)}",
                                        y: "{fmt_num(ctx_layout.plot_y0)}",
                                        width: "{fmt_num(bw)}",
                                        height: "{fmt_num(ctx_layout.plot_y1 - ctx_layout.plot_y0)}",
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

            table { "data-slot": "chart-data",
                caption { "{props.aria_label}" }
                thead {
                    tr {
                        th { scope: "col", "{props.x_label}" }
                        if is_cartesian {
                            for series in &config.series {
                                th { key: "{series.key}", "{series.label}" }
                            }
                        } else {
                            th { scope: "col",
                                {
                                    config
                                        .series
                                        .first()
                                        .map(|s| s.label.clone())
                                        .unwrap_or_else(|| "Value".to_string())
                                }
                            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart::{ChartConfig, ChartContainer, ChartDatum};
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
                ..Default::default()
            },
            ChartDatum {
                label: "February".to_string(),
                values: vec![Some(305.0), None],
                ..Default::default()
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
                    ..Default::default()
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

    // -- Stage-2 stub kinds (Pie/Radar/RadialBar) ---------------------------
    //
    // Each stub kind renders the same `Harness` used by every Cartesian
    // test above (it's generic over `kind`) -- the point of these tests is
    // exactly that nothing kind-specific needs to change to reach a
    // non-panicking render for a brand new `ChartKind`.

    #[test]
    fn pie_kind_renders_the_placeholder_group_and_the_table_without_panicking() {
        let html = render(ChartKind::Pie, false, true);
        assert!(html.contains(r#"data-slot="chart-series""#));
        assert!(html.contains(r#"data-kind="pie""#));
        assert!(html.contains(r#"data-slot="chart-data""#));
        // No Cartesian-only apparatus for a stub kind.
        assert!(!html.contains(r#"data-slot="chart-grid""#));
        assert!(!html.contains(r#"data-slot="chart-axis""#));
        assert!(!html.contains(r#"data-slot="chart-hit-bands""#));
        assert!(!html.contains(r#"data-slot="chart-cursor""#));
    }

    #[test]
    fn radar_kind_renders_the_placeholder_group_and_the_table_without_panicking() {
        let html = render(ChartKind::Radar, false, true);
        assert!(html.contains(r#"data-slot="chart-series""#));
        assert!(html.contains(r#"data-kind="radar""#));
        assert!(html.contains(r#"data-slot="chart-data""#));
        assert!(!html.contains(r#"data-slot="chart-grid""#));
        assert!(!html.contains(r#"data-slot="chart-hit-bands""#));
    }

    #[test]
    fn radial_bar_kind_renders_the_placeholder_group_and_the_table_without_panicking() {
        let html = render(ChartKind::RadialBar, false, true);
        assert!(html.contains(r#"data-slot="chart-series""#));
        assert!(html.contains(r#"data-kind="radial-bar""#));
        assert!(html.contains(r#"data-slot="chart-data""#));
        assert!(!html.contains(r#"data-slot="chart-grid""#));
        assert!(!html.contains(r#"data-slot="chart-hit-bands""#));
    }

    #[test]
    fn stub_kinds_reduce_the_table_to_category_plus_first_series_value() {
        let html = render(ChartKind::Pie, false, true);
        // Only the first configured series' column header ("Desktop"), not
        // both -- unlike the Cartesian `hidden_table_mirrors_the_data_...`
        // test above, which asserts both "Desktop" AND "Mobile" appear.
        assert!(html.contains("Desktop"));
        assert!(!html.contains("Mobile"));
        // Still one row per datum, with the category label preserved.
        assert!(html.contains("January"));
        assert!(html.contains("February"));
    }
}
