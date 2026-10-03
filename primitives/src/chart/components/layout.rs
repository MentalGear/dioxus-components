//! Shared plot geometry: margins and axis bands, the category/value scale
//! construction, and the grid/axis rendering, plus [`SeriesRenderContext`]
//! -- the read-only bundle every series family's own `render`
//! (`components::series::{area,bar,line,pie,radar,radial}`) receives.
//!
//! Everything here is in CSS pixels: the chart's svg is always drawn at its
//! measured size (`components::chart`'s sizing contract), so a margin, a
//! tick offset or a label gap is the same number of pixels Recharts uses.
//!
//! The geometry follows Recharts (what shadcn/ui's charts render), rule for
//! rule:
//! - **Plot rect** = the chart box inset by [`ChartMargin`] (default 5 on
//!   every side), minus a 30px x-axis band at the bottom when the x axis is
//!   shown and a 60px y-axis band at the left when the y axis is shown
//!   (Recharts' `XAxis height` / `YAxis width` defaults). Polar kinds get the
//!   margin box alone.
//! - **Category axis**: a [`PointScale`] for Area/Line (first and last
//!   category ON the plot's edges) and an unpadded [`BandScale`] for Bar
//!   (bars are inset inside their band by `series::bar`).
//! - **Value axis**: [`nice_ticks`] (Recharts' `getNiceTickValues`, exactly
//!   `y_tick_count` ticks), and the domain is the first..last tick.
//! - **Category tick labels**: Recharts' default `interval="preserveEnd"`
//!   with `minTickGap` ([`preserve_end_ticks`]), laid out from estimated text
//!   widths so the server and the client agree.
//!
//! Horizontal bars ([`LayoutParams::horizontal`]) swap the two axes: the
//! category scale runs down y, the value scale across x, the y-axis band
//! holds the category labels and the x-axis band the value labels -- one
//! geometry, so the bars, grid, cursor, hit test and tooltip anchors agree.

use dioxus::prelude::*;

use super::chart::ChartMargin;
use crate::chart::engine::scale::{fmt_decimal, fmt_num};
use crate::chart::{
    nice_ticks, stack_with_mode, BandScale, CategoryScale, ChartConfig, ChartDatum, ChartKind,
    Curve, LinearScale, PointScale, StackMode,
};
use crate::direction::Direction;

/// Height of the x-axis band below the plot when the x axis is shown
/// (Recharts' `XAxis` default `height`).
pub(crate) const X_AXIS_HEIGHT: f64 = 30.0;
/// Width of the y-axis band left of the plot when the y axis is shown
/// (Recharts' `YAxis` default `width`).
pub(crate) const Y_AXIS_WIDTH: f64 = 60.0;
/// Recharts' `tickSize`: the tick line's length, by which every tick label
/// is offset from the plot edge even when the tick line itself is hidden
/// (`tickLine={false}`, every shadcn demo), before `tickMargin` is added.
pub(crate) const TICK_SIZE: f64 = 6.0;

/// The inputs [`build`] needs to compute a [`SeriesRenderContext`].
pub(crate) struct LayoutParams<'a> {
    /// The svg's size in CSS px (measured, or the initial size before that).
    pub width: f64,
    pub height: f64,
    pub margin: ChartMargin,
    pub show_x_axis: bool,
    pub show_y_axis: bool,
    pub y_tick_count: usize,
    pub kind: ChartKind,
    /// Horizontal bars: categories down the y axis, values across x.
    pub horizontal: bool,
    /// Already resolved by the caller to this render's *effective* stacking.
    pub stacked: bool,
    /// Which [`stack_with_mode`] mode `stacked`'s spans use.
    pub stack_mode: StackMode,
    pub curve: Curve,
    pub dir: Direction,
    pub active_index: Option<usize>,
    pub config: &'a ChartConfig,
    pub data: &'a [ChartDatum],
}

/// Shared read-only context every series family's `render` needs: the
/// scales and plot geometry [`build`] computed once for this render, plus
/// the chart's own data/config/active index. All lengths are CSS px.
pub(crate) struct SeriesRenderContext {
    #[allow(dead_code)]
    pub kind: ChartKind,
    pub config: ChartConfig,
    pub data: Vec<ChartDatum>,
    #[allow(dead_code)]
    pub dir: Direction,
    /// The currently hovered/keyboard-focused datum index.
    pub active_index: Option<usize>,
    /// The svg's size in CSS px for this render.
    pub width: f64,
    pub height: f64,
    /// The resolved Recharts margin.
    #[allow(dead_code)]
    pub margin: ChartMargin,
    pub plot_x0: f64,
    pub plot_x1: f64,
    pub plot_y0: f64,
    pub plot_y1: f64,
    /// Horizontal bars (see [`LayoutParams::horizontal`]).
    pub horizontal: bool,
    /// The category scale: along x, or along y for horizontal bars.
    pub x_scale: CategoryScale,
    /// The value scale: its range runs up the plot (bottom to top), or for
    /// horizontal bars across it (left to right).
    pub y_scale: LinearScale,
    /// The value ticks ([`nice_ticks`]); the value domain is first..last.
    pub y_ticks: Vec<f64>,
    /// `y_scale.scale(0.0)` -- every non-stacked Area/Bar baseline.
    pub zero_y: f64,
    /// Each datum's category position (`x_scale.center(i)`), aligned to
    /// `data`.
    pub xs: Vec<f64>,
    pub curve: Curve,
    pub stacked: bool,
    /// [`stack_with_mode`]'s output, one row per datum, empty when not
    /// stacking.
    pub stacked_spans: Vec<Vec<(f64, f64)>>,
}

impl SeriesRenderContext {
    /// Series `s`'s raw value for every datum, `None` when a datum has no
    /// entry for it at all.
    pub(crate) fn series_values(&self, s: usize) -> Vec<Option<f64>> {
        self.data
            .iter()
            .map(|d| d.values.get(s).copied().flatten())
            .collect()
    }

    /// The plot rect `(x0, y0, x1, y1)`.
    pub(crate) fn plot(&self) -> (f64, f64, f64, f64) {
        (self.plot_x0, self.plot_y0, self.plot_x1, self.plot_y1)
    }

    /// Whether category labels are drawn for this layout given the two axis
    /// flags: the x axis carries them on a vertical chart, the y axis on
    /// horizontal bars.
    pub(crate) fn category_axis_shown(&self, show_x_axis: bool, show_y_axis: bool) -> bool {
        if self.horizontal {
            show_y_axis
        } else {
            show_x_axis
        }
    }

    /// Whether value labels are drawn (the other axis).
    pub(crate) fn value_axis_shown(&self, show_x_axis: bool, show_y_axis: bool) -> bool {
        if self.horizontal {
            show_x_axis
        } else {
            show_y_axis
        }
    }
}

/// Compute this render's [`SeriesRenderContext`] -- see the module doc for
/// the Recharts rules it follows.
pub(crate) fn build(p: LayoutParams<'_>) -> SeriesRenderContext {
    let cartesian = p.kind.is_cartesian();
    let horizontal = p.horizontal && matches!(p.kind, ChartKind::Bar);
    let m = p.margin;
    let left = m.left
        + if cartesian && p.show_y_axis {
            Y_AXIS_WIDTH
        } else {
            0.0
        };
    let bottom = m.bottom
        + if cartesian && p.show_x_axis {
            X_AXIS_HEIGHT
        } else {
            0.0
        };
    let plot_x0 = left;
    let plot_x1 = (p.width - m.right).max(plot_x0);
    let plot_y0 = m.top;
    let plot_y1 = (p.height - bottom).max(plot_y0);

    let n = p.data.len();
    let x_scale = match (p.kind, horizontal) {
        (ChartKind::Area | ChartKind::Line, _) => CategoryScale::Point(PointScale {
            count: n.max(1),
            range: (plot_x0, plot_x1),
        }),
        (_, true) => CategoryScale::Band(BandScale {
            count: n.max(1),
            range: (plot_y0, plot_y1),
            padding: 0.0,
        }),
        _ => CategoryScale::Band(BandScale {
            count: n.max(1),
            range: (plot_x0, plot_x1),
            padding: 0.0,
        }),
    };
    let xs: Vec<f64> = (0..n).map(|i| x_scale.center(i)).collect();

    let (y_min, y_max) = y_extent(p.config, p.data, p.stacked, p.stack_mode);
    let y_ticks = nice_ticks(y_min, y_max, p.y_tick_count.max(2));
    let domain = (
        y_ticks.first().copied().unwrap_or(0.0),
        y_ticks.last().copied().unwrap_or(1.0),
    );
    let y_scale = LinearScale {
        domain,
        range: if horizontal {
            (plot_x0, plot_x1)
        } else {
            (plot_y1, plot_y0)
        },
    };
    let zero_y = y_scale.scale(0.0);

    let stacked_spans: Vec<Vec<(f64, f64)>> = if p.stacked {
        let rows: Vec<Vec<Option<f64>>> = p.data.iter().map(|d| d.values.clone()).collect();
        stack_with_mode(&rows, p.stack_mode)
    } else {
        Vec::new()
    };

    SeriesRenderContext {
        kind: p.kind,
        config: p.config.clone(),
        data: p.data.to_vec(),
        dir: p.dir,
        active_index: p.active_index,
        width: p.width,
        height: p.height,
        margin: m,
        plot_x0,
        plot_x1,
        plot_y0,
        plot_y1,
        horizontal,
        x_scale,
        y_scale,
        y_ticks,
        zero_y,
        xs,
        curve: p.curve,
        stacked: p.stacked,
        stacked_spans,
    }
}

/// The value-axis data extent: the min/max across every configured series'
/// values, or (for `stacked`) across [`stack_with_mode`]'s own per-row
/// spans. [`nice_ticks`] widens it to include zero.
fn y_extent(
    config: &ChartConfig,
    data: &[ChartDatum],
    stacked: bool,
    stack_mode: StackMode,
) -> (f64, f64) {
    let mut lo = 0.0f64;
    let mut hi = 0.0f64;
    if stacked {
        let rows: Vec<Vec<Option<f64>>> = data.iter().map(|d| d.values.clone()).collect();
        for row in stack_with_mode(&rows, stack_mode) {
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

/// Render `g[data-slot="chart-grid"]`: one line per value tick -- horizontal
/// lines on a vertical chart (shadcn's `<CartesianGrid vertical={false} />`),
/// vertical lines for horizontal bars (`<CartesianGrid horizontal={false} />`).
pub(crate) fn render_grid(ctx: &SeriesRenderContext) -> Element {
    let ticks = grid_ticks(ctx);
    rsx! {
        g { "data-slot": "chart-grid",
            for tick in ticks {
                {
                    let at = fmt_num(ctx.y_scale.scale(tick));
                    if ctx.horizontal {
                        rsx! {
                            line {
                                key: "{tick}",
                                x1: "{at}",
                                x2: "{at}",
                                y1: "{fmt_num(ctx.plot_y0)}",
                                y2: "{fmt_num(ctx.plot_y1)}",
                            }
                        }
                    } else {
                        rsx! {
                            line {
                                key: "{tick}",
                                x1: "{fmt_num(ctx.plot_x0)}",
                                x2: "{fmt_num(ctx.plot_x1)}",
                                y1: "{at}",
                                y2: "{at}",
                            }
                        }
                    }
                }
            }
        }
    }
}

/// The height Recharts measures a hidden y axis' tick text at when it thins
/// the horizontal gridlines: `getStringSize` renders the text in a probe
/// span on `document.body`, outside the chart's `text-xs`, so it is the
/// page's body line (16px * 1.5).
const GRID_TICK_SIZE: f64 = 24.0;

/// Recharts' default `minTickGap`.
const GRID_TICK_GAP: f64 = 5.0;

/// The value ticks that get a gridline. Recharts' `CartesianGrid` thins a
/// vertical chart's horizontal lines like the (hidden) y axis' labels --
/// [`preserve_end_ticks`] over the whole svg height with
/// [`GRID_TICK_SIZE`] -- and then puts the plot's two edges back
/// (`getCoordinatesOfGrid`). So a short plot (a legend's room taken out of a
/// 208px card: ticks 37.5px apart) loses the line just below the top one,
/// exactly as shadcn's legend charts do; 42px apart keeps every line.
fn grid_ticks(ctx: &SeriesRenderContext) -> Vec<f64> {
    let n = ctx.y_ticks.len();
    if ctx.horizontal || n < 3 {
        return ctx.y_ticks.clone();
    }
    let coords: Vec<f64> = ctx.y_ticks.iter().map(|t| ctx.y_scale.scale(*t)).collect();
    let sizes = vec![GRID_TICK_SIZE; n];
    let shown = preserve_end_ticks(&coords, &sizes, 0.0, ctx.height, GRID_TICK_GAP);
    ctx.y_ticks
        .iter()
        .enumerate()
        .filter(|(i, _)| *i == 0 || *i == n - 1 || shown.iter().any(|(s, _)| s == i))
        .map(|(_, tick)| *tick)
        .collect()
}

/// Render the category tick labels: below the plot on a vertical chart
/// (`data-axis="x"`), left of it for horizontal bars (`data-axis="y"`),
/// thinned by [`preserve_end_ticks`] with `min_tick_gap`.
pub(crate) fn render_category_axis(
    ctx: &SeriesRenderContext,
    format: &Option<Callback<String, String>>,
    min_tick_gap: f64,
    tick_margin: f64,
) -> Element {
    let labels: Vec<String> = ctx
        .data
        .iter()
        .map(|datum| format_x_tick(&datum.label, format))
        .collect();
    if ctx.horizontal {
        let sizes = vec![AXIS_LINE_HEIGHT; labels.len()];
        let shown = preserve_end_ticks(&ctx.xs, &sizes, 0.0, ctx.height, min_tick_gap);
        let x = fmt_num(ctx.plot_x0 - TICK_SIZE - tick_margin);
        rsx! {
            g { "data-slot": "chart-axis", "data-axis": "y",
                for (i , y) in shown {
                    text {
                        key: "{i}",
                        "data-index": "{i}",
                        x: "{x}",
                        y: "{fmt_num(y)}",
                        dy: "0.355em",
                        "text-anchor": "end",
                        {labels[i].clone()}
                    }
                }
            }
        }
    } else {
        let sizes: Vec<f64> = labels.iter().map(|l| estimated_text_width(l)).collect();
        let shown = preserve_end_ticks(&ctx.xs, &sizes, 0.0, ctx.width, min_tick_gap);
        let y = fmt_num(ctx.plot_y1 + TICK_SIZE + tick_margin);
        rsx! {
            g { "data-slot": "chart-axis", "data-axis": "x",
                for (i , x) in shown {
                    text {
                        key: "{i}",
                        "data-index": "{i}",
                        x: "{fmt_num(x)}",
                        y: "{y}",
                        // Recharts' `verticalAnchor="start"`: the cap height
                        // hangs from `y`.
                        dy: "0.71em",
                        "text-anchor": "middle",
                        {labels[i].clone()}
                    }
                }
            }
        }
    }
}

/// Render the value tick labels: left of the plot on a vertical chart
/// (`data-axis="y"`, end-anchored and centered on each gridline), below it
/// for horizontal bars (`data-axis="x"`).
pub(crate) fn render_value_axis(ctx: &SeriesRenderContext, tick_margin: f64) -> Element {
    if ctx.horizontal {
        let y = fmt_num(ctx.plot_y1 + TICK_SIZE + tick_margin);
        rsx! {
            g { "data-slot": "chart-axis", "data-axis": "x",
                for tick in ctx.y_ticks.iter().copied() {
                    text {
                        key: "{tick}",
                        x: "{fmt_num(ctx.y_scale.scale(tick))}",
                        y: "{y}",
                        dy: "0.71em",
                        "text-anchor": "middle",
                        {fmt_decimal(tick, 2)}
                    }
                }
            }
        }
    } else {
        let x = fmt_num(ctx.plot_x0 - TICK_SIZE - tick_margin);
        rsx! {
            g { "data-slot": "chart-axis", "data-axis": "y",
                for tick in ctx.y_ticks.iter().copied() {
                    text {
                        key: "{tick}",
                        x: "{x}",
                        y: "{fmt_num(ctx.y_scale.scale(tick))}",
                        // Recharts' `verticalAnchor="middle"`.
                        dy: "0.355em",
                        "text-anchor": "end",
                        {fmt_decimal(tick, 2)}
                    }
                }
            }
        }
    }
}

/// Format a category label: the caller's own formatter if given, else its
/// first 3 characters (shadcn's own demo convention).
fn format_x_tick(label: &str, format: &Option<Callback<String, String>>) -> String {
    match format {
        Some(cb) => cb.call(label.to_string()),
        None => label.chars().take(3).collect(),
    }
}

/// Axis label font size in CSS px (`--dx-text-xs` = 0.75rem).
const AXIS_FONT_PX: f64 = 12.0;
/// The height one axis label occupies along a vertical category axis.
const AXIS_LINE_HEIGHT: f64 = 16.0;

/// Advance width of one glyph as a fraction of the font size -- Helvetica/
/// Arial metrics, which the UI sans-serifs (Geist, Inter, system-ui) track
/// to within a few percent ("Jun 30" at 12px: 36px). Non-ASCII glyphs count
/// as a full em, so the estimate errs on thinning.
fn glyph_em(c: char) -> f64 {
    match c {
        '0'..='9' => 0.556,
        ' ' | '.' | ',' | ':' | ';' | '!' | '/' | '\'' | 'f' | 't' | 'I' => 0.278,
        'i' | 'j' | 'l' => 0.222,
        'r' | '-' | '(' | ')' => 0.333,
        'm' | 'M' => 0.833,
        'w' => 0.722,
        'W' => 0.944,
        '%' => 0.889,
        'c' | 'k' | 's' | 'v' | 'x' | 'y' | 'z' | 'J' => 0.5,
        'a'..='z' => 0.556,
        'A'..='Z' => 0.667,
        c if c.is_ascii() => 0.6,
        _ => 1.0,
    }
}

/// Estimated rendered width of one axis label at [`AXIS_FONT_PX`], without a
/// DOM measurement -- a pure function of the string, so SSR and the client
/// agree for the same width. (Recharts measures the real text; this is the
/// one place we estimate.)
pub(crate) fn estimated_text_width(label: &str) -> f64 {
    label.chars().map(glyph_em).sum::<f64>() * AXIS_FONT_PX
}

/// Recharts' tick-label thinning for its default `interval="preserveEnd"`
/// (`getTicksEnd` + `isVisible`): walk from the LAST tick backwards; the last
/// one is shifted inward if it would overflow `end` (and drawn at the
/// shifted position); a tick is shown when its label (`sizes[i]` wide along
/// the axis, centered on its coordinate) fits between `start` and the
/// previously shown label minus `min_gap`. Returns `(index, drawn
/// coordinate)` of every shown tick, in index order. `start..end` is the
/// whole chart box along the axis (Recharts' axis `viewBox`), so a first
/// label may sit in the margin but never past the svg's edge.
pub(crate) fn preserve_end_ticks(
    coords: &[f64],
    sizes: &[f64],
    start: f64,
    end: f64,
    min_gap: f64,
) -> Vec<(usize, f64)> {
    let n = coords.len().min(sizes.len());
    if n == 0 {
        return Vec::new();
    }
    let sign = if n >= 2 && coords[1] < coords[0] {
        -1.0
    } else {
        1.0
    };
    let (start, mut end) = if sign > 0.0 {
        (start, end)
    } else {
        (end, start)
    };
    let mut shown = Vec::new();
    for i in (0..n).rev() {
        let size = sizes[i];
        let mut coord = coords[i];
        if i == n - 1 {
            let gap = sign * (coord + sign * size / 2.0 - end);
            if gap > 0.0 {
                coord -= gap * sign;
            }
        }
        let fits = sign * (coord - sign * size / 2.0 - start) >= -1e-9
            && sign * (coord + sign * size / 2.0 - end) <= 1e-9;
        if fits {
            end = coord - sign * (size / 2.0 + min_gap);
            shown.push((i, coord));
        }
    }
    shown.reverse();
    shown
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx_for(kind: ChartKind, horizontal: bool, width: f64, height: f64) -> SeriesRenderContext {
        let config = ChartConfig::new().series("desktop", "Desktop", "var(--dx-chart-1)");
        let data: Vec<ChartDatum> = [186.0, 305.0, 237.0, 73.0, 209.0, 214.0]
            .iter()
            .map(|v| ChartDatum {
                label: "Month".to_string(),
                values: vec![Some(*v)],
                ..Default::default()
            })
            .collect();
        build(LayoutParams {
            width,
            height,
            margin: ChartMargin {
                left: 12.0,
                right: 12.0,
                ..ChartMargin::NONE
            },
            show_x_axis: true,
            show_y_axis: false,
            y_tick_count: 5,
            kind,
            horizontal,
            stacked: false,
            stack_mode: StackMode::Normal,
            curve: Curve::Natural,
            dir: Direction::Ltr,
            active_index: None,
            config: &config,
            data: &data,
        })
    }

    #[test]
    fn area_default_plot_and_points_match_shadcn() {
        // chart-area-default at 369x208: plot x 12..357, y 0..178, points
        // 12, 81, ..., 357, five gridlines 0..320.
        let c = ctx_for(ChartKind::Area, false, 369.0, 208.0);
        assert_eq!(c.plot(), (12.0, 0.0, 357.0, 178.0));
        assert_eq!(c.xs, vec![12.0, 81.0, 150.0, 219.0, 288.0, 357.0]);
        assert_eq!(c.y_ticks, vec![0.0, 80.0, 160.0, 240.0, 320.0]);
        assert_eq!(c.y_scale.scale(320.0), 0.0);
        assert_eq!(c.y_scale.scale(0.0), 178.0);
        // shadcn's first point: 186 -> y 74.538.
        assert!((c.y_scale.scale(186.0) - 74.538).abs() < 1e-3);
    }

    #[test]
    fn bar_default_bands_match_shadcn() {
        // chart-bar-default: default margin 5, plot 5..364 x 5..173.
        let config = ChartConfig::new().series("desktop", "Desktop", "var(--dx-chart-1)");
        let data: Vec<ChartDatum> = (0..6)
            .map(|_| ChartDatum {
                label: "M".into(),
                values: vec![Some(305.0)],
                ..Default::default()
            })
            .collect();
        let c = build(LayoutParams {
            width: 369.0,
            height: 208.0,
            margin: ChartMargin::default(),
            show_x_axis: true,
            show_y_axis: false,
            y_tick_count: 5,
            kind: ChartKind::Bar,
            horizontal: false,
            stacked: false,
            stack_mode: StackMode::Normal,
            curve: Curve::Linear,
            dir: Direction::Ltr,
            active_index: None,
            config: &config,
            data: &data,
        });
        assert_eq!(c.plot(), (5.0, 5.0, 364.0, 173.0));
        let (x, w) = c.x_scale.band(0);
        assert_eq!(x, 5.0);
        assert!((w - 59.833).abs() < 1e-3);
    }

    #[test]
    fn horizontal_bars_put_categories_down_y_and_values_across_x() {
        let c = ctx_for(ChartKind::Bar, true, 369.0, 208.0);
        assert!(c.horizontal);
        assert!(c.x_scale.center(0) < c.x_scale.center(1));
        assert!(c.xs.iter().all(|y| *y >= c.plot_y0 && *y <= c.plot_y1));
        assert_eq!(c.y_scale.scale(0.0), c.plot_x0);
        assert_eq!(c.y_scale.scale(320.0), c.plot_x1);
    }

    #[test]
    fn y_axis_and_x_axis_reserve_recharts_bands() {
        let config = ChartConfig::new().series("a", "A", "red");
        let data = vec![ChartDatum {
            label: "x".into(),
            values: vec![Some(1.0)],
            ..Default::default()
        }];
        let c = build(LayoutParams {
            width: 369.0,
            height: 208.0,
            margin: ChartMargin {
                left: -20.0,
                right: 12.0,
                ..ChartMargin::NONE
            },
            show_x_axis: true,
            show_y_axis: true,
            y_tick_count: 3,
            kind: ChartKind::Area,
            horizontal: false,
            stacked: false,
            stack_mode: StackMode::Normal,
            curve: Curve::Linear,
            dir: Direction::Ltr,
            active_index: None,
            config: &config,
            data: &data,
        });
        // chart-area-axes: `margin={{ left: -20, right: 12 }}` + YAxis -> x0 40.
        assert_eq!(c.plot(), (40.0, 0.0, 357.0, 178.0));
    }

    #[test]
    fn estimated_text_width_tracks_ui_sans_metrics() {
        assert_eq!(estimated_text_width(""), 0.0);
        assert!((estimated_text_width("Jun 30") - 36.0).abs() < 0.5);
        assert!((estimated_text_width("\u{4e00}") - 12.0).abs() < 1e-9);
    }

    #[test]
    fn preserve_end_keeps_the_last_label_and_thins_backwards() {
        // Ten labels 20px apart, 30px wide, gap 5: every other one fits.
        let coords: Vec<f64> = (0..10).map(|i| 20.0 + 20.0 * i as f64).collect();
        let sizes = vec![30.0; 10];
        let shown = preserve_end_ticks(&coords, &sizes, 0.0, 220.0, 5.0);
        let idx: Vec<usize> = shown.iter().map(|(i, _)| *i).collect();
        assert_eq!(idx, vec![1, 3, 5, 7, 9]);
        // The last label is never dropped; when it would overflow the box it
        // is drawn shifted inward.
        let shown = preserve_end_ticks(&coords, &sizes, 0.0, 200.0, 5.0);
        assert_eq!(shown.last().unwrap(), &(9, 185.0));
    }

    #[test]
    fn preserve_end_keeps_every_month_of_the_six_month_demos() {
        // chart-area-default: every month label is drawn.
        let c = ctx_for(ChartKind::Area, false, 369.0, 208.0);
        let sizes: Vec<f64> = ["Jan", "Feb", "Mar", "Apr", "May", "Jun"]
            .iter()
            .map(|l| estimated_text_width(l))
            .collect();
        assert_eq!(preserve_end_ticks(&c.xs, &sizes, 0.0, 369.0, 5.0).len(), 6);
    }

    #[test]
    fn preserve_end_gives_the_shadcn_hero_its_18_labels() {
        // chart-line-interactive at 1286px: margin 12, 91 daily points,
        // labels "Apr 1".."Jun 30", `minTickGap={32}`: shadcn draws 18 labels,
        // the last one "Jun 30".
        let p = PointScale {
            count: 91,
            range: (12.0, 1274.0),
        };
        let mut labels = Vec::new();
        for (month, days) in [("Apr", 30), ("May", 31), ("Jun", 30)] {
            for d in 1..=days {
                labels.push(format!("{month} {d}"));
            }
        }
        let coords: Vec<f64> = (0..91).map(|i| p.center(i)).collect();
        let sizes: Vec<f64> = labels.iter().map(|l| estimated_text_width(l)).collect();
        let shown = preserve_end_ticks(&coords, &sizes, 0.0, 1286.0, 32.0);
        assert_eq!(shown.len(), 18, "{shown:?}");
        assert_eq!(shown.last().unwrap().0, 90);
    }

    #[test]
    fn format_x_tick_defaults_to_first_three_characters() {
        assert_eq!(format_x_tick("January", &None), "Jan");
        assert_eq!(format_x_tick("Hi", &None), "Hi");
    }
}
