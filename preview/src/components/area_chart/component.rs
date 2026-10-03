//! Area Chart gallery -- shadcn/ui `chart-area-*` demo parity
//! (`dev-docs/research/chart-2026-09-19.md` §1.2's area inventory,
//! `$S/refs/ui/apps/v4/registry/new-york-v4/charts/chart-area-*.tsx`).
//!
//! This is a documentation/demo package, not a new primitive or even a new
//! themed wrapper: every piece it composes (`ChartContainer`, `Chart`,
//! `ChartTooltip`, `ChartLegend`, `ChartConfig`, `ChartDatum`, `ChartKind`,
//! `Curve`) already lives in `crate::components::chart` -- the actually
//! *installable* package (see this folder's own `component.json`:
//! `componentDependencies: ["chart"]`, and its own `exclude` list, which
//! drops `variants/`/`docs.md`/`component.json` the same way every other
//! component's does, leaving only this file + `style.css` to ship). This
//! package's own job is purely the ten shadcn `chart-area-*` demo variants
//! under `variants/`, each citing its own upstream source file in its own
//! doc comment.
//!
//! Mirrors this repo's existing "docs-only, composes another package's
//! themed wrapper" pattern -- see `data_table`'s own header comment (it
//! composes `table`, `button`, `select`, `input`, `checkbox`); this
//! package is an even thinner instance of the same shape, since it adds no
//! composition logic of its own at all, only demos.
//!
//! Re-exports `crate::components::chart::*`, the path a user's own tree has
//! after `dx components add chart` (there `chart/mod.rs` is `mod component;
//! pub use component::*;`). Never reach into `chart::component::` directly:
//! that module is private once installed (`scripts/check-installed-paths.sh`).
use dioxus::prelude::*;

pub use crate::components::chart::*;

/// Root of every demo in this gallery. Its only job is to link this
/// package's `style.css` (the `display: contents` rule that makes the
/// wrapper transparent to layout), like every other component links its own
/// stylesheet from the component that uses it, so the rule travels with the
/// demos on every page that renders them rather than being mirrored into
/// the docs site's own CSS.
#[component]
pub fn AreaChartGallery(children: Element) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/area_chart/style.css") }
        div { class: "dx-area-chart-gallery", {children} }
    }
}
