//! Area Chart gallery -- shadcn/ui `chart-area-*` demo parity
//! (`dev-docs/research/chart-2026-09-19.md` §1.2's area inventory,
//! `$S/refs/ui/apps/v4/registry/new-york-v4/charts/chart-area-*.tsx`).
//!
//! This is a documentation/demo package, not a new primitive or even a new
//! themed wrapper: every piece it composes (`ChartContainer`, `Chart`,
//! `ChartTooltip`, `ChartLegend`, `ChartConfig`, `ChartDatum`, `ChartKind`,
//! `Curve`) already lives in `crate::components::chart` -- the actually
//! *installable* package (see this folder's own `component.json`:
//! `componentDependencies: ["chart"]`, and its own `exclude` list, which
//! drops `variants/`/`docs.md`/`component.json` the same way every other
//! component's does, leaving only this file + `style.css` to ship). This
//! package's own job is purely the ten shadcn `chart-area-*` demo variants
//! under `variants/`, each citing its own upstream source file in its own
//! doc comment.
//!
//! Mirrors this repo's existing "docs-only, composes another package's
//! themed wrapper" pattern -- see `data_table`'s own header comment (it
//! composes `table`, `button`, `select`, `input`, `checkbox`); this
//! package is an even thinner instance of the same shape, since it adds no
//! composition logic of its own at all, only demos.
//!
//! **`chart::component::*`, not `chart::*`, and this is load-bearing, not
//! a style choice:** `preview/src/components/mod.rs`'s `examples!` macro
//! expands every registered `$name` (this package included) to `mod $name
//! { pub(crate) mod component; pub use component::*; pub(crate) mod
//! variants { ... } }`. Every such module therefore has its own submodule
//! literally named `component`, `pub(crate)` (crate-wide visible). Glob-
//! importing a sibling's *whole* module (`crate::components::chart::*`)
//! transitively re-exports that sibling's `component` submodule NAME too
//! (not just its contents) -- which then collides with this module's own
//! macro-declared `pub(crate) mod component;`, `error[E0659]: `component`
//! is ambiguous` (confirmed by triggering it while writing this file).
//! Importing one level deeper, `chart::component::*`, re-exports that
//! module's *contents* (`Chart`, `ChartContainer`, ...) without also
//! naming the `component` module itself, which sidesteps the collision by
//! construction. Any other stage-2 gallery package following this same
//! "thin wrapper over `chart`" shape must import this same way.
pub use crate::components::chart::component::*;
