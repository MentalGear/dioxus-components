//! `Radial chart` -- part of the shared `chart` package (`ChartContainer`,
//! `Chart`, `ChartTooltip`; `RadialOptions`/`PieLabels` configure
//! `ChartKind::RadialBar`). This folder is a docs/gallery page for that
//! one polar family, not a separate installable package -- installing
//! `chart` (`componentDependencies: ["chart", "card"]`, this folder's own
//! `component.json`) already gives you everything below.
//!
//! Re-exports one level into `chart`'s own `component` submodule
//! (`chart::component::*`), not the whole `chart` module (`chart::*`) --
//! same `error[E0659]: "component" is ambiguous` trap
//! `pie_chart`/`radar_chart`/`bar_chart`/etc. all document identically.
pub use crate::components::card::{Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle};
pub use crate::components::chart::component::*;
