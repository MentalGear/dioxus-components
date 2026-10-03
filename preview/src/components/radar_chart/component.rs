//! Radar chart -- part of the shared `chart` package (`ChartContainer`,
//! `Chart`, `ChartTooltip`, `ChartLegend`; `RadarOptions`/`RadarGrid`
//! configure the `Radar` family). This folder is a docs/gallery page for
//! that family, not a separate installable package -- installing `chart`
//! (`componentDependencies: ["chart"]`, this folder's own `component.json`)
//! already gives you everything below.
//!
//! Re-exports `crate::components::chart::*`, the path a user's own tree has
//! after `dx components add chart` (there `chart/mod.rs` is `mod component;
//! pub use component::*;`). Never reach into `chart::component::` directly:
//! that module is private once installed (`scripts/check-installed-paths.sh`).
pub use crate::components::chart::*;

// `RadarOptions`/`RadarGrid` are plain configuration types with no theme to
// attach (same rationale `chart/component.rs`'s own header gives for
// `ChartConfig`/`ChartDatum`/`ChartKind`/`LegendAlign`) -- re-exported here
// rather than added to `chart/component.rs`'s own re-export line, mirroring
// `chart_tooltip/component.rs`'s identical choice for `TooltipIndicator`/
// `TooltipRow` (`$S/stage2-lanes.md`, s2-tooltip "FYI, not blocking").
pub use dioxus_primitives::chart::{ChartIcon, RadarGrid, RadarOptions};
