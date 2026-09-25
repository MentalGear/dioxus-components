//! Defines the [`ChartTooltip`] component.

use dioxus::prelude::*;

use crate::chart::config::ChartIcon;
use crate::chart::context::use_chart;
use crate::chart::engine::scale::fmt_decimal;
use crate::dioxus_attributes::attributes;
use crate::{fold_style_attributes, merge_attributes};

/// The shape of a [`ChartTooltip`] row's color indicator -- shadcn's
/// `ChartTooltipContent`'s own `indicator` prop (`"dot" | "line" |
/// "dashed"`, `$S/refs/ui/apps/v4/registry/new-york-v4/ui/chart.tsx`), plus
/// a `None` variant [`ChartTooltipProps::hide_indicator`] already covers as
/// a separate bool -- kept here too so a caller can express "no indicator"
/// either way (`hide_indicator: true`, matching shadcn's own distinct
/// `hideIndicator` prop byte-for-byte, or `indicator: TooltipIndicator::
/// None`) and so this enum's own render match stays a plain, exhaustive
/// four-arm match with no side condition.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum TooltipIndicator {
    /// A small filled square swatch (the default).
    #[default]
    Dot,
    /// A thin bar the full height of its row.
    Line,
    /// A dashed vertical rule with no fill.
    Dashed,
    /// No indicator at all.
    None,
}

impl TooltipIndicator {
    /// The `data-indicator` attribute value, matching this crate's existing
    /// `data-orientation`/`data-direction`-style lowercase tokens.
    fn as_str(self) -> &'static str {
        match self {
            Self::Dot => "dot",
            Self::Line => "line",
            Self::Dashed => "dashed",
            Self::None => "none",
        }
    }
}

/// One row's data, passed to [`ChartTooltipProps::formatter`] for fully
/// custom row rendering -- shadcn's `formatter(value, name, item, index,
/// item.payload)` (`chart.tsx`), reshaped as one plain struct instead of
/// five positional arguments plus a raw Recharts payload object this crate
/// has no equivalent of.
#[derive(Clone, PartialEq, Debug)]
pub struct TooltipRow {
    /// This row's series' own [`crate::chart::ChartSeries::key`] -- shadcn's
    /// `name` argument to `formatter` (the raw series key, not the display
    /// label).
    pub key: String,
    /// This row's display label -- the series' own
    /// [`crate::chart::ChartSeries::label`], or
    /// [`ChartTooltipProps::name_key`] when set.
    pub label: String,
    /// This row's value at the active datum, `None` for a gap.
    pub value: Option<f64>,
    /// This row's resolved color, as a `var(--color-<slot>)` reference.
    pub color: String,
    /// This row's position among the rendered rows (0-based).
    pub index: usize,
    /// Whether this is the last rendered row -- shadcn's own `advanced`
    /// demo appends an extra "Total" row only after the last one (a
    /// fixture-specific `index === 1` there); this flag generalizes that to
    /// any series count.
    pub is_last: bool,
    /// The sum of every series' value at this datum (`None` treated as
    /// `0.0`) -- shadcn's `advanced` demo instead sums two named fields
    /// directly off Recharts' own raw per-datum payload
    /// (`item.payload.running + item.payload.swimming`); since this crate's
    /// tooltip has no equivalent free-form payload object, the total is
    /// precomputed once and handed to every row instead.
    pub total: f64,
}

/// The props for the [`ChartTooltip`] component.
#[derive(Props, Clone, PartialEq)]
pub struct ChartTooltipProps {
    /// Format the active datum's label (its full form, e.g. `"January"` --
    /// unlike [`crate::chart::ChartProps::x_tick_format`], not truncated by
    /// default). Applied after [`Self::label_key`]'s own substitution, when
    /// both are set.
    #[props(default)]
    pub label_format: Option<Callback<String, String>>,

    /// Format a series' value. Defaults to the same up-to-2-decimals rule
    /// the hidden data table uses. Has no effect on a row [`Self::formatter`]
    /// replaces.
    #[props(default)]
    pub value_format: Option<Callback<f64, String>>,

    /// Hide the label row.
    #[props(default)]
    pub hide_label: bool,

    /// Hide each series' color swatch (or icon). [`Self::indicator`]'s own
    /// `None` variant does the same thing; see that enum's doc for why both
    /// exist.
    #[props(default)]
    pub hide_indicator: bool,

    /// The row indicator's shape -- shadcn's `indicator` prop. Reflected as
    /// `data-indicator` on both the tooltip root and (when shown) each
    /// row's own swatch, for a themed stylesheet to style by.
    #[props(default)]
    pub indicator: TooltipIndicator,

    /// Overrides the label row's text with a literal string instead of the
    /// active datum's own [`crate::chart::ChartDatum::label`] -- shadcn's
    /// `labelKey`, which instead points at a *second* lookup into
    /// `ChartConfig` (a free-form `Record<string, {label}>` there); this
    /// crate's [`crate::chart::ChartConfig`] has no such generic string
    /// registry (see that type's own module doc on why it stays a plain,
    /// positional, strongly-typed `Vec`), so the same *observable* effect
    /// -- a fixed heading instead of the per-datum category -- is exposed
    /// directly as a literal string.
    #[props(default)]
    pub label_key: Option<String>,

    /// Overrides every row's displayed name with a literal string instead
    /// of that row's own series label -- shadcn's `nameKey` (same
    /// simplification as [`Self::label_key`] above, and just as rarely
    /// useful with more than one series, since every row would then show
    /// the identical text).
    #[props(default)]
    pub name_key: Option<String>,

    /// Fully custom row rendering: called once per series with that row's
    /// [`TooltipRow`], replacing the default swatch/icon + name + value
    /// markup entirely (the row's own outer container still renders, so
    /// `data-slot="chart-tooltip-item"` stays one-per-series regardless) --
    /// shadcn's `formatter` (`chart.tsx`).
    #[props(default)]
    pub formatter: Option<Callback<TooltipRow, Element>>,

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
/// - `data-slot="chart-tooltip"`, `data-state="open"|"closed"`,
///   `data-indicator="dot"|"line"|"dashed"|"none"` ([`ChartTooltipProps::indicator`]).
/// - `data-slot="chart-tooltip-label"`.
/// - `data-slot="chart-tooltip-item"[data-series=<key>]`, each containing
///   either (a) [`ChartTooltipProps::formatter`]'s own returned markup, or
///   (b) the default row: `data-slot="chart-swatch"[data-indicator=...]`
///   (omitted when [`ChartTooltipProps::hide_indicator`] is set or
///   `indicator` is [`TooltipIndicator::None`]) or, when the series has an
///   icon, `data-slot="chart-icon"[role="graphics-symbol"]` in its place,
///   then `"chart-tooltip-name"`, `"chart-tooltip-value"`.
#[component]
pub fn ChartTooltip(props: ChartTooltipProps) -> Element {
    let ctx = use_chart();
    let config = (ctx.config)();
    let data = (ctx.data)();
    let active = (ctx.active_index)();
    let layout = (ctx.layout)();

    let state = if active.is_some() { "open" } else { "closed" };
    let indicator_str = props.indicator.as_str();

    // The active index's own `(left%, top%)` anchor, computed by whichever
    // family rendered (§4(d) of the stage-2 handoff -- `ChartLayout`'s own
    // doc) -- never read when `data-state="closed"` (a themed stylesheet
    // hides it), so an arbitrary fallback is fine whenever either half of
    // this is still unset (no layout yet, or this family hasn't populated
    // an anchor for the active index).
    let (left_pct, top_pct) = active
        .and_then(|i| layout.as_ref().and_then(|l| l.anchor_percent.get(i)))
        .copied()
        .unwrap_or((50.0, 50.0));

    let active_datum = active.and_then(|i| data.get(i));

    // `data-slot`/`data-state`/`data-indicator`/`aria-hidden` are
    // structural/aria wiring this component owns (`data-slot` is the
    // themed stylesheet's whole selector; `data-state`/`aria-hidden` are
    // this tooltip's own open/closed and sighted-only contract, this
    // module's own doc; `data-indicator` mirrors `ChartTooltipProps::
    // indicator` for a themed stylesheet to style each swatch by) --
    // `merge_attributes` (`scripts/check-attr-spread-collision.sh`'s own
    // fix, replacing a raw `..props.attributes` beside these as plain
    // literals) makes "owned wins" explicit and SSR/CSR-consistent instead
    // of accidental.
    //
    // `style` is handled separately, via `fold_style_attributes`
    // (`resizable.rs`'s/`context_menu.rs`'s own identical fix, cited in
    // that helper's own doc): this component's own position is a literal
    // `style` string, but a caller may also pass CSS the *shorthand* way
    // (e.g. a themed wrapper's `padding`), which becomes its own separate
    // `namespace="style"` attributes rather than colliding on the `style`
    // name itself -- folding them into one string first is what
    // `merge_attributes` (name+namespace keyed) cannot do.
    let (caller_style, attributes) = fold_style_attributes(props.attributes);
    let position_style = format!(
        "left:{}%;top:{}%",
        fmt_decimal(left_pct, 3),
        fmt_decimal(top_pct, 3)
    );
    let style = match caller_style {
        Some(extra) => format!("{position_style};{extra}"),
        None => position_style,
    };
    let owned = attributes!(div {
        "data-slot": "chart-tooltip",
        "data-state": state,
        "data-indicator": indicator_str,
        "aria-hidden": "true",
        style,
    });
    let merged = merge_attributes(vec![attributes, owned]);

    // The cross-series total at the active datum (`None` treated as `0.0`)
    // -- only meaningful to a caller-supplied `formatter`
    // ([`TooltipRow::total`]'s own doc), but cheap regardless: `config`'s
    // series list is small, and this whole block is skipped entirely when
    // the tooltip is closed (`active_datum` is `None`).
    let total: f64 = active_datum
        .map(|d| d.values.iter().filter_map(|v| *v).sum())
        .unwrap_or(0.0);
    let last_index = config.series.len().saturating_sub(1);

    rsx! {
        div {
            ..merged,

            if let Some(children) = &props.children {
                {children}
            } else {
                if !props.hide_label {
                    if let Some(datum) = active_datum {
                        div { "data-slot": "chart-tooltip-label",
                            {format_label(&datum.label, &props.label_key, &props.label_format)}
                        }
                    }
                }
                for (s , series) in config.series.iter().enumerate() {
                    {
                        let value = active_datum.and_then(|d| d.values.get(s).copied().flatten());
                        let label = props.name_key.clone().unwrap_or_else(|| series.label.clone());
                        let color = format!("var(--color-{})", series.slot());

                        if let Some(formatter) = &props.formatter {
                            let row = TooltipRow {
                                key: series.key.clone(),
                                label,
                                value,
                                color,
                                index: s,
                                is_last: s == last_index,
                                total,
                            };
                            rsx! {
                                div {
                                    key: "{series.key}",
                                    "data-slot": "chart-tooltip-item",
                                    "data-series": "{series.slot()}",
                                    style: "--series-color: var(--color-{series.slot()})",
                                    {formatter.call(row)}
                                }
                            }
                        } else {
                            // A series' own icon always wins over the
                            // indicator swatch entirely -- shadcn's own
                            // `itemConfig?.icon ? <itemConfig.icon /> :
                            // (!hideIndicator && <div .../>)`: `hideIndicator`
                            // and `indicator`'s shape both only govern the
                            // FALLBACK swatch, never a real icon.
                            let show_swatch = !props.hide_indicator
                                && !matches!(props.indicator, TooltipIndicator::None);
                            rsx! {
                                div {
                                    key: "{series.key}",
                                    "data-slot": "chart-tooltip-item",
                                    "data-series": "{series.slot()}",
                                    style: "--series-color: var(--color-{series.slot()})",

                                    if let Some(ChartIcon(icon)) = series.icon {
                                        span {
                                            "data-slot": "chart-icon",
                                            role: "graphics-symbol",
                                            "aria-label": "{label}",
                                            {icon.call(())}
                                        }
                                    } else if show_swatch {
                                        span {
                                            "data-slot": "chart-swatch",
                                            "data-indicator": indicator_str,
                                            role: "graphics-symbol",
                                            "aria-label": "{label}",
                                        }
                                    }
                                    span { "data-slot": "chart-tooltip-name", "{label}" }
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
}

/// Format the active datum's label: [`ChartTooltipProps::label_key`]'s own
/// literal override when set (else the datum's own label), then the
/// caller's own [`ChartTooltipProps::label_format`] if given.
fn format_label(
    label: &str,
    label_key: &Option<String>,
    format: &Option<Callback<String, String>>,
) -> String {
    let base = label_key.clone().unwrap_or_else(|| label.to_string());
    match format {
        Some(cb) => cb.call(base),
        None => base,
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
    use crate::chart::{ChartConfig, ChartContainer, ChartDatum, ChartKind, ChartSeries};
    use dioxus_core::NoOpMutations;

    /// Sets `active_index` once, synchronously, during this component's
    /// own first render (`use_hook`, not `use_effect`): since this
    /// component is always rendered as `ChartTooltip`'s own EARLIER
    /// sibling (child order: `Chart`, `ActiveIndexSetter`, `ChartTooltip`)
    /// and Dioxus renders children top-to-bottom within one pass,
    /// `ChartTooltip` observes the already-set value on `dioxus_ssr`'s
    /// very first render -- no second render/diff needed at all.
    /// Previously `use_effect` (deferred to the NEXT `render_immediate`
    /// call, needing a real two-pass mount-then-diff dance every test here
    /// relied on): switched after finding a real `dioxus_ssr` limitation
    /// that dance exposes for a spread-heavy root element specifically
    /// (`render_with`'s own doc, stage-2 chart round) -- `use_hook`
    /// sidesteps needing a diff at all, for every test in this file, not
    /// just the ones that found it. Never reads `active_index` itself
    /// (only writes it once), so this was never a self-subscribing effect
    /// either way.
    #[component]
    fn ActiveIndexSetter(index: Option<usize>) -> Element {
        let ctx = use_chart();
        let mut active_index = ctx.active_index;
        use_hook(|| {
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

    /// Every field defaulted -- the base every stage-2 test below starts
    /// from via struct-update (`ChartTooltipProps { field: val,
    /// ..default_tooltip_props() }`), since `ChartTooltipProps` has no
    /// `Default` impl of its own (`Callback`-holding props structs
    /// generally don't derive it) and a raw struct literal must otherwise
    /// name every field.
    fn default_tooltip_props() -> ChartTooltipProps {
        ChartTooltipProps {
            label_format: None,
            value_format: None,
            hide_label: false,
            hide_indicator: false,
            indicator: TooltipIndicator::default(),
            label_key: None,
            name_key: None,
            formatter: None,
            attributes: Vec::new(),
            children: None,
        }
    }

    /// Renders `ChartTooltip` with the given props, always open at index 0
    /// -- the shared harness every stage-2 test below builds on. Takes the
    /// already-built `ChartTooltipProps` directly (not a closure over
    /// [`default_tooltip_props`]): a closure's anonymous type never
    /// implements `PartialEq`, which `#[derive(Props)]` requires of every
    /// field, `ChartTooltipProps` itself included.
    /// The tooltip's own slice of a [`render_with`] page: everything from
    /// the tooltip root's own OPENING `<` onward. `render_with`'s `Harness`
    /// renders `Chart` (whose own hidden data table always mirrors every
    /// category label and every series' own label, e.g. "January"/
    /// "Desktop"/"Mobile", regardless of what `ChartTooltip`'s
    /// `label_key`/`name_key` do) BEFORE `ChartTooltip`, so a plain
    /// `html.contains("January")`-style assertion on the *whole* page can
    /// never distinguish "the tooltip still shows the raw label" from "the
    /// hidden table (correctly, always) does" -- scoping to this tail
    /// slice is what makes an override assertion meaningful.
    ///
    /// Seeks back to the preceding `<` rather than starting exactly at the
    /// `data-slot="chart-tooltip"` match: the root `<div>`'s attributes go
    /// through `merge_attributes` (§4(a) of the stage-2 handoff), which
    /// sorts them by name, so `data-slot` is very often NOT the tag's first
    /// attribute (e.g. `aria-hidden`/`data-indicator` both sort before it).
    /// Slicing from the match itself silently dropped every attribute that
    /// sorts earlier -- found via a root-tag assertion that failed despite
    /// the merged `Vec<Attribute>` being verified correct immediately
    /// before the `rsx!` spread; the dropped attributes were real, just
    /// outside this fn's own (buggy) slice.
    fn tooltip_fragment(html: &str) -> &str {
        let match_start = html
            .find(r#"data-slot="chart-tooltip""#)
            .expect("no chart-tooltip root in html");
        let start = html[..match_start]
            .rfind('<')
            .expect("no opening tag before the match");
        &html[start..]
    }

    fn render_with(tooltip_props: ChartTooltipProps) -> String {
        #[derive(Clone, PartialEq, Props)]
        struct HarnessProps {
            tooltip_props: ChartTooltipProps,
        }
        #[component]
        fn Harness(props: HarnessProps) -> Element {
            let config = use_signal(two_series_config);
            let data = use_signal(two_datum_data);
            rsx! {
                ChartContainer { config, data, kind: ChartKind::Line,
                    Chart { aria_label: "Visitors by month" }
                    ActiveIndexSetter { index: Some(0) }
                    ChartTooltip { ..props.tooltip_props }
                }
            }
        }
        let mut dom = VirtualDom::new_with_props(Harness, HarnessProps { tooltip_props });
        dom.rebuild_in_place();
        dioxus_ssr::render(&dom)
    }

    #[test]
    fn indicator_line_is_reflected_on_the_root_and_every_swatch() {
        let html = render_with(ChartTooltipProps {
            indicator: TooltipIndicator::Line,
            ..default_tooltip_props()
        });
        // Not one concatenated substring match: the root's three
        // attributes go through `merge_attributes` (§4(a) of the stage-2
        // handoff), which sorts by name -- correct, SSR/CSR-consistent
        // output, just not necessarily in the order they were written in
        // this file's own `attributes!` call. Each attribute's own
        // presence/value is what matters, not their relative order.
        let root = tooltip_fragment(&html);
        let root_tag_end = root.find('>').unwrap_or(root.len());
        let root_tag = &root[..root_tag_end];
        assert!(root_tag.contains(r#"data-slot="chart-tooltip""#));
        assert!(root_tag.contains(r#"data-state="open""#));
        assert!(root_tag.contains(r#"data-indicator="line""#));
        assert_eq!(
            html.matches(r#"data-slot="chart-swatch" data-indicator="line""#)
                .count(),
            2
        );
    }

    #[test]
    fn indicator_dashed_is_reflected_on_every_swatch() {
        let html = render_with(ChartTooltipProps {
            indicator: TooltipIndicator::Dashed,
            ..default_tooltip_props()
        });
        assert_eq!(
            html.matches(r#"data-slot="chart-swatch" data-indicator="dashed""#)
                .count(),
            2
        );
    }

    #[test]
    fn indicator_none_and_hide_indicator_both_render_zero_swatches() {
        let via_indicator = render_with(ChartTooltipProps {
            indicator: TooltipIndicator::None,
            ..default_tooltip_props()
        });
        assert!(!via_indicator.contains(r#"data-slot="chart-swatch""#));
        assert!(via_indicator.contains(r#"data-indicator="none""#));

        let via_hide_indicator = render_with(ChartTooltipProps {
            hide_indicator: true,
            ..default_tooltip_props()
        });
        assert!(!via_hide_indicator.contains(r#"data-slot="chart-swatch""#));
        // `hide_indicator` alone doesn't change `indicator`'s own default value.
        assert!(via_hide_indicator.contains(r#"data-indicator="dot""#));
    }

    #[test]
    fn dot_indicator_is_the_default() {
        let html = render_with(default_tooltip_props());
        assert!(html.contains(r#"data-indicator="dot""#));
        assert_eq!(
            html.matches(r#"data-slot="chart-swatch" data-indicator="dot""#)
                .count(),
            2
        );
    }

    #[test]
    fn label_key_overrides_the_label_and_label_format_still_applies_after() {
        // Scoped to `tooltip_fragment` (see its own doc): `Chart`'s hidden
        // table always mirrors "January" as a real row header regardless
        // of this tooltip's own `label_key`, so a whole-page
        // `!contains("January")` assertion could never be true.
        let overridden = render_with(ChartTooltipProps {
            label_key: Some("Activities".to_string()),
            ..default_tooltip_props()
        });
        let fragment = tooltip_fragment(&overridden);
        assert!(fragment.contains("Activities"));
        assert!(!fragment.contains("January"));

        // A dedicated harness for the `label_format` half: `Callback::new`
        // panics ("must be called from inside a Dioxus runtime") when
        // called directly from a plain `#[test]` fn, before any
        // `VirtualDom` exists to provide one -- `use_signal`'s own lazy
        // initializer (this file's `FormatterHarness`/`IconHarness` use
        // the identical trick) defers construction to component-render
        // time, which does run with a runtime active.
        #[component]
        fn LayeredLabelHarness() -> Element {
            let config = use_signal(two_series_config);
            let data = use_signal(two_datum_data);
            let label_format = use_signal(|| Callback::new(|s: String| s.to_uppercase()));
            rsx! {
                ChartContainer { config, data, kind: ChartKind::Line,
                    Chart { aria_label: "Visitors by month" }
                    ActiveIndexSetter { index: Some(0) }
                    ChartTooltip {
                        label_key: "Activities".to_string(),
                        label_format: Some(label_format()),
                    }
                }
            }
        }
        let mut dom = VirtualDom::new(LayeredLabelHarness);
        dom.rebuild_in_place();
        dom.render_immediate(&mut NoOpMutations);
        let layered = dioxus_ssr::render(&dom);
        assert!(tooltip_fragment(&layered).contains("ACTIVITIES"));
    }

    #[test]
    fn name_key_overrides_every_rows_name() {
        // Scoped to `tooltip_fragment` for the same reason as the previous
        // test: `Chart`'s hidden table always shows "Desktop"/"Mobile" as
        // real column headers regardless of this tooltip's own `name_key`.
        let html = render_with(ChartTooltipProps {
            name_key: Some("Metric".to_string()),
            ..default_tooltip_props()
        });
        let fragment = tooltip_fragment(&html);
        // Twice per row (`TooltipRow::label`'s own doc: this crate reuses
        // the same overridden value for both the swatch's `aria-label` and
        // the visible `chart-tooltip-name` text) * 2 rows = 4.
        assert_eq!(fragment.matches("Metric").count(), 4);
        assert!(!fragment.contains("Desktop"));
        assert!(!fragment.contains("Mobile"));
    }

    #[test]
    fn formatter_fully_replaces_row_markup_and_carries_the_right_is_last_and_total() {
        // A dedicated harness (not `render_with`, which takes an
        // already-built `ChartTooltipProps`): `Callback::new` panics
        // ("must be called from inside a Dioxus runtime") when called
        // directly from a plain `#[test]` fn, before any `VirtualDom`
        // exists to provide one. `use_signal`'s own initializer closure
        // (called lazily at component-render time, same trick this file's
        // own `IconHarness`/`config_with_icon` below already uses for
        // `ChartSeries::icon`'s `Callback`) is what makes constructing the
        // `formatter` `Callback` here sound.
        #[component]
        fn FormatterHarness() -> Element {
            let config = use_signal(two_series_config);
            let data = use_signal(two_datum_data);
            let formatter = use_signal(|| {
                Callback::new(|row: TooltipRow| {
                    rsx! {
                        span { "data-slot": "custom-row", "data-key": "{row.key}", "data-last": "{row.is_last}", "data-total": "{row.total}" }
                    }
                })
            });
            rsx! {
                ChartContainer { config, data, kind: ChartKind::Line,
                    Chart { aria_label: "Visitors by month" }
                    ActiveIndexSetter { index: Some(0) }
                    ChartTooltip { formatter: Some(formatter()) }
                }
            }
        }
        let mut dom = VirtualDom::new(FormatterHarness);
        dom.rebuild_in_place();
        dom.render_immediate(&mut NoOpMutations);
        let html = dioxus_ssr::render(&dom);
        // January: desktop=186, mobile=80 -- default markup is gone...
        assert!(!html.contains(r#"data-slot="chart-swatch""#));
        assert!(!html.contains(r#"data-slot="chart-tooltip-name""#));
        // ...replaced by the formatter's own markup, once per series, the
        // row's own outer `chart-tooltip-item` container still present.
        assert_eq!(html.matches(r#"data-slot="chart-tooltip-item""#).count(), 2);
        assert_eq!(html.matches(r#"data-slot="custom-row""#).count(), 2);
        assert!(html.contains(r#"data-key="desktop""#));
        assert!(html.contains(r#"data-key="mobile""#));
        assert!(html.contains(r#"data-key="desktop" data-last="false""#));
        assert!(html.contains(r#"data-key="mobile" data-last="true""#));
        // 186 + 80 = 266.
        assert!(
            html.contains("data-total=\"266\""),
            "expected total 266: {html}"
        );
    }

    /// A series with an icon renders `data-slot="chart-icon"` in place of
    /// the swatch, and -- unlike the swatch -- `hide_indicator`/`indicator`
    /// have no effect on it (shadcn's own icon-always-wins rule).
    #[test]
    fn a_series_icon_replaces_the_swatch_and_ignores_hide_indicator() {
        fn config_with_icon() -> ChartConfig {
            let base = two_series_config();
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

        #[component]
        fn IconHarness(hide_indicator: bool) -> Element {
            let config = use_signal(config_with_icon);
            let data = use_signal(two_datum_data);
            rsx! {
                ChartContainer { config, data, kind: ChartKind::Line,
                    Chart { aria_label: "Visitors by month" }
                    ActiveIndexSetter { index: Some(0) }
                    ChartTooltip { hide_indicator }
                }
            }
        }

        for hide_indicator in [false, true] {
            let mut dom =
                VirtualDom::new_with_props(IconHarness, IconHarnessProps { hide_indicator });
            dom.rebuild_in_place();
            dom.render_immediate(&mut NoOpMutations);
            let html = dioxus_ssr::render(&dom);
            assert_eq!(
                html.matches(r#"data-slot="chart-icon""#).count(),
                1,
                "hide_indicator={hide_indicator}: {html}"
            );
            assert!(
                html.contains(r#"data-testid="icon""#),
                "hide_indicator={hide_indicator}: {html}"
            );
            // The OTHER series (mobile, no icon) still gets a swatch unless
            // `hide_indicator` is set.
            let swatch_count = if hide_indicator { 0 } else { 1 };
            assert_eq!(
                html.matches(r#"data-slot="chart-swatch""#).count(),
                swatch_count,
                "hide_indicator={hide_indicator}: {html}"
            );
        }
    }
}
