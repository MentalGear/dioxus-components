//! Defines the [`ChartContainer`] component.

use dioxus::core::DynamicNode;
use dioxus::prelude::*;

use crate::chart::config::ChartConfig;
use crate::chart::context::{ChartContext, ChartLayout, Cursor, PointerGate};
use crate::chart::engine::{ChartDatum, ChartKind};
use crate::dioxus_attributes::attributes;
use crate::{merge_attributes, use_id_or, use_unique_id};

/// The props for the [`ChartContainer`] component.
#[derive(Props, Clone, PartialEq)]
pub struct ChartContainerProps {
    /// This chart instance's id, rendered as the container's
    /// `data-chart="<id>"` attribute (the per-instance styling hook).
    /// Defaults to an internally generated unique id when not provided.
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
/// via [`crate::chart::use_chart`], and defines the `--color-<key>` CSS
/// custom property for each configured series as an inline `style`
/// attribute on its own element (so it is scoped to this subtree by CSS
/// inheritance) -- shadcn's `ChartContainer`/`ChartStyle` mechanism
/// (`dev-docs/research/chart-2026-09-19.md` §1.1, §6.3), ported as a
/// CSS-variable technique rather than its literal React shape (the module
/// doc explains why the rest of the API differs from shadcn's). It is an
/// attribute and not a generated `<style>` rule because a dynamic text
/// node inside a `<style>` element gets an SSR hydration marker that
/// corrupts the CSS. A caller's own `style` is kept, after the generated
/// declarations.
///
/// Because the series colors are inline declarations, a stylesheet rule on
/// `--color-<key>` loses to them (it would need `!important`): to recolor a
/// series, set the series' `color` in the [`ChartConfig`], or override the
/// variable through the caller `style` (e.g. `style: "--color-desktop: red"`).
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
/// - `style`: carries the generated `--color-<slot>` declarations (plus any
///   caller `style`).
/// - `data-kind`: the chart's [`ChartKind`], as `area`, `bar`, `line`, `pie`,
///   `radar`, or `radial-bar`.
#[component]
pub fn ChartContainer(props: ChartContainerProps) -> Element {
    let id = use_id_or(use_unique_id(), props.id);
    let active_index = use_signal(|| None::<usize>);
    let layout = use_signal(|| None::<ChartLayout>);
    let cursor = use_signal(Cursor::default);
    let box_size = use_signal(|| None::<(f64, f64)>);
    let tip_size = use_signal(|| None::<(f64, f64)>);
    let gate = use_hook(|| CopyValue::new(PointerGate::default()));
    let legend = use_signal(|| None::<f64>);
    // Does this chart have a legend? Read off the children themselves (a
    // `ChartLegend` component among them, at any `if`/`for` depth), so the
    // server render and the first client render agree and `Chart` can
    // reserve the legend's room before anything is measured -- the chart
    // box is then the same size before and after hydration. Written here,
    // before any child renders, and only when it changes.
    let mut has_legend = use_signal(|| false);
    let legend_present = props.children.as_ref().is_ok_and(contains_legend);
    if *has_legend.peek() != legend_present {
        has_legend.set(legend_present);
    }

    use_context_provider(|| ChartContext {
        id,
        config: props.config,
        data: props.data,
        active_index,
        kind: props.kind,
        layout,
        cursor,
        box_size,
        tip_size,
        gate,
        legend,
        has_legend,
    });

    let kind_str = use_memo(move || (props.kind)().as_str());

    // The `--color-<slot>: <color>;` declarations -- shadcn's `ChartStyle`,
    // minus its light/dark theme-selector strings (this crate's own tokens
    // already fold light/dark into one declaration via
    // `var(--dark, X) var(--light, Y)`, so a series' `color` needs no
    // per-theme duplication here) -- are written as an inline `style`
    // attribute on the container `div`, NOT as a `<style>` element.
    //
    // Why not a `<style>` rule: a dynamic text node inside a raw-text
    // `<style>` gets a hydration marker comment from Dioxus SSR
    // (`<style><!--node-id444-->[data-chart=...]{...}<!--#--></style>`);
    // inside `<style>` that `<!--` is a CSS CDO token and the rest parses
    // as the selector `node-id444-- > [data-chart=...]`, which never
    // matches, so every `--color-<slot>` stayed undefined on the deployed
    // SSG pages (marks fell back to black fill / no stroke). An attribute
    // value has no such marker, so the class of bug cannot occur
    // (`scripts/check-raw-text-interpolation.sh` guards the whole class).
    //
    // Safety: `series.slot()` is sanitized to `[a-z0-9_-]`, so a series key
    // cannot inject a property name. `series.color` is caller-supplied
    // CSS, trusted like any other `style` the caller writes: Dioxus escapes
    // `"`/`<`/`&` in the attribute value, so it can never leave the
    // attribute; a stray `;` merely adds declarations to this same
    // container element, which the caller already owns.
    let (caller_style, rest) = crate::fold_style_attributes(props.attributes);
    let mut style = String::new();
    for series in &(props.config)().series {
        style.push_str(&format!("--color-{}:{};", series.slot(), series.color));
    }
    // Chart colors first, so a caller's own `style` can still override.
    if let Some(caller_style) = caller_style {
        style.push_str(&caller_style);
    }

    // `data-chart`/`data-kind` are structural wiring, not overridable
    // presentation: `data-chart` is the per-instance id hook, and
    // `data-kind` is what a themed wrapper's own CSS switches its whole
    // ruleset on -- either being silently dropped by a caller's same-named
    // attribute would break the component, not just its styling.
    // `merge_attributes` (not a raw `..props.attributes` beside a literal,
    // `scripts/check-attr-spread-collision.sh`'s own fix) makes that
    // precedence explicit and SSR/CSR-consistent instead of accidental:
    // owned wins, listed last. The caller's own `style` was already folded
    // into `style` above (`fold_style_attributes`), so exactly one `style`
    // attribute is ever built.
    let owned = attributes!(div {
        "data-chart": id,
        "data-kind": kind_str,
        style: style,
    });
    let merged = merge_attributes(vec![rest, owned]);

    rsx! {
        div {
            ..merged,
            {props.children}
        }
    }
}

/// Whether `node` renders a `ChartLegend` (this crate's or a themed
/// wrapper of the same name) among its dynamic children, looking through
/// `if`/`for` fragments. Components are always dynamic nodes, so the
/// template's static part never needs a look.
fn contains_legend(node: &VNode) -> bool {
    node.dynamic_nodes.iter().any(|dynamic| match dynamic {
        // The name as written at the call site, path included
        // (`chart::ChartLegend`): compare its last segment.
        DynamicNode::Component(component) => {
            component.name.rsplit("::").next().map(str::trim) == Some("ChartLegend")
        }
        DynamicNode::Fragment(nodes) => nodes.iter().any(contains_legend),
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart::ChartConfig;
    use crate::test_support::find_element;
    use dioxus_core::NoOpMutations;

    #[component]
    fn Harness(caller_style: bool) -> Element {
        let config = use_signal(|| {
            ChartConfig::new()
                .series("desktop", "Desktop", "var(--dx-chart-1)")
                .series("mobile", "Mobile", "var(--dx-chart-2)")
        });
        let data = use_signal(Vec::<ChartDatum>::new);
        rsx! {
            if caller_style {
                ChartContainer {
                    config,
                    data,
                    kind: ChartKind::Bar,
                    style: "min-height: 4rem",
                    padding: "1rem",
                    span { "child" }
                }
            } else {
                ChartContainer { config, data, kind: ChartKind::Bar, span { "child" } }
            }
        }
    }

    /// Render in pre-render (SSG/hydration) mode -- the mode that emits
    /// `<!--node-id..-->` markers before dynamic text, which is what
    /// corrupted the old `<style>` rule.
    fn render_prerender(caller_style: bool) -> String {
        let mut dom = VirtualDom::new_with_props(Harness, HarnessProps { caller_style });
        dom.rebuild_in_place();
        dom.render_immediate(&mut NoOpMutations);
        let mut renderer = dioxus_ssr::Renderer::new();
        renderer.pre_render = true;
        renderer.render(&dom)
    }

    #[test]
    fn colors_are_inline_style_not_a_style_element() {
        let html = render_prerender(false);
        assert!(!html.contains("<style"), "no <style> element: {html}");
        let container = find_element(&html, |a| {
            a.get("data-kind").map(String::as_str) == Some("bar")
        })
        .expect("container div");
        let style = container.attrs.get("style").expect("style attribute");
        assert!(
            style.contains("--color-desktop:var(--dx-chart-1)"),
            "{style}"
        );
        assert!(
            style.contains("--color-mobile:var(--dx-chart-2)"),
            "{style}"
        );
        assert!(container.attrs.contains_key("data-chart"));
    }

    #[test]
    fn caller_style_is_preserved_alongside_colors() {
        let html = render_prerender(true);
        assert!(!html.contains("<style"), "no <style> element: {html}");
        let container = find_element(&html, |a| {
            a.get("data-kind").map(String::as_str) == Some("bar")
        })
        .expect("container div");
        assert_eq!(html.matches("style=").count(), 1, "exactly one style attr");
        let style = container.attrs.get("style").expect("style attribute");
        assert!(
            style.contains("--color-desktop:var(--dx-chart-1)"),
            "{style}"
        );
        assert!(style.contains("min-height: 4rem"), "{style}");
        assert!(style.contains("padding:1rem"), "{style}");
    }
}
