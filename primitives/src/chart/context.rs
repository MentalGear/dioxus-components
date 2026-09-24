//! [`ChartContext`] -- the reactive state `ChartContainer`
//! (`components::container`) provides its descendants (`Chart`,
//! `ChartTooltip`, `ChartLegend`). This is the one place in `chart/` that
//! is deliberately *not* part of the [`super::engine`] seam: it exists
//! specifically to hold `engine`'s plain data ([`super::engine::ChartDatum`]/
//! [`super::engine::ChartKind`]) and [`super::config`]'s
//! [`super::config::ChartConfig`] behind this crate's own reactive
//! primitives (`Signal`/`ReadSignal`/`Memo`), which a standalone engine
//! crate would have no reason to depend on.

use dioxus::prelude::*;

use super::config::ChartConfig;
use super::engine::{ChartDatum, ChartKind};

/// The pixel-space layout `Chart` computed for its own SVG, shared so
/// `ChartTooltip` can position itself "from the same scales" (per
/// `$S/chart-api.md`) with no DOM measurement of its own. `Chart` is the
/// only writer (a plain assignment during its own render, not inside an
/// effect -- its width/height/axis-visibility props are the only inputs
/// this depends on, so it's cheap and correct to just recompute and
/// re-set every render; `Signal`'s own equality check keeps that a no-op
/// once the value stabilizes); `ChartTooltip` is the only reader. `None`
/// until `Chart` has rendered at least once -- `ChartTooltip` treats that
/// exactly like `active_index == None` (rendered `data-state="closed"`, so
/// its exact position doesn't matter yet), which covers every real case:
/// nothing can set `active_index` to `Some` before `Chart` -- the thing
/// that owns the hit bands that set it -- has rendered.
///
/// Family-agnostic by construction (stage-2 chart round, §4(d) of the
/// handoff): this used to store the raw `x_scale: BandScale`/`y_scale:
/// LinearScale`/`top_value: Vec<f64>` `ChartTooltip` combined into a
/// percent position itself, which only works for a Cartesian
/// (Area/Bar/Line) layout -- `BandScale::center(i)` is an affine function
/// of the datum index `i`, but a polar family's vertex position (e.g.
/// Radar's `center + point_radial(angle(i), radius(value))`) is a
/// sinusoidal function of `i` that no `BandScale` can reproduce. Storing
/// the already-resolved `(left%, top%)` per datum instead moves that
/// family-specific math to whichever side computed the layout in the
/// first place (`Chart`'s own Cartesian dispatch today; a family like
/// Radar's own `render`, once it needs an open tooltip, tomorrow), so
/// `ChartTooltip` itself indexes this uniformly with no per-`ChartKind`
/// branch of its own.
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct ChartLayout {
    /// Each datum's tooltip anchor as `(left%, top%)` of the chart's own
    /// logical width/height (the SVG viewBox, not the smaller plot area
    /// inset by margins) -- `ChartTooltip` reads `anchor_percent[i]`
    /// directly at the active index, with no scale of its own. Empty (or
    /// shorter than the data) is treated exactly like `active_index ==
    /// None` -- see `ChartTooltip`'s own fallback.
    pub anchor_percent: Vec<(f64, f64)>,
}

/// The state `ChartContainer` provides to every descendant -- fetch it
/// with [`use_chart`]. `Copy` (like this crate's other root contexts, e.g.
/// `slider`'s `SliderContext`) so every event handler and child component
/// can capture it by value with no `.clone()` noise; `config`/`data`/
/// `kind` are reactive read handles (the caller's own signals, passed
/// straight through), and `active_index` is the one piece of state this
/// module owns itself.
#[derive(Clone, Copy)]
pub struct ChartContext {
    /// This chart instance's id -- the `data-chart` attribute value the
    /// `--color-<key>` style rule (and any per-instance styling) scopes to.
    /// A [`Memo`] (not a plain `String`) so the context stays `Copy`
    /// despite an id that may come from the caller's own reactive `id` prop
    /// (`use_id_or`, this crate's existing id-resolution helper).
    pub id: Memo<String>,
    /// The series list -- colors, labels, draw order.
    pub config: ReadSignal<ChartConfig>,
    /// The data points, one per x-axis category.
    pub data: ReadSignal<Vec<ChartDatum>>,
    /// The currently-hovered/keyboard-focused datum index, or `None` when
    /// nothing is active. Written by `Chart`'s hit bands (pointer) and,
    /// when its `keyboard` prop is enabled, by its own wrapper's
    /// `onkeydown`; read by `Chart` (the cursor) and `ChartTooltip`
    /// (visibility + position).
    pub active_index: Signal<Option<usize>>,
    /// Which mark family this chart draws.
    pub kind: ReadSignal<ChartKind>,
    /// `Chart`'s own computed pixel layout, shared for `ChartTooltip`'s
    /// benefit -- see [`ChartLayout`]'s own doc. Not part of this crate's
    /// public API surface (`pub(crate)`, unlike every other field here):
    /// an internal wiring detail this design needed, not a contract this
    /// lane commits to keeping stable for outside callers of
    /// [`use_chart`].
    pub(crate) layout: Signal<Option<ChartLayout>>,
}

/// Fetch the nearest ancestor `ChartContainer`'s [`ChartContext`].
///
/// # Panics
///
/// Panics if called outside a `ChartContainer` -- `Chart`, `ChartTooltip`,
/// and `ChartLegend` all call this and are only meaningful as its
/// descendants, the same contract every other multi-piece primitive in
/// this crate has for its own root (e.g. `Slider`/`SliderTrack`).
pub fn use_chart() -> ChartContext {
    try_consume_context::<ChartContext>().unwrap_or_else(|| {
        panic!(
            "`use_chart` was called outside a `ChartContainer` -- `Chart`, `ChartTooltip`, and \
             `ChartLegend` must be rendered inside a `ChartContainer`"
        )
    })
}

// `use_chart`'s panic-outside-a-container path is a one-line
// `unwrap_or_else(|| panic!(...))`, and isn't independently unit-tested
// here: Dioxus's `VirtualDom` catches a panic raised from inside a
// component body at the component boundary (so the rest of a page doesn't
// go down with it -- confirmed empirically this lane, not assumed), which
// makes `#[should_panic]` unable to observe it through `rebuild_in_place`.
// The success path (`use_chart` returning the provided context) is
// exercised by every other test in this module tree that renders a
// `ChartContainer` around something that calls it.
