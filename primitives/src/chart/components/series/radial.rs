//! The [`ChartKind::RadialBar`](crate::chart::ChartKind::RadialBar) family:
//! concentric proportional arcs, one ring per series. **Stub** -- reserved
//! by the stage-2 chart round's `s2-refactor` lane for its owning lane,
//! **`s2-polar`**, which also owns `engine::polar`'s actual arc math (see
//! that module's own doc). `Chart` renders only this placeholder mark group
//! and a reduced (category + first-series value) data table for this kind
//! today -- see `components::chart`'s own module doc for why (grid/axes/
//! cursor/hit-bands are Cartesian-only, per `ChartKind::is_cartesian`).

use dioxus::prelude::*;

use super::super::layout::SeriesRenderContext;

/// [`crate::chart::ChartKind::RadialBar`]'s own options. Empty until
/// `s2-polar` lands (corner radius, track/background arc, start angle, ...
/// are all plausible fields here).
#[derive(Clone, PartialEq, Debug, Default)]
pub struct RadialOptions {}

/// Placeholder mark group -- `s2-polar` replaces this body with the real
/// concentric-arc geometry. Ignores `ctx`/`opts` entirely for now.
pub(crate) fn render(_ctx: &SeriesRenderContext, _opts: &RadialOptions) -> Element {
    rsx! {
        g { "data-slot": "chart-series", "data-kind": "radial-bar" }
    }
}
