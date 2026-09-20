//! Radar chart -- part of the shared `chart` package (`ChartContainer`,
//! `Chart`, `ChartTooltip`, `ChartLegend`; `RadarOptions`/`RadarGrid`
//! configure the `Radar` family). This folder is a docs/gallery page for
//! that family, not a separate installable package -- installing `chart`
//! (`componentDependencies: ["chart"]`, this folder's own `component.json`)
//! already gives you everything below.
//!
//! Re-exports one level into `chart`'s own `component` submodule
//! (`chart::component::*`), not the whole `chart` module
//! (`chart::*`) -- `preview/src/components/mod.rs`'s `examples!` macro
//! gives every registered component (this one included) its own
//! crate-visible `component` submodule, so a blanket `chart::*` glob would
//! re-export `chart`'s OWN `component` submodule NAME too, colliding with
//! this file's own macro-declared one (`error[E0659]: "component" is
//! ambiguous`) -- see `$S/stage2-lanes.md`'s cross-lane alert (s2-area,
//! 2026-09-19) for the full account; `area_chart`/`bar_chart`/
//! `radial_chart`/`chart_tooltip` all hit and fix this identically.
pub use crate::components::chart::component::*;

// `RadarOptions`/`RadarGrid` are plain configuration types with no theme to
// attach (same rationale `chart/component.rs`'s own header gives for
// `ChartConfig`/`ChartDatum`/`ChartKind`/`LegendAlign`) -- re-exported here
// rather than added to `chart/component.rs`'s own re-export line, mirroring
// `chart_tooltip/component.rs`'s identical choice for `TooltipIndicator`/
// `TooltipRow` (`$S/stage2-lanes.md`, s2-tooltip "FYI, not blocking").
pub use dioxus_primitives::chart::{ChartIcon, RadarGrid, RadarOptions};
