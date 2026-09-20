//! Defines the [`ChartLegend`] component.

use dioxus::prelude::*;

use crate::chart::config::ChartIcon;
use crate::chart::context::use_chart;
use crate::dioxus_attributes::attributes;
use crate::merge_attributes;

/// Where a [`ChartLegend`] sits relative to the chart it's paired with.
/// Rendered as this element's own `data-align` attribute (not named in the
/// original `$S/chart-api.md` sketch -- see "API changes": without some
/// attribute reflecting it, this prop would have no observable effect at
/// all, since `ChartLegend` is a plain sibling of `Chart` with no layout
/// authority over it) for a themed stylesheet to act on (e.g. flexbox
/// `order`), since this primitive does not itself control where its
/// sibling `Chart` renders in the DOM.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum LegendAlign {
    /// The legend belongs above the chart.
    Top,
    /// The legend belongs below the chart (shadcn's own default).
    Bottom,
}

impl LegendAlign {
    fn as_str(self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Bottom => "bottom",
        }
    }
}

/// The props for the [`ChartLegend`] component.
#[derive(Props, Clone, PartialEq)]
pub struct ChartLegendProps {
    /// Where this legend belongs relative to the chart.
    #[props(default = LegendAlign::Bottom)]
    pub vertical_align: LegendAlign,

    /// Hide every series' icon, falling back to the plain color swatch even
    /// for a series with a [`crate::chart::ChartSeries::icon`] -- shadcn's
    /// `ChartLegendContent`'s own `hideIcon` prop.
    #[props(default)]
    pub hide_icon: bool,

    /// Additional attributes to apply to the legend's root `<ul>`.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
}

/// # ChartLegend
///
/// One `<li>` per configured series, each with a color swatch and its
/// label -- shadcn's `ChartLegend`/`ChartLegendContent`
/// (`dev-docs/research/chart-2026-09-19.md` §1.1), ported as a plain
/// sibling of [`crate::chart::Chart`] driven by the same
/// [`crate::chart::use_chart`] context, rather than a Recharts render-prop
/// consumer.
///
/// Must be rendered inside a [`crate::chart::ChartContainer`].
///
/// ## Accessibility
///
/// Each swatch carries `role="graphics-symbol"` (WAI-ARIA Graphics Module
/// 1.0 -- an atomic, presentational-children image role: the textbook fit
/// for a color swatch) with that series' label as its accessible name,
/// mirroring [`crate::chart::ChartTooltip`]'s own swatches. The label text
/// itself is plain, real text -- no extra ARIA needed for it.
///
/// ## Styling
///
/// The [`ChartLegend`] component defines the following data attributes you
/// can use to control styling:
/// - `data-slot="chart-legend"`, `data-align="top"|"bottom"`.
/// - `data-slot="chart-legend-item"[data-series=<key>]`, each containing
///   either `data-slot="chart-swatch"` or, when the series has an icon and
///   [`ChartLegendProps::hide_icon`] isn't set, `data-slot="chart-icon"` in
///   its place.
#[component]
pub fn ChartLegend(props: ChartLegendProps) -> Element {
    let ctx = use_chart();
    let config = (ctx.config)();

    // `data-slot`/`data-align` are structural wiring, not overridable
    // presentation -- `data-slot` is the selector the themed stylesheet's
    // whole ruleset hangs off, and `data-align` is what
    // `ChartLegendProps::vertical_align` exists to control (this
    // component's own doc). `merge_attributes` (`scripts/
    // check-attr-spread-collision.sh`'s own fix, replacing a raw
    // `..props.attributes` beside these as plain literals) makes "owned
    // wins" explicit and SSR/CSR-consistent instead of accidental.
    let owned = attributes!(ul {
        "data-slot": "chart-legend",
        "data-align": props.vertical_align.as_str(),
    });
    let merged = merge_attributes(vec![props.attributes, owned]);

    rsx! {
        ul {
            ..merged,

            for series in config.series.iter() {
                li {
                    key: "{series.key}",
                    "data-slot": "chart-legend-item",
                    "data-series": "{series.slot()}",
                    style: "--series-color: var(--color-{series.slot()})",

                    // A series' own icon wins over the plain swatch unless
                    // `hide_icon` opts back out -- shadcn's own
                    // `itemConfig?.icon && !hideIcon ? <itemConfig.icon /> :
                    // <div class="swatch" />` (`chart.tsx`'s
                    // `ChartLegendContent`): unlike the tooltip's own
                    // `hide_indicator` (which can hide the marker entirely),
                    // a legend item always shows SOME color marker, so
                    // `hide_icon` alone only ever falls back to the swatch,
                    // never removes both.
                    if let Some(ChartIcon(icon)) = series.icon {
                        if props.hide_icon {
                            span {
                                "data-slot": "chart-swatch",
                                role: "graphics-symbol",
                                "aria-label": "{series.label}",
                            }
                        } else {
                            span {
                                "data-slot": "chart-icon",
                                role: "graphics-symbol",
                                "aria-label": "{series.label}",
                                {icon.call(())}
                            }
                        }
                    } else {
                        span {
                            "data-slot": "chart-swatch",
                            role: "graphics-symbol",
                            "aria-label": "{series.label}",
                        }
                    }
                    "{series.label}"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart::{ChartConfig, ChartContainer, ChartDatum, ChartKind, ChartSeries};
    use dioxus_core::NoOpMutations;

    fn render(vertical_align: LegendAlign) -> String {
        #[derive(Clone, PartialEq, Props)]
        struct HarnessProps {
            vertical_align: LegendAlign,
        }
        #[component]
        fn Harness(props: HarnessProps) -> Element {
            let config = use_signal(|| {
                ChartConfig::new()
                    .series("desktop", "Desktop", "var(--dx-chart-1)")
                    .series("mobile", "Mobile", "var(--dx-chart-2)")
            });
            let data = use_signal(|| {
                vec![ChartDatum {
                    label: "January".to_string(),
                    values: vec![Some(186.0), Some(80.0)],
                    ..Default::default()
                }]
            });
            rsx! {
                ChartContainer { config, data, kind: ChartKind::Line,
                    ChartLegend { vertical_align: props.vertical_align }
                }
            }
        }
        let mut dom = VirtualDom::new_with_props(Harness, HarnessProps { vertical_align });
        dom.rebuild_in_place();
        dom.render_immediate(&mut NoOpMutations);
        dioxus_ssr::render(&dom)
    }

    #[test]
    fn renders_one_item_per_series_with_a_labelled_swatch() {
        let html = render(LegendAlign::Bottom);
        assert!(html.contains(r#"data-slot="chart-legend""#));
        assert_eq!(html.matches(r#"data-slot="chart-legend-item""#).count(), 2);
        assert!(html.contains(r#"data-series="desktop""#));
        assert!(html.contains(r#"data-series="mobile""#));
        assert!(html.contains("Desktop"));
        assert!(html.contains("Mobile"));
        assert_eq!(
            html.matches(r#"role="graphics-symbol""#).count(),
            2,
            "one swatch per series"
        );
        assert!(html.contains("--series-color: var(--color-desktop)"));
    }

    #[test]
    fn vertical_align_is_reflected_as_a_data_attribute() {
        assert!(render(LegendAlign::Top).contains(r#"data-align="top""#));
        assert!(render(LegendAlign::Bottom).contains(r#"data-align="bottom""#));
    }

    #[test]
    fn legend_align_as_str_is_lowercase() {
        assert_eq!(LegendAlign::Top.as_str(), "top");
        assert_eq!(LegendAlign::Bottom.as_str(), "bottom");
    }

    /// A series with an icon renders `data-slot="chart-icon"` in its
    /// item, falling back to the plain swatch only when `hide_icon` is set
    /// -- the OTHER series (no icon) always gets the plain swatch either
    /// way, matching shadcn's own per-item (not global) icon/swatch choice.
    #[test]
    fn a_series_icon_replaces_the_swatch_unless_hide_icon_is_set() {
        fn config_with_icon() -> ChartConfig {
            let base = ChartConfig::new()
                .series("desktop", "Desktop", "var(--dx-chart-1)")
                .series("mobile", "Mobile", "var(--dx-chart-2)");
            ChartConfig {
                series: base
                    .series
                    .into_iter()
                    .map(|s| {
                        if s.key == "desktop" {
                            ChartSeries {
                                icon: Some(ChartIcon(Callback::new(
                                    |_| rsx! { svg { "data-testid": "icon" } },
                                ))),
                                ..s
                            }
                        } else {
                            s
                        }
                    })
                    .collect(),
            }
        }

        #[derive(Clone, PartialEq, Props)]
        struct HarnessProps {
            hide_icon: bool,
        }
        #[component]
        fn Harness(props: HarnessProps) -> Element {
            let config = use_signal(config_with_icon);
            let data = use_signal(|| {
                vec![ChartDatum {
                    label: "January".to_string(),
                    values: vec![Some(186.0), Some(80.0)],
                    ..Default::default()
                }]
            });
            rsx! {
                ChartContainer { config, data, kind: ChartKind::Line,
                    ChartLegend { hide_icon: props.hide_icon }
                }
            }
        }
        fn render_with_icon(hide_icon: bool) -> String {
            let mut dom = VirtualDom::new_with_props(Harness, HarnessProps { hide_icon });
            dom.rebuild_in_place();
            dom.render_immediate(&mut NoOpMutations);
            dioxus_ssr::render(&dom)
        }

        let shown = render_with_icon(false);
        assert_eq!(
            shown.matches(r#"data-slot="chart-icon""#).count(),
            1,
            "{shown}"
        );
        assert!(shown.contains(r#"data-testid="icon""#), "{shown}");
        // The other series (mobile, no icon) still gets a plain swatch.
        assert_eq!(
            shown.matches(r#"data-slot="chart-swatch""#).count(),
            1,
            "{shown}"
        );

        let hidden = render_with_icon(true);
        assert_eq!(
            hidden.matches(r#"data-slot="chart-icon""#).count(),
            0,
            "{hidden}"
        );
        // Both items fall back to (or already used) the plain swatch --
        // `hide_icon` never removes the marker entirely.
        assert_eq!(
            hidden.matches(r#"data-slot="chart-swatch""#).count(),
            2,
            "{hidden}"
        );
    }
}
