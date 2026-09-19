//! Defines the `Chart` component and its sub-components: a pure-RSX SVG
//! chart engine (Area/Bar/Line for the MVP), styled and composed exactly
//! like every other primitive in this crate -- no new dependency, and
//! data-driven props rather than children-as-configuration.
//!
//! ## Provenance
//!
//! This module ports shadcn/ui's *conceptual* chart layer (`ChartConfig` ->
//! `--color-<key>` CSS variables scoped by `data-chart="<id>"`,
//! `ChartContainer`/`ChartTooltip`/`ChartLegend`) rather than its literal
//! React composition API, which has no Dioxus equivalent -- see
//! `dev-docs/research/chart-2026-09-19.md` §1 (especially §1.4) for why a
//! `Chart { XAxis {...} Bar {...} }` children-introspection API is not
//! portable, and §6 for the design this module implements. Its
//! accessibility contract (§2 of the same report) follows: `role="img"` +
//! `<title>`/`<desc>` on the SVG root rather than Recharts'
//! `role="application"`, a real hidden `<table>` as the actual
//! screen-reader-facing data path, and an *optional* sighted-keyboard
//! stepping layer mirroring Recharts' own `accessibilityLayer`
//! (`recharts/recharts` commit `86ad3632ff3f83a742001ae6fc079dd27961a2a4`,
//! `src/state/keyboardEventsMiddleware.ts`). Legend swatches use
//! `role="graphics-symbol"` per the W3C WAI-ARIA Graphics Module 1.0
//! (`https://www.w3.org/TR/graphics-aria-1.0/` -- an atomic, presentational-
//! children image role, the textbook fit for a color swatch).
//!
//! ## An engine, and a components layer built on it
//!
//! [`engine`] holds every pure computation and the plain chart data model,
//! with no dependency on this crate's other modules and no Dioxus types --
//! see its own module doc for why (in short: it is meant to be offered
//! upstream to `dioxus-community/dioxus-charts` once this component has
//! landed and proven itself here). [`Chart`], [`ChartContainer`],
//! [`ChartTooltip`], and [`ChartLegend`] (this module's `components`, plus
//! [`ChartContext`]/[`use_chart`] bridging `engine`'s data into this
//! crate's reactivity, and `config` holding [`ChartConfig`]/
//! [`ChartSeries`] -- see that module's own doc for why the series model
//! specifically lives beside `context`, not under `engine`) are the
//! Dioxus-specific layer built on top of it. This split is purely
//! internal: every public item from every layer is re-exported flatly here
//! (`dioxus_primitives::chart::LinearScale`,
//! `dioxus_primitives::chart::ChartConfig`, ...), so it changes nothing
//! about how this crate's own consumers use the module.
//!
//! This module landed in a sequence of small commits per this repo's own
//! `CLAUDE.md`/lane convention -- [`engine`] (unit-tested in isolation, no
//! `dioxus` types) and [`ChartContext`]/[`use_chart`] first so a themed
//! `preview` package could build against the data model while the
//! rendering components landed, then (stage-2 chart round) a per-family
//! split of `components::chart`'s own rendering into
//! `components::series::{area,bar,line,pie,radar,radial}` behind
//! `components::layout`'s shared plot geometry, reserving `ChartKind::{
//! Pie,Radar,RadialBar}` as stub kinds for later lanes. See each commit's
//! message for what's newly available, and `$S/chart-api.md`'s "API
//! changes" for this crate's own binding contract log.
//!
//! ## Example
//!
//! ```rust
//! use dioxus_primitives::chart::{Curve, LinearScale, line_path};
//!
//! let y = LinearScale { domain: (0.0, 100.0), range: (200.0, 0.0) }.nice();
//! let path = line_path(&[(0.0, y.scale(10.0)), (100.0, y.scale(90.0))], Curve::Linear);
//! assert!(path.starts_with('M'));
//! ```

mod components;
mod config;
mod context;
pub mod engine;

// Glob re-export (stage-2 chart round): `components` (including its own
// `series::*`, e.g. `AreaOptions`/`LineOptions`/...) is where every other
// stage-2 lane lands new public types, so re-exporting it by name here
// would need a `chart/mod.rs` edit per lane -- exactly the edit the
// round's structural construction says no lane but `s2-refactor` ever
// makes (`$S/stage2-common.md`). `config`/`context` stay
// `s2-refactor`-owned forever regardless (neither is a per-lane extension
// point), so glob-exporting them too costs nothing and is simpler than
// hand-listing their few items.
pub use components::*;
pub use config::*;
pub use context::*;
// NOT a glob, deliberately, unlike the three above: `engine` declares a
// `pub mod` per math file (`curve`, `data`, `geometry`, `scale`, `stack`,
// `table`, and now the stub `polar`/`radar`) alongside its actual
// re-exported items, and `pub use engine::*;` would re-export those
// SUBMODULE NAMES too, not just their contents. `engine::radar` and
// `components::series::radar` (this round's new Radar family file) both
// then surfacing as bare `radar` collides the moment both sides are
// blanket-globbed into this same flat `chart::` namespace --
// `error: ambiguous glob re-exports`, caught by this lane's own `cargo
// check` before it ever reached a lane that owns either file. Hand-listing
// `engine`'s actual value-level items (as this crate did before this
// round) sidesteps it entirely: no lane's task adds an `engine`-level
// submodule whose name collides with a `components`-level one going
// forward (checked directly: `curve`/`data`/`geometry`/`scale`/`stack`/
// `table`/`polar`/`radar` vs. `chart`/`container`/`legend`/`series`/
// `tooltip`/`area`/`bar`/`line`/`pie`/`radial` -- `radar` was the only
// overlap). A lane adding a genuinely new top-level export name to an
// existing `engine` file is the same "request the refactor owner" case as
// any other `chart/mod.rs` change; every name this list already carries
// (`stack`, `nice_domain`, `BandScale`, `LinearScale`, `Curve`,
// `ChartDatum`, `ChartKind`, the three path builders) keeps working
// unchanged.
pub use engine::{
    area_between_path, area_path, line_path, nice_domain, stack, BandScale, ChartDatum, ChartKind,
    Curve, LinearScale,
};
