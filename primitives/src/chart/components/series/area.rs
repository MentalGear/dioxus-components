//! The [`ChartKind::Area`](crate::chart::ChartKind::Area) family: a filled
//! area under a (possibly curved) line, optionally stacked. Ported
//! unchanged (stage-2 `s2-refactor` lane) from `components::chart`'s own
//! `SeriesMarks` -- see `components::layout` for the scale/margin math this
//! reads via `SeriesRenderContext`, and `components::series`'s own module
//! doc for the `stacked`/`curve`-stay-on-`ChartProps` decision.

use dioxus::prelude::*;

use super::super::layout::SeriesRenderContext;
use crate::chart::engine::curve::{area_between_path, area_path, line_path};
use crate::chart::engine::geometry::plot_runs;

/// [`crate::chart::ChartKind::Area`]'s own options. Empty for now -- every
/// prop this family currently reads (`stacked`, `curve`) is shared with at
/// least one other family and stays on `ChartProps` itself; see
/// `components::series`'s own module doc.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct AreaOptions {}

/// Render every configured series' area + top-line mark, in config order.
pub(crate) fn render(ctx: &SeriesRenderContext, _opts: &AreaOptions) -> Element {
    rsx! {
        for (s , series) in ctx.config.series.iter().enumerate() {
            g {
                key: "{series.key}",
                "data-slot": "chart-series",
                "data-series": "{series.slot()}",
                style: "--series-color: var(--color-{series.slot()})",
                {render_one(ctx, s)}
            }
        }
    }
}

fn render_one(ctx: &SeriesRenderContext, s: usize) -> Element {
    if ctx.stacked {
        let n = ctx.xs.len();
        let top: Vec<(f64, f64)> = (0..n)
            .map(|i| (ctx.xs[i], ctx.y_scale.scale(ctx.stacked_spans[i][s].1)))
            .collect();
        let bottom: Vec<(f64, f64)> = (0..n)
            .map(|i| (ctx.xs[i], ctx.y_scale.scale(ctx.stacked_spans[i][s].0)))
            .collect();
        let area_d = area_between_path(&top, &bottom, ctx.curve);
        let line_d = line_path(&top, ctx.curve);
        rsx! {
            path { "data-slot": "chart-area", d: "{area_d}" }
            path { "data-slot": "chart-line", d: "{line_d}", fill: "none" }
        }
    } else {
        let values = ctx.series_values(s);
        rsx! {
            for run in plot_runs(&ctx.xs, &values) {
                {
                    let scaled: Vec<(f64, f64)> =
                        run.iter().map(|(x, v)| (*x, ctx.y_scale.scale(*v))).collect();
                    let area_d = area_path(&scaled, ctx.zero_y, ctx.curve);
                    let line_d = line_path(&scaled, ctx.curve);
                    rsx! {
                        path { "data-slot": "chart-area", d: "{area_d}" }
                        path { "data-slot": "chart-line", d: "{line_d}", fill: "none" }
                    }
                }
            }
        }
    }
}
