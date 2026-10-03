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
//!    just the dispatched mark group and a reduced data table (category +
//!    first configured series' value, via
//!    [`crate::chart::engine::table::table_rows_single_series`], plus a
//!    `Percent` column for [`ChartKind::Pie`] specifically via
//!    [`crate::chart::engine::table::table_rows_pie`] -- a pie slice's
//!    share of the whole is exactly what its wedge angle already encodes
//!    visually, landed alongside `s2-polar`'s own `series::pie` since a
//!    pie-shaped data table is that lane's own deliverable, not a stub)
//!    until each remaining stub's owning lane replaces its own mark group
//!    (see [`super::series`]'s own module doc for the ownership map).
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
use crate::chart::engine::table::{table_rows, table_rows_pie, table_rows_single_series};
use crate::chart::{ChartKind, Curve};
use crate::dioxus_attributes::attributes;
use crate::direction::{use_direction, Direction, HorizontalNav};
use crate::merge_attributes;

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

    /// The chart's logical width -- the `viewBox`'s own width, and the
    /// coordinate space every length in the chart's props (radii, insets,
    /// gaps) is expressed in. The SVG scales to fit its container (the
    /// themed wrapper's `width: 100%; height: auto`), so this sets the
    /// aspect ratio and the layout, not a pixel size. On a container
    /// narrower or wider than this the text is compensated to keep its
    /// authored size (see [`text_scale`]) instead of shrinking or growing
    /// with the drawing.
    #[props(default = 600.0)]
    pub width: f64,

    /// The chart's logical height -- the `viewBox`'s own height, in the same
    /// coordinate space as [`Self::width`]; the SVG scales both together to
    /// fit its container.
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
    /// `ceil(n / count)`-th datum (always including the first), leaving
    /// every hit band, mark and hidden-table row exactly as before -- this
    /// only thins the *visible tick labels*, never the underlying
    /// per-datum data or interactivity.
    ///
    /// This is an *upper bound*: the count actually drawn is
    /// `min(max_x_ticks, floor(plot_width / (longest_label_width + 8)))`,
    /// never below 1, so a narrow chart (a phone, or any container narrower
    /// than `width`) draws fewer labels instead of overlapping them. The
    /// longest label's width is estimated, not measured -- 12px axis text at
    /// ~0.6em per ASCII glyph (a full em for any other character), times the
    /// narrow-container text compensation (see [`Self::width`]; `1` on the
    /// server and first render) -- so the server render and the client agree
    /// for the same width. It is an
    /// estimate: an unusually wide font can still crowd, in which case
    /// shorten the labels with `x_tick_format` or lower `max_x_ticks`.
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

    /// [`ChartKind::RadialBar`]'s own options -- see
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
///   -- `ChartKind::is_cartesian` kinds. [`ChartKind::Pie`] renders
///   `"chart-arc"[data-index][data-series?]` slices instead (`data-series`
///   only for a stacked, multi-ring pie -- see `series::pie`'s own module
///   doc). The two remaining polar/radial stub kinds still render one
///   `data-slot="chart-series"[data-kind=<kind>]` placeholder group (no
///   `data-series`) until their own owning lane replaces it.
/// - `data-slot="chart-cursor"` (`"chart-cursor-line"` for Area/Line,
///   `"chart-cursor-rect"` for Bar) and `"chart-hit-band"[data-index]"` --
///   `ChartKind::is_cartesian` kinds only.
/// - `data-slot="chart-data"`: the hidden data table, inside a
///   `data-slot="chart-data-wrapper"` div that carries the visually-hidden
///   clamp (a table box ignores `width`/`overflow` clamps).
#[component]
pub fn Chart(props: ChartProps) -> Element {
    let ctx = use_chart();
    let config = (ctx.config)();
    let data = (ctx.data)();
    let kind = (ctx.kind)();
    let direction = use_direction(props.dir);
    let mut active_index = ctx.active_index;
    let mut layout_signal = ctx.layout;

    // The wrapper's measured content-box width (CSS px), written only by
    // `onresize` in a browser (a ResizeObserver reports once on observe, so
    // this also covers mount; one measure, one box, everywhere). `None` --
    // always the case during SSR and the first client render -- means
    // `text_scale == 1`, so the first client render is byte-identical to the
    // server HTML (no hydration mismatch); the measured width is adopted by
    // the re-render that follows. The layout itself always uses the props'
    // own `width`x`height`: only the text compensation depends on it.
    let mut measured_width = use_signal(|| None::<f64>);
    let (width, height) = (props.width, props.height);
    let text_scale = text_scale(width, measured_width());

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
    // §4(c) of the stage-2 chart-round handoff: `components::layout::build`
    // needs the *effective* family's own stack mode, not a chart-wide
    // constant, so a percent-stacked ("100%"/"expand") chart's shared grid
    // lines/y-axis ticks/tooltip anchor agree with its marks. Only
    // `AreaOptions` has a `stack_mode` field today (`BarOptions` is still
    // `s2-bar`'s empty stub) -- every other kind, and `Bar` until its own
    // field lands, keeps today's exact `StackMode::Normal` behavior.
    let stack_mode = match kind {
        ChartKind::Area => props.area.stack_mode,
        _ => crate::chart::StackMode::Normal,
    };

    let n = data.len();
    let ctx_layout = layout::build(layout::LayoutParams {
        width,
        height,
        text_scale,
        show_x_axis: props.show_x_axis,
        show_y_axis: props.show_y_axis,
        y_tick_count: props.y_tick_count,
        kind,
        stacked,
        stack_mode,
        curve: props.curve,
        dir: direction,
        active_index: active_index(),
        config: &config,
        data: &data,
    });

    // Share this render's tooltip anchors for `ChartTooltip`'s benefit --
    // see `ChartLayout`'s own doc for why a plain write here (not an
    // effect) is correct: it depends only on this component's own props,
    // so recomputing and re-setting every render is cheap and right, and
    // `Signal`'s equality check keeps it a no-op once stable. Only the
    // Cartesian kinds populate it today (§4(d) of the stage-2 handoff --
    // a family-agnostic `(left%, top%)` per datum, not a raw scale pair,
    // so a future polar family's own vertex math can populate the same
    // field without `ChartTooltip` branching on `kind`); the three stub
    // kinds render no hit-bands yet (below), so `active_index` can never
    // be `Some` for them regardless -- an empty vec here is exactly as
    // inert as a wrong one would be unreachable.
    let anchor_percent: Vec<(f64, f64)> = if is_cartesian && width > 0.0 && height > 0.0 {
        (0..n)
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
                let top = if top.is_finite() { top } else { 0.0 };
                let x = ctx_layout.x_scale.center(i);
                let y = ctx_layout.y_scale.scale(top);
                (x / width * 100.0, y / height * 100.0)
            })
            .collect()
    } else {
        Vec::new()
    };
    layout_signal.set(Some(ChartLayout { anchor_percent }));

    // `ChartKind::Pie` gets its own row shape (value + percent-of-total --
    // `s2-polar`'s own deliverable, `engine::table::table_rows_pie`'s doc):
    // a slice's share of the whole is exactly what its wedge angle already
    // encodes visually, so the hidden table should carry it too. RadialBar
    // keeps the generic single-series reduction -- "percent of the total"
    // isn't a meaningful reading of a radial bar's own value the way it is
    // for a pie slice. Radar gets the full per-category-per-series table
    // via `has_full_table()` (§4(b) of the stage-2 handoff) -- it is
    // non-Cartesian but, unlike Pie/RadialBar, genuinely holds one value
    // per series per category.
    let rows = if kind.has_full_table() {
        table_rows(&data)
    } else if matches!(kind, ChartKind::Pie) {
        table_rows_pie(&data)
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

    // `data-slot`/`role`/`aria-roledescription`/`tabindex`/`data-direction`
    // are structural/aria wiring this component owns -- e.g. `data-slot`
    // is the very selector the themed stylesheet's whole ruleset hangs
    // off, and `role`/`aria-roledescription`/`tabindex` together form the
    // keyboard-navigable-group contract `ChartProps::keyboard`'s own doc
    // describes -- not overridable presentation. `merge_attributes`
    // (`scripts/check-attr-spread-collision.sh`'s own fix, replacing a raw
    // `..props.attributes` beside these as plain literals) makes "owned
    // wins" explicit and SSR/CSR-consistent instead of accidental; `class`
    // still concatenates regardless (`merge_attributes`'s own rule), and
    // `onkeydown` stays a literal on the element itself, never routed
    // through `merge_attributes` (it isn't an attribute value merge could
    // meaningfully resolve).
    let owned = attributes!(div {
        "data-slot": "chart",
        role: wrapper_role,
        "aria-roledescription": wrapper_roledescription,
        tabindex: wrapper_tabindex,
        "data-direction": direction.as_str(),
    });
    let merged = merge_attributes(vec![props.attributes, owned]);

    rsx! {
        div {
            aria_label: wrapper_label,
            dir: direction.as_str(),
            onresize: move |evt: ResizeEvent| {
                if let Ok(size) = evt.data().get_content_box_size() {
                    adopt_measured_width(&mut measured_width, size.width);
                }
            },
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
            ..merged,

            svg {
                "data-slot": "chart-svg",
                role: "img",
                "aria-label": "{props.aria_label}",
                view_box: "0 0 {fmt_num(width)} {fmt_num(height)}",
                style: "--dx-chart-text-scale: {fmt_num(text_scale)}",
                // The drawing is NOT mirrored under `dir="rtl"` (x grows
                // rightward, the y axis sits on the left, only keyboard
                // navigation follows `dir`), so its text must not flip
                // either: SVG `text-anchor: start`/`end` are relative to the
                // text's own direction, and an inherited `rtl` would swap
                // every anchored label (y ticks would run into the plot,
                // x ticks and value labels would shift off their marks).
                // Pinned once here, on the root, so every text a family draws
                // inherits it -- axis ticks, value labels, rim and arc labels.
                "direction": "ltr",
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

            // The visually-hidden clamp lives on this WRAPPER, never on the
            // table: a `table` box cannot shrink below its min-content
            // width, so `width: 1px; overflow: hidden` on the table itself
            // does nothing and the (clipped but still laid-out) table widens
            // the page at phone widths -- while changing its `display` to
            // `block` would strip its table semantics in WebKit/VoiceOver.
            // A block-level wrapper honours the clamp and leaves the table a
            // normal `display: table`.
            div { "data-slot": "chart-data-wrapper",
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
                                if matches!(kind, ChartKind::Pie) {
                                    th { scope: "col", "Percent" }
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
}

/// The largest text compensation [`text_scale`] applies: beyond a 2.5x
/// shrink the drawing no longer has room for proportionally larger text, so
/// text is allowed to render smaller than authored instead. (Margins scale
/// with the text, so at the cap a default 600-wide chart keeps a ~17% left
/// margin and a ~20% bottom margin -- the plot stays usable at a 320px
/// viewport, where the container is ~240-270px.)
const MAX_TEXT_SCALE: f64 = 2.5;

/// The smallest text compensation [`text_scale`] applies: a chart shown more
/// than 2x *larger* than its logical size (a 300x300 radar stretched across a
/// 1200px card) draws text at half its logical size, rendering at the
/// authored CSS size instead of 2x it.
const MIN_TEXT_SCALE: f64 = 0.5;

/// Computed scales within this distance of `1` snap to exactly `1`
/// (`|1 - s| < 5%`), in either direction: a ~5% shrink or growth (e.g. a 592px
/// card around a 600 chart) leaves text visually at its authored size, and
/// compensating it would only shave labels off the desktop layout for no
/// legibility gain.
const TEXT_SCALE_DEADZONE: f64 = 0.05;

/// The text compensation scale `s`: how much larger than authored (in the
/// chart's logical units) text is drawn so it renders at its authored CSS
/// size when the SVG is shown `k = measured / width` times its logical size.
/// `s = clamp(1 / k, MIN_TEXT_SCALE, MAX_TEXT_SCALE)`, symmetric around `1`:
/// above 1 on a container narrower than the viewBox (text would shrink with
/// the drawing), below 1 on one wider (text would grow with it), snapped to
/// exactly `1` within [`TEXT_SCALE_DEADZONE`] of it. `1` also when the width
/// is unmeasured (SSR, first client render, `None`) or unusable (non-finite
/// or non-positive) -- so the server HTML and the first client render agree.
///
/// Published to CSS as `--dx-chart-text-scale` on the SVG, which the themed
/// stylesheet multiplies into every chart text's `font-size`, and read by
/// the layout (tick-count estimate, axis margins) in logical units. Only
/// *text* is compensated: every geometry length stays in the logical
/// coordinate space and scales with the drawing.
fn text_scale(width: f64, measured: Option<f64>) -> f64 {
    match measured {
        Some(m) if m.is_finite() && m > 0.0 && width.is_finite() && width > 0.0 => {
            let s = (width / m).clamp(MIN_TEXT_SCALE, MAX_TEXT_SCALE);
            if (1.0 - s).abs() < TEXT_SCALE_DEADZONE {
                1.0
            } else {
                s
            }
        }
        _ => 1.0,
    }
}

/// Store a measured container width, rounded to a whole pixel and only when
/// it changed, so sub-pixel resize jitter never re-renders the chart.
fn adopt_measured_width(slot: &mut Signal<Option<f64>>, width: f64) {
    if !width.is_finite() || width <= 0.0 {
        return;
    }
    let width = width.round();
    if *slot.peek() != Some(width) {
        slot.set(Some(width));
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
    fn text_scale_is_the_inverse_shrink_factor_clamped() {
        // 600 logical shown at 400 CSS px: k = 2/3, s = 1.5.
        assert!((text_scale(600.0, Some(400.0)) - 1.5).abs() < 1e-9);
        // 600 shown at 300: s = 2 (under the cap now).
        assert_eq!(text_scale(600.0, Some(300.0)), 2.0);
        // 600 shown at 240: exactly the 2.5 cap; beyond it stays there
        // (390 phone: 600 -> 229).
        assert_eq!(text_scale(600.0, Some(240.0)), MAX_TEXT_SCALE);
        assert_eq!(text_scale(600.0, Some(229.0)), MAX_TEXT_SCALE);
        assert_eq!(text_scale(600.0, Some(1.0)), MAX_TEXT_SCALE);
    }

    #[test]
    fn text_scale_shrinks_text_on_a_container_wider_than_the_viewbox() {
        // A 300x300 radar stretched across a 592px card: text is drawn at
        // ~half size in logical units so it renders at its authored size.
        let s = text_scale(300.0, Some(592.0));
        assert!((s - 300.0 / 592.0).abs() < 1e-9, "{s}");
        assert!(s < 1.0);
        // A default 600x300 chart in a 1200px container.
        assert_eq!(text_scale(600.0, Some(1200.0)), 0.5);
        // The shrink is floored: a hugely wider container stays at the floor.
        assert_eq!(text_scale(300.0, Some(5000.0)), MIN_TEXT_SCALE);
        // Symmetric around 1: width/m and m/width mirror each other.
        let (up, down) = (
            text_scale(600.0, Some(400.0)),
            text_scale(400.0, Some(600.0)),
        );
        assert!((up * down - 1.0).abs() < 1e-9, "{up} * {down}");
    }

    #[test]
    fn text_scale_is_one_when_unmeasured_or_unusable() {
        assert_eq!(text_scale(600.0, None), 1.0);
        assert_eq!(text_scale(600.0, Some(600.0)), 1.0);
        assert_eq!(text_scale(600.0, Some(0.0)), 1.0);
        assert_eq!(text_scale(600.0, Some(-5.0)), 1.0);
        assert_eq!(text_scale(600.0, Some(f64::NAN)), 1.0);
        assert_eq!(text_scale(600.0, Some(f64::INFINITY)), 1.0);
        assert_eq!(text_scale(0.0, Some(300.0)), 1.0);
        assert_eq!(text_scale(-600.0, Some(300.0)), 1.0);
        assert_eq!(text_scale(f64::NAN, Some(300.0)), 1.0);
    }

    #[test]
    fn text_scale_dead_zone_is_symmetric() {
        // Narrower: a 592px card around a 600 chart.
        assert_eq!(text_scale(600.0, Some(592.0)), 1.0);
        assert_eq!(text_scale(600.0, Some(572.0)), 1.0);
        assert!(text_scale(600.0, Some(570.0)) > 1.0);
        // Wider: the same 5% in the other direction.
        assert_eq!(text_scale(600.0, Some(608.0)), 1.0);
        assert_eq!(text_scale(600.0, Some(630.0)), 1.0);
        assert!(text_scale(600.0, Some(640.0)) < 1.0);
        assert_eq!(text_scale(600.0, Some(900.0)), 600.0 / 900.0);
    }

    #[test]
    fn first_render_uses_the_props_size_so_it_matches_ssr() {
        // Nothing is measured during SSR / the first client render, so the
        // viewBox is exactly the props' default 600x300.
        let html = render(ChartKind::Line, false, true);
        assert!(html.contains(r#"viewBox="0 0 600 300""#), "{html}");
        // ...and the text compensation starts at 1 (no hydration mismatch).
        assert!(html.contains("--dx-chart-text-scale: 1"), "{html}");
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
        #[props(default = 600.0)]
        width: f64,
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
                    width: props.width,
                }
            }
        }
    }

    fn render_axis(data_len: usize, max_x_ticks: usize, x_label: &str) -> String {
        render_axis_at(data_len, max_x_ticks, x_label, 600.0)
    }

    fn render_axis_at(data_len: usize, max_x_ticks: usize, x_label: &str, width: f64) -> String {
        let mut dom = VirtualDom::new_with_props(
            AxisHarness,
            AxisHarnessProps {
                data_len,
                max_x_ticks,
                x_label: x_label.to_string(),
                width,
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

    /// The number of `<text>` tick labels in the x-axis group of `html`.
    fn x_tick_label_count(html: &str) -> usize {
        let start = html.find(r#"data-axis="x""#).expect("x-axis group");
        let after_open = &html[start..];
        let end = after_open.find("</g>").expect("x-axis group closes");
        after_open[..end].matches("<text ").count()
    }

    #[test]
    fn x_axis_tick_count_also_shrinks_to_fit_a_narrow_width() {
        // 90 points, max 12, 3-char labels ("D00": ~21.6 + 8 gap = ~30 per label).
        let wide = x_tick_label_count(&render_axis_at(90, 12, "Category", 600.0));
        let narrow = x_tick_label_count(&render_axis_at(90, 12, "Category", 229.0));
        // This harness draws no y axis, so the plot spans `width - 16`.
        // Wide plot (584) fits 19 labels, so `max_x_ticks` stays the bound.
        assert_eq!(wide, 12);
        // Narrow plot (213) fits floor(213 / 29.6) = 7 -> step 13 -> 7 labels.
        assert_eq!(narrow, 7);
        assert!(narrow < wide);
    }

    #[test]
    fn hidden_table_is_wrapped_so_the_clamp_never_sits_on_the_table() {
        let html = render(ChartKind::Bar, false, true);
        let wrapper = html
            .find(r#"data-slot="chart-data-wrapper""#)
            .expect("wrapper div");
        let table = html.find(r#"data-slot="chart-data""#).expect("table");
        assert!(wrapper < table, "the table is inside the wrapper");
        assert!(html[table..].starts_with(r#"data-slot="chart-data""#));
        assert!(html[..table].rfind("<table").unwrap() > wrapper);
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

    // -- Stage-2 polar kinds (Pie/RadialBar) --------------------------------
    //
    // Each kind renders the same `Harness` used by every Cartesian test
    // above (it's generic over `kind`). RadialBar is still `s2-refactor`'s
    // placeholder stub; Pie is `s2-polar`'s own real `series::pie::render`
    // -- see that module's own doc for why `sample_config`'s two series
    // (desktop/mobile) make this a *stacked* (two-ring) pie, not the
    // single-ring case (covered in detail by `series::pie`'s own test
    // module instead). `ChartKind::Radar` moved out of this section (below,
    // "Radar (landed, s2-radar)") once `s2-radar` replaced its own stub --
    // see that test's own comment for why this section's original
    // assertions about it went stale, not wrong-from-the-start.

    #[test]
    fn pie_kind_renders_one_ring_per_series_and_the_table_without_panicking() {
        let html = render(ChartKind::Pie, false, true);
        assert!(html.contains(r#"data-slot="chart-series""#));
        assert!(html.contains(r#"data-slot="chart-arc""#));
        // Two series (desktop/mobile) -> two rings, each with its own
        // `data-series`; two data points (January/February) each -> 4
        // slices total.
        assert!(html.contains(r#"data-series="desktop""#));
        assert!(html.contains(r#"data-series="mobile""#));
        assert_eq!(html.matches(r#"data-slot="chart-arc""#).count(), 4);
        assert!(html.contains(r#"data-slot="chart-data""#));
        // No Cartesian-only apparatus.
        assert!(!html.contains(r#"data-slot="chart-grid""#));
        assert!(!html.contains(r#"data-slot="chart-axis""#));
        assert!(!html.contains(r#"data-slot="chart-hit-bands""#));
        assert!(!html.contains(r#"data-slot="chart-cursor""#));
    }

    // -- Radar (landed, s2-radar) -------------------------------------------
    //
    // `series::radar::render` replaced its own placeholder body (stage-2
    // chart round, `s2-radar` lane) with real per-category-angle geometry:
    // its own grid/spokes/rim labels and angular hit-sectors (`is_cartesian`
    // is still `false` for `Radar`, so none of that comes from `Chart`'s own
    // Cartesian-only rendering above -- `series::radar`'s own module doc has
    // the full account). This test's ORIGINAL two assertions (`!contains
    // "chart-grid"`, `!contains "chart-hit-bands"`) encoded that placeholder
    // state, not a permanent contract -- flipped here to match, rather than
    // left stale, since a green `cargo test --workspace` gate can't
    // otherwise pass once that family has real content. `chart.rs` is
    // "`s2-refactor` only, forever" per `$S/stage2-lanes.md`'s ownership
    // table for everything else in this file; this one pre-existing test's
    // now-incorrect assertions about a specific `ChartKind`'s stub state are
    // the documented, narrow exception -- `series::radar`'s own module tests
    // carry the actual grid/hit-sector/dot/label coverage.
    #[test]
    fn radar_kind_renders_its_own_grid_and_hit_sectors() {
        let html = render(ChartKind::Radar, false, true);
        assert!(html.contains(r#"data-slot="chart-series""#));
        assert!(html.contains(r#"data-kind="radar""#));
        assert!(html.contains(r#"data-slot="chart-data""#));
        // Radar draws its OWN grid/hit-sectors (not `Chart`'s Cartesian
        // ones, which stay gated off by `is_cartesian` either way) --
        // `series::radar`'s own test module has the detailed coverage.
        assert!(html.contains(r#"data-slot="chart-grid""#));
        assert!(html.contains(r#"data-slot="chart-hit-bands""#));
        // Radar's own rim category labels reuse the `chart-axis` slot with
        // `data-axis="angle"` (not Cartesian `"x"`/`"y"`, which stay
        // absent) -- see `series::radar`'s own module doc for why reusing
        // this slot name costs the themed stylesheet zero new CSS.
        assert!(html.contains(r#"data-axis="angle""#));
        assert!(!html.contains(r#"data-axis="x""#));
        assert!(!html.contains(r#"data-axis="y""#));
        // Still no Cartesian-only apparatus: no rectangular cursor.
        assert!(!html.contains(r#"data-slot="chart-cursor""#));
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
    fn radar_and_radial_bar_reduce_the_table_to_category_plus_first_series_value() {
        for kind in [ChartKind::Radar, ChartKind::RadialBar] {
            let html = render(kind, false, true);
            // Only the first configured series' column header ("Desktop"),
            // not both -- unlike the Cartesian
            // `hidden_table_mirrors_the_data_...` test above, which asserts
            // both "Desktop" AND "Mobile" appear. No "Percent" column --
            // that's `ChartKind::Pie`'s own addition (see the test below).
            assert!(html.contains("Desktop"), "kind={kind:?}");
            assert!(!html.contains("Mobile"), "kind={kind:?}");
            assert!(!html.contains("Percent"), "kind={kind:?}");
            assert!(html.contains("January"), "kind={kind:?}");
            assert!(html.contains("February"), "kind={kind:?}");
        }
    }

    #[test]
    fn pie_kind_table_adds_a_percent_column() {
        let html = render(ChartKind::Pie, false, true);
        // Same single-value-column reduction as Radar/RadialBar (still
        // only "Desktop", never "Mobile" -- `ChartDatum::values[0]` is the
        // hidden table's own reduction, same as `series::pie::render`'s
        // own single-ring geometry reads `values[0]` when there is exactly
        // one series), PLUS this kind's own "Percent" column.
        assert!(html.contains("Desktop"));
        assert!(!html.contains("Mobile"));
        assert!(html.contains(r#"<th scope="col">Percent</th>"#));
        assert!(html.contains("January"));
        assert!(html.contains("February"));
        // January (186) is 100% of the two non-`None` values' sum -- wait,
        // both January (186) and February (305) are real, positive
        // `values[0]`s, so the table's percent column reads their own
        // share of that sum (186+305=491): 186/491 ~= 37.9%.
        assert!(
            html.contains("37.9%"),
            "expected January's own percent share: {html}"
        );
        assert!(
            html.contains("62.1%"),
            "expected February's own percent share: {html}"
        );
    }

    /// The `<g data-axis="...">` group's own markup (up to its first `</g>`).
    fn axis_group<'a>(html: &'a str, axis: &str) -> &'a str {
        let start = html
            .find(&format!(r#"data-axis="{axis}""#))
            .unwrap_or_else(|| panic!("no {axis} axis group: {html}"));
        let rest = &html[start..];
        &rest[..rest.find("</g>").expect("axis group closes")]
    }

    fn text_tags(group: &str) -> Vec<&str> {
        group
            .split("<text ")
            .skip(1)
            .map(|t| &t[..t.find('>').unwrap()])
            .collect()
    }

    #[test]
    fn x_tick_labels_are_centered_on_their_band() {
        let html = render(ChartKind::Bar, false, true);
        let tags = text_tags(axis_group(&html, "x"));
        assert_eq!(tags.len(), 2, "{html}");
        for tag in tags {
            assert!(tag.contains(r#"text-anchor="middle""#), "{tag}");
        }
    }

    #[test]
    fn y_tick_labels_are_end_anchored_and_centered_on_their_gridline() {
        let html = render_dir(None);
        let tags = text_tags(axis_group(&html, "y"));
        assert!(!tags.is_empty(), "{html}");
        for tag in tags {
            assert!(tag.contains(r#"text-anchor="end""#), "{tag}");
            assert!(tag.contains(r#"dominant-baseline="central""#), "{tag}");
            // Right edge 8 units left of the plot (margin 40, scale 1).
            assert!(tag.contains(r#"x="32""#), "{tag}");
        }
    }

    #[test]
    fn the_svg_pins_ltr_so_anchors_do_not_flip_under_rtl() {
        use crate::direction::Direction;
        for dir in [None, Some(Direction::Rtl)] {
            let html = render_dir(dir);
            let svg_open = &html[html.find("<svg").unwrap()..];
            let svg_open = &svg_open[..svg_open.find('>').unwrap()];
            assert!(
                svg_open.contains(r#"direction="ltr""#),
                "{dir:?}: {svg_open}"
            );
            // The chart's own wrapper still reports the requested direction.
            if dir.is_some() {
                assert!(html.contains(r#"data-direction="rtl""#), "{html}");
            }
        }
    }

    fn render_dir(dir: Option<crate::direction::Direction>) -> String {
        let mut dom = VirtualDom::new_with_props(DirHarness, DirHarnessProps { dir });
        dom.rebuild_in_place();
        dom.render_immediate(&mut NoOpMutations);
        dioxus_ssr::render(&dom)
    }

    #[derive(Clone, PartialEq, Props)]
    struct DirHarnessProps {
        dir: Option<crate::direction::Direction>,
    }

    #[component]
    fn DirHarness(props: DirHarnessProps) -> Element {
        let config = use_signal(sample_config);
        let data = use_signal(sample_data);
        rsx! {
            ChartContainer { config, data, kind: ChartKind::Bar,
                Chart { aria_label: "Dir", dir: props.dir, show_y_axis: true }
            }
        }
    }
}
