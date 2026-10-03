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
    /// Which axes of the tooltip position come from the pointer instead of
    /// the active datum's anchor -- see [`Follow`].
    pub follow: Follow,
    /// Whether the tooltip's rows are the hovered *datum* (name, value, own
    /// color) instead of one row per configured series: a single-ring pie
    /// and an unstacked radial chart, whose one series only names the
    /// measure while every datum is its own slice/ring -- shadcn's pie and
    /// radial tooltips. Set by `Chart` from the family's own options, so
    /// `ChartTooltip` has no per-`ChartKind` branch for it.
    pub slice_rows: bool,
}

/// How the tooltip's position is derived from the pointer and the active
/// datum's [`ChartLayout::anchor_percent`]. Chosen per chart family by
/// `Chart` (the family knows its own geometry); `ChartTooltip` just applies
/// it, with no per-`ChartKind` branch of its own.
///
/// With no pointer driving (keyboard focus, or a pointer that has not
/// reported a position yet) every variant falls back to the anchor itself,
/// so a keyboard user gets the tooltip at the data point.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) enum Follow {
    /// The anchor on both axes, even with a pointer: a pie slice's
    /// centroid (shadcn/Recharts: the tooltip does not move within a
    /// slice, it jumps slice to slice).
    #[default]
    Anchor,
    /// x from the anchor (snapped to the category), y from the pointer:
    /// vertical Area/Bar/Line, shadcn/Recharts' category-chart rule.
    PointerY,
    /// y from the anchor (snapped to the category), x from the pointer:
    /// horizontal bars.
    PointerX,
    /// Both axes from the pointer: the polar families, whose active index
    /// is a spoke/ring/wedge rather than a column.
    Pointer,
}

/// Where the pointer is, as far as the tooltip is concerned.
///
/// The pointer's position and the datum under it are published together
/// (`components::pointer`), so a pointer-driven open never renders at the
/// keyboard anchor first: the tooltip's first visible frame is already at
/// the pointer.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub(crate) enum Cursor {
    /// No pointer: nothing hovered, or the keyboard is driving.
    #[default]
    None,
    /// The pointer's position in CSS px, relative to the chart box's
    /// top-left (client coordinates minus a fresh bounding rect).
    At(f64, f64),
}

/// Ordering guard for the asynchronous pointer pipeline
/// (`components::pointer::track_pointer`).
///
/// A pointer event is turned into a position only after an *async* bounding
/// rect read, so its result lands after the event handler returned. Anything
/// that happens in between -- a `pointerleave`/`blur`/`pointercancel` that
/// closes the tooltip, a key press that takes over, a newer move that
/// already landed -- must win over that stale result, or it would reopen a
/// tooltip the user just closed (a synthetic move + leave in one JS task
/// left it stuck open every time). Two counters make that unable to happen:
///
/// - `epoch` is bumped by every close/takeover; a [`Ticket`] issued under an
///   older epoch is rejected.
/// - `issued`/`applied` order the moves: a result is applied only if it is
///   newer than the last one applied, so results can arrive out of order
///   without moving the tooltip backwards -- but, unlike "only the latest
///   issued may land", a slow read never starves: if no newer result has
///   landed yet, the older one still does.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) struct PointerGate {
    epoch: u64,
    issued: u64,
    applied: u64,
}

/// A claim on the [`PointerGate`], taken when a pointer event fires and
/// presented again when its async measurement finishes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Ticket {
    epoch: u64,
    seq: u64,
}

impl PointerGate {
    /// A pointer event fired: take the next ticket under the current epoch.
    pub(crate) fn issue(&mut self) -> Ticket {
        self.issued += 1;
        Ticket {
            epoch: self.epoch,
            seq: self.issued,
        }
    }

    /// The tooltip closed (or something else took over): every ticket issued
    /// so far is void.
    pub(crate) fn close(&mut self) {
        self.epoch += 1;
    }

    /// The measurement behind `ticket` finished: may it be applied? `true`
    /// at most once per ticket, and only while the epoch is unchanged and no
    /// newer ticket was applied first.
    pub(crate) fn admit(&mut self, ticket: Ticket) -> bool {
        if ticket.epoch != self.epoch || ticket.seq <= self.applied {
            return false;
        }
        self.applied = ticket.seq;
        true
    }
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
    /// per-instance styling scopes to (the `--color-<key>` declarations
    /// themselves live in the container's inline `style` attribute).
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
    /// The pointer, relative to the chart box -- see [`Cursor`]. Written by
    /// `Chart`'s pointer/keyboard handlers, read by `ChartTooltip`.
    pub(crate) cursor: Signal<Cursor>,
    /// The chart box's CSS-pixel size (the tooltip's containing block),
    /// from the box's bounding rect at pointer time and its `onresize`.
    /// `None` until measured (always, on the server).
    pub(crate) box_size: Signal<Option<(f64, f64)>>,
    /// The tooltip's own measured border-box size, for flip/clamp. `None`
    /// until measured; `ChartTooltip` assumes a typical size meanwhile.
    pub(crate) tip_size: Signal<Option<(f64, f64)>>,
    /// Orders the async pointer measurements against closes -- see
    /// [`PointerGate`]. Not reactive (nothing renders from it).
    pub(crate) gate: CopyValue<PointerGate>,
}

impl ChartContext {
    /// Nothing is hovered or focused any more: close and forget the pointer.
    /// Also voids every pointer measurement still in flight
    /// ([`PointerGate::close`]), so a late result cannot reopen it.
    pub(crate) fn clear(self) {
        self.release_pointer();
        let (mut active, mut cursor) = (self.active_index, self.cursor);
        if active().is_some() {
            active.set(None);
        }
        if cursor() != Cursor::None {
            cursor.set(Cursor::None);
        }
    }

    /// Void every pointer measurement still in flight, without touching the
    /// open state: the keyboard (or a close) has the last word.
    pub(crate) fn release_pointer(self) {
        let mut gate = self.gate;
        gate.write().close();
    }

    /// The keyboard drives from now on: forget the pointer (the tooltip then
    /// hangs off the data point, not a stale position) and void any pointer
    /// measurement still in flight.
    pub(crate) fn take_over_by_keyboard(self) {
        self.release_pointer();
        let mut cursor = self.cursor;
        if cursor() != Cursor::None {
            cursor.set(Cursor::None);
        }
    }

    /// A pointer event fired: claim the next [`Ticket`] (see [`PointerGate`]).
    pub(crate) fn begin_pointer(self) -> Ticket {
        let mut gate = self.gate;
        let ticket = gate.write().issue();
        ticket
    }

    /// The measurement behind `ticket` finished: whether it may be applied.
    pub(crate) fn admit_pointer(self, ticket: Ticket) -> bool {
        let mut gate = self.gate;
        let admitted = gate.write().admit(ticket);
        admitted
    }

    /// Record the chart box's size, only when it changed (a stored-equal
    /// write would still re-render the tooltip on every pointer move).
    pub(crate) fn set_box_size(self, size: (f64, f64)) {
        let mut slot = self.box_size;
        if slot() != Some(size) {
            slot.set(Some(size));
        }
    }
}

/// Whether a pointer event came from a finger (`pointerType == "touch"`).
/// Touch gets its own rules: a lift fires `pointerleave`, which must not
/// close a tooltip the user just tapped open, and a drag is scrubbed from
/// coordinates because the browser pins every move to the first target.
pub(crate) fn is_touch(evt: &Event<PointerData>) -> bool {
    evt.data().pointer_type() == "touch"
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_close_voids_a_measurement_still_in_flight() {
        // pointermove -> (rect read pending) -> pointerleave/blur/cancel.
        let mut gate = PointerGate::default();
        let moved = gate.issue();
        gate.close();
        assert!(!gate.admit(moved), "the late result must not reopen it");
    }

    #[test]
    fn a_move_issued_after_a_close_lands() {
        let mut gate = PointerGate::default();
        let stale = gate.issue();
        gate.close();
        let fresh = gate.issue();
        assert!(!gate.admit(stale));
        assert!(gate.admit(fresh));
    }

    #[test]
    fn every_close_voids_every_older_ticket() {
        let mut gate = PointerGate::default();
        let tickets: Vec<_> = (0..5).map(|_| gate.issue()).collect();
        gate.close();
        gate.close();
        assert!(tickets.into_iter().all(|t| !gate.admit(t)));
    }

    #[test]
    fn results_arriving_out_of_order_never_move_the_tooltip_backwards() {
        let mut gate = PointerGate::default();
        let (first, second) = (gate.issue(), gate.issue());
        assert!(gate.admit(second));
        assert!(!gate.admit(first), "older than what is already shown");
    }

    #[test]
    fn a_slow_measurement_is_not_starved_by_newer_moves() {
        // Moves keep being issued while the first read is slow: until a newer
        // one actually lands, the older one still may.
        let mut gate = PointerGate::default();
        let first = gate.issue();
        let _second = gate.issue();
        let _third = gate.issue();
        assert!(gate.admit(first));
    }

    #[test]
    fn a_ticket_is_admitted_at_most_once() {
        let mut gate = PointerGate::default();
        let t = gate.issue();
        assert!(gate.admit(t));
        assert!(!gate.admit(t));
    }
}
