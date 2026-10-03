//! Line chart gallery: a thin doc-page wrapper around the shared `Chart`
//! primitive (`crate::components::chart`) -- see that package for the
//! actual themed components (`ChartContainer`, `Chart`, `ChartTooltip`,
//! `ChartLegend`) and its own module doc for the data-driven
//! `ChartConfig`/`ChartDatum` model this package's demos build on.
//!
//! This package defines no new component. shadcn splits its "Line Chart"
//! catalog entry into ten separate demo files
//! (`chart-line-default.tsx` .. `chart-line-interactive.tsx`,
//! `dev-docs/research/chart-2026-09-19.md` §1.2) that all compose the same
//! underlying `<ChartContainer>`/`<Line>`/`<ChartTooltip>` -- this package
//! mirrors that same one-family/many-demos split for this repo's
//! `ChartKind::Line`, one `variants/<demo>/mod.rs` per shadcn source file
//! (see each variant's own doc comment for its citation). The installable
//! unit is still `chart` (`dx components add chart`); this gallery has
//! nothing of its own to install beyond what that package already ships.
//!
//! Re-exports `chart`'s themed pieces unchanged so every variant here
//! composes `crate::components::line_chart::*` (via `use super::super::
//! component::*;`, this crate's own established variant-import idiom)
//! without reaching into `crate::components::chart` directly -- a plain
//! alias, not a new component (`scripts/check-preview-composition.sh`
//! already exempts every `component.rs`; there is no new markup here for
//! it to flag either way, since a `pub use` of an existing themed
//! component is not itself a `dioxus_primitives::` reference).
//!
//! Re-exports `crate::components::chart::*`, the path a user's own tree has
//! after `dx components add chart` (there `chart/mod.rs` is `mod component;
//! pub use component::*;`). Never reach into `chart::component::` directly:
//! that module is private once installed (`scripts/check-installed-paths.sh`).
pub use crate::components::chart::*;

/// `Curve` is a plain, render-nothing enum (dev-docs/preview-composition.md's
/// allowlist rationale: no theme for a wrapper to attach), so it is
/// re-exported straight from the primitive here rather than by editing
/// `chart::component`'s own re-export line -- keeps this gallery's
/// dependency on a value the `chart` package doesn't currently re-export
/// entirely inside this lane's own files, with nothing for a sibling
/// gallery lane (`area_chart`, `bar_chart`, ...) editing that same shared
/// line at the same time to conflict with. This file is itself the
/// "themed wrapper" layer `check-preview-composition.sh` exempts, so this
/// one raw `dioxus_primitives::chart::Curve` reference is not a violation.
pub use dioxus_primitives::chart::Curve;

/// `LineOptions` itself is already re-exported by `chart::component`
/// (that package's own `component.rs`, added alongside the stage-2
/// refactor's `Chart.line: LineOptions` field) -- reached here through
/// the `pub use crate::components::chart::*;` above. Its own
/// nested types (`LineLabels`, `DotRenderer`, `DotContext`) landed after
/// that re-export line was written and are not in it yet; re-exported
/// straight from the primitive here for the same reason and with the same
/// no-shared-file-edit benefit as `Curve` above, rather than adding a
/// second lane's own names to `chart::component`'s shared list.
pub use dioxus_primitives::chart::{DotContext, DotRenderer, LineLabels, TooltipIndicator};
