//! The hovered point's dot on every series of an Area or Line chart --
//! shadcn/Recharts' `activeDot` (a filled circle in the series color with a
//! background-colored ring, `r = 4`), drawn above the marks and the cursor.
//!
//! It is its own layer (`g[data-slot="chart-active-dots"]` after the marks)
//! rather than part of each series' own group, so one series' line can never
//! paint over another series' dot where they cross. Positions come from the
//! same scales and stack spans the marks use ([`point`]), so a stacked area's
//! dot sits on its own band's top edge, not on the raw value.
//!
//! A line chart that draws its own dots ([`super::line::LineOptions::dots`]
//! or a custom renderer) does not use this layer: it enlarges the active dot
//! it already has. Server-rendered when the chart has a
//! [`crate::chart::ChartProps::default_index`], since that opens the active
//! index on the server too.

use dioxus::prelude::*;

use super::super::layout::SeriesRenderContext;
use crate::chart::engine::scale::fmt_num;

/// The dot's radius in px (Recharts' default `activeDot`, `r: 4`).
pub(crate) const ACTIVE_DOT_RADIUS: f64 = 4.0;

/// Series `s`'s point at datum `i`, in the chart's px, or `None`
/// when that series has no value there (a gap draws no dot): the value's own
/// position, or -- stacked -- the top edge of its band.
pub(crate) fn point(ctx: &SeriesRenderContext, s: usize, i: usize) -> Option<(f64, f64)> {
    let x = *ctx.xs.get(i)?;
    let value = ctx.data.get(i)?.values.get(s).copied().flatten()?;
    let y = if ctx.stacked {
        ctx.y_scale.scale(ctx.stacked_spans.get(i)?.get(s)?.1)
    } else {
        ctx.y_scale.scale(value)
    };
    (x.is_finite() && y.is_finite()).then_some((x, y))
}

/// The active index's dot for every series (config order), or nothing when no
/// datum is active.
pub(crate) fn render(ctx: &SeriesRenderContext) -> Element {
    let active = ctx.active_index.filter(|i| *i < ctx.xs.len());
    let radius = fmt_num(ACTIVE_DOT_RADIUS);
    rsx! {
        g { "data-slot": "chart-active-dots",
            if let Some(i) = active {
                for (s , series) in ctx.config.series.iter().enumerate() {
                    if let Some((cx , cy)) = point(ctx, s, i) {
                        circle {
                            key: "{series.key}",
                            "data-slot": "chart-active-dot",
                            "data-series": "{series.slot()}",
                            "data-index": "{i}",
                            style: "--series-color: var(--color-{series.slot()})",
                            cx: "{fmt_num(cx)}",
                            cy: "{fmt_num(cy)}",
                            r: "{radius}",
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
    use crate::chart::components::layout::{build, LayoutParams};
    use crate::chart::{ChartConfig, ChartDatum, ChartKind, Curve, StackMode};
    use crate::direction::Direction;

    fn config() -> ChartConfig {
        ChartConfig::new()
            .series("a", "A", "var(--dx-chart-1)")
            .series("b", "B", "var(--dx-chart-2)")
    }

    fn data() -> Vec<ChartDatum> {
        [(10.0, Some(5.0)), (20.0, None), (30.0, Some(15.0))]
            .into_iter()
            .map(|(a, b)| ChartDatum {
                label: "x".to_string(),
                values: vec![Some(a), b],
                ..Default::default()
            })
            .collect()
    }

    fn ctx(stacked: bool) -> SeriesRenderContext {
        build(LayoutParams {
            width: 600.0,
            height: 300.0,
            margin: crate::chart::ChartMargin::default(),
            show_x_axis: true,
            show_y_axis: false,
            y_tick_count: 5,
            kind: ChartKind::Area,
            horizontal: false,
            stacked,
            stack_mode: StackMode::Normal,
            curve: Curve::Linear,
            dir: Direction::Ltr,
            active_index: Some(0),
            config: &config(),
            data: &data(),
        })
    }

    #[test]
    fn the_dot_sits_on_the_marks_own_point() {
        let c = ctx(false);
        let (x, y) = point(&c, 0, 1).expect("series a has a value at 1");
        assert_eq!(x, c.xs[1]);
        assert_eq!(y, c.y_scale.scale(20.0));
    }

    #[test]
    fn a_gap_draws_no_dot() {
        for stacked in [false, true] {
            assert_eq!(point(&ctx(stacked), 1, 1), None, "stacked={stacked}");
        }
    }

    #[test]
    fn a_stacked_dot_sits_on_its_bands_top_edge() {
        let c = ctx(true);
        // Series b at datum 0 is stacked on a's 10: its top is 15, not 5.
        let (_, y) = point(&c, 1, 0).unwrap();
        assert_eq!(y, c.y_scale.scale(15.0));
        assert_ne!(y, c.y_scale.scale(5.0));
    }

    #[test]
    fn out_of_range_indices_and_series_are_none() {
        let c = ctx(false);
        assert_eq!(point(&c, 0, 9), None);
        assert_eq!(point(&c, 7, 0), None);
    }
}
