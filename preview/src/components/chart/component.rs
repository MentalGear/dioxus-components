use dioxus::prelude::*;
use dioxus_primitives::chart::{self, ChartContainerProps, ChartLegendProps, ChartProps, ChartTooltipProps};
use dioxus_primitives::dioxus_attributes::attributes;
use dioxus_primitives::merge_attributes;

// Plain, render-nothing value/data types a caller configures a chart with
// (dev-docs/preview-composition.md's allowlist rationale: "plain value
// types/enums that render nothing themselves have no theme for a wrapper to
// attach"). Re-exported here -- rather than left for demo code to import
// raw from `dioxus_primitives::chart` -- so every demo composes exclusively
// through `crate::components::chart::*`, never a raw `dioxus_primitives::`
// path outside this file (`scripts/check-preview-composition.sh`).
pub use dioxus_primitives::chart::{ChartConfig, ChartDatum, ChartKind, LegendAlign};

/// The themed chart container: scopes the `--color-<key>` CSS variables
/// generated from `config` to this instance via `data-chart="<id>"`. Always
/// the outermost chart piece -- `Chart`/`ChartTooltip`/`ChartLegend` are
/// composed as its children. The focusable/keyboard-navigable root
/// (`Chart`'s `keyboard` prop) is `Chart`'s own `[data-slot="chart"]` div,
/// not this container's -- see `Chart`'s doc comment below (`$S/chart-api.md`
/// "API changes" #4: an `Element`-typed child has no way to attach
/// attributes to its already-rendered parent without a post-mount effect,
/// which would make them absent from the first SSR render).
#[component]
pub fn ChartContainer(props: ChartContainerProps) -> Element {
    let base = attributes!(div {
        class: "dx-chart",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/chart/style.css") }
        chart::ChartContainer {
            id: props.id,
            config: props.config,
            data: props.data,
            kind: props.kind,
            attributes: merged,
            {props.children}
        }
    }
}

/// The chart's own visual: axes, grid, one mark per configured series (area
/// fill, bar, or line, per the container's `kind`), the hover hit-bands, and
/// the visually-hidden data table that carries the same series/category/
/// value data for assistive tech. Must be rendered inside a `ChartContainer`
/// -- reads its `kind`/`config`/`data` from that context, not from its own
/// props. Renders its own top-level `div[data-slot="chart"]` (wrapping the
/// svg + hidden table) that carries `tabindex`/`role="group"`/
/// `aria-roledescription="chart"`/`aria-label` and the keyboard handler when
/// `keyboard` is on -- `$S/chart-api.md`'s "API changes" #4. No base class
/// of its own: every mark (including this wrapper div) is unstyled by the
/// primitive and selected off `[data-slot="..."]` inside the container's
/// own `.dx-chart` scope (see `style.css`), so there is nothing new to
/// attach here beyond forwarding attributes through untouched.
#[component]
pub fn Chart(props: ChartProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/chart/style.css") }
        chart::Chart {
            aria_label: props.aria_label,
            description: props.description,
            width: props.width,
            height: props.height,
            stacked: props.stacked,
            curve: props.curve,
            show_grid: props.show_grid,
            show_x_axis: props.show_x_axis,
            show_y_axis: props.show_y_axis,
            x_tick_format: props.x_tick_format,
            y_tick_count: props.y_tick_count,
            show_dots: props.show_dots,
            keyboard: props.keyboard,
            dir: props.dir,
            attributes: props.attributes,
        }
    }
}

/// The hover tooltip: one row per series at the active data point, plus a
/// `role="graphics-symbol"` swatch per row. Rendered even when closed
/// (`data-state="closed"`, hidden by CSS) so there is nothing to attach
/// post-hydration. Optional -- omit it for a chart that doesn't need one.
#[component]
pub fn ChartTooltip(props: ChartTooltipProps) -> Element {
    let base = attributes!(div {
        class: "dx-chart-tooltip",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/chart/style.css") }
        chart::ChartTooltip {
            label_format: props.label_format,
            value_format: props.value_format,
            hide_label: props.hide_label,
            hide_indicator: props.hide_indicator,
            attributes: merged,
            // A named field, not the trailing `{props.children}` brace
            // sugar: that sugar always populates the inner component's
            // `children` with `Some(..)` (something was syntactically
            // written in the child-node position, even if the expression
            // itself evaluates to `None`), so every demo's plain
            // `ChartTooltip {}` -- providing no custom content -- was
            // tripping the primitive's "custom children replace the
            // default" branch and rendering nothing at all. Assigning the
            // `Option<Element>` value directly to the named field forwards
            // it unchanged, so `None` reaches the primitive as `None`.
            children: props.children,
        }
    }
}

/// The legend: one `role="graphics-symbol"` swatch + label per configured
/// series, in `config` order. Optional -- a single/dual-series chart
/// usually doesn't need one.
#[component]
pub fn ChartLegend(props: ChartLegendProps) -> Element {
    let base = attributes!(ul {
        class: "dx-chart-legend",
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/chart/style.css") }
        chart::ChartLegend {
            vertical_align: props.vertical_align,
            attributes: merged,
        }
    }
}
