//! `Pie chart` -- part of the shared `chart` package (`ChartContainer`,
//! `Chart`, `ChartTooltip`, `ChartLegend`; `PieOptions`/`PieLabels`
//! configure `ChartKind::Pie`). This folder is a docs/gallery page for that
//! one polar family, not a separate installable package -- installing
//! `chart` (`componentDependencies: ["chart", "card"]`, this folder's own
//! `component.json`) already gives you everything below.
//!
//! Re-exports one level into `chart`'s own `component` submodule
//! (`chart::component::*`), not the whole `chart` module (`chart::*`) --
//! `preview/src/components/mod.rs`'s `examples!` macro gives every
//! registered component (this one included) its own crate-visible
//! `component` submodule, so a blanket `chart::*` glob would re-export
//! `chart`'s OWN `component` submodule NAME too, colliding with this file's
//! own macro-declared one (`error[E0659]: "component" is ambiguous`) --
//! `$S/stage2-lanes.md`'s cross-lane alert (s2-area, 2026-09-19);
//! `area_chart`/`bar_chart`/`line_chart`/`radar_chart`/`chart_tooltip` all
//! hit and fix this identically.
pub use crate::components::card::{
    Card, CardAction, CardContent, CardDescription, CardFooter, CardHeader, CardTitle,
};
pub use crate::components::chart::component::*;
