//! Defines the [`ChartContainer`] component.

use dioxus::prelude::*;

use crate::chart::config::ChartConfig;
use crate::chart::context::{ChartContext, ChartLayout};
use crate::chart::engine::{ChartDatum, ChartKind};
use crate::dioxus_attributes::attributes;
use crate::{merge_attributes, use_id_or, use_unique_id};

/// The props for the [`ChartContainer`] component.
#[derive(Props, Clone, PartialEq)]
pub struct ChartContainerProps {
    /// This chart instance's id, used to scope the generated
    /// `--color-<key>` style rule (`data-chart="<id>"`). Defaults to an
    /// internally generated unique id when not provided.
    pub id: ReadSignal<Option<String>>,

    /// The series list -- colors, labels, draw order. Every descendant
    /// (`Chart`, `ChartTooltip`, `ChartLegend`) reads this same value via
    /// [`crate::chart::use_chart`].
    pub config: ReadSignal<ChartConfig>,

    /// The data points, one per x-axis category.
    pub data: ReadSignal<Vec<ChartDatum>>,

    /// Which mark family the contained `Chart` draws. Also rendered as
    /// this element's own `data-kind` attribute, for a themed wrapper to
    /// style by.
    pub kind: ReadSignal<ChartKind>,

    /// Additional attributes to apply to the container element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the chart container -- typically a `Chart`,
    /// optionally alongside a `ChartTooltip` and/or `ChartLegend`.
    pub children: Element,
}

/// # ChartContainer
///
/// The root of a chart: provides [`ChartContainerProps::config`]/
/// [`ChartContainerProps::data`]/[`ChartContainerProps::kind`] (plus the
/// hover/keyboard `active_index` state it owns itself) to its descendants
/// via [`crate::chart::use_chart`], and emits the `--color-<key>` CSS
/// custom property for each configured series, scoped to this instance by
/// a generated `data-chart` id -- shadcn's `ChartContainer`/`ChartStyle`
/// mechanism (`dev-docs/research/chart-2026-09-19.md` §1.1, §6.3), ported
/// as a CSS-variable technique rather than its literal React shape (the
/// module doc explains why the rest of the API differs from shadcn's).
///
/// This must contain a `Chart` (or any other consumer of
/// [`crate::chart::use_chart`]) to be useful on its own.
///
/// ## Example
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::chart::{ChartConfig, ChartContainer, ChartDatum, ChartKind};
///
/// #[component]
/// fn Demo() -> Element {
///     let config = use_signal(|| {
///         ChartConfig::new()
///             .series("desktop", "Desktop", "var(--dx-chart-1)")
///             .series("mobile", "Mobile", "var(--dx-chart-2)")
///     });
///     let data = use_signal(|| {
///         vec![ChartDatum { label: "January".to_string(), values: vec![Some(186.0), Some(80.0)], ..Default::default() }]
///     });
///
///     rsx! {
///         ChartContainer { config, data, kind: ChartKind::Bar,
///             // A `Chart` (and optionally `ChartTooltip`/`ChartLegend`) goes here.
///         }
///     }
/// }
/// ```
///
/// ## Styling
///
/// The [`ChartContainer`] component defines the following data attributes
/// you can use to control styling:
/// - `data-chart`: this chart instance's generated or caller-provided id.
/// - `data-kind`: the chart's [`ChartKind`], as `area`, `bar`, or `line`.
#[component]
pub fn ChartContainer(props: ChartContainerProps) -> Element {
    let id = use_id_or(use_unique_id(), props.id);
    let active_index = use_signal(|| None::<usize>);
    let layout = use_signal(|| None::<ChartLayout>);

    use_context_provider(|| ChartContext {
        id,
        config: props.config,
        data: props.data,
        active_index,
        kind: props.kind,
        layout,
    });

    let kind_str = use_memo(move || (props.kind)().as_str());

    // One `[data-chart="<id>"] { --color-<key>: <color>; ... }` rule --
    // shadcn's `ChartStyle`, minus its light/dark theme-selector strings
    // (this crate's own tokens already fold light/dark into one
    // declaration via `var(--dark, X) var(--light, Y)`, so a series'
    // `color` needs no per-theme duplication here).
    let style_rule = use_memo(move || {
        let id = id();
        let config = (props.config)();
        let mut rule = format!("[data-chart=\"{id}\"]{{");
        for series in &config.series {
            rule.push_str(&format!("--color-{}:{};", series.slot(), series.color));
        }
        rule.push('}');
        rule
    });

    // `data-chart`/`data-kind` are structural wiring, not overridable
    // presentation: `data-chart` scopes the `--color-<key>` style rule this
    // very component just built, and `data-kind` is what a themed
    // wrapper's own CSS switches its whole ruleset on -- either being
    // silently dropped by a caller's same-named attribute would break the
    // component, not just its styling. `merge_attributes` (not a raw
    // `..props.attributes` beside a literal, `scripts/
    // check-attr-spread-collision.sh`'s own fix) makes that precedence
    // explicit and SSR/CSR-consistent instead of accidental: owned wins,
    // listed last.
    let owned = attributes!(div {
        "data-chart": id,
        "data-kind": kind_str,
    });
    let merged = merge_attributes(vec![props.attributes, owned]);

    rsx! {
        div {
            ..merged,

            style { "{style_rule}" }
            {props.children}
        }
    }
}
