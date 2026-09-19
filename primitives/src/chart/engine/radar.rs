//! Reserved for the Radar family's own pure math (per-datum radial axis
//! angle placement, closed-polygon path generation -- `d3-shape/src/
//! lineRadial.js`/`areaRadial.js`, per `$S/chart/forks-research.md` §4-6 and
//! `$S/stage2-common.md`). Empty for now: this stage-2 chart round's
//! `s2-refactor` lane only reserves the module (and the matching
//! `components::series::radar` stub file/`ChartKind::Radar` variant) so the
//! owning lane, **`s2-radar`**, has a file to land in without also needing a
//! `primitives/src/chart/engine/mod.rs` edit (already declared there). No
//! dioxus types here either, same rule as every other file under `engine`
//! -- see that module's own doc.
