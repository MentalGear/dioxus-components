//! The [`ChartKind::Line`](crate::chart::ChartKind::Line) family: a
//! (possibly curved) line with no fill, optionally with a dot at each
//! defined data point. Ported unchanged (stage-2 `s2-refactor` lane) from
//! `components::chart`'s own `SeriesMarks` -- see `components::layout` for
//! the scale/margin math this reads via `SeriesRenderContext`.

use dioxus::prelude::*;

use super::super::layout::SeriesRenderContext;
use crate::chart::engine::curve::line_path;
use crate::chart::engine::geometry::plot_runs;
use crate::chart::engine::scale::fmt_num;

/// [`crate::chart::ChartKind::Line`]'s own options.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct LineOptions {
    /// Show a dot at each defined data point. Renamed from `ChartProps`'
    /// pre-stage-2 `show_dots` prop (`components::series`'s own module doc)
    /// -- this is the only prop that was ever Line-specific.
    pub dots: bool,
}

/// Render every configured series' line (+ dots when [`LineOptions::dots`]
/// is set), in config order. `stacked` is meaningless for Line (see
/// `ChartProps::stacked`'s own doc) and is not read here at all.
pub(crate) fn render(ctx: &SeriesRenderContext, opts: &LineOptions) -> Element {
    rsx! {
        for (s , series) in ctx.config.series.iter().enumerate() {
            g {
                key: "{series.key}",
                "data-slot": "chart-series",
                "data-series": "{series.slot()}",
                style: "--series-color: var(--color-{series.slot()})",
                {render_one(ctx, s, opts.dots)}
            }
        }
    }
}

fn render_one(ctx: &SeriesRenderContext, s: usize, show_dots: bool) -> Element {
    let values = ctx.series_values(s);
    let n = ctx.xs.len();
    rsx! {
        for run in plot_runs(&ctx.xs, &values) {
            {
                let scaled: Vec<(f64, f64)> =
                    run.iter().map(|(x, v)| (*x, ctx.y_scale.scale(*v))).collect();
                let d = line_path(&scaled, ctx.curve);
                rsx! {
                    path { "data-slot": "chart-line", d: "{d}", fill: "none" }
                }
            }
        }
        if show_dots {
            for i in 0..n {
                if let Some(v) = values[i] {
                    circle {
                        key: "{i}",
                        "data-slot": "chart-dot",
                        "data-index": "{i}",
                        cx: "{fmt_num(ctx.xs[i])}",
                        cy: "{fmt_num(ctx.y_scale.scale(v))}",
                        r: "3",
                    }
                }
            }
        }
    }
}
