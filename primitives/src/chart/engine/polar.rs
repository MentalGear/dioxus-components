//! Reserved for the Pie/RadialBar families' own pure math (angle/arc
//! computation -- `d3-shape/src/arc.js` and `d3-shape/src/pie.js`'s
//! `padAngle`/`cornerRadius`/wedge-angle math, per `$S/chart/forks-research.md`
//! §4-6 and `$S/stage2-common.md`). Empty for now: this stage-2 chart round's
//! `s2-refactor` lane only reserves the module (and the matching
//! `components::series::{pie, radial}` stub files/`ChartKind::{Pie,
//! RadialBar}` variants) so the owning lane, **`s2-polar`**, has a file to
//! land in without also needing a `primitives/src/chart/engine/mod.rs` edit
//! (already declared there). No dioxus types here either, same rule as
//! every other file under `engine` -- see that module's own doc.
