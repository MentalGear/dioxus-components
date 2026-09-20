//! The [`ChartKind::Line`](crate::chart::ChartKind::Line) family: a
//! (possibly curved) line with no fill, optionally with a dot and/or a
//! text label at each defined data point. Ported (stage-2 `s2-refactor`
//! lane) from `components::chart`'s own `SeriesMarks`, then extended
//! (stage-2 `s2-line` lane) with everything shadcn's own ten
//! `chart-line-*` demos need beyond a plain line: a configurable dot
//! radius and stroke width, a caller-supplied custom dot renderer, a
//! per-point label, and active-dot enlargement on hover/keyboard-focus --
//! see [`LineOptions`]'s own field docs for each one's shadcn citation.

use dioxus::prelude::*;

use super::super::layout::SeriesRenderContext;
use crate::chart::engine::curve::line_path;
use crate::chart::engine::geometry::plot_runs;
use crate::chart::engine::scale::{fmt_decimal, fmt_num};

/// Per-point context passed to a caller-supplied dot renderer
/// ([`LineOptions::dot`]) -- everything it needs to draw its own mark in
/// the default circle's place, already in the SVG's own coordinate space
/// (no further scale lookups required).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct DotContext {
    /// This point's index into the series' own value list (and into
    /// [`crate::chart::ChartConfig`]'s underlying `data` list) -- stable
    /// per datum, suitable as an RSX `key`.
    pub index: usize,
    /// This point's already-scaled SVG x coordinate.
    pub cx: f64,
    /// This point's already-scaled SVG y coordinate.
    pub cy: f64,
    /// This point's raw (unscaled) value.
    pub value: f64,
    /// Whether this is the chart's currently hovered/keyboard-focused
    /// point (`Chart`'s own `active_index`, read-only here). shadcn's own
    /// per-demo `activeDot={{ r: 6 }}` is a Recharts-level concept with no
    /// per-point callback of its own; a custom [`LineOptions::dot`]
    /// renderer decides for itself whether/how to react to this (e.g.
    /// shadcn's own `chart-line-dots-custom.tsx` ignores it entirely --
    /// its custom dot never sets `activeDot` either).
    pub active: bool,
}

/// A [`LineOptions::dot`]-shaped custom dot renderer: `Clone`/`Copy`/
/// `PartialEq`/`Debug` exactly the way [`Callback`] itself already is --
/// see [`crate::chart::ChartIcon`]'s own doc comment for the full "compare
/// by identity, not by the closure's behavior" rationale this mirrors
/// (the same reason [`LineOptions`] as a whole can still derive
/// `PartialEq` with this as a field).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct DotRenderer(pub Callback<DotContext, Element>);

/// How (or whether) to label each of a line's own defined data points.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum LineLabels {
    /// No label (the default).
    #[default]
    None,
    /// The point's own value, formatted like every other numeric readout
    /// in this crate ([`fmt_decimal`]: up to 2 decimals, trailing zeros
    /// trimmed). shadcn's `chart-line-label.tsx`.
    Value,
    /// A caller-supplied label, given this point's index -- e.g. shadcn's
    /// `chart-line-label-custom.tsx`, which labels each point with a
    /// category name instead of its value.
    Custom(Callback<usize, String>),
}

/// [`crate::chart::ChartKind::Line`]'s own options.
#[derive(Clone, PartialEq, Debug)]
pub struct LineOptions {
    /// Show a dot at each defined data point. Renamed from `ChartProps`'
    /// pre-stage-2 `show_dots` prop (`components::series`'s own module doc)
    /// -- this was already the only prop that was ever Line-specific.
    /// Ignored (a dot is drawn regardless of this flag) when [`Self::dot`]
    /// is `Some`: a caller supplying a custom renderer wants it drawn at
    /// every point, not gated behind a second flag it would always also
    /// have to set. shadcn's `chart-line-dots.tsx`.
    pub dots: bool,
    /// The default dot's radius, in SVG user units. The currently active
    /// (hovered/keyboard-focused) point's dot renders at twice this radius
    /// -- shadcn's own `activeDot={{ r: 6 }}` paired with an implicit
    /// default dot radius of `3` (Recharts' own default), the exact 3-to-6
    /// jump this crate's pre-stage-2 hardcoded `r="3"` already implied.
    /// Has no effect when [`Self::dot`] is `Some`: a custom renderer is
    /// given [`DotContext::active`] instead and decides its own sizing.
    pub dot_radius: f64,
    /// A custom dot renderer, in place of the default filled circle.
    /// `None` (the default) draws a plain circle, colored from
    /// [`crate::chart::ChartDatum::color`] when that datum sets one
    /// (shadcn's `chart-line-dots-colors.tsx`) or the series' own
    /// `--series-color` otherwise. `Some` calls back with a [`DotContext`]
    /// and renders whatever `Element` it returns in the dot's place
    /// (shadcn's `chart-line-dots-custom.tsx`, a per-point icon).
    pub dot: Option<DotRenderer>,
    /// Draw a text label ([`data-slot="chart-label"]`) above each defined
    /// data point.
    pub labels: LineLabels,
    /// The line's stroke width, in SVG user units. Every shadcn line demo
    /// sets this to `2` explicitly, matching this crate's pre-stage-2
    /// hardcoded value -- a real, caller-settable prop nonetheless (not
    /// merely for parity), so nothing about a themed line's weight is
    /// stuck in CSS-only territory the way a fully unstyled primitive
    /// shouldn't be.
    pub stroke_width: f64,
}

impl Default for LineOptions {
    fn default() -> Self {
        Self {
            dots: false,
            dot_radius: 3.0,
            dot: None,
            labels: LineLabels::None,
            stroke_width: 2.0,
        }
    }
}

/// Render every configured series' line (+ dots/labels per [`LineOptions`]),
/// in config order. `stacked` is meaningless for Line (see
/// `ChartProps::stacked`'s own doc) and is not read here at all.
pub(crate) fn render(ctx: &SeriesRenderContext, opts: &LineOptions) -> Element {
    rsx! {
        for (s , series) in ctx.config.series.iter().enumerate() {
            g {
                key: "{series.key}",
                "data-slot": "chart-series",
                "data-series": "{series.slot()}",
                style: "--series-color: var(--color-{series.slot()})",
                {render_one(ctx, s, opts)}
            }
        }
    }
}

fn render_one(ctx: &SeriesRenderContext, s: usize, opts: &LineOptions) -> Element {
    let values = ctx.series_values(s);
    let n = ctx.xs.len();
    let show_marks = opts.dots || opts.dot.is_some();
    let show_labels = !matches!(opts.labels, LineLabels::None);
    rsx! {
        for run in plot_runs(&ctx.xs, &values) {
            {
                let scaled: Vec<(f64, f64)> =
                    run.iter().map(|(x, v)| (*x, ctx.y_scale.scale(*v))).collect();
                let d = line_path(&scaled, ctx.curve);
                rsx! {
                    path {
                        "data-slot": "chart-line",
                        d: "{d}",
                        fill: "none",
                        "stroke-width": "{fmt_num(opts.stroke_width)}",
                    }
                }
            }
        }
        if show_marks || show_labels {
            for i in 0..n {
                if let Some(v) = values[i] {
                    {
                        let cx = ctx.xs[i];
                        let cy = ctx.y_scale.scale(v);
                        let active = ctx.active_index == Some(i);
                        rsx! {
                            if show_marks {
                                {render_dot(ctx, opts, i, cx, cy, v, active)}
                            }
                            if let Some(label) = label_text(opts, i, v) {
                                text {
                                    key: "{i}",
                                    "data-slot": "chart-label",
                                    x: "{fmt_num(cx)}",
                                    y: "{fmt_num(cy - 12.0)}",
                                    "text-anchor": "middle",
                                    {label}
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Draw point `i`'s dot: [`LineOptions::dot`] when set, else the default
/// circle (active-doubled, per-datum-colored when
/// [`crate::chart::ChartDatum::color`] is set -- see [`LineOptions`]'s own
/// field docs for both).
fn render_dot(
    ctx: &SeriesRenderContext,
    opts: &LineOptions,
    i: usize,
    cx: f64,
    cy: f64,
    value: f64,
    active: bool,
) -> Element {
    if let Some(DotRenderer(callback)) = opts.dot {
        return callback.call(DotContext {
            index: i,
            cx,
            cy,
            value,
            active,
        });
    }
    let radius = if active {
        opts.dot_radius * 2.0
    } else {
        opts.dot_radius
    };
    // An inline `style` (not a plain `fill`/`stroke` presentation
    // attribute) is required for this per-datum override to actually take
    // effect: an SVG presentation attribute loses to ANY matching author
    // stylesheet rule regardless of specificity, and `chart/style.css`'s
    // own `[data-slot="chart-dot"] { fill: var(--series-color); ... }`
    // rule already matches every dot unconditionally. The same reasoning
    // `ChartContainer`'s own `--color-<key>` custom property already
    // relies on (`$S/chart-api.md`), applied per-element instead of
    // per-series.
    let dot_style = ctx
        .data
        .get(i)
        .and_then(|d| d.color.as_deref())
        .map(|c| format!("fill: {c}; stroke: {c};"));
    rsx! {
        circle {
            key: "{i}",
            "data-slot": "chart-dot",
            "data-index": "{i}",
            "data-active": active,
            cx: "{fmt_num(cx)}",
            cy: "{fmt_num(cy)}",
            r: "{fmt_num(radius)}",
            style: dot_style,
        }
    }
}

/// This point's label text, or `None` when [`LineOptions::labels`] is
/// [`LineLabels::None`] (the caller already skips calling this in that
/// case -- see [`render_one`] -- this stays a total function regardless,
/// simpler to reason about and to unit-test on its own).
fn label_text(opts: &LineOptions, i: usize, value: f64) -> Option<String> {
    match &opts.labels {
        LineLabels::None => None,
        LineLabels::Value => Some(fmt_decimal(value, 2)),
        LineLabels::Custom(callback) => Some(callback.call(i)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart::{Chart, ChartConfig, ChartContainer, ChartDatum, ChartKind};
    use dioxus_core::NoOpMutations;

    fn sample_config() -> ChartConfig {
        ChartConfig::new().series("desktop", "Desktop", "var(--dx-chart-1)")
    }

    fn sample_data() -> Vec<ChartDatum> {
        vec![
            ChartDatum {
                label: "January".to_string(),
                values: vec![Some(186.0)],
                ..Default::default()
            },
            ChartDatum {
                label: "February".to_string(),
                values: vec![Some(305.0)],
                ..Default::default()
            },
        ]
    }

    #[derive(Clone, PartialEq, Props)]
    struct HarnessProps {
        line: LineOptions,
        #[props(default)]
        data: Option<Vec<ChartDatum>>,
    }

    #[component]
    fn Harness(props: HarnessProps) -> Element {
        let config = use_signal(sample_config);
        let data_vec = props.data.clone().unwrap_or_else(sample_data);
        let data = use_signal(move || data_vec.clone());
        rsx! {
            ChartContainer { config, data, kind: ChartKind::Line,
                Chart {
                    aria_label: "Visitors by month, desktop",
                    line: props.line.clone(),
                    // Keyboard off: `Chart`'s `active_index` is its own
                    // internal signal with no prop to preset it, only set
                    // for real by a client-side hover/keyboard event --
                    // neither ever fires during an SSR-only render, so
                    // every dot below is genuinely at rest
                    // (`data-active="false"`) and this harness cannot
                    // exercise the *active* dot's doubled radius. That
                    // path is covered live instead
                    // (`playwright/line_chart.spec.ts`'s hover assertion),
                    // the same split `chart.rs`'s own pre-existing
                    // `cursor_group_is_present_but_empty_before_any_hover`
                    // test already draws for the same reason.
                    keyboard: false,
                }
            }
        }
    }

    fn render(line: LineOptions) -> String {
        render_with(line, None)
    }

    fn render_with(line: LineOptions, data: Option<Vec<ChartDatum>>) -> String {
        let mut dom = VirtualDom::new_with_props(Harness, HarnessProps { line, data });
        dom.rebuild_in_place();
        dom.render_immediate(&mut NoOpMutations);
        dioxus_ssr::render(&dom)
    }

    /// A second harness, for the two tests that need a real `Callback`
    /// (`DotRenderer`/`LineLabels::Custom`) as part of `LineOptions`:
    /// `Callback::new` panics ("must be called from inside a Dioxus
    /// runtime") when called from a plain `#[test]` fn, before any
    /// `VirtualDom` exists to provide one -- so unlike every other test
    /// here (which builds its own `LineOptions` value directly and hands
    /// it to [`render`]/[`render_with`]), these two build theirs INSIDE a
    /// component body instead, which runs with a runtime already active.
    /// `dots_also_true` exercises the "a custom renderer suppresses the
    /// plain default circle even when `dots: true` too" edge, distinct
    /// from the plain "a custom renderer draws at all" case.
    #[derive(Clone, Copy, PartialEq, Props)]
    struct CustomDotHarnessProps {
        #[props(default)]
        dots_also_true: bool,
    }

    #[component]
    fn CustomDotHarness(props: CustomDotHarnessProps) -> Element {
        let config = use_signal(sample_config);
        let data = use_signal(sample_data);
        let dot = DotRenderer(Callback::new(|ctx: DotContext| {
            rsx! {
                rect {
                    key: "{ctx.index}",
                    "data-slot": "chart-custom-dot",
                    "data-index": "{ctx.index}",
                    x: "{ctx.cx}",
                    y: "{ctx.cy}",
                }
            }
        }));
        rsx! {
            ChartContainer { config, data, kind: ChartKind::Line,
                Chart {
                    aria_label: "Visitors by month, desktop",
                    line: LineOptions {
                        dots: props.dots_also_true,
                        dot: Some(dot),
                        ..Default::default()
                    },
                    keyboard: false,
                }
            }
        }
    }

    fn render_custom_dot(dots_also_true: bool) -> String {
        let mut dom =
            VirtualDom::new_with_props(CustomDotHarness, CustomDotHarnessProps { dots_also_true });
        dom.rebuild_in_place();
        dom.render_immediate(&mut NoOpMutations);
        dioxus_ssr::render(&dom)
    }

    #[component]
    fn CustomLabelHarness() -> Element {
        let config = use_signal(sample_config);
        let data = use_signal(sample_data);
        rsx! {
            ChartContainer { config, data, kind: ChartKind::Line,
                Chart {
                    aria_label: "Visitors by month, desktop",
                    line: LineOptions {
                        labels: LineLabels::Custom(Callback::new(|i: usize| format!("point-{i}"))),
                        ..Default::default()
                    },
                    keyboard: false,
                }
            }
        }
    }

    fn render_custom_label() -> String {
        let mut dom = VirtualDom::new(CustomLabelHarness);
        dom.rebuild_in_place();
        dom.render_immediate(&mut NoOpMutations);
        dioxus_ssr::render(&dom)
    }

    // -- dots -----------------------------------------------------------

    #[test]
    fn no_dots_by_default() {
        let html = render(LineOptions::default());
        assert!(!html.contains(r#"data-slot="chart-dot""#));
    }

    #[test]
    fn dots_true_renders_one_dot_per_defined_point() {
        let html = render(LineOptions {
            dots: true,
            ..Default::default()
        });
        assert_eq!(html.matches(r#"data-slot="chart-dot""#).count(), 2);
        assert!(html.contains(r#"data-slot="chart-dot" data-index="0""#));
        assert!(html.contains(r#"data-slot="chart-dot" data-index="1""#));
    }

    #[test]
    fn dots_use_the_configured_radius_and_are_not_active_by_default() {
        let html = render(LineOptions {
            dots: true,
            dot_radius: 5.0,
            ..Default::default()
        });
        assert!(html.contains(r#"r="5""#));
        // A `bool`-valued attribute (unlike every `&str`/`String` one)
        // renders unquoted (`data-active=false`, not `data-active="false"`)
        // -- `chart.rs`'s own pre-existing `tabindex=0` test already pins
        // this exact dioxus-ssr behavior for a numeric attribute; the same
        // renderer code path (`AttributeValue::Bool`/`::Int` both go
        // through `write_attribute`'s plain `{name}={value}` branch, not
        // the quoted `::Text` one) applies here too -- confirmed by
        // actually hitting the quoted form's assertion failure first.
        assert!(html.contains("data-active=false"));
        assert!(!html.contains("data-active=true"));
    }

    #[test]
    fn a_gap_still_draws_no_dot_for_that_index() {
        let data = vec![
            ChartDatum {
                label: "January".to_string(),
                values: vec![Some(186.0)],
                ..Default::default()
            },
            ChartDatum {
                label: "February".to_string(),
                values: vec![None],
                ..Default::default()
            },
        ];
        let html = render_with(
            LineOptions {
                dots: true,
                ..Default::default()
            },
            Some(data),
        );
        assert_eq!(html.matches(r#"data-slot="chart-dot""#).count(), 1);
        // Scoped to the DOT's own two adjacent attributes (matching
        // `render_dot`'s exact declaration order), not a bare
        // `data-index="1"` search -- `chart.rs`'s hit-bands render that
        // same attribute, once per datum, regardless of gaps, so an
        // unscoped search would find index 1's *hit-band* and false-fail.
        assert!(html.contains(r#"data-slot="chart-dot" data-index="0""#));
        assert!(!html.contains(r#"data-slot="chart-dot" data-index="1""#));
    }

    #[test]
    fn per_datum_color_overrides_the_default_dot_via_inline_style() {
        let data = vec![
            ChartDatum {
                label: "chrome".to_string(),
                values: vec![Some(275.0)],
                color: Some("var(--dx-chart-1)".to_string()),
            },
            ChartDatum {
                label: "safari".to_string(),
                values: vec![Some(200.0)],
                color: Some("var(--dx-chart-2)".to_string()),
            },
        ];
        let html = render_with(
            LineOptions {
                dots: true,
                ..Default::default()
            },
            Some(data),
        );
        assert!(html.contains("fill: var(--dx-chart-1); stroke: var(--dx-chart-1);"));
        assert!(html.contains("fill: var(--dx-chart-2); stroke: var(--dx-chart-2);"));
    }

    #[test]
    fn no_color_override_omits_the_inline_style_entirely() {
        let html = render(LineOptions {
            dots: true,
            ..Default::default()
        });
        // Not merely "no fill:" -- the whole `style="..."` attribute must
        // be absent, so the dot falls through to `chart/style.css`'s own
        // `fill: var(--series-color)` rule undisturbed.
        assert!(!html.contains("style=\"fill:"));
    }

    // -- custom dot renderer ---------------------------------------------

    #[test]
    fn a_custom_dot_renderer_replaces_the_default_circle() {
        let dot = DotRenderer(Callback::new(|ctx: DotContext| {
            rsx! {
                rect {
                    key: "{ctx.index}",
                    "data-slot": "chart-custom-dot",
                    "data-index": "{ctx.index}",
                    x: "{ctx.cx}",
                    y: "{ctx.cy}",
                }
            }
        }));
        let html = render(LineOptions {
            dot: Some(dot),
            ..Default::default()
        });
        assert_eq!(html.matches(r#"data-slot="chart-custom-dot""#).count(), 2);
        assert!(!html.contains(r#"data-slot="chart-dot""#));
    }

    #[test]
    fn dots_flag_is_ignored_once_a_custom_dot_renderer_is_set() {
        // `dots: false` alongside `dot: Some(..)` still draws every point
        // -- `Self::dot`'s own doc: a custom renderer is never gated
        // behind a second flag.
        let dot = DotRenderer(Callback::new(|ctx: DotContext| {
            rsx! {
                rect { key: "{ctx.index}", "data-slot": "chart-custom-dot" }
            }
        }));
        let html = render(LineOptions {
            dots: false,
            dot: Some(dot),
            ..Default::default()
        });
        assert_eq!(html.matches(r#"data-slot="chart-custom-dot""#).count(), 2);
    }

    // -- labels -----------------------------------------------------------

    #[test]
    fn no_labels_by_default() {
        let html = render(LineOptions::default());
        assert!(!html.contains(r#"data-slot="chart-label""#));
    }

    #[test]
    fn value_labels_render_the_formatted_value_per_point() {
        let html = render(LineOptions {
            labels: LineLabels::Value,
            ..Default::default()
        });
        assert_eq!(html.matches(r#"data-slot="chart-label""#).count(), 2);
        assert!(html.contains(">186<"));
        assert!(html.contains(">305<"));
    }

    #[test]
    fn custom_labels_call_back_with_the_point_index() {
        let html = render(LineOptions {
            labels: LineLabels::Custom(Callback::new(|i: usize| format!("point-{i}"))),
            ..Default::default()
        });
        assert!(html.contains(">point-0<"));
        assert!(html.contains(">point-1<"));
    }

    #[test]
    fn labels_and_dots_can_both_be_on_at_once() {
        let html = render(LineOptions {
            dots: true,
            labels: LineLabels::Value,
            ..Default::default()
        });
        assert_eq!(html.matches(r#"data-slot="chart-dot""#).count(), 2);
        assert_eq!(html.matches(r#"data-slot="chart-label""#).count(), 2);
    }

    // -- stroke width -----------------------------------------------------

    #[test]
    fn stroke_width_defaults_to_2() {
        let html = render(LineOptions::default());
        assert!(html.contains(r#"stroke-width="2""#));
    }

    #[test]
    fn stroke_width_is_settable() {
        let html = render(LineOptions {
            stroke_width: 4.0,
            ..Default::default()
        });
        assert!(html.contains(r#"stroke-width="4""#));
    }

    // -- pure helpers, no VirtualDom needed ------------------------------

    #[test]
    fn label_text_matches_line_labels_variant() {
        assert_eq!(label_text(&LineOptions::default(), 0, 186.0), None);
        assert_eq!(
            label_text(
                &LineOptions {
                    labels: LineLabels::Value,
                    ..Default::default()
                },
                0,
                186.5,
            ),
            Some("186.5".to_string())
        );
        assert_eq!(
            label_text(
                &LineOptions {
                    labels: LineLabels::Custom(Callback::new(|i: usize| format!("#{i}"))),
                    ..Default::default()
                },
                3,
                0.0,
            ),
            Some("#3".to_string())
        );
    }

    #[test]
    fn line_options_default_matches_pre_stage_2_hardcoded_values() {
        // Pins the exact pre-stage-2 constants (`r="3"`, `stroke-width: 2`
        // from `chart/style.css`) as this struct's own defaults, so a
        // caller who never touches `LineOptions` sees byte-identical
        // output to before this round.
        let opts = LineOptions::default();
        assert!(!opts.dots);
        assert_eq!(opts.dot_radius, 3.0);
        assert_eq!(opts.dot, None);
        assert_eq!(opts.labels, LineLabels::None);
        assert_eq!(opts.stroke_width, 2.0);
    }
}
