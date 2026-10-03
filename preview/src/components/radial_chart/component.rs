//! `Radial chart` -- part of the shared `chart` package (`ChartContainer`,
//! `Chart`, `ChartTooltip`; `RadialOptions`/`PieLabels` configure
//! `ChartKind::RadialBar`). This folder is a docs/gallery page for that
//! one polar family, not a separate installable package -- installing
//! `chart` (`componentDependencies: ["chart", "card"]`, this folder's own
//! `component.json`) already gives you everything below.
//!
//! Re-exports `crate::components::chart::*`, the path a user's own tree has
//! after `dx components add chart` (there `chart/mod.rs` is `mod component;
//! pub use component::*;`). Never reach into `chart::component::` directly:
//! that module is private once installed (`scripts/check-installed-paths.sh`).
pub use crate::components::card::{
    Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle,
};
pub use crate::components::chart::*;
pub use dioxus_primitives::chart::engine::polar::{RadialGrid, Radius};
