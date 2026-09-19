//! The [`ChartKind::Bar`](crate::chart::ChartKind::Bar) family: discrete
//! bars, grouped side-by-side per series unless stacked. Ported unchanged
//! (stage-2 `s2-refactor` lane) from `components::chart`'s own
//! `SeriesMarks` -- see `components::layout` for the scale/margin math this
//! reads via `SeriesRenderContext`.

use dioxus::prelude::*;

use super::super::layout::SeriesRenderContext;
use crate::chart::engine::scale::fmt_num;
use crate::chart::BandScale;

/// Padding between grouped (non-stacked, multi-series) bars sharing one
/// category band. Bar-only (grouped-bar sub-positioning is this family's
/// own concern), unlike `components::layout`'s `BAND_PADDING`, which every
/// family's outer per-category band scale shares.
const GROUP_PADDING: f64 = 0.15;

/// [`crate::chart::ChartKind::Bar`]'s own options. Empty for now -- every
/// prop this family currently reads (`stacked`) is shared with Area and
/// stays on `ChartProps` itself; see `components::series`'s own module doc.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct BarOptions {}

/// Render every configured series' bars, in config order.
pub(crate) fn render(ctx: &SeriesRenderContext, _opts: &BarOptions) -> Element {
    let series_count = ctx.config.series.len();
    rsx! {
        for (s , series) in ctx.config.series.iter().enumerate() {
            g {
                key: "{series.key}",
                "data-slot": "chart-series",
                "data-series": "{series.slot()}",
                style: "--series-color: var(--color-{series.slot()})",
                {render_one(ctx, s, series_count)}
            }
        }
    }
}

fn render_one(ctx: &SeriesRenderContext, s: usize, series_count: usize) -> Element {
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
                        let (rect_y, rect_h) = if y1 <= ctx.zero_y {
                            (y1, ctx.zero_y - y1)
                        } else {
                            (ctx.zero_y, y1 - ctx.zero_y)
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
        }
    }
}
