//! The chart's engine: every pure computation (scales, "nice" ticks,
//! curve/path math, stacking, series geometry, the data-table row model)
//! and its plain data model (`ChartConfig`/`ChartSeries`/`ChartDatum`/
//! `ChartKind`), with **no dependency on any other module of this crate,
//! and no Dioxus types** beyond what a standalone, framework-agnostic
//! charting crate would itself contain.
//!
//! # Why this seam exists
//!
//! Per this session's 2026-09-19 user decision (recorded in this repo's
//! own chart-development ledger): this engine is intended, once the
//! `Chart` component has landed here and proven itself (oracle green,
//! deployed), to be offered upstream to the `dioxus-community/
//! dioxus-charts` project. That only works if this folder is *actually*
//! liftable into another crate unchanged -- not merely "conceptually
//! separable" while quietly importing `crate::direction` or a `dioxus`
//! `Signal` somewhere. Concretely, that means every item in this module
//! and its children:
//!
//! - never writes `use crate::` (this crate's other modules -- `context`,
//!   `direction`, the `components` tree -- are all downstream of this
//!   module, never the reverse);
//! - never names a `dioxus`/`dioxus_core` type (`Signal`, `Element`,
//!   `Callback`, ...) -- every public item here is plain Rust (`String`,
//!   `Vec`, `f64`, `Option`, tuples, small owned structs/enums);
//! - is exercised entirely by `#[test]`, never by anything that needs a
//!   `VirtualDom` -- matching `dev-docs/conformance-harness.md`'s own
//!   coverage table ("Cargo unit tests for algorithms ... the algorithmic
//!   ones") and this crate's existing math-module precedent (`slider.rs`'s
//!   `ordered_range`/`snap_value`/..., `calendar.rs`'s date-grid helpers)
//!   -- this module is simply the first one large enough to earn its own
//!   directory rather than living inline in one file.
//!
//! This crate's `chart::components` module (the Dioxus-facing layer: `ChartContainer`,
//! `Chart`, `ChartTooltip`, `ChartLegend`) and [`super::ChartContext`]
//! depend on this module through its public API only, same as any external
//! consumer would.
//!
//! # Modules
//!
//! - [`scale`] -- [`scale::LinearScale`] (with "nice" tick generation) and
//!   [`scale::BandScale`], plus [`scale::nice_domain`].
//! - [`curve`] -- [`curve::Curve`] and the [`curve::line_path`]/
//!   [`curve::area_path`] SVG path builders.
//! - [`mod@stack`] -- [`stack::stack`], series stacking, plus
//!   [`stack::stack_with_mode`]/[`stack::StackMode`] for percent
//!   ("100% stacked") stacking.
//! - [`geometry`] -- [`geometry::plot_runs`], gap-aware point-run
//!   splitting for a line/area series.
//! - [`table`] -- [`table::table_rows`], the hidden data table's row
//!   model.
//! - [`data`] -- the plain data types ([`data::ChartDatum`],
//!   [`data::ChartKind`]). `ChartConfig`/`ChartSeries` are NOT here --
//!   see `super::config`'s own module doc for why the series model
//!   specifically had to move out of this dioxus-free tree.
//! - [`polar`]/[`radar`] -- reserved, currently-empty modules for the
//!   Pie/RadialBar and Radar families' own math (owned by later stage-2
//!   lanes `s2-polar`/`s2-radar`; see each module's own doc comment).
//!
//! Every public item here is also re-exported flatly from
//! [`crate::chart`] (e.g. `dioxus_primitives::chart::LinearScale`), so this
//! internal folder split is invisible to a normal consumer of the crate --
//! it exists for the upstreaming seam above, not as part of the public API
//! shape.

pub mod curve;
pub mod data;
pub mod geometry;
pub mod polar;
pub mod radar;
pub mod scale;
pub mod stack;
pub mod table;

pub use curve::{area_between_path, area_path, line_path, Curve};
pub use data::{ChartDatum, ChartKind};
pub use scale::{nice_domain, BandScale, LinearScale};
pub use stack::{stack, stack_with_mode, StackMode};
