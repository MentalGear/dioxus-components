//! The [`ChartKind::RadialBar`](crate::chart::ChartKind::RadialBar) family:
//! concentric proportional arcs, one ring per datum (the default), or every
//! configured series stacked cumulatively into ONE ring
//! ([`RadialOptions::stacked`]) -- shadcn/ui's own two spellings of the
//! same idea (`chart-radial-simple.tsx`'s one-`<RadialBar>`-per-row default
//! versus `chart-radial-stacked.tsx`'s explicit `stackId`).
//!
//! Shares [`crate::chart::engine::polar`] with [`super::pie`] (same
//! `arc_path`/`centroid` mark geometry, same [`crate::chart::PieLabels`]
//! per-arc label enum) -- the two families differ only in what an arc's
//! angular span *means* (Pie: a category's share of one ring's whole;
//! RadialBar: a category's own value against a shared domain, laid out via
//! [`angle_scale`]) and in how many rings a chart draws when unstacked
//! (Pie: always one, unless multiple series are configured; RadialBar:
//! always one ring PER DATUM, since each row is its own bar-wrapped-into-
//! a-circle by definition).

use dioxus::prelude::*;

use super::super::layout::SeriesRenderContext;
use crate::chart::context::use_chart;
use crate::chart::engine::polar::{angle_scale, arc_path, centroid};
use crate::chart::engine::scale::{fmt_decimal, fmt_num};
use crate::chart::{BandScale, ChartDatum, PieLabels};

/// Radial padding (as a fraction of one ring's own step) between two
/// concentric, unstacked rings -- same role as [`super::pie`]'s own
/// `RING_PADDING`.
const RING_PADDING: f64 = 0.15;
/// Same fallback fraction [`super::pie`] uses when a caller leaves
/// [`RadialOptions::outer_radius`] at its default (`0.0`, meaning "size me
/// from the chart's own dimensions").
const OUTER_RADIUS_FRACTION: f64 = 0.9;

/// [`crate::chart::ChartKind::RadialBar`]'s own options.
#[derive(Clone, PartialEq, Debug)]
pub struct RadialOptions {
    /// The innermost ring's own inner radius (logical SVG units). `0.0`
    /// (the default) starts the innermost ring right at the chart's own
    /// center -- a real RadialBar chart almost always wants some positive
    /// value here (a bare center point makes the innermost ring's own
    /// share of a small value visually indistinguishable from a much
    /// larger one), left to the caller to set explicitly per shadcn's own
    /// `chart-radial-*.tsx` fixtures, which all do.
    pub inner_radius: f64,
    /// The outermost ring's own outer radius (logical SVG units). `0.0`
    /// (the default) auto-sizes it from the chart's own `width`/`height`
    /// (`(min(width, height) / 2) * 0.9`, the same fraction
    /// [`super::pie`] uses for its own auto-computed outer radius) --
    /// unlike Pie, this crate exposes RadialBar's outer radius directly
    /// (matching `RadialBarChart`'s own explicit prop upstream), so a
    /// caller who wants an exact size can set one.
    pub outer_radius: f64,
    /// Where the angular domain's `0.0` end maps to, in radians. `0.0` is
    /// three o'clock (this crate's engine convention, `engine::polar`'s
    /// own doc); shadcn's own demos commonly start at twelve o'clock
    /// (`-PI / 2`) or a custom offset -- passed straight to
    /// [`angle_scale`]'s own `range`.
    pub start_angle: f64,
    /// Where the angular domain's max end maps to, in radians -- may
    /// exceed `start_angle + TAU` (e.g. shadcn's own 380° sweep on some
    /// demos: slightly more than a full turn, so the first and last bars'
    /// own ends don't visually touch at 0/360).
    pub end_angle: f64,
    /// Rounds each ring's own two ends by this radius (logical SVG units)
    /// -- forwarded to [`arc_path`] unchanged, same convention
    /// [`super::pie::PieOptions::corner_radius`] uses.
    pub corner_radius: f64,
    /// Draw a muted full-sweep background track
    /// (`data-slot="chart-polar-grid"`) behind every ring's own value arc
    /// -- shadcn's own `<RadialBar background />` prop (unstacked) or a
    /// separate `<PolarGrid />` element (stacked); this crate unifies both
    /// spellings as one boolean, since the visual result -- a muted track
    /// at each ring's own inner/outer radius, drawn before its value arc
    /// -- is identical either way.
    pub grid: bool,
    /// Per-ring text -- see [`PieLabels`] (shared with [`super::pie`]:
    /// same enum, same centroid placement, same "shorter than the ring
    /// count skips the rest" `List` behavior).
    pub labels: PieLabels,
    /// `false` (the default): one ring per datum, each an independent
    /// share of `ctx.data`'s own max value (`chart-radial-simple.tsx`'s
    /// own shape). `true`: every configured series' value for the FIRST
    /// datum stacks cumulatively into one ring instead
    /// (`chart-radial-stacked.tsx`) -- meaningful only when at least two
    /// series are configured; with one, stacking one value alone is the
    /// same ring `false` would draw for that one datum.
    pub stacked: bool,
    /// Text centered in the hole: `(primary, secondary)`, e.g. `("1,999",
    /// "Visitors")` -- matches shadcn's `chart-radial-text.tsx`. Meaningful
    /// only when `inner_radius > 0.0`; not itself part of shadcn's own
    /// `RadialBar`-specific prop surface (its own `<PolarRadiusAxis>`
    /// `Label` render-prop plays this role upstream), but the exact same
    /// mechanism [`super::pie::PieOptions::center_text`] already gives
    /// Pie, reused here rather than invented twice.
    pub center_text: Option<(String, String)>,
}

impl Default for RadialOptions {
    fn default() -> Self {
        Self {
            inner_radius: 0.0,
            outer_radius: 0.0,
            start_angle: 0.0,
            end_angle: std::f64::consts::TAU,
            corner_radius: 0.0,
            grid: false,
            labels: PieLabels::None,
            stacked: false,
            center_text: None,
        }
    }
}

/// Render one ring per datum (the default), or every configured series'
/// value for the first datum stacked into one ring
/// ([`RadialOptions::stacked`]), plus the optional center text.
pub(crate) fn render(ctx: &SeriesRenderContext, opts: &RadialOptions) -> Element {
    let chart_ctx = use_chart();
    let active_index_signal = chart_ctx.active_index;
    let hover_active = ctx.active_index;

    let cx = ctx.width / 2.0;
    let cy = ctx.height / 2.0;
    let auto_outer = (ctx.width.min(ctx.height) / 2.0) * OUTER_RADIUS_FRACTION;
    let outer_radius = if opts.outer_radius > 0.0 {
        opts.outer_radius
    } else {
        auto_outer
    };
    let inner_radius = opts
        .inner_radius
        .max(0.0)
        .min((outer_radius - 1.0).max(0.0));

    let translate = format!("translate({}, {})", fmt_num(cx), fmt_num(cy));

    rsx! {
        g { "data-slot": "chart-series",
            g { transform: "{translate}",
                if opts.stacked {
                    {render_stacked_ring(ctx, opts, inner_radius, outer_radius, hover_active, active_index_signal)}
                } else {
                    {render_rings(ctx, opts, inner_radius, outer_radius, hover_active, active_index_signal)}
                }
            }
            if let Some((primary, secondary)) = &opts.center_text {
                text {
                    "data-slot": "chart-pie-center-text",
                    x: "{fmt_num(cx)}",
                    y: "{fmt_num(cy)}",
                    text_anchor: "middle",
                    tspan { x: "{fmt_num(cx)}", dy: "-0.1em", "{primary}" }
                    tspan { x: "{fmt_num(cx)}", dy: "1.4em", "{secondary}" }
                }
            }
        }
    }
}

/// One ring per datum -- `RadialOptions::stacked == false`, this family's
/// own default. Each ring is its own concentric band
/// ([`BandScale`] over `inner_radius..outer_radius`, one band per datum,
/// innermost = index `0`); its own value maps to an angular span via
/// [`angle_scale`] over `(0.0, max_value)`, so the largest datum's own
/// ring always reaches `end_angle` exactly and every other ring's own
/// share is proportionally shorter.
fn render_rings(
    ctx: &SeriesRenderContext,
    opts: &RadialOptions,
    inner_radius: f64,
    outer_radius: f64,
    hover_active: Option<usize>,
    mut active_index_signal: Signal<Option<usize>>,
) -> Element {
    let n = ctx.data.len().max(1);
    let band = BandScale {
        count: n,
        range: (inner_radius, outer_radius),
        padding: RING_PADDING,
    };
    let values: Vec<f64> = ctx
        .data
        .iter()
        .map(|d| d.values.first().copied().flatten().unwrap_or(0.0))
        .collect();
    let max_value = values
        .iter()
        .copied()
        .fold(0.0_f64, f64::max)
        .max(f64::EPSILON);
    let scale = angle_scale((0.0, max_value), opts.start_angle, opts.end_angle);

    rsx! {
        for (i , datum) in ctx.data.iter().enumerate() {
            {
                let (ring_inner, ring_width) = band.band(i);
                let ring_outer = ring_inner + ring_width;
                let value = values[i];
                let start = opts.start_angle;
                let end = scale.scale(value);
                let color = datum
                    .color
                    .clone()
                    .unwrap_or_else(|| format!("var(--dx-chart-{})", (i % 8) + 1));
                let is_active = hover_active == Some(i);
                let label_text = label_text(&opts.labels, i, datum, value, max_value);
                let (label_x, label_y) = centroid(ring_inner, ring_outer, start, end);
                rsx! {
                    g { key: "{i}",
                        if opts.grid {
                            path {
                                "data-slot": "chart-polar-grid",
                                d: "{arc_path(ring_inner, ring_outer, opts.start_angle, opts.end_angle, 0.0, 0.0)}",
                            }
                        }
                        path {
                            "data-slot": "chart-arc",
                            "data-index": "{i}",
                            "data-active": is_active.then_some("true"),
                            "data-start-angle": "{fmt_num(start)}",
                            "data-end-angle": "{fmt_num(end)}",
                            d: "{arc_path(ring_inner, ring_outer, start, end, 0.0, opts.corner_radius)}",
                            fill: "{color}",
                            onpointerenter: move |_| active_index_signal.set(Some(i)),
                        }
                        if let Some(text_content) = label_text {
                            text {
                                "data-slot": "chart-arc-label",
                                x: "{fmt_num(label_x)}",
                                y: "{fmt_num(label_y)}",
                                text_anchor: "middle",
                                "{text_content}"
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Every configured series' own value for the FIRST datum, stacked
/// cumulatively into one ring -- `RadialOptions::stacked == true`
/// (`chart-radial-stacked.tsx`). Series `0`'s own share starts at
/// `start_angle`; series `1` continues exactly where series `0` ended, and
/// so on -- contiguous by construction (each span's own start is the
/// previous span's own end), never independently positioned.
fn render_stacked_ring(
    ctx: &SeriesRenderContext,
    opts: &RadialOptions,
    inner_radius: f64,
    outer_radius: f64,
    hover_active: Option<usize>,
    mut active_index_signal: Signal<Option<usize>>,
) -> Element {
    let datum = ctx.data.first();
    let n_series = ctx.config.series.len().max(1);
    let values: Vec<f64> = (0..n_series)
        .map(|s| {
            datum
                .and_then(|d| d.values.get(s).copied().flatten())
                .unwrap_or(0.0)
        })
        .collect();
    let total: f64 = values.iter().sum::<f64>().max(f64::EPSILON);
    let span = opts.end_angle - opts.start_angle;

    let mut cursor = opts.start_angle;
    let spans: Vec<(f64, f64)> = values
        .iter()
        .map(|v| {
            let start = cursor;
            let end = cursor + span * (v / total);
            cursor = end;
            (start, end)
        })
        .collect();

    let is_active = hover_active == Some(0);

    rsx! {
        if opts.grid {
            path {
                "data-slot": "chart-polar-grid",
                d: "{arc_path(inner_radius, outer_radius, opts.start_angle, opts.end_angle, 0.0, 0.0)}",
            }
        }
        for (s , series) in ctx.config.series.iter().enumerate() {
            {
                let (start, end) = spans.get(s).copied().unwrap_or((opts.start_angle, opts.start_angle));
                let slot = series.slot();
                let color = format!("var(--color-{slot})");
                let value = values.get(s).copied().unwrap_or(0.0);
                let label_text = label_text(&opts.labels, s, datum.unwrap_or(&EMPTY_DATUM), value, total);
                let (label_x, label_y) = centroid(inner_radius, outer_radius, start, end);
                rsx! {
                    path {
                        key: "{series.key}",
                        "data-slot": "chart-arc",
                        "data-index": "0",
                        "data-series": "{slot}",
                        "data-active": is_active.then_some("true"),
                        "data-start-angle": "{fmt_num(start)}",
                        "data-end-angle": "{fmt_num(end)}",
                        d: "{arc_path(inner_radius, outer_radius, start, end, 0.0, opts.corner_radius)}",
                        fill: "{color}",
                        onpointerenter: move |_| active_index_signal.set(Some(0)),
                    }
                    if let Some(text_content) = label_text {
                        text {
                            "data-slot": "chart-arc-label",
                            x: "{fmt_num(label_x)}",
                            y: "{fmt_num(label_y)}",
                            text_anchor: "middle",
                            "{text_content}"
                        }
                    }
                }
            }
        }
    }
}

/// A `ChartDatum::default()`-shaped placeholder, used only when a stacked
/// ring's own first datum is missing entirely (an empty `data` slice) --
/// `label_text`'s own `PieLabels::List` arm reads a series' index into it,
/// which an absent datum has no real value for anyway.
const EMPTY_DATUM: ChartDatum = ChartDatum {
    label: String::new(),
    values: Vec::new(),
    color: None,
};

/// Shared with [`super::pie`]'s own identically-shaped helper -- not
/// literally reused (each family's own `SliceParams`-equivalent bundles
/// different fields), but the same four `PieLabels` arms, the same
/// two-decimal `fmt_decimal` convention, and the same "index past the end
/// of `List`'s own `Vec` renders nothing" defensive default.
fn label_text(
    labels: &PieLabels,
    index: usize,
    datum: &ChartDatum,
    value: f64,
    total: f64,
) -> Option<String> {
    match labels {
        PieLabels::None => None,
        PieLabels::Value => Some(fmt_decimal(value, 2)),
        PieLabels::Percent => {
            let pct = if total > 0.0 {
                value / total * 100.0
            } else {
                0.0
            };
            Some(format!("{}%", fmt_decimal(pct, 1)))
        }
        PieLabels::List(items) => items.get(index).filter(|s| !s.is_empty()).cloned(),
    }
    .or_else(|| {
        // `datum` is otherwise unused when `labels` is `None`/`Value`/
        // `Percent` -- referencing it here (a no-op `None` fallback) keeps
        // every arm using the same function signature every call site
        // shares, rather than a datum-less signature for two arms and a
        // datum-needing one for `List` alone.
        let _ = datum;
        None
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart::components::chart::Chart;
    use crate::chart::{ChartConfig, ChartContainer, ChartKind};
    use dioxus_core::NoOpMutations;

    fn sample_config() -> ChartConfig {
        ChartConfig::new()
            .series("desktop", "Desktop", "var(--dx-chart-1)")
            .series("mobile", "Mobile", "var(--dx-chart-2)")
    }

    fn one_series_data() -> Vec<ChartDatum> {
        vec![
            ChartDatum {
                label: "Chrome".to_string(),
                values: vec![Some(275.0)],
                color: Some("var(--dx-chart-1)".to_string()),
            },
            ChartDatum {
                label: "Safari".to_string(),
                values: vec![Some(200.0)],
                color: Some("var(--dx-chart-2)".to_string()),
            },
            ChartDatum {
                label: "Firefox".to_string(),
                values: vec![Some(100.0)],
                color: Some("var(--dx-chart-3)".to_string()),
            },
        ]
    }

    fn one_series_config() -> ChartConfig {
        ChartConfig::new().series("visitors", "Visitors", "var(--dx-chart-1)")
    }

    fn render(
        kind: ChartKind,
        config: ChartConfig,
        data: Vec<ChartDatum>,
        opts: RadialOptions,
    ) -> String {
        #[derive(Clone, PartialEq, Props)]
        struct HarnessProps {
            kind: ChartKind,
            config: ChartConfig,
            data: Vec<ChartDatum>,
            opts: RadialOptions,
        }
        #[component]
        fn Harness(props: HarnessProps) -> Element {
            let config = use_signal(|| props.config.clone());
            let data = use_signal(|| props.data.clone());
            rsx! {
                ChartContainer { config, data, kind: props.kind,
                    Chart { width: 300.0, height: 300.0, aria_label: "Test", radial: props.opts.clone() }
                }
            }
        }
        let mut dom = VirtualDom::new_with_props(
            Harness,
            HarnessProps {
                kind,
                config,
                data,
                opts,
            },
        );
        dom.rebuild_in_place();
        dom.render_immediate(&mut NoOpMutations);
        dioxus_ssr::render(&dom)
    }

    #[test]
    fn default_options_are_zero_hole_full_turn_unstacked() {
        let opts = RadialOptions::default();
        assert_eq!(opts.inner_radius, 0.0);
        assert_eq!(opts.outer_radius, 0.0);
        assert_eq!(opts.start_angle, 0.0);
        assert_eq!(opts.end_angle, std::f64::consts::TAU);
        assert!(!opts.stacked);
        assert!(!opts.grid);
        assert_eq!(opts.labels, PieLabels::None);
        assert!(opts.center_text.is_none());
    }

    #[test]
    fn unstacked_renders_one_ring_per_datum_each_its_own_data_index() {
        let html = render(
            ChartKind::RadialBar,
            one_series_config(),
            one_series_data(),
            RadialOptions::default(),
        );
        assert_eq!(html.matches(r#"data-slot="chart-arc""#).count(), 3);
        for i in 0..3 {
            assert!(html.contains(&format!(r#"data-index="{i}""#)), "{html}");
        }
    }

    /// Pulls one `f64`-valued attribute (e.g. `data-end-angle`) out of a
    /// single `<path ...>` tag slice -- forward-only (the attribute always
    /// appears somewhere AFTER the tag's own opening `<path`, never
    /// before), avoiding the backward-`rfind`-into-a-fixed-size-window
    /// bug an earlier version of these two tests had (it searched BEFORE
    /// a later attribute's own match, which only works when the wanted
    /// attribute happens to sort earlier in the tag than the anchor -- it
    /// doesn't here, `data-series` comes before `data-end-angle`).
    fn attr_f64(tag: &str, name: &str) -> f64 {
        let needle = format!(r#"{name}=""#);
        let start = tag
            .find(&needle)
            .unwrap_or_else(|| panic!("no {name} in {tag}"))
            + needle.len();
        tag[start..]
            .chars()
            .take_while(|c| *c != '"')
            .collect::<String>()
            .parse()
            .unwrap()
    }

    #[test]
    fn unstacked_largest_value_reaches_end_angle_exactly() {
        let html = render(
            ChartKind::RadialBar,
            one_series_config(),
            one_series_data(),
            RadialOptions::default(),
        );
        // Chrome (275.0) is the max of {275, 200, 100} -> its own
        // `data-end-angle` is exactly `end_angle` (TAU by default),
        // `fmt_num`'s own rounding aside (hence the coarser tolerance than
        // an exact-arithmetic assertion would use).
        let tag_start = html.find(r#"data-index="0""#).unwrap();
        let tag_end = html[tag_start..]
            .find("</path>")
            .map(|i| tag_start + i)
            .unwrap_or(html.len());
        let end = attr_f64(&html[tag_start..tag_end], "data-end-angle");
        assert!(
            (end - std::f64::consts::TAU).abs() < 1e-2,
            "expected TAU, got {end}: {html}"
        );
    }

    #[test]
    fn stacked_produces_contiguous_spans_summing_to_the_full_range() {
        let data = vec![ChartDatum {
            label: "January".to_string(),
            values: vec![Some(186.0), Some(80.0)],
            ..Default::default()
        }];
        let html = render(
            ChartKind::RadialBar,
            sample_config(),
            data,
            RadialOptions {
                stacked: true,
                ..Default::default()
            },
        );
        assert_eq!(html.matches(r#"data-slot="chart-arc""#).count(), 2);
        assert_eq!(html.matches(r#"data-index="0""#).count(), 2);
        assert!(html.contains(r#"data-series="desktop""#));
        assert!(html.contains(r#"data-series="mobile""#));

        // The two spans are contiguous: series 0's own end angle equals
        // series 1's own start angle. Each `<path>` is self-closing on one
        // line, so slicing from its own `data-series="..."` match to the
        // next `</path>`/`>` safely isolates just that one tag's own
        // attributes.
        let desktop_tag_start = html.find(r#"data-series="desktop""#).unwrap();
        let desktop_tag_end = html[desktop_tag_start..]
            .find('>')
            .map(|i| desktop_tag_start + i)
            .unwrap();
        let desktop_end = attr_f64(&html[desktop_tag_start..desktop_tag_end], "data-end-angle");

        let mobile_tag_start = html.find(r#"data-series="mobile""#).unwrap();
        let mobile_tag_end = html[mobile_tag_start..]
            .find('>')
            .map(|i| mobile_tag_start + i)
            .unwrap();
        let mobile_start = attr_f64(&html[mobile_tag_start..mobile_tag_end], "data-start-angle");

        assert!(
            (desktop_end - mobile_start).abs() < 1e-2,
            "expected contiguous spans: {html}"
        );
    }

    #[test]
    fn grid_renders_a_background_track_per_ring() {
        let opts = RadialOptions {
            grid: true,
            ..Default::default()
        };
        let html = render(
            ChartKind::RadialBar,
            one_series_config(),
            one_series_data(),
            opts,
        );
        assert_eq!(html.matches(r#"data-slot="chart-polar-grid""#).count(), 3);
    }

    #[test]
    fn stacked_grid_renders_exactly_one_background_track() {
        let data = vec![ChartDatum {
            label: "January".to_string(),
            values: vec![Some(186.0), Some(80.0)],
            ..Default::default()
        }];
        let opts = RadialOptions {
            stacked: true,
            grid: true,
            ..Default::default()
        };
        let html = render(ChartKind::RadialBar, sample_config(), data, opts);
        assert_eq!(html.matches(r#"data-slot="chart-polar-grid""#).count(), 1);
    }

    #[test]
    fn value_labels_render_one_per_ring() {
        let opts = RadialOptions {
            labels: PieLabels::Value,
            ..Default::default()
        };
        let html = render(
            ChartKind::RadialBar,
            one_series_config(),
            one_series_data(),
            opts,
        );
        assert_eq!(html.matches(r#"data-slot="chart-arc-label""#).count(), 3);
    }

    #[test]
    fn list_labels_use_the_callers_own_text() {
        let opts = RadialOptions {
            labels: PieLabels::List(vec![
                "Chrome".to_string(),
                "Safari".to_string(),
                "Firefox".to_string(),
            ]),
            ..Default::default()
        };
        let html = render(
            ChartKind::RadialBar,
            one_series_config(),
            one_series_data(),
            opts,
        );
        assert!(html.contains(">Chrome<"));
        assert!(html.contains(">Safari<"));
        assert!(html.contains(">Firefox<"));
    }

    #[test]
    fn center_text_renders_both_lines() {
        let opts = RadialOptions {
            inner_radius: 30.0,
            center_text: Some(("1,999".to_string(), "Visitors".to_string())),
            ..Default::default()
        };
        let html = render(
            ChartKind::RadialBar,
            one_series_config(),
            one_series_data(),
            opts,
        );
        assert!(html.contains(r#"data-slot="chart-pie-center-text""#));
        assert!(html.contains("1,999"));
        assert!(html.contains("Visitors"));
    }

    #[test]
    fn corner_radius_adds_fillet_arc_commands() {
        let sharp = render(
            ChartKind::RadialBar,
            one_series_config(),
            one_series_data(),
            RadialOptions::default(),
        );
        let rounded = render(
            ChartKind::RadialBar,
            one_series_config(),
            one_series_data(),
            RadialOptions {
                corner_radius: 8.0,
                ..Default::default()
            },
        );
        let sharp_as = sharp.matches('A').count();
        let rounded_as = rounded.matches('A').count();
        assert!(
            rounded_as > sharp_as,
            "sharp={sharp_as} rounded={rounded_as}"
        );
    }

    #[test]
    fn does_not_panic_with_empty_data() {
        let html = render(
            ChartKind::RadialBar,
            one_series_config(),
            Vec::new(),
            RadialOptions::default(),
        );
        assert_eq!(html.matches(r#"data-slot="chart-arc""#).count(), 0);
    }
}
