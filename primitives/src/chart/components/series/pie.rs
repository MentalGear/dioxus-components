//! The [`ChartKind::Pie`](crate::chart::ChartKind::Pie) family: a single
//! ring of proportional wedges, one per datum. **Stub** -- reserved by the
//! stage-2 chart round's `s2-refactor` lane for its owning lane,
//! **`s2-polar`**, which also owns `engine::polar`'s actual arc math (see
//! that module's own doc). `Chart` renders only this placeholder mark group
//! and a reduced (category + first-series value) data table for this kind
//! today -- see `components::chart`'s own module doc for why (grid/axes/
//! cursor/hit-bands are Cartesian-only, per `ChartKind::is_cartesian`).

use dioxus::prelude::*;

use super::super::layout::SeriesRenderContext;

/// [`crate::chart::ChartKind::Pie`]'s own options. Empty until `s2-polar`
/// lands (padAngle, corner radius, inner/outer radius, per-slice label
/// placement, ... are all plausible fields here).
#[derive(Clone, PartialEq, Debug, Default)]
pub struct PieOptions {}

/// Placeholder mark group -- `s2-polar` replaces this body with the real
/// per-datum wedge geometry. Ignores `ctx`/`opts` entirely for now.
pub(crate) fn render(_ctx: &SeriesRenderContext, _opts: &PieOptions) -> Element {
    rsx! {
        g { "data-slot": "chart-series", "data-kind": "pie" }
    }
}
