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
//! One level deeper than it looks (`crate::components::chart::component::*`,
//! not `crate::components::chart::*`): `chart`'s own module (built by this
//! same `examples!` macro) also carries `pub(crate) mod component;`/
//! `pub(crate) mod variants;` at that same level, which a glob import
//! re-exports too (`pub(crate)` is still crate-visible to a glob from
//! elsewhere in this crate) -- re-exported into *this* module's own
//! `component`/`variants` names, which then collide with this module's own
//! identically-named siblings once the outer macro's own `pub use
//! component::*;` tries to glob them back up into `line_chart` itself
//! (`error[E0659]`, confirmed by actually hitting it with `pub use
//! crate::components::chart::*;` here). This is a cross-lane class, not a
//! one-off: every "thin wrapper over `chart`" gallery lane hits it
//! identically (`$S/stage2-lanes.md`'s s2-area cross-lane alert), and this
//! import shape is that alert's own fix, applied here verbatim so every
//! sibling gallery's `component.rs` reads the same way.
pub use crate::components::chart::component::*;

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
