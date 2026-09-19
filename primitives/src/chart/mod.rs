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
//! This module is under construction, landing in a sequence of small
//! commits per this repo's own `CLAUDE.md`/lane convention -- the pure
//! scale/path math (unit-tested in isolation with no `dioxus` types, see
//! [`LinearScale`]/[`BandScale`]/[`line_path`]/[`area_path`]/[`stack`]/
//! [`nice_domain`]) lands first so a themed `preview` package can build
//! against it while the rest of the component lands. See each commit's
//! message for what's newly available.
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

mod scale;

pub use scale::{area_path, line_path, nice_domain, stack, BandScale, Curve, LinearScale};
