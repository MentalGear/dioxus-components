//! Defines the [`ChartTooltip`] component.

use dioxus::prelude::*;

use crate::chart::context::use_chart;
use crate::chart::engine::scale::fmt_decimal;

/// The props for the [`ChartTooltip`] component.
#[derive(Props, Clone, PartialEq)]
pub struct ChartTooltipProps {
    /// Format the active datum's label (its full form, e.g. `"January"` --
    /// unlike [`crate::chart::ChartProps::x_tick_format`], not truncated by
    /// default).
    #[props(default)]
    pub label_format: Option<Callback<String, String>>,

    /// Format a series' value. Defaults to the same up-to-2-decimals rule
    /// the hidden data table uses.
    #[props(default)]
    pub value_format: Option<Callback<f64, String>>,

    /// Hide the label row.
    #[props(default)]
    pub hide_label: bool,

    /// Hide each series' color swatch.
    #[props(default)]
    pub hide_indicator: bool,

    /// Additional attributes to apply to the tooltip root element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// Custom content, replacing the default label + per-series rows.
    #[props(default)]
    pub children: Option<Element>,
}

/// # ChartTooltip
///
/// A percent-positioned tooltip that follows [`crate::chart::use_chart`]'s
/// `active_index` -- shadcn's `ChartTooltip`/`ChartTooltipContent`
/// (`dev-docs/research/chart-2026-09-19.md` §1.1), ported as a plain
/// sibling of [`crate::chart::Chart`] rather than a Recharts render-prop
/// consumer (the module doc explains why the rest of this API differs from
/// shadcn's own children-introspection shape).
///
/// Position comes entirely from [`crate::chart::Chart`]'s own scales
/// (shared via context -- see that module's `ChartLayout`, not part of
/// this crate's public API) evaluated at the active index: never a DOM
/// measurement. Always rendered, even when closed
/// (`data-state="closed"`, which a themed stylesheet hides with CSS) --
/// so hydration never has to create or remove this element, only flip one
/// attribute.
///
/// Must be rendered inside a [`crate::chart::ChartContainer`], as a
/// sibling of [`crate::chart::Chart`] (not nested inside it).
///
/// ## Accessibility
///
/// `aria-hidden="true"` on the root: this tooltip is a sighted-user
/// convenience, not the accessible data path -- the hidden `<table>`
/// `Chart` always renders is (`dev-docs/research/chart-2026-09-19.md`
/// §2.5). Each swatch carries `role="graphics-symbol"` (WAI-ARIA Graphics
/// Module 1.0 -- an atomic, presentational-children image role, the
/// textbook fit for a color swatch) and its series' label as its own
/// accessible name, mirroring [`crate::chart::ChartLegend`]'s swatches.
///
/// ## Styling
///
/// The [`ChartTooltip`] component defines the following data attributes
/// you can use to control styling:
/// - `data-slot="chart-tooltip"`, `data-state="open"|"closed"`.
/// - `data-slot="chart-tooltip-label"`.
/// - `data-slot="chart-tooltip-item"[data-series=<key>]`, each containing
///   `data-slot="chart-swatch"` (omitted when
///   [`ChartTooltipProps::hide_indicator`] is set), `"chart-tooltip-name"`,
///   `"chart-tooltip-value"`.
#[component]
pub fn ChartTooltip(props: ChartTooltipProps) -> Element {
    let ctx = use_chart();
    let config = (ctx.config)();
    let data = (ctx.data)();
    let active = (ctx.active_index)();
    let layout = (ctx.layout)();

    let state = if active.is_some() { "open" } else { "closed" };

    // Percent position from the active index's band center / top value --
    // never read when `data-state="closed"` (a themed stylesheet hides
    // it), so an arbitrary fallback is fine whenever either half of this
    // is still unset.
    let (left_pct, top_pct) = match (active, &layout) {
        (Some(i), Some(layout)) if i < data.len() && layout.width > 0.0 && layout.height > 0.0 => {
            let x = layout.x_scale.center(i);
            let y = layout
                .top_value
                .get(i)
                .map_or(0.0, |v| layout.y_scale.scale(*v));
            (x / layout.width * 100.0, y / layout.height * 100.0)
        }
        _ => (50.0, 50.0),
    };

    let active_datum = active.and_then(|i| data.get(i));

    rsx! {
        div {
            "data-slot": "chart-tooltip",
            "data-state": state,
            "aria-hidden": "true",
            style: "left:{fmt_decimal(left_pct, 3)}%;top:{fmt_decimal(top_pct, 3)}%",
            ..props.attributes,

            if let Some(children) = &props.children {
                {children}
            } else {
                if !props.hide_label {
                    if let Some(datum) = active_datum {
                        div { "data-slot": "chart-tooltip-label",
                            {format_label(&datum.label, &props.label_format)}
                        }
                    }
                }
                for (s , series) in config.series.iter().enumerate() {
                    {
                        let value = active_datum.and_then(|d| d.values.get(s).copied().flatten());
                        rsx! {
                            div {
                                key: "{series.key}",
                                "data-slot": "chart-tooltip-item",
                                "data-series": "{series.slot()}",
                                style: "--series-color: var(--color-{series.slot()})",

                                if !props.hide_indicator {
                                    span {
                                        "data-slot": "chart-swatch",
                                        role: "graphics-symbol",
                                        "aria-label": "{series.label}",
                                    }
                                }
                                span { "data-slot": "chart-tooltip-name", "{series.label}" }
                                span { "data-slot": "chart-tooltip-value",
                                    {format_value(value, &props.value_format)}
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Format the active datum's label: the caller's own formatter if given,
/// else unchanged.
fn format_label(label: &str, format: &Option<Callback<String, String>>) -> String {
    match format {
        Some(cb) => cb.call(label.to_string()),
        None => label.to_string(),
    }
}

/// Format one series' value at the active datum: `"—"` for `None`, else
/// the caller's own formatter or (default) up to 2 decimal places --
/// matching the hidden data table's own convention.
fn format_value(value: Option<f64>, format: &Option<Callback<f64, String>>) -> String {
    match value {
        None => "—".to_string(),
        Some(v) => match format {
            Some(cb) => cb.call(v),
            None => fmt_decimal(v, 2),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart::components::chart::Chart;
    use crate::chart::{ChartConfig, ChartContainer, ChartDatum, ChartKind};
    use dioxus_core::NoOpMutations;

    /// Sets `active_index` once via an effect, so `render_immediate` (this
    /// crate's existing SSR-test pattern -- `tooltip.rs`'s *other* tooltip,
    /// the tooltip primitive's own tests) can flush it into the tree
    /// before `dioxus_ssr::render` serializes it. Never reads
    /// `active_index` itself (only writes it), so this cannot be a
    /// self-subscribing effect.
    #[component]
    fn ActiveIndexSetter(index: Option<usize>) -> Element {
        let ctx = use_chart();
        let mut active_index = ctx.active_index;
        use_effect(move || {
            active_index.set(index);
        });
        rsx! {}
    }

    fn two_series_config() -> ChartConfig {
        ChartConfig::new()
            .series("desktop", "Desktop", "var(--dx-chart-1)")
            .series("mobile", "Mobile", "var(--dx-chart-2)")
    }

    fn two_datum_data() -> Vec<ChartDatum> {
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
        #[props(default)]
        active: Option<usize>,
    }

    #[component]
    fn Harness(props: HarnessProps) -> Element {
        let config = use_signal(two_series_config);
        let data = use_signal(two_datum_data);
        rsx! {
            ChartContainer { config, data, kind: ChartKind::Line,
                Chart { aria_label: "Visitors by month" }
                ActiveIndexSetter { index: props.active }
                ChartTooltip {}
            }
        }
    }

    fn render(active: Option<usize>) -> String {
        let mut dom = VirtualDom::new_with_props(Harness, HarnessProps { active });
        dom.rebuild_in_place();
        dom.render_immediate(&mut NoOpMutations);
        dioxus_ssr::render(&dom)
    }

    #[test]
    fn closed_by_default_and_aria_hidden() {
        let html = render(None);
        assert!(html.contains(r#"data-slot="chart-tooltip""#));
        assert!(html.contains(r#"data-state="closed""#));
        assert!(html.contains(r#"aria-hidden="true""#));
        assert!(
            html.contains("left:") && html.contains("top:"),
            "always positioned, even closed: {html}"
        );
    }

    #[test]
    fn open_shows_the_active_label_and_every_series_value_including_gaps() {
        let html = render(Some(1)); // February: desktop=305, mobile=None
        assert!(html.contains(r#"data-state="open""#));
        assert!(html.contains(r#"data-slot="chart-tooltip-label""#));
        assert!(html.contains("February"));
        assert_eq!(html.matches(r#"data-slot="chart-tooltip-item""#).count(), 2);
        assert!(html.contains(r#"data-series="desktop""#));
        assert!(html.contains(r#"data-series="mobile""#));
        assert!(html.contains("Desktop"));
        assert!(html.contains("Mobile"));
        assert!(html.contains("305"));
        assert!(html.contains('—'), "mobile is None at February: {html}");
        assert_eq!(
            html.matches(r#"role="graphics-symbol""#).count(),
            2,
            "one swatch per series"
        );
    }

    #[test]
    fn hide_label_and_hide_indicator_props_are_respected() {
        #[component]
        fn HiddenHarness(active: Option<usize>) -> Element {
            let config = use_signal(two_series_config);
            let data = use_signal(two_datum_data);
            rsx! {
                ChartContainer { config, data, kind: ChartKind::Line,
                    Chart { aria_label: "Visitors by month" }
                    ActiveIndexSetter { index: active }
                    ChartTooltip { hide_label: true, hide_indicator: true }
                }
            }
        }
        let mut dom =
            VirtualDom::new_with_props(HiddenHarness, HiddenHarnessProps { active: Some(0) });
        dom.rebuild_in_place();
        dom.render_immediate(&mut NoOpMutations);
        let html = dioxus_ssr::render(&dom);
        assert!(!html.contains(r#"data-slot="chart-tooltip-label""#));
        assert!(!html.contains(r#"data-slot="chart-swatch""#));
        // The values themselves are still shown -- only the label row and
        // the swatches are hidden.
        assert!(html.contains("186"));
    }

    #[test]
    fn custom_children_replace_the_default_content() {
        #[component]
        fn CustomHarness(active: Option<usize>) -> Element {
            let config = use_signal(two_series_config);
            let data = use_signal(two_datum_data);
            rsx! {
                ChartContainer { config, data, kind: ChartKind::Line,
                    Chart { aria_label: "Visitors by month" }
                    ActiveIndexSetter { index: active }
                    ChartTooltip {
                        div { "data-slot": "custom-tooltip-body", "custom!" }
                    }
                }
            }
        }
        let mut dom =
            VirtualDom::new_with_props(CustomHarness, CustomHarnessProps { active: Some(0) });
        dom.rebuild_in_place();
        dom.render_immediate(&mut NoOpMutations);
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("custom!"));
        assert!(!html.contains(r#"data-slot="chart-tooltip-label""#));
        assert!(!html.contains(r#"data-slot="chart-tooltip-item""#));
    }

    /// A single series/single datum dataset makes the percent position
    /// hand-derivable exactly: one band always centers at the exact
    /// midpoint of the plot range regardless of padding, and one value
    /// equal to the (nice-rounded) domain's own max scales to the exact
    /// top of the plot. Cross-checks `ChartTooltip`'s own percent
    /// arithmetic against `Chart`'s default 600x300/margin layout without
    /// re-deriving `Chart`'s internal margin constants here.
    #[test]
    fn open_position_is_the_exact_band_center_and_top_value_percent() {
        #[component]
        fn OneSeriesHarness(active: Option<usize>) -> Element {
            let config = use_signal(|| ChartConfig::new().series("a", "A", "red"));
            let data = use_signal(|| {
                vec![ChartDatum {
                    label: "X".to_string(),
                    values: vec![Some(100.0)],
                    ..Default::default()
                }]
            });
            rsx! {
                ChartContainer { config, data, kind: ChartKind::Line,
                    Chart { aria_label: "Solo" }
                    ActiveIndexSetter { index: active }
                    ChartTooltip {}
                }
            }
        }
        let mut dom =
            VirtualDom::new_with_props(OneSeriesHarness, OneSeriesHarnessProps { active: Some(0) });
        dom.rebuild_in_place();
        dom.render_immediate(&mut NoOpMutations);
        let html = dioxus_ssr::render(&dom);
        // x: the lone band's center is exactly the plot's own midpoint
        // ((8 + 592) / 2 = 300), 50% of the 600-wide viewBox.
        assert!(html.contains("left:50%"), "expected left:50%: {html}");
        // y: nice_domain(0, 100) is exactly (0, 100), so the value 100
        // scales to exactly the plot's top (y = 8 of 300), i.e. 8/300*100.
        assert!(html.contains("top:2.667%"), "expected top:2.667%: {html}");
    }
}
