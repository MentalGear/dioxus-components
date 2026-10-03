//! Defines the [`Chart`] component: the actual SVG drawing surface.
//!
//! ## Sizing contract (Recharts `ResponsiveContainer` semantics)
//!
//! **One user unit is one CSS pixel, always.** The svg's `viewBox` is the
//! chart's own measured size, so every length a chart is authored with --
//! [`ChartMargin`], bar radius, stroke width, dot radius, label offset, a
//! polar radius -- renders at exactly that many pixels, the way shadcn/ui's
//! Recharts charts do. Text is never scaled or compensated.
//!
//! - **Width** = the wrapper's measured content-box width (min 100px).
//! - **Height** = [`ChartProps::height`] when given (a fixed-height,
//!   fluid-width chart: shadcn's `h-[250px]` hero), otherwise width /
//!   [`ChartProps::aspect`] rounded to a whole px (Recharts reads the
//!   container's integer `clientHeight`) -- by default 16/9 for Area/Bar/Line (shadcn's
//!   `aspect-video`) and 1 for Pie/Radar/RadialBar (`aspect-square`; the
//!   themed stylesheet caps a polar chart's container at 250px, shadcn's
//!   `max-h-[250px] mx-auto`).
//! - **Before measuring** (the server render and the first client render,
//!   which therefore agree byte for byte -- no hydration mismatch) the size
//!   is [`ChartProps::width`] (default 369 Cartesian / 250 polar: shadcn's
//!   3-column card) by the same height rule. That render already fills its
//!   container: in aspect mode the svg is `width: 100%; height: auto`, a
//!   uniform scale of the final drawing; in fixed-height mode it is exactly
//!   `height` px tall and stretched across (`preserveAspectRatio="none"`,
//!   text hidden until measured -- `data-measured="false"`).
//! - **A legend lives inside that box**, as in Recharts (whose `<Legend>`
//!   shrinks the plot by its own height): with a `ChartLegend` in the same
//!   container, the svg is the box minus the legend's block size and the
//!   legend takes the rest, so a card is as tall with a legend as without.
//!   The legend's measured size is used once known; before that the
//!   container's children say whether there is one and
//!   [`ChartProps::legend_size`] (default [`DEFAULT_LEGEND_SIZE`], one row)
//!   how big it is -- the same answer on the server and the first client
//!   render. Radar and RadialBar keep their whole square (see
//!   `legend_reserve`).
//! - Every series renderer reads the resolved px size and the plot rect
//!   from `layout::SeriesRenderContext` (`width`, `height`, `plot_*`; for
//!   polar kinds the plot rect is the box inset by the margin).
//!
//! ## What this component does
//!
//! 1. resolves the size (above), the effective `stacked` flag, and the
//!    shared plot geometry via `super::layout::build` (Recharts margins,
//!    axis bands, category/value scales, ticks, stack spans);
//! 2. dispatches on [`ChartKind`] to exactly one family's `render` in
//!    [`super::series`] for the marks;
//! 3. for the three Cartesian kinds additionally renders the grid, axes,
//!    hover cursor and hit bands; the polar kinds get just their marks and a
//!    reduced data table (category + first series' value; plus a `Percent`
//!    column for [`ChartKind::Pie`]).
//!
//! After the first measured client render the svg carries
//! `data-animate="true"` (unless [`ChartProps::animate`] is off), which the
//! themed stylesheet turns into Recharts' load animation -- bars grow from
//! the baseline, lines and areas reveal left to right -- once, never under
//! `prefers-reduced-motion`, and never on the server render (no JS: the
//! final drawing).

use std::rc::Rc;

use dioxus::prelude::*;

use super::pointer::{track_pointer, Axis, HitTest, Sector};
use super::series::{
    AreaOptions, BarOptions, LineOptions, PieOptions, RadarOptions, RadialOptions,
};
use super::{layout, series};
use crate::chart::context::{is_touch, use_chart, ChartLayout, Follow};
use crate::chart::engine::scale::fmt_num;
use crate::chart::engine::table::{table_rows, table_rows_pie, table_rows_single_series};
use crate::chart::{ChartKind, Curve};
use crate::dioxus_attributes::attributes;
use crate::direction::{use_direction, Direction, HorizontalNav};
use crate::merge_attributes;

/// The space between the chart box's edges and its plot, in CSS px --
/// Recharts' `margin` prop. Axis bands (30px for a shown x axis, 60px for a
/// shown y axis) are reserved *inside* it, as Recharts does. Values may be
/// negative (shadcn's `chart-area-axes` pulls the y axis in with `left: -20`).
///
/// `Default` is Recharts' own default, 5px on every side. A Recharts
/// `margin={{ left: 12, right: 12 }}` replaces the whole default (the sides
/// it omits are 0), so its port is `ChartMargin { left: 12.0, right: 12.0,
/// ..ChartMargin::NONE }`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ChartMargin {
    /// Top margin, px.
    pub top: f64,
    /// Right margin, px.
    pub right: f64,
    /// Bottom margin, px.
    pub bottom: f64,
    /// Left margin, px.
    pub left: f64,
}

impl ChartMargin {
    /// No margin on any side -- the base for a partial Recharts margin.
    pub const NONE: Self = Self::all(0.0);

    /// The same margin on every side.
    pub const fn all(px: f64) -> Self {
        Self {
            top: px,
            right: px,
            bottom: px,
            left: px,
        }
    }
}

impl Default for ChartMargin {
    /// Recharts' default: 5px on every side.
    fn default() -> Self {
        Self::all(5.0)
    }
}

/// The initial width (px) of a Cartesian chart before it is measured:
/// shadcn's 3-column card content box (369 x 208 at 16/9).
pub const DEFAULT_CARTESIAN_WIDTH: f64 = 369.0;
/// The initial width (px) of a polar chart before it is measured: shadcn's
/// `max-h-[250px]` square.
pub const DEFAULT_POLAR_WIDTH: f64 = 250.0;
/// The legend's block size (px) assumed before it is measured: one row of
/// shadcn's `ChartLegendContent` (`pt-3` + a 16px `text-xs` line).
pub const DEFAULT_LEGEND_SIZE: f64 = 28.0;
/// Default width / height of a Cartesian chart (shadcn's `aspect-video`).
pub const DEFAULT_CARTESIAN_ASPECT: f64 = 16.0 / 9.0;
/// Default width / height of a polar chart (shadcn's `aspect-square`).
pub const DEFAULT_POLAR_ASPECT: f64 = 1.0;

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

    /// The width in CSS px used before the container is measured (the server
    /// render and the first client render). Once measured, the chart is
    /// always exactly as wide as its container. Default: 369 for
    /// Area/Bar/Line, 250 for Pie/Radar/RadialBar. See the module doc's
    /// sizing contract.
    #[props(default)]
    pub width: Option<f64>,

    /// A fixed height in CSS px: the chart is then fluid in width and exactly
    /// this tall at every container width (shadcn's `h-[250px]` hero
    /// charts). `None` (the default) derives the height from the width and
    /// [`Self::aspect`].
    #[props(default)]
    pub height: Option<f64>,

    /// Width / height when [`Self::height`] is not set. Default 16/9 for
    /// Area/Bar/Line (shadcn's `aspect-video`), 1 for Pie/Radar/RadialBar.
    #[props(default)]
    pub aspect: Option<f64>,

    /// Recharts' `margin`, in CSS px -- see [`ChartMargin`].
    #[props(default)]
    pub margin: ChartMargin,

    /// The block size (CSS px) a `ChartLegend` in the same container is
    /// assumed to take before it is measured (the server render and the
    /// first client render). Like Recharts, the legend is drawn inside the
    /// chart box and the plot gives up that room on the legend's side, so the
    /// box is the same with or without a legend. Defaults to
    /// [`DEFAULT_LEGEND_SIZE`] (one row of shadcn's `ChartLegendContent`);
    /// set it for a legend that wraps. Once measured, the legend's own size
    /// is used. Ignored when the container holds no `ChartLegend`.
    #[props(default)]
    pub legend_size: Option<f64>,

    /// Stack each datum's series values instead of drawing them
    /// independently, in config order (the first series at the bottom, as
    /// Recharts stacks in declaration order). Area and Bar only.
    #[props(default)]
    pub stacked: bool,

    /// The line/area interpolation (Recharts' `type`). Area and Line only.
    #[props(default = Curve::Monotone)]
    pub curve: Curve,

    /// Show gridlines at each value tick (shadcn's `<CartesianGrid
    /// vertical={false} />`; vertical lines for horizontal bars).
    /// Cartesian kinds only.
    #[props(default = true)]
    pub show_grid: bool,

    /// Show the x axis: category labels on a vertical chart, value labels
    /// on horizontal bars. Reserves a 30px band below the plot. Cartesian
    /// kinds only.
    #[props(default = true)]
    pub show_x_axis: bool,

    /// Show the y axis: value labels on a vertical chart, category labels
    /// on horizontal bars. Reserves a 60px band left of the plot. Off by
    /// default, matching shadcn's demos. Cartesian kinds only.
    #[props(default)]
    pub show_y_axis: bool,

    /// The x-axis category column's header in the hidden data table (its
    /// top-left `<th scope="col">`). Defaults to `"Category"`.
    #[props(default = "Category".to_string())]
    pub x_label: String,

    /// Format a category label. Defaults to its first 3 characters
    /// (shadcn's own demo convention, e.g. `"January"` -> `"Jan"`).
    /// Cartesian kinds only.
    #[props(default)]
    pub x_tick_format: Option<Callback<String, String>>,

    /// The minimum gap in px between two category labels (Recharts'
    /// `minTickGap`, default 5). Labels are thinned the way Recharts' default
    /// `interval="preserveEnd"` does: the last category is always labelled,
    /// and walking back from it, a label is drawn only where it clears the
    /// previous one by this gap and fits inside the chart -- from estimated
    /// text widths, so the server and the client agree. Hidden labels never
    /// affect the data, marks or hover. Cartesian kinds only.
    #[props(default = 5.0)]
    pub min_tick_gap: f64,

    /// The distance in px between a tick label and the plot edge beyond the
    /// 6px tick size (Recharts' `tickMargin`; shadcn's demos use 8, bar
    /// demos 10). Cartesian kinds only.
    #[props(default = 8.0)]
    pub tick_margin: f64,

    /// The number of value ticks (Recharts' `tickCount`, default 5): exactly
    /// this many "nice" values ([`crate::chart::nice_ticks`]), and the value
    /// axis spans first..last tick. Cartesian kinds only.
    #[props(default = 5)]
    pub y_tick_count: usize,

    /// Draw the hover cursor (Recharts' `<Tooltip cursor>`, on by default):
    /// a 1px vertical line at the active point on Area/Line charts, a muted
    /// band behind the active category on bar charts. shadcn's demos mostly
    /// turn it off (`cursor={false}`). Cartesian kinds only.
    #[props(default = true)]
    pub cursor: bool,

    /// Play the load animation once after the chart is first measured in
    /// the browser (Recharts' `isAnimationActive`). Never on the server
    /// render, never under `prefers-reduced-motion` (the themed stylesheet).
    #[props(default = true)]
    pub animate: bool,

    /// Enable arrow-key/Home/End/Escape stepping of the active index on
    /// this chart's own focusable wrapper. Tier-3 opinion, cited to
    /// Recharts' `accessibilityLayer` -- additive to, never a replacement
    /// for, the hidden data table every chart always renders.
    #[props(default = true)]
    pub keyboard: bool,

    /// Show the tooltip already open at this datum index until the user
    /// interacts (shadcn's `defaultIndex`): the server render and the first
    /// client render both have it open. The first hover, key press or tap
    /// takes over, and when the tooltip closes it is closed for good. Read
    /// once, on mount. An index past the data's end is ignored.
    /// [`crate::chart::ChartTooltip`] must come after this `Chart` among its
    /// siblings so it sees the index on the server's single render pass.
    #[props(default)]
    pub default_index: Option<usize>,

    /// The text direction for the keyboard layer's ArrowLeft/ArrowRight
    /// swap: a local override that wins over the nearest
    /// [`crate::direction::DirectionProvider`], or LTR if neither is present.
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

    /// [`ChartKind::Pie`]'s own options.
    #[props(default)]
    pub pie: PieOptions,

    /// [`ChartKind::Radar`]'s own options.
    #[props(default)]
    pub radar: RadarOptions,

    /// [`ChartKind::RadialBar`]'s own options.
    #[props(default)]
    pub radial: RadialOptions,

    /// Additional attributes to apply to the chart's own wrapper element
    /// (`Chart` renders one `div[data-slot="chart"]` around its `svg` and
    /// hidden `table`, and this is that div).
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
}

/// # Chart
///
/// Renders the chart's actual drawing surface: an accessible SVG (grid,
/// axes, one mark per configured series, the hover cursor, and the
/// invisible per-datum hit bands for the three Cartesian kinds; the polar
/// families' own marks otherwise) plus a real, visually-hidden `<table>`
/// mirroring the same data, both inside one `div[data-slot="chart"]`
/// wrapper. Must be rendered inside a [`crate::chart::ChartContainer`]. See
/// the module doc for the sizing contract (1 unit = 1 CSS px).
///
/// ## Hover, touch and the tooltip's position
///
/// The wrapper tracks the pointer (`pointermove`/`pointerdown`, in
/// `components::pointer`): client coordinates minus the wrapper's bounding
/// rect, read fresh per event, give the pointer's position in the chart
/// box. A category chart (Area/Bar/Line) resolves the nearest category from
/// that position (the nearest point on Area/Line, the band on bars), hides
/// the tooltip outside the plot, and works unchanged for a finger dragging
/// across it; the polar families resolve the sector under the pointer.
/// `ChartTooltip` then positions itself from the pointer and the active
/// datum's anchor. A mouse leaving closes the tooltip; a finger lifting does
/// not -- a tapped tooltip stays until the wrapper loses focus.
///
/// A pointer result that arrives after the tooltip closed is discarded,
/// never reopening it. Under an ancestor `transform: scale(..)` or CSS
/// `zoom` the pointer's screen-pixel position is mapped onto the chart box's
/// layout pixels, so the tooltip still lands at the pointer.
///
/// Giving [`ChartProps::default_index`] shows the tooltip open at that datum
/// from the first render (shadcn's `defaultIndex`) until the user interacts;
/// Tab-focusing the chart opens it at the first point (pointer-initiated
/// focus does not). On an Area or Line chart the active datum also gets a dot
/// on every series (`data-slot="chart-active-dot"`, Recharts' `activeDot`).
///
/// ## Wrapper element
///
/// This component's own top-level element is a `div`, not the `svg`
/// itself: [`ChartProps::keyboard`]'s `tabindex`/`role="group"`/
/// `aria-roledescription`/`aria-label` land on that wrapper `div`, and
/// [`ChartProps::attributes`] merges onto it too. The `svg` itself stays
/// non-focusable (`role="img"` only).
///
/// ## A11y contract (`dev-docs/research/chart-2026-09-19.md` §2.5/§6.4)
///
/// 1. `svg[role="img"][aria-label]` + `<title>`/`<desc>`.
/// 2. A real `<table>` mirroring the chart's data -- the screen-reader
///    "browse the data" mechanism.
/// 3. Optional (default on) sighted-keyboard stepping of the active index,
///    mirroring Recharts' `accessibilityLayer`: ArrowRight/ArrowLeft move
///    the active index by one (direction-aware), Home/End jump to the first/
///    last datum, Escape clears it. No `role="application"`.
///
/// **Known Dioxus limitation:** `dioxus-html` 0.7.9 has no SVG-namespaced
/// `<title>` element, so the `<title>` inside the `<svg>` is created without
/// the SVG namespace on a CSR-only render. SSR + hydrate (this repo's site)
/// is unaffected, and `aria-label` on the `svg` provides the accessible name
/// either way.
///
/// ## Styling
///
/// - `data-slot="chart"`: the wrapper div.
/// - `data-slot="chart-svg"`: the SVG root; `data-measured="true"|"false"`,
///   `data-fit="width"` (fixed-height mode), `data-animate="true"` (after
///   the first measured client render, unless `animate` is off),
///   `data-bars="horizontal"`, and an inline `--dx-chart-zero` (the value
///   axis' zero, px) for the bar animation's origin.
/// - `data-slot="chart-grid"`/`"chart-axis"` (`data-axis="x"|"y"`) --
///   Cartesian kinds only.
/// - `data-slot="chart-series"[data-series=<key>]`: one per configured
///   series, wrapping that series' own `"chart-area"`/`"chart-line"`/
///   `"chart-bar"`/`"chart-dot"` marks (`data-index` on the per-datum ones).
/// - `data-slot="chart-active-dots"` > `"chart-active-dot"[data-series]
///   [data-index]`: the hovered datum's dot on each series.
/// - `data-slot="chart-cursor"` (`"chart-cursor-line"` for Area/Line,
///   `"chart-cursor-rect"` for Bar) and `"chart-hit-band"[data-index]
///   [data-x]` (`data-x`: the category's own position, svg px).
/// - `data-slot="chart-data"`: the hidden data table, inside a
///   `data-slot="chart-data-wrapper"` div that carries the visually-hidden
///   clamp.
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
    // this also covers mount). `None` -- always the case during SSR and the
    // first client render -- means the props' initial size, so the first
    // client render is byte-identical to the server HTML.
    let mut measured_width = use_signal(|| None::<f64>);
    // The wrapper's mounted handle, whose bounding rect each pointer event
    // reads fresh (see `pointer.rs`).
    let mut mounted = use_signal(|| None::<Rc<MountedData>>);
    // Whether the current focus came from a pointer (`pointerdown` precedes
    // the focus it causes), as opposed to the keyboard: only keyboard focus
    // opens the tooltip (`:focus-visible` semantics -- see `onfocus` below).
    let mut pointer_focus = use_hook(|| CopyValue::new(false));
    let is_cartesian = kind.is_cartesian();
    let ChartSize {
        width,
        height,
        fixed_height,
        pending,
    } = ChartSize::resolve(
        kind,
        props.width,
        props.height,
        props.aspect,
        measured_width(),
    )
    .without_legend(legend_reserve(
        kind,
        (ctx.legend)(),
        (ctx.has_legend)(),
        props.legend_size,
    ));

    // Only Area and Bar have a stacking construction -- an allow-list, so a
    // kind added later is un-stacked until someone opts it in here.
    let stacked = props.stacked && matches!(kind, ChartKind::Area | ChartKind::Bar);
    let stack_mode = match kind {
        ChartKind::Area => props.area.stack_mode,
        _ => crate::chart::StackMode::Normal,
    };
    let horizontal_bars = matches!(kind, ChartKind::Bar) && props.bar.horizontal;

    let n = data.len();
    // `default_index`: open from the very first render (server and client
    // alike). A plain write during this first render, not an effect --
    // `use_hook` runs exactly once, before anything below reads the index.
    use_hook(|| {
        if let Some(i) = props.default_index.filter(|i| *i < n) {
            active_index.set(Some(i));
        }
    });
    let ctx_layout = layout::build(layout::LayoutParams {
        width,
        height,
        margin: props.margin,
        show_x_axis: props.show_x_axis,
        show_y_axis: props.show_y_axis,
        y_tick_count: props.y_tick_count,
        kind,
        horizontal: horizontal_bars,
        stacked,
        stack_mode,
        curve: props.curve,
        dir: direction,
        active_index: active_index(),
        config: &config,
        data: &data,
    });

    // Share this render's tooltip anchors (and how the tooltip follows the
    // pointer) for `ChartTooltip`'s benefit -- a plain write here (not an
    // effect): it depends only on this component's own inputs, and
    // `Signal`'s equality check keeps it a no-op once stable.
    let plot = ctx_layout.plot();
    // The top of the tallest mark at datum `i` (the stacked top when
    // stacked), in data units -- where a tooltip anchored to the data point
    // hangs from.
    let top_value = |i: usize| -> f64 {
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
    };
    let polar_hit = |sectors: Vec<Sector>| HitTest::Sectors {
        view: (width, height),
        center: ((plot.0 + plot.2) / 2.0, (plot.1 + plot.3) / 2.0),
        sectors: sectors.into(),
    };
    let (anchors, follow, hit): (Vec<(f64, f64)>, Follow, HitTest) = if width > 0.0 && height > 0.0
    {
        match kind {
            ChartKind::Bar if horizontal_bars => (
                (0..n)
                    .map(|i| (ctx_layout.y_scale.scale(top_value(i)), ctx_layout.xs[i]))
                    .collect(),
                Follow::PointerX,
                HitTest::Categories {
                    axis: Axis::Y,
                    band: ctx_layout.x_scale,
                    plot,
                    view: (width, height),
                },
            ),
            ChartKind::Area | ChartKind::Line | ChartKind::Bar => (
                (0..n)
                    .map(|i| (ctx_layout.xs[i], ctx_layout.y_scale.scale(top_value(i))))
                    .collect(),
                Follow::PointerY,
                HitTest::Categories {
                    axis: Axis::X,
                    band: ctx_layout.x_scale,
                    plot,
                    view: (width, height),
                },
            ),
            ChartKind::Pie => (
                series::pie::anchors(&ctx_layout, &props.pie),
                // A single ring jumps slice to slice; concentric rings share
                // one datum index across rings, so follow the pointer.
                if config.series.len() > 1 {
                    Follow::Pointer
                } else {
                    Follow::Anchor
                },
                polar_hit(series::pie::sectors(&ctx_layout, &props.pie)),
            ),
            ChartKind::Radar => (
                series::radar::anchors(&ctx_layout, &props.radar),
                Follow::Pointer,
                polar_hit(series::radar::sectors(&ctx_layout, &props.radar)),
            ),
            ChartKind::RadialBar => (
                series::radial::anchors(&ctx_layout, &props.radial),
                Follow::Pointer,
                polar_hit(series::radial::sectors(&ctx_layout, &props.radial)),
            ),
        }
    } else {
        (Vec::new(), Follow::Anchor, HitTest::None)
    };
    let anchor_percent: Vec<(f64, f64)> = anchors
        .into_iter()
        .map(|(x, y)| (x / width * 100.0, y / height * 100.0))
        .collect();
    let slice_rows = match kind {
        ChartKind::Pie => config.series.len() <= 1,
        ChartKind::RadialBar => !props.radial.stacked,
        _ => false,
    };
    layout_signal.set(Some(ChartLayout {
        anchor_percent,
        follow,
        slice_rows,
    }));

    // Pie gets its own row shape (value + percent of the total), Radar the
    // full per-category-per-series table, RadialBar the single-series
    // reduction.
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

    let show_active_dots = match kind {
        ChartKind::Area => true,
        ChartKind::Line => !(props.line.dots || props.line.dot.is_some()),
        _ => false,
    };
    let show_category_axis = ctx_layout.category_axis_shown(props.show_x_axis, props.show_y_axis);
    let show_value_axis = ctx_layout.value_axis_shown(props.show_x_axis, props.show_y_axis);

    let wrapper_role = props.keyboard.then_some("group");
    let wrapper_roledescription = props.keyboard.then_some("chart");
    let wrapper_label = props.keyboard.then(|| props.aria_label.clone());
    // Always focusable by pointer (`-1`) even with `keyboard` off: a tap on
    // the chart focuses its wrapper, and the later blur is how a tapped
    // touch tooltip learns the user tapped elsewhere.
    let wrapper_tabindex = if props.keyboard { 0 } else { -1 };

    // Structural/aria wiring this component owns wins over a caller's
    // same-named attribute (`merge_attributes`, owned last); `class` still
    // concatenates.
    let owned = attributes!(div {
        "data-slot": "chart",
        role: wrapper_role,
        "aria-roledescription": wrapper_roledescription,
        tabindex: wrapper_tabindex,
        "data-direction": direction.as_str(),
    });
    let merged = merge_attributes(vec![props.attributes, owned]);
    let animate = (props.animate && !pending).then_some("true");

    rsx! {
        div {
            aria_label: wrapper_label,
            dir: direction.as_str(),
            // The one writer of the chart box's layout size (the pointer
            // handler maps its screen-pixel rect onto it, `pointer.rs`). The
            // border box, to match the bounding rect the pointer reads; the
            // svg's own width (`measured_width`) is the content box.
            onresize: move |evt: ResizeEvent| {
                let data = evt.data();
                if let Ok(size) = data.get_content_box_size() {
                    adopt_measured_width(&mut measured_width, size.width);
                }
                if let Ok(size) = data.get_border_box_size() {
                    if size.width > 0.0 && size.height > 0.0 {
                        ctx.set_box_size((size.width, size.height));
                    }
                }
            },
            onmounted: move |evt: MountedEvent| mounted.set(Some(evt.data())),
            onpointermove: {
                let hit = hit.clone();
                move |evt| track_pointer(&evt, ctx, mounted, hit.clone())
            },
            // A touch tap is a pointerdown with no preceding move: show the
            // tooltip for the tapped category right away.
            onpointerdown: move |evt| {
                pointer_focus.set(true);
                track_pointer(&evt, ctx, mounted, hit.clone());
            },
            // Keyboard focus (Tab) shows the first point at once, like
            // shadcn's `accessibilityLayer`. A pointer-initiated focus
            // (`pointerdown` came first) does not.
            onfocus: move |_| {
                let from_pointer = *pointer_focus.peek();
                pointer_focus.set(false);
                if focus_opens_tooltip(from_pointer, props.keyboard, n) {
                    ctx.take_over_by_keyboard();
                    active_index.set(Some(0));
                }
            },
            // A mouse leaving closes the tooltip. A finger lifting fires the
            // same event but must not: a tapped tooltip stays until the next
            // tap elsewhere (`onblur` below).
            onpointerleave: move |evt| {
                if !is_touch(&evt) {
                    ctx.clear();
                }
            },
            // The browser took the touch over (a page scroll): drop it.
            onpointercancel: move |_| ctx.clear(),
            // Focus left the chart: close whatever is open.
            onblur: move |_| {
                pointer_focus.set(false);
                ctx.clear();
            },
            onkeydown: move |evt| {
                if !props.keyboard || n == 0 {
                    return;
                }
                let key = evt.key();
                let take_over = || ctx.take_over_by_keyboard();
                if key == Key::Home {
                    take_over();
                    active_index.set(Some(0));
                    evt.prevent_default();
                } else if key == Key::End {
                    take_over();
                    active_index.set(Some(n - 1));
                    evt.prevent_default();
                } else if key == Key::Escape {
                    take_over();
                    active_index.set(None);
                    evt.prevent_default();
                } else if let Some(nav) = direction.resolve_horizontal(&key) {
                    take_over();
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
                style: svg_style(fixed_height.then_some(height), ctx_layout.zero_y),
                // Fixed-height mode, pre-measure: stretch the initial-width
                // viewBox across the container (the height is already exact,
                // so only x is scaled; text is hidden by the stylesheet
                // meanwhile). Measured, the viewBox IS the box.
                preserve_aspect_ratio: (fixed_height && pending).then_some("none"),
                "data-fit": fixed_height.then_some("width"),
                "data-measured": if pending { "false" } else { "true" },
                "data-animate": animate,
                // Which way touch may scrub (`touch-action` in the themed
                // stylesheet): a horizontal bar chart scrubs along y.
                "data-bars": horizontal_bars.then_some("horizontal"),
                // The drawing is NOT mirrored under `dir="rtl"`, so its text
                // must not flip either: pinned once here, on the root.
                "direction": "ltr",

                title { "{props.aria_label}" }
                if let Some(description) = &props.description {
                    desc { "{description}" }
                }

                if is_cartesian {
                    if props.show_grid {
                        {layout::render_grid(&ctx_layout)}
                    }
                    if show_category_axis {
                        {
                            layout::render_category_axis(
                                &ctx_layout,
                                &props.x_tick_format,
                                props.min_tick_gap,
                                props.tick_margin,
                            )
                        }
                    }
                    if show_value_axis {
                        {layout::render_value_axis(&ctx_layout, props.tick_margin)}
                    }
                    // Recharts draws the tooltip cursor below the marks.
                    g { "data-slot": "chart-cursor",
                        if props.cursor {
                            if let Some(i) = active_index().filter(|i| *i < n) {
                                {render_cursor(&ctx_layout, kind, i)}
                            }
                        }
                    }
                }

                {marks}

                if is_cartesian {
                    // The hovered point's dot on every series (shadcn's
                    // Recharts `activeDot`), above the marks. A line chart
                    // that draws its own dots enlarges the active one
                    // instead (`series::line`).
                    if show_active_dots {
                        {series::active_dot::render(&ctx_layout)}
                    }

                    // The stable per-datum hit regions tests and styles
                    // address (hover itself is resolved from coordinates,
                    // `pointer.rs`): each category's band, kept inside the
                    // svg box, with the category's own position as `data-x`.
                    // A horizontal bar chart draws its own, along y
                    // (`series::bar`).
                    g { "data-slot": "chart-hit-bands",
                        for i in 0..(if horizontal_bars { 0 } else { n }) {
                            {
                                let (bx, bw) = ctx_layout.x_scale.band(i);
                                let x0 = bx.max(0.0);
                                let bw = ((bx + bw).min(width) - x0).max(0.0);
                                let bx = x0;
                                rsx! {
                                    rect {
                                        key: "{i}",
                                        "data-slot": "chart-hit-band",
                                        "data-index": "{i}",
                                        "data-x": "{fmt_num(ctx_layout.xs[i])}",
                                        x: "{fmt_num(bx)}",
                                        y: "{fmt_num(ctx_layout.plot_y0)}",
                                        width: "{fmt_num(bw)}",
                                        height: "{fmt_num(ctx_layout.plot_y1 - ctx_layout.plot_y0)}",
                                        fill: "transparent",
                                        "pointer-events": "all",
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // The visually-hidden clamp lives on this WRAPPER, never on the
            // table: a `table` box cannot shrink below its min-content
            // width, and changing its `display` would strip its table
            // semantics in WebKit/VoiceOver.
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

/// The hover cursor at datum `i` (Recharts' default tooltip cursor, as
/// shadcn styles it): a full-height line through the point on Area/Line, a
/// band over the whole category on bars (across the plot for horizontal
/// bars).
fn render_cursor(ctx: &layout::SeriesRenderContext, kind: ChartKind, i: usize) -> Element {
    let (x0, y0, x1, y1) = ctx.plot();
    if matches!(kind, ChartKind::Bar) {
        let (start, size) = ctx.x_scale.band(i);
        let (x, y, w, h) = if ctx.horizontal {
            (x0, start, x1 - x0, size)
        } else {
            (start, y0, size, y1 - y0)
        };
        rsx! {
            rect {
                "data-slot": "chart-cursor-rect",
                x: "{fmt_num(x)}",
                y: "{fmt_num(y)}",
                width: "{fmt_num(w)}",
                height: "{fmt_num(h)}",
            }
        }
    } else {
        let cx = fmt_num(ctx.xs[i]);
        rsx! {
            line {
                "data-slot": "chart-cursor-line",
                x1: "{cx}",
                x2: "{cx}",
                y1: "{fmt_num(y0)}",
                y2: "{fmt_num(y1)}",
            }
        }
    }
}

/// Whether a focus event opens the tooltip at the first point: only focus the
/// keyboard moved there, on a chart whose keyboard layer is on and that has
/// data. Focus a pointer caused (`from_pointer`) never does.
fn focus_opens_tooltip(from_pointer: bool, keyboard: bool, n: usize) -> bool {
    !from_pointer && keyboard && n > 0
}

/// The narrowest width [`ChartSize::resolve`] follows: a container narrower
/// than this (a collapsed or hidden one) still gets a layout with a positive
/// plot, drawn scaled down instead.
const MIN_WIDTH: f64 = 100.0;

/// The chart's size in CSS px for one render -- the single place the sizing
/// props and the measured container width combine (see the module doc).
#[derive(Debug, Clone, Copy, PartialEq)]
struct ChartSize {
    width: f64,
    height: f64,
    /// [`ChartProps::height`] was given: a fixed height, fluid width.
    fixed_height: bool,
    /// No usable measurement yet (the server and the first client render):
    /// drawn at the initial width.
    pending: bool,
}

impl ChartSize {
    fn resolve(
        kind: ChartKind,
        width: Option<f64>,
        height: Option<f64>,
        aspect: Option<f64>,
        measured: Option<f64>,
    ) -> Self {
        let polar = !kind.is_cartesian();
        let positive = |v: f64| v.is_finite() && v > 0.0;
        let initial = width.filter(|w| positive(*w)).unwrap_or(if polar {
            DEFAULT_POLAR_WIDTH
        } else {
            DEFAULT_CARTESIAN_WIDTH
        });
        let aspect = aspect.filter(|a| positive(*a)).unwrap_or(if polar {
            DEFAULT_POLAR_ASPECT
        } else {
            DEFAULT_CARTESIAN_ASPECT
        });
        let fixed = height.filter(|h| positive(*h));
        let usable = measured.filter(|m| positive(*m));
        let width = usable.map_or(initial, |m| m.max(MIN_WIDTH));
        ChartSize {
            width,
            // Whole px, like the `clientHeight` Recharts' ResponsiveContainer
            // reads (shadcn's 369px card: 208, not 207.56).
            height: fixed.unwrap_or((width / aspect).round()),
            fixed_height: fixed.is_some(),
            pending: usable.is_none(),
        }
    }
}

/// How much of the chart box a `ChartLegend` in the same container takes
/// (Recharts draws its legend inside the `ResponsiveContainer` box and the
/// plot gives up the legend's height): its measured box once known, else --
/// the server render and the first client render, which must agree -- the
/// assumed `initial` size when the container has a legend. Area, Bar, Line
/// and Pie only: Recharts centres Radar and RadialBar on the whole box
/// (`formatAxisMap`) and shadcn's radar demos cancel their legend with
/// negative margins, so those keep their full square with the legend after.
fn legend_reserve(
    kind: ChartKind,
    measured: Option<f64>,
    has_legend: bool,
    initial: Option<f64>,
) -> f64 {
    if matches!(kind, ChartKind::Radar | ChartKind::RadialBar) {
        return 0.0;
    }
    measured.unwrap_or_else(|| {
        if has_legend {
            initial
                .filter(|s| s.is_finite() && *s >= 0.0)
                .unwrap_or(DEFAULT_LEGEND_SIZE)
        } else {
            0.0
        }
    })
}

impl ChartSize {
    /// The svg's share of the box once the legend's `reserve` px are taken
    /// out (the legend sits in the container right above or below the svg,
    /// so svg + legend is the box the sizing props asked for). Never less
    /// than half the box, so a runaway legend cannot collapse the plot.
    fn without_legend(self, reserve: f64) -> Self {
        let reserve = reserve.clamp(0.0, (self.height / 2.0).floor());
        Self {
            height: self.height - reserve,
            ..self
        }
    }
}

/// The SVG's inline style: the value axis' zero (the bar animation's
/// origin), plus -- fixed-height mode -- the CSS height that beats the
/// stylesheet's `height: auto`.
fn svg_style(fixed_height: Option<f64>, zero: f64) -> String {
    let zero = format!("--dx-chart-zero: {}px", fmt_num(zero));
    match fixed_height {
        Some(h) => format!("{zero}; height: {}px", fmt_num(h)),
        None => zero,
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
    use crate::chart::{ChartConfig, ChartContainer, ChartDatum, ChartTooltip};
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
        #[props(default)]
        height: Option<f64>,
        #[props(default)]
        default_index: Option<usize>,
        #[props(default)]
        line_dots: bool,
        #[props(default)]
        with_tooltip: bool,
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
                    height: props.height,
                    default_index: props.default_index,
                    line: crate::chart::LineOptions {
                        dots: props.line_dots,
                        ..Default::default()
                    },
                }
                if props.with_tooltip {
                    ChartTooltip {}
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
        render_with(kind, stacked, keyboard, false)
    }

    fn render_with(kind: ChartKind, stacked: bool, keyboard: bool, fixed_height: bool) -> String {
        render_props(HarnessProps {
            kind,
            stacked,
            keyboard,
            height: fixed_height.then_some(250.0),
            default_index: None,
            line_dots: false,
            with_tooltip: false,
        })
    }

    fn render_props(props: HarnessProps) -> String {
        let mut dom = VirtualDom::new_with_props(Harness, props);
        dom.rebuild_in_place();
        dom.render_immediate(&mut NoOpMutations);
        dioxus_ssr::render(&dom)
    }

    fn render_default(kind: ChartKind, default_index: Option<usize>, line_dots: bool) -> String {
        render_props(HarnessProps {
            kind,
            stacked: false,
            keyboard: true,
            height: None,
            default_index,
            line_dots,
            with_tooltip: true,
        })
    }

    #[component]
    fn LegendHarness(legend: bool, legend_size: Option<f64>) -> Element {
        let config = use_signal(sample_config);
        let data = use_signal(sample_data);
        rsx! {
            ChartContainer { config, data, kind: ChartKind::Area,
                Chart { aria_label: "Visitors", legend_size }
                if legend {
                    crate::chart::ChartLegend {}
                }
            }
        }
    }

    /// The y of the lowest gridline (the plot's bottom edge).
    fn plot_bottom(legend: bool, legend_size: Option<f64>) -> (String, f64) {
        let mut dom = VirtualDom::new_with_props(
            LegendHarness,
            LegendHarnessProps {
                legend,
                legend_size,
            },
        );
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        let grid = &html[html.find(r#"data-slot="chart-grid""#).unwrap()..];
        let grid = &grid[..grid.find("</g>").unwrap()];
        let bottom = grid
            .match_indices(r#"y1=""#)
            .map(|(at, key)| {
                let v = &grid[at + key.len()..];
                v[..v.find('"').unwrap()].parse::<f64>().unwrap()
            })
            .fold(f64::NEG_INFINITY, f64::max);
        (html, bottom)
    }

    #[test]
    fn a_legend_takes_its_room_inside_the_chart_box_from_the_first_render() {
        // Recharts draws the legend inside the chart box: svg + legend is
        // the 369x208 box and the plot gives up the legend's height -- known
        // on the server render already, so hydration finds the same tree.
        let (plain, without) = plot_bottom(false, None);
        let (with_html, with) = plot_bottom(true, None);
        assert!(plain.contains(r#"viewBox="0 0 369 208""#), "{plain}");
        assert!(
            with_html.contains(r#"viewBox="0 0 369 180""#),
            "{with_html}"
        );
        assert_eq!(without - with, DEFAULT_LEGEND_SIZE);
        // A wrapping legend declares its own initial size.
        let (_, wrapped) = plot_bottom(true, Some(52.0));
        assert_eq!(without - wrapped, 52.0);
        // Without a legend in the container the hint is ignored.
        assert_eq!(plot_bottom(false, Some(52.0)).1, without);
    }

    #[test]
    fn the_measured_legend_wins_over_the_assumed_one() {
        let measured = Some;
        let area = ChartKind::Area;
        assert_eq!(legend_reserve(area, None, false, None), 0.0);
        assert_eq!(legend_reserve(area, None, true, None), DEFAULT_LEGEND_SIZE);
        assert_eq!(legend_reserve(area, None, true, Some(52.0)), 52.0);
        assert_eq!(legend_reserve(area, measured(52.0), true, Some(28.0)), 52.0);
        // A legend wrapped in a component of another name is still measured.
        assert_eq!(legend_reserve(area, measured(30.0), false, None), 30.0);
        assert_eq!(legend_reserve(ChartKind::Pie, None, true, None), 28.0);
        // Radar/RadialBar keep their whole square (see `legend_reserve`).
        for kind in [ChartKind::Radar, ChartKind::RadialBar] {
            assert_eq!(legend_reserve(kind, measured(28.0), true, None), 0.0);
        }
        let s = ChartSize::resolve(area, None, None, None, None).without_legend(28.0);
        assert_eq!((s.width, s.height), (369.0, 180.0));
        let s = ChartSize::resolve(area, None, Some(250.0), None, Some(900.0)).without_legend(28.0);
        assert_eq!((s.width, s.height, s.fixed_height), (900.0, 222.0, true));
        let s = ChartSize::resolve(area, None, None, None, None).without_legend(500.0);
        assert_eq!(s.height, 104.0, "never below half the box");
    }

    #[test]
    fn first_render_uses_the_initial_size_so_it_matches_ssr() {
        // Nothing is measured during SSR / the first client render: a
        // Cartesian chart is shadcn's 369px card at 16/9, a polar one 250x250.
        let html = render(ChartKind::Line, false, true);
        assert!(html.contains(r#"viewBox="0 0 369 208""#), "{html}");
        assert!(html.contains(r#"data-measured="false""#), "{html}");
        // No animation on the server render (no JS: the final drawing).
        assert!(!html.contains("data-animate"), "{html}");
        for kind in [ChartKind::Pie, ChartKind::Radar, ChartKind::RadialBar] {
            let html = render(kind, false, true);
            assert!(html.contains(r#"viewBox="0 0 250 250""#), "{kind:?}");
        }
    }

    #[test]
    fn aspect_mode_tracks_the_measured_width_at_one_unit_per_px() {
        for measured in [329.0, 369.0, 642.0, 1286.0] {
            let s = ChartSize::resolve(ChartKind::Area, None, None, None, Some(measured));
            assert_eq!(s.width, measured, "1 unit = 1 CSS px");
            assert_eq!(s.height, (measured * 9.0 / 16.0).round(), "whole px");
            assert!(!s.fixed_height && !s.pending);
            let p = ChartSize::resolve(ChartKind::Pie, None, None, None, Some(measured));
            assert_eq!((p.width, p.height), (measured, measured), "polar square");
        }
        // A caller's own aspect.
        let s = ChartSize::resolve(ChartKind::Bar, None, None, Some(2.0), Some(400.0));
        assert_eq!((s.width, s.height), (400.0, 200.0));
    }

    #[test]
    fn fixed_height_mode_is_fluid_in_width_only() {
        for measured in [390.0, 642.0, 1068.0, 1440.0] {
            let s = ChartSize::resolve(
                ChartKind::Area,
                Some(700.0),
                Some(250.0),
                None,
                Some(measured),
            );
            assert_eq!((s.width, s.height), (measured, 250.0));
            assert!(s.fixed_height && !s.pending);
        }
    }

    #[test]
    fn unmeasured_uses_the_initial_width_and_never_collapses() {
        let s = ChartSize::resolve(ChartKind::Area, Some(700.0), Some(250.0), None, None);
        assert_eq!((s.width, s.height, s.pending), (700.0, 250.0, true));
        for bad in [0.0, -3.0, f64::NAN, f64::INFINITY] {
            let s = ChartSize::resolve(ChartKind::Bar, None, None, None, Some(bad));
            assert_eq!(s.width, DEFAULT_CARTESIAN_WIDTH, "{bad}");
            assert!(s.pending, "{bad}");
            // Unusable props fall back to the defaults too.
            let s = ChartSize::resolve(ChartKind::Bar, Some(bad), Some(bad), Some(bad), None);
            assert_eq!(
                (s.width, s.fixed_height),
                (DEFAULT_CARTESIAN_WIDTH, false),
                "{bad}"
            );
        }
    }

    #[test]
    fn a_tiny_container_keeps_a_positive_plot() {
        let s = ChartSize::resolve(ChartKind::Line, None, Some(250.0), None, Some(12.0));
        assert_eq!(s.width, MIN_WIDTH);
    }

    #[test]
    fn fixed_height_ssr_is_stretched_to_the_box_with_its_text_held_back() {
        let html = render_with(ChartKind::Area, false, true, true);
        assert!(html.contains(r#"viewBox="0 0 369 250""#), "{html}");
        assert!(html.contains("height: 250px"), "{html}");
        assert!(html.contains(r#"preserveAspectRatio="none""#), "{html}");
        assert!(html.contains(r#"data-fit="width""#), "{html}");
        assert!(html.contains(r#"data-measured="false""#), "{html}");
    }

    #[test]
    fn aspect_mode_ssr_is_a_uniform_scale_of_the_final_drawing() {
        for kind in [ChartKind::Area, ChartKind::Bar, ChartKind::Line] {
            let html = render(kind, false, true);
            assert!(!html.contains("data-fit"), "{kind:?}");
            assert!(!html.contains("preserveAspectRatio"), "{kind:?}");
            assert!(!html.contains("height: "), "{kind:?}: {html}");
        }
    }

    #[test]
    fn keyboard_focus_opens_the_first_point_but_pointer_focus_does_not() {
        assert!(focus_opens_tooltip(false, true, 6), "Tab");
        assert!(!focus_opens_tooltip(true, true, 6), "tap / click focus");
        assert!(!focus_opens_tooltip(false, false, 6), "keyboard layer off");
        assert!(!focus_opens_tooltip(false, true, 0), "no data");
    }

    #[test]
    fn default_index_opens_the_tooltip_on_the_server_render() {
        let html = render_default(ChartKind::Area, Some(1), false);
        assert!(html.contains(r#"data-state="open""#), "{html}");
        // February's label and desktop value are in the tooltip...
        assert!(
            html.contains(r#"data-slot="chart-tooltip-label""#),
            "{html}"
        );
        assert!(html.contains("305"), "{html}");
        // ...with the cursor and the active dots at that datum.
        assert!(html.contains(r#"data-slot="chart-cursor-line""#), "{html}");
        assert!(html.contains(r#"data-index="1""#));
    }

    #[test]
    fn default_index_is_closed_when_unset_or_past_the_data() {
        for default in [None, Some(2), Some(99)] {
            let html = render_default(ChartKind::Area, default, false);
            assert!(html.contains(r#"data-state="closed""#), "{default:?}");
            assert!(!html.contains("chart-cursor-line"), "{default:?}");
            assert!(
                !html.contains(r#"data-slot="chart-active-dot""#),
                "{default:?}"
            );
        }
    }

    #[test]
    fn default_index_works_for_every_family() {
        for kind in [
            ChartKind::Area,
            ChartKind::Bar,
            ChartKind::Line,
            ChartKind::Pie,
            ChartKind::Radar,
            ChartKind::RadialBar,
        ] {
            let html = render_default(kind, Some(0), false);
            assert!(html.contains(r#"data-state="open""#), "{kind:?}: {html}");
        }
    }

    /// `data-slot="chart-active-dot"` circles in `html`, as (series, index).
    fn active_dots(html: &str) -> Vec<(String, String)> {
        let attr = |tag: &str, name: &str| {
            let key = format!(r#"{name}=""#);
            let at = tag.find(&key).unwrap() + key.len();
            tag[at..at + tag[at..].find('"').unwrap()].to_string()
        };
        html.split("<circle ")
            .skip(1)
            .map(|t| &t[..t.find('>').unwrap()])
            .filter(|t| t.contains(r#"data-slot="chart-active-dot""#))
            .map(|t| (attr(t, "data-series"), attr(t, "data-index")))
            .collect()
    }

    #[test]
    fn area_and_line_draw_an_active_dot_per_series_with_a_value() {
        for kind in [ChartKind::Area, ChartKind::Line] {
            // January has both values; February's mobile is a gap.
            let both = active_dots(&render_default(kind, Some(0), false));
            assert_eq!(
                both,
                vec![
                    ("desktop".to_string(), "0".to_string()),
                    ("mobile".to_string(), "0".to_string())
                ],
                "{kind:?}"
            );
            let gap = active_dots(&render_default(kind, Some(1), false));
            assert_eq!(
                gap,
                vec![("desktop".to_string(), "1".to_string())],
                "{kind:?}"
            );
            // Nothing active: no dots.
            assert!(active_dots(&render_default(kind, None, false)).is_empty());
        }
    }

    #[test]
    fn the_active_dot_is_r4_in_the_series_color() {
        let html = render_default(ChartKind::Area, Some(0), false);
        let dot = html
            .split("<circle ")
            .skip(1)
            .map(|t| &t[..t.find('>').unwrap()])
            .find(|t| t.contains(r#"data-series="desktop""#) && t.contains("chart-active-dot"))
            .expect("desktop's active dot");
        assert!(dot.contains(r#"r="4""#), "{dot}");
        assert!(
            dot.contains("--series-color: var(--color-desktop)"),
            "{dot}"
        );
    }

    #[test]
    fn a_line_chart_with_its_own_dots_and_bars_use_no_active_dot_layer() {
        // Line `dots: true` enlarges its own active dot instead (`series::line`).
        let html = render_default(ChartKind::Line, Some(0), true);
        assert!(active_dots(&html).is_empty(), "{html}");
        assert!(!html.contains("chart-active-dots"), "{html}");
        let bar = render_default(ChartKind::Bar, Some(0), false);
        assert!(!bar.contains("chart-active-dots"), "{bar}");
    }

    #[test]
    fn the_cursor_prop_turns_the_hover_cursor_off() {
        let on = render_default(ChartKind::Area, Some(1), false);
        assert!(on.contains("chart-cursor-line"), "{on}");
        let mut dom = VirtualDom::new(NoCursorHarness);
        dom.rebuild_in_place();
        dom.render_immediate(&mut NoOpMutations);
        let off = dioxus_ssr::render(&dom);
        assert!(off.contains(r#"data-slot="chart-cursor""#), "{off}");
        assert!(!off.contains("chart-cursor-rect"), "{off}");
        // The tooltip and the bar's own data are unaffected.
        assert!(off.contains(r#"data-state="open""#), "{off}");
    }

    #[component]
    fn NoCursorHarness() -> Element {
        let config = use_signal(sample_config);
        let data = use_signal(sample_data);
        rsx! {
            ChartContainer { config, data, kind: ChartKind::Bar,
                Chart { aria_label: "T", cursor: false, default_index: 1 }
                ChartTooltip {}
            }
        }
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
        // Pointer-focusable (`-1`) but never in the tab order: a tap on the
        // chart focuses its wrapper so a later blur can close a touch tooltip.
        assert!(html.contains("tabindex=-1"));
        assert!(!html.contains("tabindex=0"));
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
    /// `min_tick_gap`/an arbitrary data length) -- kept separate from
    /// [`Harness`] above so every existing test's fixed 2-datum shape is
    /// untouched.
    #[derive(Clone, PartialEq, Props)]
    struct AxisHarnessProps {
        #[props(default = 2)]
        data_len: usize,
        #[props(default = 5.0)]
        min_tick_gap: f64,
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
                    min_tick_gap: props.min_tick_gap,
                    width: props.width,
                }
            }
        }
    }

    fn render_axis(data_len: usize, min_tick_gap: f64, x_label: &str) -> String {
        render_axis_at(data_len, min_tick_gap, x_label, 600.0)
    }

    fn render_axis_at(data_len: usize, min_tick_gap: f64, x_label: &str, width: f64) -> String {
        let mut dom = VirtualDom::new_with_props(
            AxisHarness,
            AxisHarnessProps {
                data_len,
                min_tick_gap,
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
        let html = render_axis(2, 5.0, "Date");
        assert!(
            html.contains(r#"<th scope="col">Date</th>"#),
            "expected the overridden corner header: {html}"
        );
    }

    #[test]
    fn x_axis_tick_labels_are_thinned_from_the_end_for_a_dense_chart() {
        let html = render_axis(90, 5.0, "Category");
        let axis_group = axis_group(&html, "x");
        let tick_labels = axis_group.matches("<text ").count();
        assert!(tick_labels > 1 && tick_labels < 90, "{axis_group}");
        // Recharts' `preserveEnd`: the LAST datum is always labelled.
        assert!(axis_group.contains("D89"), "{axis_group}");
        // Thinning only removes labels, never data or interactivity.
        assert_eq!(html.matches(r#"data-slot="chart-hit-band""#).count(), 90);
    }

    /// The number of `<text>` tick labels in the x-axis group of `html`.
    fn x_tick_label_count(html: &str) -> usize {
        axis_group(html, "x").matches("<text ").count()
    }

    #[test]
    fn x_axis_tick_count_follows_the_width_and_the_min_tick_gap() {
        // The initial width stands in for a measured one.
        let wide = x_tick_label_count(&render_axis_at(90, 5.0, "Category", 1200.0));
        let narrow = x_tick_label_count(&render_axis_at(90, 5.0, "Category", 300.0));
        assert!(narrow < wide, "{narrow} !< {wide}");
        let gappy = x_tick_label_count(&render_axis_at(90, 32.0, "Category", 1200.0));
        assert!(gappy < wide, "{gappy} !< {wide}");
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
            assert!(tag.contains(r#"dy="0.355em""#), "{tag}");
            // Right edge tickSize 6 + tickMargin 8 left of the plot (margin
            // 5 + the 60px y-axis band).
            assert!(tag.contains(r#"x="51""#), "{tag}");
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
