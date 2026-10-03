//! Defines the [`ChartTooltip`] component.

use dioxus::prelude::*;

use crate::chart::config::ChartIcon;
use crate::chart::context::{use_chart, Cursor, Follow};
use crate::chart::engine::placement::{place_tooltip, TooltipPlacement};
use crate::chart::engine::scale::fmt_decimal;
use crate::chart::ChartDatum;
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
/// A tooltip that follows the pointer the way shadcn/Recharts tooltips do,
/// and tracks [`crate::chart::use_chart`]'s `active_index` -- shadcn's
/// `ChartTooltip`/`ChartTooltipContent`
/// (`dev-docs/research/chart-2026-09-19.md` §1.1), ported as a plain
/// sibling of [`crate::chart::Chart`] rather than a Recharts render-prop
/// consumer (the module doc explains why the rest of this API differs from
/// shadcn's own children-introspection shape).
///
/// ## Position
///
/// A Cartesian chart snaps the tooltip's x to the nearest category's point
/// and follows the pointer's y (a horizontal bar chart swaps the axes); the
/// polar families follow the pointer on both axes, except a single-ring pie,
/// which anchors at the hovered slice's centroid. The box sits 10px right
/// of and below that point, **flips** to the left/above when it would
/// overflow the chart's right/bottom edge, and is **clamped** into the chart
/// box (`engine::placement::place_tooltip`). With no pointer driving
/// (keyboard focus) it hangs off the active data point instead, by the same
/// rules. It is positioned with a `transform` on a box pinned to the chart's
/// top-left, so a stylesheet can animate it; the side it ended up on is
/// published as `data-side-x`/`data-side-y`.
///
/// The pointer position comes from [`crate::chart::Chart`]'s wrapper (client
/// coordinates minus its bounding rect, read at event time), the chart box
/// size from the same rect and `onresize`, and the tooltip's own size from
/// its own `onresize` -- the only measurements involved, all client-side, so
/// the server renders the same closed tooltip every time and hydration never
/// has to reconcile a position. Always rendered, even when closed
/// (`data-state="closed"`, which a themed stylesheet hides with CSS) -- so
/// hydration never has to create or remove this element, only flip
/// attributes.
///
/// A pie slice's (or unstacked radial ring's) tooltip row is the slice (its
/// datum's name, value and color), not the chart's one series, matching
/// shadcn.
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
///   `data-indicator="dot"|"line"|"dashed"|"none"` ([`ChartTooltipProps::indicator`]),
///   `data-side-x="left"|"right"` and `data-side-y="top"|"bottom"` (which
///   side of its anchor the box is on; `left`/`top` mean it flipped),
///   `data-placed="true"|"false"` (`false` while an open tooltip is still
///   waiting for its own size or the chart's -- a stylesheet hides it then,
///   which includes the server render of a chart with a
///   [`crate::chart::ChartProps::default_index`]) and
///   `data-motion="true"|"false"` (whether a position change may animate:
///   `false` for the jump that first shows it).
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
    let cursor = (ctx.cursor)();
    let box_size = (ctx.box_size)();
    let mut tip_size = ctx.tip_size;
    // Whether the tooltip's own size has been measured since it last became
    // visible (a closed `display: none` box reports 0x0). Until it has, the
    // box is laid out but hidden (`data-placed="false"`), so the first
    // visible frame already has its final flip/clamp -- never an estimate
    // that is corrected (and animated) a frame later.
    let mut placed = use_signal(|| false);
    // Counts consecutive renders this tooltip has been open AND placed. The
    // first (the jump from "hidden at an estimate" to "visible at the
    // target") must not animate; only a later one -- a pointer or key move
    // -- may. See the stylesheet's `data-motion` rule.
    let mut placed_renders = use_hook(|| CopyValue::new(0u32));

    // An active index is all it takes -- on the server that is never the
    // case, so the SSR tooltip is always closed.
    let open = active.is_some();
    // Placed needs both measurements: its own size (`placed`) AND the chart
    // box (`box_size`, from the chart's `onresize`). With only the first, a
    // tooltip that is open from the start (`default_index`) could show for a
    // frame at the neutral top-left and then slide to its anchor.
    let is_placed = open && placed() && box_size.is_some();
    let motion = {
        let n = if is_placed {
            *placed_renders.peek() + 1
        } else {
            0
        };
        *placed_renders.write() = n;
        n >= 2
    };
    let state = if open { "open" } else { "closed" };
    let indicator_str = props.indicator.as_str();

    // The active index's own `(left%, top%)` anchor, computed by whichever
    // family rendered (§4(d) of the stage-2 handoff -- `ChartLayout`'s own
    // doc) -- never read when `data-state="closed"` (a themed stylesheet
    // hides it), so an arbitrary fallback is fine whenever either half of
    // this is still unset (no layout yet, or this family hasn't populated
    // an anchor for the active index).
    let anchor_pct = active
        .and_then(|i| layout.as_ref().and_then(|l| l.anchor_percent.get(i)))
        .copied()
        .unwrap_or((50.0, 50.0));
    let follow = layout.as_ref().map_or(Follow::Anchor, |l| l.follow);
    let slice_rows = layout.as_ref().is_some_and(|l| l.slice_rows);
    // Unmeasured (the server, the first client render): the neutral
    // top-left placement, so SSR and the first client render agree.
    let placement = box_size.map_or(
        TooltipPlacement {
            x: 0.0,
            y: 0.0,
            flipped_x: false,
            flipped_y: false,
        },
        |bounds| {
            place_tooltip(
                tooltip_anchor(anchor_pct, follow, cursor, bounds),
                tip_size().unwrap_or(ESTIMATED_TIP_SIZE),
                bounds,
            )
        },
    );
    let side_x = if placement.flipped_x { "left" } else { "right" };
    let side_y = if placement.flipped_y { "top" } else { "bottom" };

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
        "transform:translate({}px,{}px)",
        fmt_decimal(placement.x, 1),
        fmt_decimal(placement.y, 1)
    );
    let style = match caller_style {
        Some(extra) => format!("{position_style};{extra}"),
        None => position_style,
    };
    let owned = attributes!(div {
        "data-slot": "chart-tooltip",
        "data-state": state,
        "data-indicator": indicator_str,
        "data-placed": if is_placed { "true" } else { "false" },
        "data-motion": if motion { "true" } else { "false" },
        "data-side-x": side_x,
        "data-side-y": side_y,
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
    let rows = tooltip_rows(
        &config.series,
        slice_rows,
        active,
        active_datum,
        props.name_key.as_deref(),
    );
    let last_index = rows.len().saturating_sub(1);

    rsx! {
        div {
            // Its own border-box size, for the flip/clamp decision above. A
            // closed (`display: none`) tooltip reports 0x0: un-placed again.
            onresize: move |evt: ResizeEvent| {
                if let Ok(size) = evt.data().get_border_box_size() {
                    let next = (size.width.ceil(), size.height.ceil());
                    if next.0 > 0.0 && next.1 > 0.0 {
                        if tip_size() != Some(next) {
                            tip_size.set(Some(next));
                        }
                        if !placed() {
                            placed.set(true);
                        }
                    } else if placed() {
                        placed.set(false);
                    }
                }
            },
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
                for (s , row) in rows.iter().enumerate() {
                    {
                        let value = row.value;
                        let label = row.label.clone();
                        let color = row.color.clone();

                        if let Some(formatter) = &props.formatter {
                            let tooltip_row = TooltipRow {
                                key: row.key.clone(),
                                label,
                                value,
                                color: color.clone(),
                                index: s,
                                is_last: s == last_index,
                                total,
                            };
                            rsx! {
                                div {
                                    key: "{row.key}",
                                    "data-slot": "chart-tooltip-item",
                                    "data-series": "{row.slot}",
                                    style: "--series-color: {color}",
                                    {formatter.call(tooltip_row)}
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
                                    key: "{row.key}",
                                    "data-slot": "chart-tooltip-item",
                                    "data-series": "{row.slot}",
                                    style: "--series-color: {color}",

                                    if let Some(ChartIcon(icon)) = row.icon {
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

/// The tooltip's assumed size (CSS px) until its own `onresize` has
/// reported one: roughly a label plus one row at the stylesheet's
/// `min-width` (shadcn's 128px tooltip, padding included: 128 x 48 for a
/// label and one row). Only the flip/clamp decision on the very first open
/// uses it.
const ESTIMATED_TIP_SIZE: (f64, f64) = (128.0, 48.0);

/// The point the tooltip hangs off, in CSS px from the chart box's
/// top-left: the active datum's own anchor (`anchor_pct`, percent of the
/// box), with the axes the family's [`Follow`] rule hands to the pointer
/// replaced by the pointer's own coordinate -- but only while a measured
/// pointer is driving. Anything else (keyboard, nothing measured yet)
/// uses the data point itself.
fn tooltip_anchor(
    anchor_pct: (f64, f64),
    follow: Follow,
    cursor: Cursor,
    bounds: (f64, f64),
) -> (f64, f64) {
    let ax = anchor_pct.0 / 100.0 * bounds.0;
    let ay = anchor_pct.1 / 100.0 * bounds.1;
    match (follow, cursor) {
        (Follow::PointerY, Cursor::At(_, y)) => (ax, y),
        (Follow::PointerX, Cursor::At(x, _)) => (x, ay),
        (Follow::Pointer, Cursor::At(x, y)) => (x, y),
        _ => (ax, ay),
    }
}

/// One rendered tooltip row, resolved up front so the default and the
/// `formatter` paths read the same data.
struct RowSpec {
    /// Stable `key` (the series' own key, or a pie slice's name).
    key: String,
    /// The `data-series` slot.
    slot: String,
    /// The displayed name.
    label: String,
    value: Option<f64>,
    /// The `--series-color` the row's swatch paints.
    color: String,
    icon: Option<ChartIcon>,
}

/// The rows to show for the active datum: one per configured series --
/// except when `slice_rows` (a single-ring pie, an unstacked radial chart),
/// where the one "series" only names the measure (and would label every
/// slice "Visitors"): there the row IS the hovered slice/ring, its datum's
/// name and value in its own color (shadcn's pie/radial tooltip). A stacked,
/// multi-ring pie keeps per-series rows (one value per ring at the datum).
fn tooltip_rows(
    series: &[crate::chart::ChartSeries],
    slice_rows: bool,
    active: Option<usize>,
    datum: Option<&ChartDatum>,
    name_key: Option<&str>,
) -> Vec<RowSpec> {
    if slice_rows {
        let (Some(i), Some(d)) = (active, datum) else {
            return Vec::new();
        };
        return vec![RowSpec {
            key: d.label.clone(),
            slot: series
                .first()
                .map_or_else(|| "value".to_string(), |s| s.slot()),
            label: name_key.map_or_else(|| d.label.clone(), str::to_string),
            value: d.values.first().copied().flatten(),
            color: super::series::pie::slice_color(Some(d), i),
            icon: None,
        }];
    }
    series
        .iter()
        .enumerate()
        .map(|(s, series)| RowSpec {
            key: series.key.clone(),
            slot: series.slot(),
            label: name_key.map_or_else(|| series.label.clone(), str::to_string),
            value: datum.and_then(|d| d.values.get(s).copied().flatten()),
            color: format!("var(--color-{})", series.slot()),
            icon: series.icon,
        })
        .collect()
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
    use crate::test_support::{find_element, find_elements};
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
        // Pinned to the box's top-left with a transform, even closed: the
        // server renders one deterministic position and hydration only ever
        // flips attributes (no position is measured on the server).
        assert!(
            html.contains("transform:translate(0px,0px)"),
            "always positioned, even closed: {html}"
        );
        assert!(html.contains(r#"data-side-x="right""#));
        assert!(html.contains(r#"data-side-y="bottom""#));
    }

    #[test]
    fn an_open_tooltip_stays_hidden_until_it_and_the_chart_are_measured() {
        // The server render of an open tooltip (a `default_index`): open, but
        // unmeasured, so `data-placed="false"` (the stylesheet hides it) and
        // the neutral position -- no guessed placement to correct later.
        let html = render(Some(0));
        assert!(html.contains(r#"data-state="open""#), "{html}");
        assert!(html.contains(r#"data-placed="false""#), "{html}");
        assert!(html.contains("transform:translate(0px,0px)"), "{html}");
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

    /// Reads the shared `ChartLayout` the way `ChartTooltip` does and prints
    /// the active index's anchor, so a test can pin `Chart`'s own anchor
    /// arithmetic (the tooltip itself now turns it into a pixel `transform`
    /// with a measured box, which a server render has none of).
    #[component]
    fn AnchorProbe(index: usize) -> Element {
        let ctx = use_chart();
        let (x, y) = (ctx.layout)()
            .and_then(|l| l.anchor_percent.get(index).copied())
            .unwrap_or((f64::NAN, f64::NAN));
        rsx! {
            span { "data-probe": "anchor", "data-x": "{x}", "data-y": "{y}" }
        }
    }

    fn probe_value(html: &str, name: &str) -> f64 {
        // Search from the probe itself: the chart's hit bands carry a
        // `data-x` of their own.
        let html = &html[html.find(r#"data-probe="anchor""#).expect("probe")..];
        let key = format!(r#"{name}=""#);
        let start = html.find(&key).expect("probe attr") + key.len();
        let end = html[start..].find('"').expect("closing quote") + start;
        html[start..end].parse().expect("a number")
    }

    /// A single series/single datum dataset makes the percent anchor
    /// hand-derivable exactly: one band always centers at the exact
    /// midpoint of the plot range regardless of padding, and one value
    /// equal to the (nice-rounded) domain's own max scales to the exact
    /// top of the plot. Cross-checks `Chart`'s anchor arithmetic against its
    /// default initial size and Recharts margin.
    #[test]
    fn open_anchor_is_the_exact_band_center_and_top_value_percent() {
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
                    AnchorProbe { index: 0 }
                    ChartTooltip {}
                }
            }
        }
        let mut dom =
            VirtualDom::new_with_props(OneSeriesHarness, OneSeriesHarnessProps { active: Some(0) });
        dom.rebuild_in_place();
        dom.render_immediate(&mut NoOpMutations);
        let html = dioxus_ssr::render(&dom);
        // x: a lone point sits at the plot's own midpoint ((5 + 364) / 2 =
        // 184.5), 50% of the initial 369px width.
        assert_eq!(probe_value(&html, "data-x"), 50.0);
        // y: the nice ticks of (0, 100) are 0, 25, ..., 100, so the value 100
        // scales to exactly the plot's top (the 5px default margin of a
        // 369 x 208 chart).
        let height = (369.0f64 * 9.0 / 16.0).round();
        assert!((probe_value(&html, "data-y") - 5.0 / height * 100.0).abs() < 1e-9);
    }

    #[test]
    fn a_pointer_drives_only_the_axes_the_follow_rule_gives_it() {
        let bounds = (400.0, 200.0);
        let anchor = (25.0, 50.0); // (100px, 100px) in the box
        let at = Cursor::At(300.0, 30.0);
        // Vertical category charts: x snaps to the datum, y follows.
        assert_eq!(
            tooltip_anchor(anchor, Follow::PointerY, at, bounds),
            (100.0, 30.0)
        );
        // Horizontal bars: the other way round.
        assert_eq!(
            tooltip_anchor(anchor, Follow::PointerX, at, bounds),
            (300.0, 100.0)
        );
        // Polar: the pointer, both axes.
        assert_eq!(
            tooltip_anchor(anchor, Follow::Pointer, at, bounds),
            (300.0, 30.0)
        );
        // A pie centroid ignores the pointer.
        assert_eq!(
            tooltip_anchor(anchor, Follow::Anchor, at, bounds),
            (100.0, 100.0)
        );
    }

    #[test]
    fn without_a_pointer_every_rule_uses_the_data_point() {
        let bounds = (400.0, 200.0);
        let anchor = (25.0, 50.0);
        for follow in [
            Follow::Anchor,
            Follow::PointerY,
            Follow::PointerX,
            Follow::Pointer,
        ] {
            assert_eq!(
                tooltip_anchor(anchor, follow, Cursor::None, bounds),
                (100.0, 100.0),
                "{follow:?}"
            );
        }
    }

    #[test]
    fn slice_rows_show_the_datum_not_the_series() {
        let series = vec![crate::chart::ChartSeries {
            key: "visitors".into(),
            label: "Visitors".into(),
            color: "red".into(),
            icon: None,
        }];
        let datum = ChartDatum {
            label: "Chrome".into(),
            values: vec![Some(275.0)],
            color: Some("var(--dx-chart-3)".into()),
        };
        let rows = tooltip_rows(&series, true, Some(2), Some(&datum), None);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].label, "Chrome");
        assert_eq!(rows[0].value, Some(275.0));
        assert_eq!(rows[0].color, "var(--dx-chart-3)");
        assert_eq!(rows[0].slot, "visitors");
        // No datum color: the positional palette token, like the slice.
        let plain = ChartDatum {
            color: None,
            ..datum.clone()
        };
        let rows = tooltip_rows(&series, true, Some(2), Some(&plain), None);
        assert_eq!(rows[0].color, "var(--dx-chart-3)");
        // Closed: no row to show.
        assert!(tooltip_rows(&series, true, None, None, None).is_empty());
        // `name_key` still wins, like for every other kind.
        let rows = tooltip_rows(&series, true, Some(0), Some(&datum), Some("Share"));
        assert_eq!(rows[0].label, "Share");
    }

    #[test]
    fn a_pie_tooltip_renders_the_slice_name_value_and_color() {
        #[component]
        fn PieHarness() -> Element {
            let config = use_signal(|| ChartConfig::new().series("visitors", "Visitors", "red"));
            let data = use_signal(|| {
                vec![
                    ChartDatum {
                        label: "Chrome".into(),
                        values: vec![Some(275.0)],
                        color: Some("var(--dx-chart-1)".into()),
                    },
                    ChartDatum {
                        label: "Safari".into(),
                        values: vec![Some(200.0)],
                        color: Some("var(--dx-chart-2)".into()),
                    },
                ]
            });
            rsx! {
                ChartContainer { config, data, kind: ChartKind::Pie,
                    Chart { aria_label: "Browsers" }
                    ActiveIndexSetter { index: Some(1) }
                    ChartTooltip { hide_label: true }
                }
            }
        }
        let mut dom = VirtualDom::new(PieHarness);
        dom.rebuild_in_place();
        dom.render_immediate(&mut NoOpMutations);
        let html = dioxus_ssr::render(&dom);
        let tip = tooltip_fragment(&html);
        assert!(tip.contains(r#"data-state="open""#), "{tip}");
        assert!(tip.contains(">Safari<"), "slice name: {tip}");
        assert!(tip.contains(">200<"), "slice value: {tip}");
        assert!(
            tip.contains("--series-color: var(--dx-chart-2)"),
            "slice color: {tip}"
        );
        assert!(!tip.contains(">Visitors<"), "not the series label: {tip}");
        assert_eq!(tip.matches(r#"data-slot="chart-tooltip-item""#).count(), 1);
    }

    #[test]
    fn without_slice_rows_every_series_keeps_its_row() {
        let series = vec![
            crate::chart::ChartSeries {
                key: "desktop".into(),
                label: "Desktop".into(),
                color: "red".into(),
                icon: None,
            },
            crate::chart::ChartSeries {
                key: "mobile".into(),
                label: "Mobile".into(),
                color: "blue".into(),
                icon: None,
            },
        ];
        let datum = ChartDatum {
            label: "January".into(),
            values: vec![Some(1.0), None],
            color: None,
        };
        let rows = tooltip_rows(&series, false, Some(0), Some(&datum), None);
        let labels: Vec<_> = rows.iter().map(|r| r.label.as_str()).collect();
        assert_eq!(labels, ["Desktop", "Mobile"]);
        assert_eq!(rows[0].color, "var(--color-desktop)");
        assert_eq!(rows[1].value, None);
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
    /// Backlog row 108: located via [`find_element`] (an order-independent
    /// attribute-map lookup), not by slicing the raw HTML string from the
    /// `data-slot="chart-tooltip"` match's own position -- the root
    /// `<div>`'s attributes go through `merge_attributes` (§4(a) of the
    /// stage-2 handoff), which sorts them by name, so `data-slot` is very
    /// often NOT the tag's first attribute (e.g. `aria-hidden`/
    /// `data-indicator` both sort before it), and slicing from the match
    /// itself used to silently drop every attribute that sorts earlier --
    /// found via a root-tag assertion that failed despite the merged
    /// `Vec<Attribute>` being verified correct immediately before the
    /// `rsx!` spread; the dropped attributes were real, just outside this
    /// fn's own (buggy) slice.
    fn tooltip_fragment(html: &str) -> &str {
        let root = find_element(html, |attrs| {
            attrs.get("data-slot").map(String::as_str) == Some("chart-tooltip")
        })
        .expect("no chart-tooltip root in html");
        &html[root.start..]
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
        // presence/value is what matters, not their relative order, so
        // this looks each one up in an order-independent attribute map
        // (backlog row 108) rather than slicing on a match's position or
        // concatenating two attributes and asserting they're adjacent.
        let root = find_element(&html, |attrs| {
            attrs.get("data-slot").map(String::as_str) == Some("chart-tooltip")
        })
        .expect("no chart-tooltip root in html");
        assert_eq!(
            root.attrs.get("data-state").map(String::as_str),
            Some("open")
        );
        assert_eq!(
            root.attrs.get("data-indicator").map(String::as_str),
            Some("line")
        );

        let swatches = find_elements(&html, |attrs| {
            attrs.get("data-slot").map(String::as_str) == Some("chart-swatch")
        });
        assert_eq!(swatches.len(), 2);
        assert!(swatches
            .iter()
            .all(|el| el.attrs.get("data-indicator").map(String::as_str) == Some("line")));
    }

    #[test]
    fn indicator_dashed_is_reflected_on_every_swatch() {
        let html = render_with(ChartTooltipProps {
            indicator: TooltipIndicator::Dashed,
            ..default_tooltip_props()
        });
        // Backlog row 108: looked up order-independently, not via a
        // concatenated two-attribute substring match.
        let swatches = find_elements(&html, |attrs| {
            attrs.get("data-slot").map(String::as_str) == Some("chart-swatch")
        });
        assert_eq!(swatches.len(), 2);
        assert!(swatches
            .iter()
            .all(|el| el.attrs.get("data-indicator").map(String::as_str) == Some("dashed")));
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
        // Backlog row 108: looked up order-independently, not via a
        // concatenated two-attribute substring match.
        let swatches = find_elements(&html, |attrs| {
            attrs.get("data-slot").map(String::as_str) == Some("chart-swatch")
        });
        assert_eq!(swatches.len(), 2);
        assert!(swatches
            .iter()
            .all(|el| el.attrs.get("data-indicator").map(String::as_str) == Some("dot")));
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
        // Backlog row 108: looked up order-independently rather than
        // asserting `data-key`/`data-last` are adjacent, in that order, in
        // the raw string.
        let desktop_row = find_element(&html, |attrs| {
            attrs.get("data-key").map(String::as_str) == Some("desktop")
        })
        .expect("desktop row");
        assert_eq!(
            desktop_row.attrs.get("data-last").map(String::as_str),
            Some("false")
        );
        let mobile_row = find_element(&html, |attrs| {
            attrs.get("data-key").map(String::as_str) == Some("mobile")
        })
        .expect("mobile row");
        assert_eq!(
            mobile_row.attrs.get("data-last").map(String::as_str),
            Some("true")
        );
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
