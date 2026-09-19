//! The [`ChartKind::Radar`](crate::chart::ChartKind::Radar) family: a
//! closed polygon per series across N radial (per-datum) axes. **Stub** --
//! reserved by the stage-2 chart round's `s2-refactor` lane for its owning
//! lane, **`s2-radar`**, which also owns `engine::radar`'s actual polygon
//! math (see that module's own doc). `Chart` renders only this placeholder
//! mark group and a reduced (category + first-series value) data table for
//! this kind today -- see `components::chart`'s own module doc for why
//! (grid/axes/cursor/hit-bands are Cartesian-only, per
//! `ChartKind::is_cartesian`).

use dioxus::prelude::*;

use super::super::layout::SeriesRenderContext;

/// [`crate::chart::ChartKind::Radar`]'s own options. Empty until `s2-radar`
/// lands (grid shape, per-axis label formatting, dot markers, ... are all
/// plausible fields here).
#[derive(Clone, PartialEq, Debug, Default)]
pub struct RadarOptions {}

/// Placeholder mark group -- `s2-radar` replaces this body with the real
/// per-series polygon geometry. Ignores `ctx`/`opts` entirely for now.
pub(crate) fn render(_ctx: &SeriesRenderContext, _opts: &RadarOptions) -> Element {
    rsx! {
        g { "data-slot": "chart-series", "data-kind": "radar" }
    }
}
