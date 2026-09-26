//! Defines the [`Carousel`] component and its sub-components.
//!
//! # Scope (v1)
//!
//! Built per `dev-docs/research/carousel-2026-09-19.md` and the approved
//! decisions (2026-09-19): the APG "basic" (prev/next, no picker) style
//! only (`carousel-1-prev-next` in the vendored reference). Deliberately
//! **not** in v1, each filed as a fast-follow rather than a silent gap:
//! infinite `loop`ing (§8.2), autoplay / a rotation control and the
//! `aria-live` region that only matters once one exists (§8.3 -- the APG
//! basic style has no rotation requirement at all, so a manually-paged
//! carousel is fully conformant without it, §1.2), and the tablist picker
//! variant (§8.5, composed on [`crate::tabs`] once this variant is
//! proven out).
//!
//! # Engine
//!
//! CSS scroll-snap, not a ported physics engine and not a JS dependency
//! (research §8.1/§2.2): [`CarouselContent`] is a native
//! `overflow-{x,y}: auto; scroll-snap-type` track; [`CarouselItem`]s are
//! `scroll-snap-align` children. Paging (buttons, keyboard) computes the
//! physical pixel delta between the target item's own
//! `getBoundingClientRect()` and the scroller's, per axis, and calls
//! `scroller.scrollBy({ left | top: delta })` on the scroller itself --
//! never `Element.scrollIntoView()` on the target (backlog row 91's dated
//! addendum has the incident that replaced it: `scrollIntoView` walks and
//! scrolls *every* scrollable ancestor between the target and the
//! viewport, including the document, so autoplay/Next dragged the whole
//! page back to the carousel whenever it was scrolled elsewhere --
//! `scrollBy`, called on the scroller directly, only ever writes that one
//! element's own scroll position). No manual RTL sign-flip step is needed
//! either: `getBoundingClientRect`/`scrollBy`'s delta are both physical
//! (per research §2.2's RTL paragraph and
//! `dev-docs/research/carousel-overscroll-2026-09-23.md` §6 invariant 1),
//! so "the next slide" resolving to the correct physical position under
//! `dir="rtl"` falls out of the same physical-delta construction the drag
//! gesture below already uses, rather than needing its own. A second,
//! independent JS bridge (`use_carousel_scroll_tracking`, a private
//! helper) keeps `CarouselContext`'s own `selected` correct after a
//! *native* drag/wheel/trackpad scroll the caller's own buttons/keyboard
//! never drove. A third (`use_carousel_drag`, also private -- see
//! [`CarouselContent`]'s own "Pointer drag" doc) adds a mouse/pen
//! drag-to-scroll gesture on top of the *same* track, moving it with
//! plain `scrollBy` calls rather than a parallel transform-based engine;
//! every `scrollBy` it issues is native scrolling as far as the browser
//! (and the second bridge above) is concerned, so dragging is never a
//! second source of truth for `selected`. Release settles the track with
//! the same scroller-only `scrollBy`-by-delta paging mechanism
//! buttons/keyboard already use, not a hand-rolled offset -- see
//! [`CarouselContent`]'s own "Release settle" doc. All three are
//! `document::eval` call sites, and
//! all three no-op harmlessly off a real document (native/Blitz, or a
//! plain `cargo test`), the same as every other un-gated `document::eval`
//! helper in `crate::lib` (`use_outside_dismiss`, `use_form_reset_listener`,
//! ...): this module's rendered markup never branches on `feature = "web"`
//! at all, so unlike the native-`<dialog>` overlays there is nothing here
//! for a `feature = "web"` gate to protect.
//!
//! # Accessibility
//!
//! Follows `dev-docs/research/carousel-2026-09-19.md` §1.2/§5.2's account
//! of the APG Carousel pattern's basic style: [`Carousel`] is
//! `role="region"` + `aria-roledescription="carousel"` with a **required**
//! accessible name (a `tracing::warn!` when neither `aria-label` nor
//! `aria-labelledby` is supplied -- shadcn's own `carousel.tsx` leaves this
//! entirely to the caller, undocumented; see the research's §1.4 gap
//! table), each [`CarouselItem`] is `role="group"` +
//! `aria-roledescription="slide"` with a default `"{n} of {m}"` label
//! (APG's own sanctioned exception to "don't put position/size in an
//! accessible name," since `group` supports neither `aria-setsize` nor
//! `aria-posinset`), and [`CarouselPrevious`]/[`CarouselNext`] are native
//! `<button>`s, genuinely `disabled` at the ends (v1 has no `loop`, so the
//! ends are real, reachable boundaries, not merely visual ones). A root-level
//! `ArrowLeft`/`ArrowRight` (or `ArrowUp`/`ArrowDown` for
//! [`CarouselOrientation::Vertical`]) key is a Radix/shadcn-style
//! addition, not an APG requirement (research §1.2/§1.5) -- RTL-swapped
//! via [`crate::direction`], Radix's convention rather than shadcn's own
//! (shadcn's `handleKeyDown` never swaps; it relies on embla's engine
//! silently compensating underneath it, which this crate does not have).

use crate::{
    collection::{collection_item, use_collection_provider, use_item, CollectionState},
    direction::{use_direction, Direction, HorizontalNav},
    fold_style_attributes, has_own_accessible_name, merge_attributes, use_controlled,
    use_effect_cleanup, use_id_or, use_previous, use_unique_id,
};
use dioxus::prelude::*;
use dioxus_attributes::attributes;
use dioxus_core::Task;
use dioxus_sdk_time::sleep;
use std::time::Duration;

/// The axis a [`Carousel`] pages along.
///
/// Matches [`crate::resizable::ResizableDirection`]'s own
/// horizontal/vertical shape and `data-orientation` convention. Vertical
/// support fell out of the horizontal construction for free (the same
/// component, an axis-swapped CSS declaration and scroll-delta axis, and
/// `ArrowUp`/`ArrowDown` in place of the RTL-aware
/// `ArrowLeft`/`ArrowRight` pair), so it ships in v1 rather than being
/// deferred -- see the module doc's "Scope" section for what *is*
/// deferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CarouselOrientation {
    /// Slides are arranged left-to-right (or right-to-left under RTL) and
    /// paged with `ArrowLeft`/`ArrowRight`.
    #[default]
    Horizontal,
    /// Slides are arranged top-to-bottom and paged with
    /// `ArrowUp`/`ArrowDown` -- never RTL-aware (see the private
    /// `carousel_key_intent`, this module's own key-resolution function).
    Vertical,
}

impl CarouselOrientation {
    /// Returns `"horizontal"` or `"vertical"` -- the exact string this
    /// module's `data-orientation` styling hook uses.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Horizontal => "horizontal",
            Self::Vertical => "vertical",
        }
    }
}

/// Where each [`CarouselItem`] snaps to rest against
/// [`CarouselContent`]'s own scrollport -- matches Embla's (and shadcn's
/// own `opts={{ align }}`) three-way `start`/`center`/`end`, and maps
/// directly onto the CSS Scroll Snap spec's own `scroll-snap-align`
/// keywords of the same names (this crate's own [`CarouselItem`] sets
/// `scroll-snap-align` to whichever one is in force -- see
/// [`Self::as_str`]).
///
/// Every place this module computes "where a slide rests" -- the explicit
/// `scrollBy`-by-delta paging call ([`CAROUSEL_SCROLL_TO_JS`]), the
/// "which slide is nearest" search a native scroll/drag release settles
/// to ([`CAROUSEL_SCROLL_TRACKING_JS`]/[`CAROUSEL_DRAG_JS`]) -- reads this
/// value and anchors on the matching point (an item/the scrollport's own
/// leading edge, midpoint, or trailing edge) rather than always the
/// leading edge, so a caller who sets `align: CarouselAlign::Center` (for
/// example) gets a *consistent* rest position regardless of whether the
/// user paged there with a button, the keyboard, a drag, or native
/// wheel/trackpad/touch scrolling (the last of which the browser itself
/// already snaps correctly from `scroll-snap-align` alone -- this value
/// only needs to reach this module's own JS so its explicit paging/nearest-
/// slide logic agrees with what the browser will have already done
/// natively).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CarouselAlign {
    /// The item's leading edge rests against the scrollport's leading
    /// edge -- shadcn's own default (`opts={{ align: "start" }}` in every
    /// one of its demos that sets `align` explicitly, and Embla's
    /// documented default even where a demo omits the option entirely).
    #[default]
    Start,
    /// The item's midpoint rests against the scrollport's midpoint.
    Center,
    /// The item's trailing edge rests against the scrollport's trailing
    /// edge.
    End,
}

impl CarouselAlign {
    /// Returns `"start"`, `"center"`, or `"end"` -- both this module's own
    /// JS anchor-point argument and the exact keyword [`CarouselItem`]'s
    /// `scroll-snap-align` uses, so a single value serves both without a
    /// second mapping to keep in sync.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Center => "center",
            Self::End => "end",
        }
    }
}

/// # Gap model
///
/// shadcn's own carousel puts the inter-slide gap INSIDE each slide's own
/// border-box (a leading-edge `padding-inline`/`padding-block`), with
/// `CarouselContent`'s own scroller carrying the exactly-compensating
/// negative margin on that same edge/axis -- the standard Tailwind
/// `-ml-4`/`pl-4` idiom -- rather than a real flex `gap` between items.
/// This crate ports that construction (backlog row 91's shadcn-parity
/// addendum) via [`item_gap_padding`]/[`content_gap_margin`] below, exposed
/// as one custom property, `--dx-carousel-gap` (raw-primitive default
/// `0px` -- a themed stylesheet is what gives it an actual token-based
/// value, e.g. `--dx-space-4`, matching this module's own
/// "structural-only, zero-theme-CSS" posture for every other layout-critical
/// declaration in this file).
///
/// Why this matters, not just "matches shadcn": a real flex `gap` is
/// ADDITIVE to a percentage `flex-basis` -- `N` items at `flex-basis:
/// calc(100%/N)` plus `(N-1)` real gaps demand more main-axis size than the
/// container has, by exactly `(N-1) * gap`, so the track overflows and the
/// visible slide count is no longer a whole number (see
/// `dev-docs/backlog.md` row 91's shadcn-carousel-parity research for the
/// measured "~2.3 slides visible" incident this construction closes with no
/// compensating arithmetic needed at the call site). With the gap living
/// INSIDE each item's own border-box instead, `N` items' basis fractions
/// still sum to exactly 100% of the (`-ml`-widened) content box regardless
/// of `N`, so [`CarouselItem`]/[`CarouselVirtualContent`]'s own
/// `--dx-carousel-per-view`/`--dx-carousel-peek` basis `calc()` (see their
/// own "Sizes" doc) never needs to subtract a gap term at all.
fn item_gap_padding(orientation: CarouselOrientation) -> &'static str {
    match orientation {
        CarouselOrientation::Horizontal => "padding-inline-start:var(--dx-carousel-gap, 0px);",
        CarouselOrientation::Vertical => "padding-block-start:var(--dx-carousel-gap, 0px);",
    }
}

/// # Sizes (whole slides by default, opt-in peek)
///
/// [`CarouselItem`]/[`CarouselVirtualContent`]'s own default `flex-basis`
/// is `calc((100% - var(--dx-carousel-peek, 0%)) / var(--dx-carousel-per-view, 1))`
/// -- `--dx-carousel-per-view` (an integer, default `1`) is how many WHOLE
/// slides show at once, and `--dx-carousel-peek` (a percentage of one
/// slide's own share of the track, default `0%`) is how much of the
/// *next* slide additionally peeks into view, opt-in only. With both left
/// at their defaults this is exactly `100%` -- byte-identical to the
/// pre-this-feature hardcoded value, so every existing caller (anything
/// that overrides `flex-basis` directly via an inline `style`, e.g. this
/// package's own pre-shadcn-parity `multiple` variant) is unaffected: an
/// inline `style`-supplied `flex-basis` still simply appears later in the
/// same `style` attribute and wins, exactly as it always has (this
/// module's own established "caller style always wins" construction,
/// [`fold_style_attributes`]'s own call sites throughout this file).
///
/// Because the gap (see the "Gap model" doc above) already lives inside
/// each item's own border-box as padding, this `calc()` never needs to
/// subtract a gap term itself -- `N` whole slides' basis fractions already
/// sum to exactly `100%` of the (gap-widened) content box regardless of
/// `N`.
fn item_basis_style() -> &'static str {
    "flex:0 0 calc((100% - var(--dx-carousel-peek, 0%)) / var(--dx-carousel-per-view, 1));"
}

/// The mirror of [`item_gap_padding`], applied to [`CarouselContent`]'s own
/// scroller element (see that function's own doc for the full construction):
/// a negative margin on the identical edge/axis, so the FIRST item's own
/// leading-edge padding lands flush with the viewport's own clip boundary
/// (`CarouselContent`'s own `carousel-viewport` wrapper) rather than shifting
/// every slide's visible content rightward/downward by one gap's worth.
fn content_gap_margin(orientation: CarouselOrientation) -> &'static str {
    match orientation {
        CarouselOrientation::Horizontal => {
            "margin-inline-start:calc(var(--dx-carousel-gap, 0px) * -1);"
        }
        CarouselOrientation::Vertical => {
            "margin-block-start:calc(var(--dx-carousel-gap, 0px) * -1);"
        }
    }
}

/// Clamp `index` into the valid range for a carousel of `count` slides
/// (`0` when `count` is `0`, otherwise `[0, count - 1]`). The one
/// definition every path that could produce an out-of-range index
/// (a controlled `value` prop, a stale `selected` before the first
/// [`CarouselItem`] has registered, a JS-reported scroll position) is
/// funneled through, so a caller passing e.g. `value: Some(99)` on a
/// 3-slide carousel can never desync [`CarouselApi::can_scroll_next`]
/// from what is actually rendered.
fn clamp_selected(index: usize, count: usize) -> usize {
    if count == 0 {
        0
    } else {
        index.min(count - 1)
    }
}

/// Defensive clamp for a `(start, end)` visible-range reported by a JS
/// settle bridge -- guards against a reply that raced a shrinking `count`
/// (an item unregistering between the geometry read and this signal
/// write). Never lets either end point past the last valid index, and
/// never lets `start` exceed `end` once clamped.
fn clamp_visible_range(range: (usize, usize), count: usize) -> (usize, usize) {
    if count == 0 {
        return (0, 0);
    }
    let last = count - 1;
    let start = range.0.min(last);
    let end = range.1.min(last).max(start);
    (start, end)
}

/// Whether a "previous" action is currently meaningful. v1 has no `loop`
/// (research §8.2), so this is a real, reachable boundary -- `index == 0`
/// -- not a cosmetic one.
fn can_scroll_prev(selected: usize) -> bool {
    selected > 0
}

/// Whether a "next" action is currently meaningful -- the mirror of
/// [`can_scroll_prev`]. `count == 0` (no slides registered yet, e.g. the
/// very first render before any [`CarouselItem`] has mounted) is never
/// scrollable either way.
fn can_scroll_next(selected: usize, count: usize) -> bool {
    count > 0 && selected + 1 < count
}

/// The index one step forward from `selected`, saturating at the last
/// slide (never wrapping -- see [`can_scroll_next`]'s doc).
fn next_selected(selected: usize, count: usize) -> usize {
    if count == 0 {
        0
    } else {
        (selected + 1).min(count - 1)
    }
}

/// The index one step back from `selected`, saturating at `0` (never
/// wrapping -- see [`can_scroll_prev`]'s doc).
fn prev_selected(selected: usize) -> usize {
    selected.saturating_sub(1)
}

/// The index one step back from `selected`, honoring `loop_enabled` --
/// approved fast-follow decision (backlog row 91, `dev-docs/research/carousel-2026-09-19.md`
/// §8.2): **rewind-style** looping. From the first slide, wraps to the
/// last (`count - 1`) rather than saturating; the actual scroll is still a
/// single [`CAROUSEL_SCROLL_TO_JS`] paging call to that far index
/// (a visible "rewind" across every intervening slide), not an
/// embla-style cloned-node illusion -- clones would violate the overscroll
/// port's own invariant 5 (`dev-docs/research/carousel-overscroll-2026-09-23.md`
/// §6): they would enter the snap engine's own candidate list, the "N of
/// M" slide count, and `:nth-child` styling. Falls back to
/// [`prev_selected`]'s ordinary saturating behavior when `loop_enabled` is
/// `false` (still v1's default) or `count == 0` (nothing to wrap to
/// either way).
fn step_prev(selected: usize, count: usize, loop_enabled: bool) -> usize {
    if loop_enabled && count > 0 {
        if selected == 0 {
            count - 1
        } else {
            selected - 1
        }
    } else {
        prev_selected(selected)
    }
}

/// The mirror of [`step_prev`]: one step forward, wrapping from the last
/// slide back to the first under `loop_enabled`.
fn step_next(selected: usize, count: usize, loop_enabled: bool) -> usize {
    if loop_enabled && count > 0 {
        (selected + 1) % count
    } else {
        next_selected(selected, count)
    }
}

/// The shortest signed step, in `[-(count/2), count/2]`, from data index
/// `old` to data index `new` modulo `count` -- [`CarouselVirtualContent`]'s
/// own seamless-loop paging math: the position nearest the current anchor
/// that carries data index `new` is `anchor + shortest_signed_delta(old,
/// new, count)`, for any `old`/`new` in `[0, count)`. `count == 0` (or
/// `old == new`, which can't otherwise arise from two distinct valid data
/// indices) returns `0` -- a caller-checked precondition
/// (`old != new`, both `< count`) rather than a panic, so a defensive
/// double-check costs nothing.
fn shortest_signed_delta(old: usize, new: usize, count: usize) -> isize {
    if count == 0 || old == new {
        return 0;
    }
    let n = count as isize;
    let raw = new as isize - old as isize;
    let mut wrapped = ((raw % n) + n) % n;
    if wrapped > n / 2 {
        wrapped -= n;
    }
    wrapped
}

/// The default accessible name for a slide with no name of its own:
/// `"{one-based index} of {count}"`. APG's own sanctioned exception to
/// "don't encode position/size in an accessible name" -- `role="group"`
/// supports neither `aria-setsize` nor `aria-posinset`
/// (dev-docs/research/carousel-2026-09-19.md §1.2).
fn slide_label(index: usize, count: usize) -> String {
    format!("{} of {}", index + 1, count)
}

/// Whether [`Carousel`]'s own mount-time paging effect (below,
/// in its component body) should ask [`CAROUSEL_SCROLL_TO_JS`] for
/// an instant jump
/// (`true`) rather than a smooth transition (`false`) on this run.
///
/// `true` on exactly one run per mount: the first one where `has_items`
/// is `true` (registration has completed), regardless of `is_first`'s
/// *own* history of early-returning runs before that point -- see the
/// effect's own doc for why gating on `has_items` rather than "did this
/// particular id lookup succeed" is what makes it impossible for a real
/// user navigation to ever inherit this `true` (round5's own fixed bug).
/// Never `true` again afterward, once the caller has also set `is_first`
/// to `false` on any run where `has_items` was `true` (this function
/// itself is pure and does not do that write -- see the call site).
fn is_mount_settle(is_first: bool, has_items: bool) -> bool {
    has_items && is_first
}

/// Resolve a keydown on (or bubbled up to) the [`Carousel`] root to a
/// paging intent, or `None` for every other key.
///
/// [`HorizontalNav`] is [`crate::direction`]'s own "which arrow key means
/// which paging intent" result type; reused here as a plain `Prev`/`Next`
/// result rather than duplicating an identical two-variant enum under a
/// new name -- nothing about its own definition or `resolve_horizontal`'s
/// doc restricts it to a *horizontal* caller; only its name does, and
/// this function's own doc is where that reuse is recorded. For
/// [`CarouselOrientation::Horizontal`] this defers entirely to
/// [`Direction::resolve_horizontal`], so it swaps under `dir="rtl"`
/// exactly the way `tabs.rs`'s roving focus does. For
/// [`CarouselOrientation::Vertical`], `ArrowUp`/`ArrowDown` are used
/// unconditionally -- direction never affects a vertical axis (see
/// `Direction::resolve_horizontal`'s own doc).
fn carousel_key_intent(
    key: &Key,
    orientation: CarouselOrientation,
    direction: Direction,
) -> Option<HorizontalNav> {
    match orientation {
        CarouselOrientation::Horizontal => direction.resolve_horizontal(key),
        CarouselOrientation::Vertical => match key {
            Key::ArrowUp => Some(HorizontalNav::Prev),
            Key::ArrowDown => Some(HorizontalNav::Next),
            _ => None,
        },
    }
}

/// Fire-and-forget: scroll the carousel's own scroller element
/// (`scrollerId`) by the physical pixel delta needed to align
/// `targetId`'s slide with the scroller's own leading edge, along the
/// carousel's own axis only -- `scroller.scrollBy({ left: delta })` (or
/// `top` for [`CarouselOrientation::Vertical`]).
///
/// Replaced a `target.scrollIntoView()` call (the original v1/round5
/// construction; see backlog row 91's dated addendum for the incident
/// this replaced it over): `scrollIntoView` walks and scrolls *every*
/// scrollable ancestor between the target and the viewport, including the
/// document itself, in order to bring the target fully into view -- which
/// is exactly the "autoplay/Next drags the whole page back to the
/// carousel" bug the owner reported on the live site, not merely a
/// coincidental side effect of it. `Element.scrollBy()`, called directly
/// on the scroller, only ever writes that one element's own scroll
/// position, so there is no ancestor for it to reach in the first place --
/// this is a *by-construction* fix, not a guard against the symptom.
///
/// The delta is a `getBoundingClientRect()` difference -- a physical
/// pixel value -- so, per invariant 1 of
/// `dev-docs/research/carousel-overscroll-2026-09-23.md` §6, it needs no
/// RTL sign-flip: `scrollBy`'s delta is always physical, the same
/// property this module's own drag gesture (`CAROUSEL_DRAG_JS`'s
/// `feedOverdrag`) already relies on for its own `scrollBy` calls. The
/// `endDrag` release settle below duplicates this exact computation
/// rather than importing it -- a fresh `document::eval` string is its own
/// standalone script with no module system to import across (see
/// [`CarouselContent`]'s own "Release settle" doc for the established
/// precedent of duplicating rather than sharing JS this way).
///
/// `instant` forces `behavior: 'auto'` on the very first call (mount), so
/// a non-zero `default_value` does not visibly animate in on page load;
/// every later call additionally respects `prefers-reduced-motion` the
/// same way. No response is read back -- [`CAROUSEL_SCROLL_TRACKING_JS`]
/// is what keeps `selected` in sync with wherever the scroll position
/// actually ends up.
///
/// `scroll-snap-type` is suspended for the duration of the scroll, then
/// restored once it actually finishes -- the exact same construction
/// [`CAROUSEL_DRAG_JS`]'s own release settle already uses, and for the
/// identical reason (that constant's own "Release settle" doc has the
/// full account): found live in this session, `scrollBy({ behavior:
/// 'smooth' })` on a `scroll-snap-type: ... mandatory` container, left
/// enabled, truncates a multi-slide jump at the FIRST intervening snap
/// point rather than reaching the requested delta -- confirmed with a
/// direct, Dioxus-independent repro (a bare `el.scrollBy({left: 672,
/// behavior: 'smooth'})` on this exact element landed at `336`, one slide
/// short, with snapping left enabled; the identical call reached `672`
/// exactly with `scrollSnapType` set to `'none'` first). `scrollIntoView`
/// (what this replaced) never had this failure mode, so it was never
/// visible before this construction existed -- a single-slide step (the
/// overwhelmingly common case) happens to truncate at the only snap point
/// in its path anyway, which is also the correct destination, so it read
/// as correct in isolation; a multi-slide jump (`loop`'s rewind
/// wraparound, a `CarouselTab` activation more than one tab away,
/// [`CarouselApi::scroll_to`] to a distant index) is what exposed it, via
/// a pre-existing, previously-green `carousel.spec.ts` test
/// (`clicking a tab activates its slide and moves the roving tab stop`,
/// jumping tab 1 -> tab 3) going red the moment this construction landed
/// without the fix below. Since a fresh `document::eval` call has no
/// closure shared with any other call (unlike `CAROUSEL_DRAG_JS`'s single
/// long-lived script), the "is a restore already pending" hand-off
/// [`CAROUSEL_DRAG_JS`]'s `onPointerDown` keeps in a closure variable is
/// instead stashed directly on the scroller element itself
/// (`scroller.__dxCancelSnapRestore`) -- a plain DOM property survives
/// across independent eval calls the same way the element's own
/// `style.scrollSnapType` already does, so two paging calls in quick
/// succession (before the first one's own scroll has finished) still
/// cancel-and-replace rather than race a stale restore into firing mid-
/// second-scroll. Reuses [`CAROUSEL_SNAP_RESTORE_FALLBACK_MS`] for the
/// same non-`scrollend`-browser fallback [`CAROUSEL_DRAG_JS`] already
/// needs it for.
const CAROUSEL_SCROLL_TO_JS: &str = "\
    const [scrollerId, targetId, orientation, instant, snapRestoreFallbackMs, align] = await dioxus.recv();
    const behavior = (instant || window.matchMedia('(prefers-reduced-motion: reduce)').matches)
        ? 'auto' : 'smooth';
    const scroller = document.getElementById(scrollerId);
    const target = document.getElementById(targetId);
    // The point on `rect`, along this carousel's own axis, that
    // `align` (CarouselAlign::as_str()'s own 'start'/'center'/'end')
    // rests against -- applied identically to the target slide and the
    // scroller's own viewport rect below, so e.g. `align: 'center'`
    // aligns the slide's midpoint with the viewport's midpoint rather
    // than always the leading edge. Mirrors the native `scroll-snap-align`
    // keyword `CarouselItem` already sets to the same value -- this is
    // what keeps an explicit `scrollBy`-by-delta paging call consistent
    // with wherever the browser's own native snapping would have already
    // landed.
    const anchorOf = (rect) => {
        const start = orientation === 'horizontal' ? rect.left : rect.top;
        const end = orientation === 'horizontal' ? rect.right : rect.bottom;
        if (align === 'center') return (start + end) / 2;
        if (align === 'end') return end;
        return start;
    };
    // A real drag (CAROUSEL_DRAG_JS) already owns the scroll position and
    // `scroll-snap-type` for as long as `data-dragging` is set -- found
    // live in this session: `use_carousel_scroll_tracking` reports a
    // changing `selected` continuously WHILE a drag is still in progress
    // (every incremental `feedOverdrag` scrollBy fires its own `scrollend`
    // too), so this effect fires repeatedly mid-gesture regardless of this
    // fix. The old `target.scrollIntoView()` tolerated that safely --
    // already-near-enough was a harmless no-op (this effect's own Rust-side
    // doc comment already documents this exact case). A `scrollBy`-by-delta
    // is not equally forgiving: it competes with the drag's own direct-
    // manipulation `scrollBy` calls for the same scroll position, and (mode
    // B3's own already-measured failure, `CAROUSEL_DRAG_JS`'s 'Pointer
    // drag' doc) its own scroll-snap-type restore firing mid-drag
    // re-enables mandatory snapping before the gesture ends, which then
    // fights every remaining `feedOverdrag` call and sends the drag flying
    // several slides past where the pointer actually stopped -- reproduced
    // live as every pointer-drag test in this file overshooting by exactly
    // 2 slide-pitches. Skipping entirely while `data-dragging` is set
    // defers this bridge to the drag's own release settle, which already
    // performs the identical scroll-to-nearest-slide alignment once the
    // gesture actually ends.
    if (scroller && target && !scroller.hasAttribute('data-dragging')) {
        const s = scroller.getBoundingClientRect();
        const t = target.getBoundingClientRect();
        const delta = anchorOf(t) - anchorOf(s);
        // A genuine no-op (already aligned -- the mount settle when
        // `default_value` needs no jump, or `use_carousel_scroll_tracking`
        // reporting the position a native scroll already reached) must
        // touch NOTHING, the same as this replaced `scrollIntoView` on an
        // already-fully-visible element: `scrollBy({ left: 0 })` never
        // fires `scrollend` (nothing scrolled), so suspending
        // `scroll-snap-type` for a scroll that is never going to happen
        // would leave it stuck at `none` forever -- found live in this
        // session (the mount settle's own zero-delta call is the common
        // case that hits this on every single carousel).
        if (Math.abs(delta) < 1) {
            return;
        }
        // Cancel a still-pending restore from a previous paging call, the
        // same idiom CAROUSEL_DRAG_JS's own onPointerDown uses -- see this
        // constant's own doc for why this is stashed on the element rather
        // than a closure variable.
        if (scroller.__dxCancelSnapRestore) {
            scroller.__dxCancelSnapRestore();
            scroller.__dxCancelSnapRestore = null;
        }
        scroller.style.scrollSnapType = 'none';
        if (orientation === 'horizontal') {
            scroller.scrollBy({ left: delta, behavior });
        } else {
            scroller.scrollBy({ top: delta, behavior });
        }
        const restoreSnap = () => {
            scroller.style.scrollSnapType = orientation === 'horizontal' ? 'x mandatory' : 'y mandatory';
            scroller.__dxCancelSnapRestore = null;
        };
        if ('onscrollend' in window) {
            scroller.addEventListener('scrollend', restoreSnap, { once: true, passive: true });
            scroller.__dxCancelSnapRestore = () => scroller.removeEventListener('scrollend', restoreSnap);
        } else {
            const timer = setTimeout(restoreSnap, snapRestoreFallbackMs);
            scroller.__dxCancelSnapRestore = () => clearTimeout(timer);
        }
    }";

/// Long-lived (mount-to-unmount): translate a [`CarouselContent`]
/// element's *actual* scroll position into a slide index, so `selected`
/// stays correct after a native drag/wheel/trackpad scroll -- one this
/// crate never drove via [`CAROUSEL_SCROLL_TO_JS`] itself.
///
/// Deliberately not an `IntersectionObserver` (the shape
/// `dev-docs/research/carousel-2026-09-19.md` §2.2 sketches as the
/// primary mechanism): that needs live per-item intersection-ratio
/// bookkeeping and threshold tuning to avoid mid-animation flicker
/// between two partially-visible neighbors. Settling once, on
/// `scrollend` (falling back to a debounced plain `scroll` listener where
/// `scrollend` is absent -- Safari only since v26.2 per the research's
/// own caniuse check, ~90% global today) and then comparing every
/// slide's `getBoundingClientRect()` against the container's own, is
/// simpler, fires exactly once per gesture (no flicker to begin with),
/// and is still zero-JS-dependency, feature-detected, degrade-not-break,
/// matching this repo's own platform-features-first posture
/// (`dev-docs/recommended-implementations.md`'s `<dialog>` finding).
/// Comparing viewport (`getBoundingClientRect`) positions rather than
/// `scrollLeft`/`offsetLeft` sidesteps the well-known cross-browser
/// disagreement over what `scrollLeft` itself means under `dir="rtl"` --
/// a `getBoundingClientRect` is always a physical, direction-agnostic
/// viewport rectangle regardless of `dir`.
///
/// One read (`getBoundingClientRect`, of the container and every slide)
/// followed by nothing but a `dioxus.send` -- no DOM write at all in this
/// callback -- so there is no read-then-write-layout main-thread hazard
/// (`dev-docs/recommended-implementations.md` §11 Rule 1) to begin with.
/// Both listeners are `passive: true` (§11 Rule 2).
const CAROUSEL_SCROLL_TRACKING_JS: &str = "\
    const [id, orientation, align] = await dioxus.recv();
    const container = document.getElementById(id);
    if (!container) {
        await dioxus.recv();
        return;
    }
    // See `CAROUSEL_SCROLL_TO_JS`'s own identical helper's doc: which
    // slide counts as 'nearest' (and therefore `selected`) must anchor on
    // the same point `align` snaps to, so this agrees with wherever the
    // browser's own native scroll-snap settling already landed.
    const anchorOf = (start, end) => {
        if (align === 'center') return (start + end) / 2;
        if (align === 'end') return end;
        return start;
    };
    let lastNearest = null;
    let lastNearestPos = null;
    let lastVisStart = null;
    let lastVisEnd = null;
    const settle = () => {
        // Freeze rule (loop-a11y-guidance.md §7 / carousel-overscroll-2026-09-23.md
        // §6): a live mouse/pen drag (`CAROUSEL_DRAG_JS`) fires its own
        // 'scroll'/'scrollend' on every single incremental `scrollBy` it
        // issues -- confirmed live (`CAROUSEL_SCROLL_TO_JS`'s own doc,
        // 'A real drag already owns the scroll position' paragraph) -- so
        // without this guard `selected` (and, now, the visible range this
        // drives `inert` from) would change continuously mid-drag rather
        // than only at rest. `data-dragging` is removed at the very start
        // of `endDrag`, before its own release-settle `scrollBy` runs, so
        // this never suppresses the genuine post-release settle -- only
        // the mid-gesture noise. `CAROUSEL_DRAG_JS`'s own release settle
        // reuses this exact geometry search for the identical reason (see
        // that constant's own 'Release settle' doc) and is what a drag
        // defers to instead.
        if (container.hasAttribute('data-dragging')) {
            return;
        }
        const children = Array.from(container.children);
        if (children.length === 0) {
            return;
        }
        const containerRect = container.getBoundingClientRect();
        const containerStart = orientation === 'horizontal' ? containerRect.left : containerRect.top;
        const containerEnd = orientation === 'horizontal' ? containerRect.right : containerRect.bottom;
        let nearest = 0;
        let nearestPos = 0;
        let nearestDist = Infinity;
        // Which item indices are actually inside the viewport right now --
        // an item counts as visible if its own midpoint falls within the
        // container's rect (\"at least half the slide is in view\", not a
        // stricter full-containment test, which would flicker a slide out
        // right at a scroll-snap boundary's own sub-pixel settle, nor a
        // looser any-overlap test, which would count a 1px sliver of a
        // fully-clipped neighbour as \"visible\" for a multi-per-view
        // layout with no peeking at all). Contiguous by construction (flex
        // row/column, no reordering), so only the first/last matching
        // index need to be kept.
        //
        // Every mapping from a DOM child to a slide reads `data-index`
        // (the stable data index -- may repeat across children in a
        // virtualised loop window smaller than the data set) and
        // `data-position` (the unique logical slot -- equals `data-index`
        // for the plain children API, so this one code path serves both;
        // `CarouselVirtualContent`'s own doc has the fuller account) --
        // never the child's own array position `i`. A bench regression
        // this generalises against: a single leading non-slide DOM child
        // sent an index-position-based version of this exact search into
        // a runaway (`dev-docs/research/carousel-loop-2026-09-25/loop-libraries.md`).
        let visStart = null;
        let visEnd = null;
        children.forEach((child) => {
            const dataIndex = parseInt(child.dataset.index, 10);
            const position = child.dataset.position !== undefined
                ? parseInt(child.dataset.position, 10)
                : dataIndex;
            if (Number.isNaN(dataIndex)) {
                return;
            }
            const rect = child.getBoundingClientRect();
            const childStart = orientation === 'horizontal' ? rect.left : rect.top;
            const childEnd = orientation === 'horizontal' ? rect.right : rect.bottom;
            const dist = Math.abs(
                anchorOf(childStart, childEnd) - anchorOf(containerStart, containerEnd)
            );
            if (dist < nearestDist) {
                nearestDist = dist;
                nearest = dataIndex;
                nearestPos = position;
            }
            const center = (childStart + childEnd) / 2;
            if (center >= containerStart && center <= containerEnd) {
                if (visStart === null) {
                    visStart = dataIndex;
                }
                visEnd = dataIndex;
            }
        });
        if (visStart === null) {
            visStart = nearest;
            visEnd = nearest;
        }
        // One wire shape for both message kinds (`[tag, a, b]`) -- `recv`
        // deserializes into a single fixed Rust tuple type regardless of
        // which fired. For 'selected', `b` is the nearest slide's own
        // logical POSITION (not its data index -- these disagree exactly
        // when a virtualised loop window is smaller than the data set),
        // consumed only by a caller that tracks an anchor position
        // (`CarouselVirtualContent`); the plain children API ignores it,
        // since position and data index are the same value there.
        if (nearest !== lastNearest || nearestPos !== lastNearestPos) {
            lastNearest = nearest;
            lastNearestPos = nearestPos;
            dioxus.send(['selected', nearest, nearestPos]);
        }
        if (visStart !== lastVisStart || visEnd !== lastVisEnd) {
            lastVisStart = visStart;
            lastVisEnd = visEnd;
            dioxus.send(['visible', visStart, visEnd]);
        }
    };
    const supportsScrollEnd = 'onscrollend' in window;
    let debounceTimer = null;
    const onScroll = () => {
        if (supportsScrollEnd) {
            return;
        }
        clearTimeout(debounceTimer);
        debounceTimer = setTimeout(settle, 120);
    };
    container.addEventListener('scroll', onScroll, { passive: true });
    if (supportsScrollEnd) {
        container.addEventListener('scrollend', settle, { passive: true });
    }
    // A resize of the scroller's own box re-settles too -- not just a
    // scroll -- closing a real, measured mount-time race (this session):
    // `CarouselContent`'s own layout depends on the themed wrapper's
    // *external* stylesheet (`document::Link`, loaded asynchronously),
    // which can still be in flight the instant this bridge attaches, so
    // the very first geometry read can measure a temporarily wider box
    // (found live: 544px before `.dx-carousel`'s own `padding-inline`
    // reservation applied, 448px after) -- and for a `default_value == 0`
    // carousel that never scrolls again on its own, that first, too-early
    // read would otherwise never be corrected, permanently over-widening
    // the visible range one slide's worth. `ResizeObserver`'s own first
    // callback fires once for the box's *current* size the moment
    // `observe()` is called (so this alone establishes the initial visible
    // range even if the carousel never scrolls at all -- the mount case
    // `CAROUSEL_SCROLL_TO_JS`'s own zero-delta settle used to special-case,
    // now redundant and removed), and fires again on every subsequent
    // genuine size change (that stylesheet finishing, a responsive
    // breakpoint, a caller resizing the wrapper) -- a self-correcting
    // construction for the whole class of 'the box was momentarily the
    // wrong size when this measured it', not a fix for this one instance.
    const resizeObserver = new ResizeObserver(() => settle());
    resizeObserver.observe(container);
    await dioxus.recv();
    container.removeEventListener('scroll', onScroll);
    if (supportsScrollEnd) {
        container.removeEventListener('scrollend', settle);
    }
    resizeObserver.disconnect();
    clearTimeout(debounceTimer);";

/// Attach [`CAROUSEL_SCROLL_TRACKING_JS`] to the element with the given
/// `id` for as long as the calling component stays mounted, forwarding
/// every reported `('selected', data_index, position)` message to
/// `set_selected` (and, if present, `on_position`) and every `('visible',
/// start, end)` message to `visible_range` -- the a11y contract's own
/// "visible" set (module doc, "Accessibility"; see [`CarouselItem`]'s own
/// "Visible slides / `inert`" doc). Mirrors `crate::lib`'s
/// `use_outside_dismiss`/`use_form_reset_listener` shape exactly: an
/// initial `eval.send(..)` of the (id, orientation) the script's own
/// `await dioxus.recv()` unpacks, a spawned task looping on `eval.recv()`
/// for as long as the script keeps sending, and a cleanup closure that
/// sends a teardown value so the script's own trailing `await
/// dioxus.recv()` can resolve and remove its listeners before the element
/// is gone.
///
/// `on_position`, when `Some`, is called with the nearest slide's own
/// logical POSITION (`data-position`, may differ from its data index) --
/// used only by [`CarouselVirtualContent`] to keep its own anchor
/// bookkeeping in sync with a native drag/wheel/trackpad settle (or with
/// its own animated paging call finishing); `None` for the plain children
/// API ([`CarouselContent`]), where position and data index always agree.
fn use_carousel_scroll_tracking(
    id: impl Readable<Target = String> + Copy + 'static,
    orientation: ReadSignal<CarouselOrientation>,
    align: ReadSignal<CarouselAlign>,
    set_selected: Callback<usize>,
    count: Memo<usize>,
    mut visible_range: Signal<(usize, usize)>,
    on_position: Option<Callback<isize>>,
) {
    crate::use_effect_with_cleanup(move || {
        let id = id.cloned();
        let orientation_str = orientation().as_str().to_string();
        let align_str = align().as_str().to_string();
        let mut eval = document::eval(CAROUSEL_SCROLL_TRACKING_JS);
        let _ = eval.send((id, orientation_str, align_str));
        spawn(async move {
            while let Ok((kind, a, b)) = eval.recv::<(String, i64, i64)>().await {
                if kind == "selected" {
                    set_selected.call(a.max(0) as usize);
                    if let Some(on_position) = on_position {
                        on_position.call(b as isize);
                    }
                } else {
                    let count_now = *count.peek();
                    let a = a.max(0) as usize;
                    let b = b.max(0) as usize;
                    visible_range.set(clamp_visible_range((a, b), count_now));
                }
            }
        });
        move || {
            let _ = eval.send(true);
        }
    });
}

/// Movement, in CSS pixels, a mouse/pen pointer must travel from its
/// `pointerdown` origin before [`CAROUSEL_DRAG_JS`] treats the gesture as
/// a drag rather than a click -- see [`CarouselContent`]'s own "Pointer
/// drag" doc for what this threshold is for.
const CAROUSEL_DRAG_THRESHOLD_PX: f64 = 5.0;

/// How long, in milliseconds, a suspended `scroll-snap-type` waits for a
/// `scrollend` event before falling back to restoring it unconditionally
/// -- only reached on an engine without `scrollend` support (this
/// module's own doc: Safari before v26.2). Shared by both places that
/// suspend `scroll-snap-type` for a discrete `scrollBy`:
/// [`CAROUSEL_DRAG_JS`]'s own release-time settle, and
/// [`CAROUSEL_SCROLL_TO_JS`]'s every-other-paging-path call (see that
/// constant's own doc for why it needs the identical suspend/restore
/// construction). Comfortably longer than a `scroller.scrollBy({behavior:
/// 'smooth'})` transition normally takes to finish, so the fallback
/// essentially never fires *before* that transition has visibly
/// completed.
const CAROUSEL_SNAP_RESTORE_FALLBACK_MS: f64 = 500.0;

/// Long-lived (mount-to-unmount): a mouse/pen drag-to-scroll gesture on
/// [`CarouselContent`]'s own element, layered on the *same* scroll-snap
/// track [`CAROUSEL_SCROLL_TO_JS`]/[`CAROUSEL_SCROLL_TRACKING_JS`]
/// already use -- not a second, competing engine. See [`CarouselContent`]'s
/// own "Pointer drag" doc for the construction and why each piece is
/// shaped the way it is; the short version: every `scrollBy` this issues
/// fires the exact same native `scroll` events [`CAROUSEL_SCROLL_TRACKING_JS`]
/// (already attached to the same element) already listens for, so a drag
/// settles on `CarouselContext`'s `selected` through that one existing
/// bridge, never a second source of truth for the index.
///
/// Mirrors [`CAROUSEL_SCROLL_TRACKING_JS`]'s own long-lived shape: an
/// initial `dioxus.send` of static config, then `await dioxus.recv()`
/// again only for teardown -- everything in between is real
/// `addEventListener`s this script installs and removes itself, driving
/// no Rust-side state at all while active (see the module doc's "Engine"
/// section for why `document::eval` is this crate's established escape
/// hatch for DOM access -- `setPointerCapture` and a real, ordered
/// capture-phase `click` listener -- that Dioxus's synthetic event props
/// don't reach, the same reasoning `drawer.rs`'s own drag gesture
/// documents for its own escape to `crate::pointer`).
///
/// One non-obvious piece, found only by driving a real drag (round5):
/// `scroll-snap-type: ... mandatory` re-snaps after *every* scroll
/// operation it considers complete, and a plain `scrollBy` -- with
/// nothing telling the browser that many small ones in a row are one
/// continuous gesture, the way a real touch/wheel drag is -- counts as
/// one. Left enabled during the drag, this fought every single
/// `scrollBy` call and sent a short, slow real-mouse drag flying several
/// slides past where the pointer actually stopped. So the drag
/// temporarily sets `el.style.scrollSnapType = 'none'` the moment it
/// starts (after the movement threshold, alongside `data-dragging`).
/// Originally (round5) it was restored right on release, trusting the
/// browser's own re-snap to glide to the nearest slide the same way it
/// would after a real touch/trackpad drag -- measured later (see
/// [`CarouselContent`]'s own "Release settle" doc), that glide never
/// actually happened, since setting the property back on an
/// already-stationary position is a static re-evaluation, not an
/// animated scroll. `endDrag` below settles explicitly instead, and
/// defers restoring this property until that explicit settle has
/// actually finished.
const CAROUSEL_DRAG_JS: &str = "\
    const [id, orientation, thresholdPx, enabled, snapRestoreFallbackMs, align] = await dioxus.recv();
    const el = document.getElementById(id);
    if (!el || !enabled) {
        await dioxus.recv();
        return;
    }
    // See `CAROUSEL_SCROLL_TO_JS`'s own identical helper's doc -- release
    // settle below finds the 'nearest' slide (and the delta to it) by the
    // same align-aware anchor point, so a drag lands consistently with
    // wherever a button/keyboard/native scroll would have.
    const anchorOf = (start, end) => {
        if (align === 'center') return (start + end) / 2;
        if (align === 'end') return end;
        return start;
    };
    const thresholdSq = thresholdPx * thresholdPx;
    let pointerId = null;
    let originX = 0;
    let originY = 0;
    let lastX = 0;
    let lastY = 0;
    let dragging = false;
    let suppressNextClick = false;
    // Cancels whichever of `endDrag`'s two release-settle mechanisms
    // (the `scrollend` listener or its timeout fallback) is currently
    // pending, or null if neither is. See `onPointerDown`'s own comment.
    let cancelPendingSnapRestore = null;

    // -- Edge rubber-band (mode B3, port of the closed design in
    // dev-docs/research/carousel-overscroll-2026-09-23.md; the eight
    // numbered invariants below are that doc's own §6) --
    //
    // A mouse/pen drag has no native scrolling behind it, so THE RULE
    // (research doc §3) permits this path to keep owning the scroll
    // position the way it already did before this port -- unlike the
    // wheel bridge (`CAROUSEL_WHEEL_BOUNCE_JS`, attached separately to
    // this same element), which never may, because a wheel/trackpad
    // gesture has a compositor and a snap engine also trying to write it.
    let rawOver = 0; // signed content displacement the scroller refused, physical px
    let bounceToken = 0; // see `bounceHome`'s own doc -- invariant 4
    const RUBBER_C = 0.55; // WebKit's own published rubber-band constant (research doc §4)

    function axisSize() {
        return (orientation === 'horizontal' ? el.clientWidth : el.clientHeight) || 320;
    }
    function rubberLimit() {
        // Asymptote at trackWidth/0.55 (research doc §4/§5) -- no fixed
        // room to run out of, so the depth can never reach a wall the way
        // a padded-margin approach's would.
        return axisSize() / RUBBER_C;
    }
    function rubber(depth) {
        const L = rubberLimit();
        return (L * depth) / (depth + L);
    }
    function rubberSigned(x) {
        return x < 0 ? -rubber(-x) : rubber(x);
    }
    // Invariant 1: never `scrollLeft`/`scrollTop` -- a `getBoundingClientRect`
    // diff is a physical rectangle regardless of `dir`, so this needs no
    // RTL branch at all.
    function contentOffset() {
        const first = el.children[0];
        if (!first) {
            return 0;
        }
        const r = first.getBoundingClientRect();
        const c = el.getBoundingClientRect();
        return orientation === 'horizontal' ? r.left - c.left : r.top - c.top;
    }
    function applyBounce() {
        if (!rawOver) {
            el.style.transform = '';
            return;
        }
        const depth = rubberSigned(rawOver).toFixed(2);
        el.style.transform =
            orientation === 'horizontal' ? `translateX(${depth}px)` : `translateY(${depth}px)`;
    }
    // Ask the scroller for the whole of `cd` first (a real `scrollBy`,
    // exactly as this drag already issued before this port); whatever it
    // REFUSES -- because it is already at an edge -- is the overdrag. That
    // refusal is how the edge is located: no margin, no scroll-coordinate
    // reasoning, no direction branch. `prefers-reduced-motion` drops the
    // whole effect rather than merely skipping the spring-back animation:
    // this is cosmetic feedback, not functional scrolling, so 'no bounce'
    // is the correct reading of that preference here.
    function feedOverdrag(cd) {
        if (!cd) {
            return;
        }
        if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
            if (rawOver) {
                rawOver = 0;
                applyBounce();
            }
            if (orientation === 'horizontal') {
                el.scrollBy({ left: -cd, behavior: 'instant' });
            } else {
                el.scrollBy({ top: -cd, behavior: 'instant' });
            }
            return;
        }
        const before = contentOffset();
        if (orientation === 'horizontal') {
            el.scrollBy({ left: -cd, behavior: 'instant' });
        } else {
            el.scrollBy({ top: -cd, behavior: 'instant' });
        }
        const moved = contentOffset() - before;
        const refused = cd - moved;
        if (!refused && !rawOver) {
            return;
        }
        const prev = rawOver;
        const next = prev + refused;
        // Collapse through zero rather than let a sign flip leave a
        // residual fractional depth once the scroller has fully absorbed
        // the request -- a corner case the bench's own equivalent
        // (`feedBounce`) does not guard against.
        rawOver = prev !== 0 && prev > 0 !== next > 0 ? 0 : next;
        applyBounce();
    }
    // The rubber-band return, driven by us (never the discrete-paging
    // `scrollBy`-by-delta helper -- invariant 3) so it can be SUPERSEDED
    // rather than merely cancelled: a
    // fresh drag starting mid-ease bumps `bounceToken` (see `onPointerDown`),
    // and the superseded loop below stops touching `rawOver` on its very
    // next frame instead of racing a new gesture's own `feedOverdrag`
    // writes to the same variable -- invariant 4, restated for a token
    // rather than the bench's own boolean guard, which left a stale
    // in-flight loop free to keep writing after being 'cancelled.'
    function bounceHome(done) {
        const reduce = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
        const from = rawOver;
        if (!from || reduce) {
            rawOver = 0;
            applyBounce();
            done && done();
            return;
        }
        const myToken = ++bounceToken;
        const t0 = performance.now();
        const DUR = 340;
        (function step() {
            if (myToken !== bounceToken) {
                return;
            }
            const t = Math.min(1, (performance.now() - t0) / DUR);
            const k = 1 - Math.pow(1 - t, 3); // ease-out cubic
            rawOver = from * (1 - k);
            applyBounce();
            if (t < 1) {
                window.requestAnimationFrame(step);
            } else {
                rawOver = 0;
                applyBounce();
                done && done();
            }
        })();
    }

    const onPointerDown = (e) => {
        // Mouse and pen only -- touch already scrolls this track natively
        // (this module's own 'never intercept touch' rule -- touch-action
        // is never set to 'none' here, see style.css's own comment), and a
        // second pointer going down while one is already tracked is
        // ignored rather than restarting the gesture from the new one.
        if (e.pointerType === 'touch' || pointerId !== null || e.button !== 0) {
            return;
        }
        suppressNextClick = false;
        pointerId = e.pointerId;
        originX = lastX = e.clientX;
        originY = lastY = e.clientY;
        dragging = false;
        // Supersede any spring-back still easing from the previous gesture
        // (invariant 4) -- this new gesture's own `feedOverdrag` is about to
        // start writing `rawOver` again, and a stale `bounceHome` loop must
        // stop touching it rather than race those writes.
        bounceToken++;
        if (cancelPendingSnapRestore !== null) {
            // The previous gesture's own scroll-snap-type restore never
            // got to run -- harmless to abandon: this new drag's own
            // threshold-crossing below sets scroll-snap-type to 'none'
            // unconditionally regardless of its current value, and this
            // new drag's own release will schedule a fresh restore.
            // Leaving the old one pending instead would risk it firing
            // *during* this new drag and re-enabling snap mid-gesture --
            // exactly the 'fights every scrollBy' failure mode this
            // module's own doc already measured once.
            cancelPendingSnapRestore();
            cancelPendingSnapRestore = null;
        }
    };

    const onPointerMove = (e) => {
        if (e.pointerId !== pointerId) {
            return;
        }
        if (!dragging) {
            const dxFromOrigin = e.clientX - originX;
            const dyFromOrigin = e.clientY - originY;
            if (dxFromOrigin * dxFromOrigin + dyFromOrigin * dyFromOrigin < thresholdSq) {
                return;
            }
            dragging = true;
            el.setAttribute('data-dragging', 'true');
            try { el.setPointerCapture(pointerId); } catch (err) {}
            // `scroll-snap-type: ... mandatory` (set inline by this same
            // element's own Rust-rendered `style`) re-snaps after EVERY
            // individual scroll operation it sees as complete -- and each
            // `scrollBy` below is exactly that, since nothing tells the
            // browser these many small instant scrolls are one ongoing
            // gesture the way a real touch/wheel drag would. Confirmed
            // live (round5): left enabled, a single ~80px drag snapped
            // forward one slide per `scrollBy` call and landed 4 slides
            // away instead of 1. Suspending it for the drag's own
            // duration lets these calls move the track freely; `endDrag`
            // below settles it back on release -- see that function's own
            // doc, and this element's own 'Release settle' doc, for why
            // that settle is no longer simply 'restore this property and
            // let the browser re-snap'.
            el.style.scrollSnapType = 'none';
        }
        // Only reached once dragging -- a still-below-threshold move never
        // calls this, so a plain click's own tiny jitter never suppresses
        // text selection/native drag-ghost for nothing.
        e.preventDefault();
        const dx = e.clientX - lastX;
        const dy = e.clientY - lastY;
        lastX = e.clientX;
        lastY = e.clientY;
        // scrollBy's own delta is always physical-pixel, regardless of
        // `dir` -- unlike reading/writing `scrollLeft` itself, whose
        // zero-point and sign this module's own doc already warns has
        // historically disagreed across browsers, `scrollBy`'s relative
        // delta does not. So this formula never branches on direction --
        // and still gets the well-known RTL swap for free, the same way
        // this module's own keyboard handler does (`carousel_key_intent`/
        // `Direction::resolve_horizontal`, no scrollBy/scrollLeft
        // involved there either): dragging is direct manipulation (the
        // content tracks the pointer 1:1), and content tracking the
        // pointer is itself a physical, dir-independent relationship --
        // it is *which physical direction reveals 'the next slide'* that
        // flips under RTL (DOM order never changes, but RTL lays later
        // slides physically further left, the mirror of LTR), exactly
        // like the familiar swipe-left-for-next vs. swipe-right-for-next
        // split between LTR and RTL photo galleries/story viewers.
        // Verified live (round5, both directions on the same `rtl`
        // variant): dragging left from slide 1 hit the start boundary and
        // never moved (correct -- there is nothing before slide 1);
        // dragging right produced the identical `scrollLeft` change
        // (`0 -> -153`) the Next button's own `scrollBy`-by-delta produces.
        //
        // Routed through `feedOverdrag` rather than a bare `scrollBy`
        // (mode B3 port, see this constant's own 'Edge rubber-band' doc
        // above): mid-range this is a no-op wrapper around the identical
        // `scrollBy` call this always issued, and at an edge it is what
        // turns the refused remainder into the bounce transform.
        feedOverdrag(orientation === 'horizontal' ? dx : dy);
    };

    const endDrag = (e) => {
        if (e.pointerId !== pointerId) {
            return;
        }
        if (dragging) {
            suppressNextClick = true;
            el.removeAttribute('data-dragging');
            try { el.releasePointerCapture(pointerId); } catch (err) {}

            // Mode B3: ease any overdrag back to identity BEFORE searching
            // for the nearest slide below -- that search reads live
            // `getBoundingClientRect` geometry, which the bounce transform
            // itself would otherwise skew. When there was no overdrag
            // (the overwhelmingly common case) `bounceHome` invokes this
            // callback immediately, so release behaves exactly as before
            // this port. This never restarts an in-flight ease (invariant
            // 4) -- see `bounceHome`'s own doc.
            bounceHome(() => {
            // This element's own 'Release settle' doc has the full
            // reasoning; short version: find the slide nearest the raw
            // drag position -- the same getBoundingClientRect-based
            // geometry `use_carousel_scroll_tracking`'s own settle()
            // already uses elsewhere on this identical element, since a
            // fresh `document::eval` string cannot import a shared JS
            // helper -- and explicitly `scrollBy` this same element (`el`,
            // the scroller itself -- never `nearest.scrollIntoView()`,
            // which would also walk and scroll every scrollable ancestor,
            // including the page) by the physical delta between `nearest`'s
            // own leading edge and this element's, with `behavior: 'smooth'`
            // (respecting reduced motion, the exact policy
            // CAROUSEL_SCROLL_TO_JS already applies to every other paging
            // path -- see that constant's own doc for why a scroller-only
            // `scrollBy` replaced `scrollIntoView` everywhere in this
            // module), rather than leaving the settle to however restoring
            // `scroll-snap-type` happens to behave.
            const children = Array.from(el.children);
            let nearest = null;
            let nearestDist = Infinity;
            if (children.length > 0) {
                const containerRect = el.getBoundingClientRect();
                const containerStart = orientation === 'horizontal' ? containerRect.left : containerRect.top;
                const containerEnd = orientation === 'horizontal' ? containerRect.right : containerRect.bottom;
                children.forEach((child) => {
                    const rect = child.getBoundingClientRect();
                    const childStart = orientation === 'horizontal' ? rect.left : rect.top;
                    const childEnd = orientation === 'horizontal' ? rect.right : rect.bottom;
                    const dist = Math.abs(
                        anchorOf(childStart, childEnd) - anchorOf(containerStart, containerEnd)
                    );
                    if (dist < nearestDist) {
                        nearestDist = dist;
                        nearest = child;
                    }
                });
            }

            const restoreSnap = () => {
                el.style.scrollSnapType = orientation === 'horizontal' ? 'x mandatory' : 'y mandatory';
                cancelPendingSnapRestore = null;
            };
            if (nearest) {
                const reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
                const settleBehavior = reduced ? 'auto' : 'smooth';
                const s = el.getBoundingClientRect();
                const t = nearest.getBoundingClientRect();
                const sStart = orientation === 'horizontal' ? s.left : s.top;
                const sEnd = orientation === 'horizontal' ? s.right : s.bottom;
                const tStart = orientation === 'horizontal' ? t.left : t.top;
                const tEnd = orientation === 'horizontal' ? t.right : t.bottom;
                const settleDelta = anchorOf(tStart, tEnd) - anchorOf(sStart, sEnd);
                // `nearest` is already exactly where it should be (the
                // drag released right on a boundary) -- `scrollBy({left:
                // 0})` never fires `scrollend` (CAROUSEL_SCROLL_TO_JS's
                // own doc has the same finding), and `scroll-snap-type` is
                // ALREADY 'none' from this drag's own start, so unlike
                // that constant's own zero-delta case (which can just
                // touch nothing) restoring must happen synchronously here
                // -- there is no scroll for a deferred listener/timeout to
                // ever key off, and skipping it entirely would leave
                // snapping suspended until the next gesture happens to
                // trigger it.
                if (Math.abs(settleDelta) < 1) {
                    restoreSnap();
                } else {
                    // Deferred, not synchronous: setting scroll-snap-type
                    // back to mandatory on an already-stationary position is
                    // what made release never animate in the first place
                    // (measured -- see 'Release settle'). Waiting for this
                    // specific scroll to actually finish (`scrollend`,
                    // falling back to a timeout where unsupported -- the same
                    // feature detection `use_carousel_scroll_tracking` already
                    // uses) means scroll-snap-type is still 'none' for the
                    // whole smooth-scroll animation this call starts, so
                    // there is nothing stationary for the browser to
                    // instantly correct until that animation has already
                    // finished on its own.
                    if ('onscrollend' in window) {
                        el.addEventListener('scrollend', restoreSnap, { once: true, passive: true });
                        cancelPendingSnapRestore = () => el.removeEventListener('scrollend', restoreSnap);
                    } else {
                        const timer = setTimeout(restoreSnap, snapRestoreFallbackMs);
                        cancelPendingSnapRestore = () => clearTimeout(timer);
                    }
                    if (orientation === 'horizontal') {
                        el.scrollBy({ left: settleDelta, behavior: settleBehavior });
                    } else {
                        el.scrollBy({ top: settleDelta, behavior: settleBehavior });
                    }
                }
            } else {
                // No children at all (shouldn't happen in practice) --
                // fall back to the old unconditional-restore behavior
                // rather than leaving scroll-snap-type suspended forever.
                restoreSnap();
            }
            });
        }
        pointerId = null;
        dragging = false;
    };

    // Capture phase, attached to this element itself (an ancestor of
    // every slide, so this still only ever sees clicks that landed
    // somewhere inside this carousel's own content): fires before the
    // click reaches whatever child it landed on (a link, a button) and
    // before Dioxus's own delegated bubble-phase handling, so both a
    // native default action (link navigation) and any `onclick` a caller
    // attached are suppressed together -- exactly once, only immediately
    // after a real drag, never for a plain click that never crossed the
    // threshold.
    const onClickCapture = (e) => {
        if (suppressNextClick) {
            suppressNextClick = false;
            e.preventDefault();
            e.stopPropagation();
        }
    };

    el.addEventListener('pointerdown', onPointerDown);
    el.addEventListener('pointermove', onPointerMove);
    el.addEventListener('pointerup', endDrag);
    el.addEventListener('pointercancel', endDrag);
    el.addEventListener('lostpointercapture', endDrag);
    el.addEventListener('click', onClickCapture, true);
    await dioxus.recv();
    el.removeEventListener('pointerdown', onPointerDown);
    el.removeEventListener('pointermove', onPointerMove);
    el.removeEventListener('pointerup', endDrag);
    el.removeEventListener('pointercancel', endDrag);
    el.removeEventListener('lostpointercapture', endDrag);
    el.removeEventListener('click', onClickCapture, true);";

/// Attach [`CAROUSEL_DRAG_JS`] to the element with the given `id` for as
/// long as the calling component stays mounted and `enabled()` is `true`
/// -- mirrors [`use_carousel_scroll_tracking`]'s own shape exactly (an
/// initial `eval.send(..)`, teardown via a second send read by the
/// script's own trailing `await dioxus.recv()`), one layer simpler since
/// this bridge reports nothing back to Rust at all (see [`CAROUSEL_DRAG_JS`]'s
/// own doc for why: every `scrollBy` it issues -- including `endDrag`'s
/// own release settle -- is picked up by [`use_carousel_scroll_tracking`]'s
/// own listener on the same element, which is what recomputes both
/// `selected` and the a11y contract's "visible" set once a real,
/// `data-dragging`-free scroll/scrollend event follows the release).
///
/// `enabled` is read with tracked syntax so toggling
/// [`CarouselContentProps::draggable`] at runtime actually attaches/tears
/// down the listeners, not just the initial mount value -- the gate itself
/// lives in the script (`if (!el || !enabled) { ...; return; }`) rather
/// than in whether this hook installs an eval at all, so every path
/// through this function sends the exact same shape of config and returns
/// the exact same shape of cleanup closure.
fn use_carousel_drag(
    id: impl Readable<Target = String> + Copy + 'static,
    orientation: ReadSignal<CarouselOrientation>,
    align: ReadSignal<CarouselAlign>,
    enabled: ReadSignal<bool>,
) {
    crate::use_effect_with_cleanup(move || {
        let id = id.cloned();
        let orientation_str = orientation().as_str().to_string();
        let align_str = align().as_str().to_string();
        let eval = document::eval(CAROUSEL_DRAG_JS);
        // No spawned receive loop here, unlike `use_carousel_scroll_tracking`
        // just above -- this script never `dioxus.send`s anything back (see
        // this function's own doc), so there is nothing to poll for until
        // the cleanup closure's own teardown `send` below, which needs no
        // receiver on this side to take effect.
        let _ = eval.send((
            id,
            orientation_str,
            CAROUSEL_DRAG_THRESHOLD_PX,
            enabled(),
            CAROUSEL_SNAP_RESTORE_FALLBACK_MS,
            align_str,
        ));
        move || {
            let _ = eval.send(true);
        }
    });
}

/// Long-lived (mount-to-unmount): the wheel/trackpad half of the edge
/// rubber-band (mode B3), attached to [`CarouselContent`]'s own element
/// alongside [`CAROUSEL_DRAG_JS`] -- a **separate** `document::eval`, not
/// folded into that one, because the two obey different rules and must
/// not share a JS closure that could let one's state leak into the
/// other's: a mouse/pen drag has no native scrolling behind it, so
/// [`CAROUSEL_DRAG_JS`] may keep issuing `scrollBy` (THE RULE,
/// dev-docs/research/carousel-overscroll-2026-09-23.md §3), while a
/// wheel/trackpad gesture on this same element has a compositor and a
/// snap engine also trying to write its scroll position, so this script
/// may **never** call `scrollBy`, `scrollLeft`/`scrollTop`, `preventDefault`,
/// or touch `scroll-snap-type` -- the whole reason ~20 bench revisions of
/// exactly that glitched (research doc §2/§5). The numbered invariants
/// cited below are that doc's own §6.
///
/// The edge is read directly from layout (`edgeGaps`, invariant 2) rather
/// than from a scroll coordinate, so this needs no [`Direction`]/RTL
/// branch either -- `getBoundingClientRect` is already a physical,
/// direction-agnostic rectangle. `passive: true` throughout: this script
/// makes no decision that ever needs to block the browser's own native
/// scroll, since it only ever *observes* geometry and writes its own
/// `transform`, never the scroll position.
const CAROUSEL_WHEEL_BOUNCE_JS: &str = "\
    const [id, orientation] = await dioxus.recv();
    const el = document.getElementById(id);
    if (!el) {
        await dioxus.recv();
        return;
    }

    const RUBBER_C = 0.55;
    // Fallbacks only, until the device's own quantum is learned below --
    // invariant 8: every pixel threshold here is derived from that
    // quantum once it is known, never a bare guess.
    const WHEEL_RELEASE_DELTA = 1.6;
    const WHEEL_FLOOR_MULT = 2;
    const WHEEL_PLATEAU_MULT = 3;
    const WHEEL_PLATEAU_MAX = 4;
    const WHEEL_RELEASE_FRACTION = 0.12;
    const WHEEL_RELEASE_MAX = 10;
    const WHEEL_RELEASE_RUNS = 2;
    const WHEEL_INTERRUPT_DELTA = 6;
    // Backstop only -- measured rest after the last wheel event on real
    // hardware is 64-78ms (research doc §4); this is twice that, so it
    // essentially never fires before the decay/plateau tests above already
    // have.
    const WHEEL_IDLE_MS = 90;

    let rawOver = 0;
    let bounceHoming = false;
    let bounceToken = 0;
    let bounceSettling = false;
    let wheelTimer = 0;
    let wheelLowRun = 0;
    let wheelPeakDelta = 0;
    let wheelLastAbs = Infinity;
    let wheelInterruptFloor = WHEEL_INTERRUPT_DELTA;
    let deviceMinDelta = Infinity; // learned: the smallest step this hardware sends

    function axisSize() {
        return (orientation === 'horizontal' ? el.clientWidth : el.clientHeight) || 320;
    }
    // Apple's own published rubber-band curve, `f(x) = x*c*d / (d + c*x)`
    // (research doc §4: `b(x) = (1 - 1/(x*c/d + 1))*d`, algebraically the
    // same function) -- slope `c` (0.55) at `x = 0`, asymptote `d`
    // (`axisSize()`) as `x` grows. **Correction, 2026-09-25**: this used to
    // be `L*x/(x+L)` with `L = d/c`, which is a DIFFERENT curve, not an
    // algebraic rewrite of the same one -- it has slope 1 (not `c`) at
    // `x = 0` and asymptotes at `d/c` (~1.8*d), not `d`. That mistranscription
    // is why a fast trackpad flick could push the track roughly twice as
    // far as native's own feel calls for; see the dated correction in
    // `dev-docs/research/carousel-overscroll-2026-09-23.md` §9 and
    // `dev-docs/backlog.md` row 102's own addendum for the full algebra.
    function rubber(x) {
        const d = axisSize();
        return (x * RUBBER_C * d) / (d + RUBBER_C * x);
    }
    function rubberSigned(x) {
        return x < 0 ? -rubber(-x) : rubber(x);
    }
    function applyBounce() {
        if (!rawOver) {
            el.style.transform = '';
            return;
        }
        const depth = rubberSigned(rawOver).toFixed(2);
        el.style.transform =
            orientation === 'horizontal' ? `translateX(${depth}px)` : `translateY(${depth}px)`;
    }
    // Invariant 2: the edge is read synchronously from layout, in the same
    // turn as the decision -- never inferred by comparing this event's
    // request against a measurement taken a frame later (that phase
    // mismatch against the compositor is what produced 198-212px of false
    // overdrag mid-range on the bench, research doc §6).
    function edgeGaps() {
        const children = el.children;
        if (!children.length) {
            return { start: 0, end: 0 };
        }
        const c = el.getBoundingClientRect();
        let minStart = Infinity;
        let maxEnd = -Infinity;
        for (let i = 0; i < children.length; i++) {
            const r = children[i].getBoundingClientRect();
            const s = orientation === 'horizontal' ? r.left : r.top;
            const e = orientation === 'horizontal' ? r.right : r.bottom;
            if (s < minStart) minStart = s;
            if (e > maxEnd) maxEnd = e;
        }
        const cs = orientation === 'horizontal' ? c.left : c.top;
        const ce = orientation === 'horizontal' ? c.right : c.bottom;
        return { start: minStart - cs, end: maxEnd - ce };
    }
    function noteDeviceDelta(ad) {
        if (ad > 0 && ad < deviceMinDelta) {
            deviceMinDelta = ad;
        }
    }
    function deviceFloor() {
        return isFinite(deviceMinDelta) ? deviceMinDelta * WHEEL_FLOOR_MULT : WHEEL_RELEASE_DELTA;
    }
    function plateauCeiling() {
        return isFinite(deviceMinDelta) ? deviceMinDelta * WHEEL_PLATEAU_MULT : WHEEL_PLATEAU_MAX;
    }
    function wheelReleaseThreshold() {
        return Math.min(WHEEL_RELEASE_MAX, Math.max(deviceFloor(), wheelPeakDelta * WHEEL_RELEASE_FRACTION));
    }
    // A wheel has no `pointerup`; its release is inferred from velocity
    // DECAY, not from the event stream going silent (a trackpad's momentum
    // tail keeps firing events long after the fingers lift). A PLATEAU
    // (small and not rising) also counts as spent, so a hand still pushing
    // gently at a low, flat delta is not mistaken for one still building.
    function wheelSpent(d) {
        const ad = Math.abs(d);
        if (ad > wheelPeakDelta) wheelPeakDelta = ad;
        noteDeviceDelta(ad);
        const spent = ad <= wheelReleaseThreshold() || (ad <= plateauCeiling() && ad <= wheelLastAbs);
        wheelLastAbs = ad;
        return spent;
    }
    function armInterruptFloor() {
        // The interrupt floor MUST stay strictly above the release
        // threshold that was just used to release, or a delta between the
        // two would both trigger a new release and immediately re-trigger
        // this one on the very next event.
        wheelInterruptFloor = Math.max(WHEEL_INTERRUPT_DELTA, wheelReleaseThreshold() * 3);
    }
    // Driven by us, never the discrete-paging `scrollBy`-by-delta helper
    // (invariant 3) -- and superseded rather than restarted (invariant 4)
    // via `bounceToken`, the same construction `CAROUSEL_DRAG_JS`'s own
    // `bounceHome` uses, for the identical reason: a stale in-flight ease
    // must stop touching `rawOver` the moment a new gesture owns it, not
    // race that gesture's writes.
    function bounceHome(done) {
        const reduce = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
        const from = rawOver;
        if (!from || reduce) {
            rawOver = 0;
            applyBounce();
            bounceHoming = false;
            done && done();
            return;
        }
        bounceHoming = true;
        const myToken = ++bounceToken;
        const t0 = performance.now();
        const DUR = 340;
        (function step() {
            if (myToken !== bounceToken) {
                return;
            }
            const t = Math.min(1, (performance.now() - t0) / DUR);
            const k = 1 - Math.pow(1 - t, 3);
            rawOver = from * (1 - k);
            applyBounce();
            if (t < 1) {
                window.requestAnimationFrame(step);
            } else {
                rawOver = 0;
                applyBounce();
                bounceHoming = false;
                done && done();
            }
        })();
    }
    function release(how) {
        if (wheelTimer) {
            window.clearTimeout(wheelTimer);
            wheelTimer = 0;
        }
        armInterruptFloor();
        wheelLowRun = 0;
        wheelPeakDelta = 0;
        wheelLastAbs = Infinity;
        if (rawOver) {
            bounceSettling = true;
            // No scroll-based settle here (unlike the pointer-drag path):
            // this scroller never left its snap point in the first place --
            // only the transform ever moved -- so there is nothing to
            // realign to a slide, only the transform to bring home.
            bounceHome(() => {
                bounceSettling = false;
            });
        }
    }

    // THE RULE, restated for this listener specifically: no `preventDefault`,
    // no `scrollBy`, no `scrollLeft`/`scrollTop` read or write, no
    // `scroll-snap-type` write, ever, in this function. Past a genuinely
    // clamped edge the scroller cannot move, so its state is stable and
    // uncontested while this animates the transform; the shortfall between
    // what the gesture asked for and what the scroller actually did IS the
    // overdrag, and it is read from `edgeGaps()`, never from attempting a
    // write and measuring the refusal (that trick is only safe on the
    // pointer-drag path above, which owns the scroll position).
    const onWheel = (e) => {
        if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
            if (rawOver) {
                rawOver = 0;
                applyBounce();
            }
            return;
        }
        const raw = e.deltaX !== 0 ? e.deltaX : e.deltaY;
        const d = e.deltaMode === 1 ? raw * 16 : e.deltaMode === 2 ? raw * axisSize() : raw;
        const cd = -d; // desired physical content displacement, + = toward the end

        if (bounceSettling) {
            if (Math.abs(d) < wheelInterruptFloor) {
                return;
            }
            bounceSettling = false;
            bounceToken++; // supersede the in-flight ease -- this gesture owns rawOver now
        }

        const g = edgeGaps();
        const atLimit = cd > 0 ? g.start >= -0.5 : g.end <= 0.5;

        if (atLimit) {
            rawOver += cd;
            applyBounce();
        }

        if (!atLimit && rawOver !== 0 && !bounceHoming) {
            // Reversed away from the edge: let the browser scroll
            // unimpeded and spring the band home, never unwind it by hand
            // at the same time (that would move the content twice as fast
            // as the gesture).
            release('reversed');
            return;
        }

        if (rawOver !== 0 && wheelSpent(d)) {
            if (++wheelLowRun >= WHEEL_RELEASE_RUNS) {
                release('decay');
                return;
            }
        } else {
            wheelLowRun = 0;
            wheelSpent(d);
        }
        if (wheelTimer) {
            window.clearTimeout(wheelTimer);
        }
        wheelTimer = window.setTimeout(() => {
            wheelTimer = 0;
            release('idle');
        }, WHEEL_IDLE_MS);
    };

    el.addEventListener('wheel', onWheel, { passive: true });
    await dioxus.recv();
    el.removeEventListener('wheel', onWheel);
    if (rawOver) {
        rawOver = 0;
        el.style.transform = '';
    }";

/// Attach [`CAROUSEL_WHEEL_BOUNCE_JS`] to the element with the given `id`
/// for as long as the calling component stays mounted -- mirrors
/// [`use_carousel_drag`]'s own shape exactly, one layer simpler still
/// (nothing here is ever disableable the way [`CarouselContentProps::draggable`]
/// disables the pointer-drag gesture: a wheel/trackpad user has no
/// equivalent opt-out today, and the edge bounce is purely cosmetic
/// feedback on top of scrolling that already happens regardless).
fn use_carousel_wheel_bounce(
    id: impl Readable<Target = String> + Copy + 'static,
    orientation: ReadSignal<CarouselOrientation>,
) {
    crate::use_effect_with_cleanup(move || {
        let id = id.cloned();
        let orientation_str = orientation().as_str().to_string();
        let eval = document::eval(CAROUSEL_WHEEL_BOUNCE_JS);
        let _ = eval.send((id, orientation_str));
        move || {
            let _ = eval.send(true);
        }
    });
}

/// Shared state and actions, provided by [`Carousel`] and consumed by
/// every sub-component plus [`use_carousel`].
#[derive(Clone, Copy)]
struct CarouselContext {
    orientation: ReadSignal<CarouselOrientation>,
    /// Where each slide rests against the scrollport -- see
    /// [`CarouselAlign`]'s own doc. Read by [`CarouselItem`]/
    /// [`CarouselVirtualContent`] for `scroll-snap-align`, and by every
    /// JS bridge that computes a paging delta or a "nearest slide" search
    /// ([`CAROUSEL_SCROLL_TO_JS`], [`CAROUSEL_SCROLL_TRACKING_JS`],
    /// [`CAROUSEL_DRAG_JS`]).
    align: ReadSignal<CarouselAlign>,
    /// Always in `[0, count)` (or `0` if `count == 0`) -- see
    /// [`clamp_selected`]. The single source of truth every sub-component
    /// reads instead of re-deriving its own notion of "which slide is
    /// current."
    selected: Memo<usize>,
    set_selected: Callback<usize>,
    /// Number of currently-registered [`CarouselItem`]s. Empty
    /// (`count == 0`) until the first render's effects have run --
    /// deliberately: see [`CarouselItem`]'s doc, "SSR stability."
    count: Memo<usize>,
    /// `index -> DOM id`, grown as each [`CarouselItem`] registers (the
    /// same grow-only-`Vec`-by-index idiom `tabs.rs`'s own
    /// `tab_content_ids` uses for the identical reason: SSR and a
    /// pre-effect client render must agree, and both start from an empty
    /// `Vec`).
    item_ids: Signal<Vec<String>>,
    /// [`CarouselContent`]'s own rendered id, published by itself so
    /// [`CarouselPrevious`]/[`CarouselNext`] can point `aria-controls` at
    /// it (mirrors the `trigger_id`-publishing construction
    /// `dev-docs/conformance-harness.md`'s tier-2 Rule 14 describes for
    /// `SelectTrigger`/`DropdownMenuTrigger`/etc.).
    content_id: Signal<String>,
    /// Whether `loop`ing is enabled -- see [`CarouselProps::r#loop`].
    loop_enabled: ReadSignal<bool>,
    /// The resolved text direction -- the exact same value [`Carousel`]'s
    /// own root already computed via `use_direction(props.dir)`, republished
    /// here so [`CarouselTabList`]/[`CarouselTab`] reuse it verbatim rather
    /// than calling `use_direction(None)` a second time, which would
    /// silently disagree with the root's own resolution whenever a caller
    /// passes an explicit `dir` prop on [`Carousel`] itself rather than
    /// relying on an ambient [`crate::direction::DirectionProvider`].
    direction: Direction,
    /// Autoplay/rotation-control state -- always present, every field
    /// initialized to an inert "no autoplay" value, so [`CarouselContent`]
    /// can read `autoplay.present`/`autoplay.rotating` unconditionally for
    /// its own `aria-live` whether or not a [`CarouselAutoplay`] child
    /// actually exists. The same "publish into a shared, always-initialized
    /// slot" idiom `content_id` above already uses.
    autoplay: AutoplayContext,
    /// Published (`true`) by [`CarouselTabList`] on mount so
    /// [`CarouselItem`] can switch its own role from `group` to
    /// `tabpanel` -- see that component's own "Tablist variant" doc.
    tablist_present: Signal<bool>,
    /// `[start, end]` (inclusive) item indices actually inside
    /// [`CarouselContent`]'s own viewport *at rest* -- the a11y contract's
    /// own "visible" set (`dev-docs/research/carousel-loop-2026-09-25/loop-a11y-guidance.md`
    /// §7, `loop-libraries.md`'s cross-library convergence table).
    /// Defaults to `(0, 0)` -- deterministic across SSR and a client's own
    /// pre-hydration first render, matching [`Self::selected`]'s own
    /// forced-`0`-until-registration value (see [`CarouselItem`]'s own
    /// "Visible slides / `inert`" doc for why this makes the two agree at
    /// first render without either needing a special case). Updated only
    /// by [`use_carousel_scroll_tracking`]'s own settle (on `scrollend`/the
    /// debounced-`scroll` fallback, and on a `ResizeObserver` firing for
    /// the scroller's own box -- see that constant's own doc for why a
    /// resize matters here too) -- never mid-drag, mid-momentum, or
    /// mid-smooth-page (the same "freeze" rule already applies to
    /// [`Self::selected`]; a drag's own release settle reaches this
    /// signal only indirectly, through the ordinary native
    /// scroll/scrollend event its release `scrollBy` produces once
    /// `data-dragging` is gone).
    visible_range: Signal<(usize, usize)>,
    /// The index of the [`CarouselItem`] that currently contains DOM
    /// focus, if any -- maintained purely by each item's own
    /// `onfocusin`/`onfocusout` (native, synchronous; no `document::eval`
    /// involved). Read once, synchronously, by the focus-redirect effect
    /// below whenever [`Self::selected`] changes, to decide whether the
    /// slide that just lost "current" status (and is about to -- or just
    /// did -- become `inert`) held focus at the moment of the transition.
    focused_item: Signal<Option<usize>>,
}

/// Autoplay/rotation-control state, grouped out of [`CarouselContext`]
/// only for readability -- every field lives for the whole [`Carousel`]'s
/// lifetime regardless of whether a [`CarouselAutoplay`] is ever mounted.
///
/// ## The pause/resume state machine
///
/// `rotating` (the one field every other component actually reads) is
/// `true` only when ALL of the following hold: `playing` (the user's own
/// high-level intent, toggled only by [`CarouselRotationControl`]'s click
/// handler), NOT `resume_blocked`, NOT `focus_within`, and NOT (`hover`
/// AND `stop_on_mouse_enter`). Two different "pause" shapes are
/// deliberate, not an oversight, both read directly from the vendored
/// tabbed reference's own accessibility-features prose
/// (`examples/carousel-2-tablist.html`, "Controlling Automatic Slide
/// Rotation"), not the basic reference's own JS (which has a documented
/// latch bug on this exact point, `dev-docs/research/carousel-2026-09-19.md`
/// §1.3 point 3 -- deliberately not ported):
///
/// - **Hover** clears freely: `hover` going back to `false` alone can make
///   `rotating` `true` again ("Automatic rotation resumes when the mouse
///   moves away... unless another condition... has been triggered").
/// - **Focus** is sticky: `focus_within` going back to `false` does *not*
///   by itself resume anything, because `resume_blocked` (set the instant
///   focus enters, alongside `focus_within`) stays `true` until
///   [`CarouselRotationControl`] is clicked again ("Automatic rotation
///   only resumes if the user explicitly activates the ... button").
///
/// [`CarouselRotationControl`]'s own click handler always clears
/// `resume_blocked` (whichever direction it toggles `playing`), matching
/// the same reference's own "if a user activates the rotation control
/// button ... it is assumed the user wants auto-rotation to start
/// immediately."
#[derive(Clone, Copy, PartialEq)]
struct AutoplayContext {
    /// `true` once a [`CarouselAutoplay`] has mounted at least once.
    present: Signal<bool>,
    /// The user's own high-level intent -- see this struct's own doc.
    playing: Signal<bool>,
    hover: Signal<bool>,
    focus_within: Signal<bool>,
    resume_blocked: Signal<bool>,
    /// Mirrors [`CarouselAutoplayProps::stop_on_interaction`]; published
    /// here (not read directly off the props) so [`AutoplayContext::note_interaction`]
    /// can be called from components (`CarouselPrevious`/`CarouselNext`/
    /// `CarouselTab`/[`use_carousel`]) that have no direct access to
    /// [`CarouselAutoplay`]'s own props.
    stop_on_interaction: Signal<bool>,
    /// Mirrors [`CarouselAutoplayProps::stop_on_mouse_enter`]; see
    /// `stop_on_interaction`'s own doc for why this is republished here.
    stop_on_mouse_enter: Signal<bool>,
    /// `true` exactly when the timer should be actively ticking -- see
    /// this struct's own "pause/resume state machine" doc.
    rotating: Memo<bool>,
}

impl AutoplayContext {
    /// Stop autoplay for good (until [`CarouselRotationControl`] is
    /// explicitly clicked again) if a [`CarouselAutoplay`] is present with
    /// `stop_on_interaction` enabled (its own default `true`) -- called
    /// from every *manual, discrete* paging entry point:
    /// [`CarouselPrevious`]/[`CarouselNext`], [`Carousel`]'s own root
    /// keyboard handler, [`CarouselTab`], and [`CarouselApi::scroll_to`].
    /// Mirrors embla-carousel's own `embla-carousel-autoplay` plugin
    /// option of the same name.
    ///
    /// Deliberately **not** called from the native pointer-drag/wheel
    /// scroll-tracking bridge (`use_carousel_scroll_tracking`) -- a
    /// documented v1 scope cut: only the discrete actions above count as
    /// "interaction" for this purpose. See
    /// [`CarouselAutoplayProps::stop_on_interaction`]'s own doc.
    fn note_interaction(&self) {
        if (self.present)() && (self.stop_on_interaction)() {
            let mut playing = self.playing;
            playing.set(false);
        }
    }
}

/// The props for the [`Carousel`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CarouselProps {
    /// The axis the carousel pages along. Defaults to
    /// [`CarouselOrientation::Horizontal`].
    #[props(default)]
    pub orientation: ReadSignal<CarouselOrientation>,

    /// Where each slide rests against the scrollport. Defaults to
    /// [`CarouselAlign::Start`] -- matches shadcn's own default and every
    /// one of its demos (`opts={{ align: "start" }}`, `dev-docs/research/shadcn-carousel-parity.md`).
    /// See [`CarouselAlign`]'s own doc for what changes with `Center`/`End`.
    #[props(default)]
    pub align: ReadSignal<CarouselAlign>,

    /// The controlled selected slide index (0-based). `None` for
    /// uncontrolled use (see [`Self::default_value`]).
    pub value: ReadSignal<Option<usize>>,

    /// Whether Previous/Next (and the root's own `ArrowLeft`/`ArrowRight`)
    /// wrap around at the ends -- **rewind-style**, not an
    /// embla-style seamless illusion: from the last slide, Next goes to
    /// the first (and vice versa for Previous), via the same
    /// scroller-only `scrollBy`-by-delta paging path every other
    /// transition already uses, which visibly scrolls back across the
    /// intervening slides rather
    /// than teleporting. No cloned edge slides are ever added -- they
    /// would violate the overscroll port's own invariant 5 (they'd enter
    /// the snap engine's candidate list, the "N of M" slide count, and
    /// `:nth-child` styling). Dragging or wheeling past a physical edge
    /// still rubber-bands (mode B3) regardless of this flag -- there is no
    /// wrap on a drag/wheel gesture, only on Previous/Next/the root
    /// keyboard. When `true`, [`CarouselPrevious`]/[`CarouselNext`] are
    /// never `disabled`. Defaults to `false` (matches shadcn's own
    /// `opts={{ loop: false }}` default). Approved fast-follow decision,
    /// backlog row 91 / `dev-docs/research/carousel-2026-09-19.md` §8.2.
    #[props(default)]
    pub r#loop: ReadSignal<bool>,

    /// The initial selected slide index when uncontrolled.
    #[props(default)]
    pub default_value: usize,

    /// Called whenever the selected slide settles on a new index, from
    /// any source: [`CarouselPrevious`]/[`CarouselNext`], the root
    /// keyboard handler, [`use_carousel`]'s `scroll_to`, or a user's own
    /// drag/wheel/trackpad scroll.
    #[props(default)]
    pub on_value_change: Callback<usize>,

    /// The text direction for the root-level `ArrowLeft`/`ArrowRight`
    /// paging keys. Defaults to the nearest
    /// [`crate::direction::DirectionProvider`], or LTR if there is none.
    /// See [`crate::direction::use_direction`].
    pub dir: Option<Direction>,

    /// Additional attributes to apply to the carousel's root element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the carousel -- typically [`CarouselPrevious`] and
    /// [`CarouselNext`] followed by [`CarouselContent`] (see
    /// [`Carousel`]'s own doc for why that order matters).
    pub children: Element,
}

/// # Carousel
///
/// A slideshow of [`CarouselItem`]s the user pages through with
/// [`CarouselPrevious`]/[`CarouselNext`] buttons, `ArrowLeft`/`ArrowRight`
/// (RTL-aware) at the root, or by dragging/scrolling the track natively.
/// Implements the APG Carousel pattern's "basic" (prev/next, no picker)
/// style -- see the module doc for exactly what v1 does and defers.
///
/// `Carousel` itself renders `role="region"` +
/// `aria-roledescription="carousel"` and requires an accessible name (an
/// `aria-label`/`aria-labelledby` in `attributes`) -- a
/// `tracing::warn!` fires when neither is supplied, since the pattern
/// requires one and shadcn's own reference component leaves this gap
/// entirely undocumented (`dev-docs/research/carousel-2026-09-19.md`
/// §1.4).
///
/// ## Tab order
///
/// The reference implementation keeps its prev/next buttons **before**
/// the slide content in the page's `Tab` sequence (its own keyboard
/// table: "Rotation control, previous slide, and next slide buttons
/// precede the slide content"). Because `Carousel` renders `children` in
/// whatever order the caller writes them, and [`CarouselPrevious`]/
/// [`CarouselNext`] are meant to be positioned visually with CSS (not
/// DOM order -- absolute positioning does not care which sibling comes
/// first), the recommended composition puts them **before**
/// [`CarouselContent`], matching the reference and giving any interactive
/// content inside a slide (e.g. a card's own link) its natural place
/// after the controls, not before:
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::carousel::{Carousel, CarouselContent, CarouselItem, CarouselNext, CarouselPrevious};
/// #[component]
/// fn Demo() -> Element {
///     rsx! {
///         Carousel { aria_label: "Featured photos",
///             CarouselPrevious { "Previous" }
///             CarouselNext { "Next" }
///             CarouselContent {
///                 CarouselItem { index: 0usize, "Slide 1" }
///                 CarouselItem { index: 1usize, "Slide 2" }
///                 CarouselItem { index: 2usize, "Slide 3" }
///             }
///         }
///     }
/// }
/// ```
///
/// ## Keyboard
///
/// `ArrowLeft`/`ArrowRight` (or `ArrowUp`/`ArrowDown` under
/// [`CarouselOrientation::Vertical`]) page the carousel whenever focus is
/// anywhere inside it -- a Radix/shadcn-style addition, not an APG
/// requirement (the reference never binds arrow keys at the carousel-root
/// level at all), RTL-swapped like every other roving/orientable
/// primitive in this crate once focus is inside a
/// [`CarouselOrientation::Horizontal`] carousel. As with every native
/// `<button>`, activating [`CarouselPrevious`]/[`CarouselNext`] with
/// `Enter`/`Space` never moves focus.
///
/// ## Styling
///
/// The [`Carousel`] component defines the following data attributes you
/// can use to control styling:
/// - `data-orientation`: `horizontal` or `vertical`, matching
///   [`CarouselProps::orientation`].
/// - `data-direction`: the resolved text direction, `ltr` or `rtl`.
#[component]
pub fn Carousel(props: CarouselProps) -> Element {
    let (raw_selected, set_raw_selected) =
        use_controlled(props.value, props.default_value, props.on_value_change);
    let direction = use_direction(props.dir);
    let orientation = props.orientation;
    let align = props.align;

    let item_ids: Signal<Vec<String>> = use_signal(Vec::new);
    let count = use_memo(move || item_ids.len());
    let selected = use_memo(move || clamp_selected(raw_selected(), count()));
    let set_selected = use_callback(move |index: usize| {
        set_raw_selected.call(clamp_selected(index, *count.peek()));
    });
    let content_id: Signal<String> = use_signal(String::new);
    let loop_enabled = props.r#loop;

    // Autoplay state -- always created (see `AutoplayContext`'s own doc
    // for why: `CarouselContent` reads `autoplay.present`/`autoplay.rotating`
    // unconditionally for its own `aria-live`, regardless of whether a
    // `CarouselAutoplay` child exists at all). `stop_on_interaction`/
    // `stop_on_mouse_enter` default to `true` -- the same defaults
    // `CarouselAutoplayProps` itself uses, so a carousel with no
    // `CarouselAutoplay` at all behaves identically to one whose (never
    // mounted) autoplay would have used the ordinary defaults.
    let autoplay_present = use_signal(|| false);
    let autoplay_playing = use_signal(|| false);
    let mut autoplay_hover = use_signal(|| false);
    let mut autoplay_focus_within = use_signal(|| false);
    let mut autoplay_resume_blocked = use_signal(|| false);
    let autoplay_stop_on_interaction = use_signal(|| true);
    let autoplay_stop_on_mouse_enter = use_signal(|| true);
    let autoplay_rotating = use_memo(move || {
        if !autoplay_playing() || autoplay_resume_blocked() || autoplay_focus_within() {
            return false;
        }
        !(autoplay_hover() && autoplay_stop_on_mouse_enter())
    });
    let autoplay = AutoplayContext {
        present: autoplay_present,
        playing: autoplay_playing,
        hover: autoplay_hover,
        focus_within: autoplay_focus_within,
        resume_blocked: autoplay_resume_blocked,
        stop_on_interaction: autoplay_stop_on_interaction,
        stop_on_mouse_enter: autoplay_stop_on_mouse_enter,
        rotating: autoplay_rotating,
    };
    let tablist_present = use_signal(|| false);
    let visible_range: Signal<(usize, usize)> = use_signal(|| (0, 0));
    let focused_item: Signal<Option<usize>> = use_signal(|| None);

    use_context_provider(|| CarouselContext {
        orientation,
        align,
        selected,
        set_selected,
        count,
        item_ids,
        content_id,
        loop_enabled,
        direction,
        autoplay,
        tablist_present,
        visible_range,
        focused_item,
    });

    // Page the scroller to the selected slide whenever it changes,
    // regardless of source (buttons, keyboard, or
    // `use_carousel_scroll_tracking` reporting a native scroll). In the
    // last case the target slide is already at/near its snap point, so
    // the computed delta is ~0 and `scrollBy` is a harmless no-op.
    // `is_first` (peeked, then set -- never tracked-read, so this
    // effect never subscribes to its own write, matching
    // `scripts/check-self-subscribing-effects.sh`'s construction) forces
    // an instant jump on mount so a non-zero `default_value` never
    // visibly animates in on page load, and must be consumed *exactly*
    // once, by that mount-time settle -- never by a later navigation.
    //
    // It used to be consumed by "the first run that finds a valid item
    // id," which silently assumed that run was always the mount settle.
    // That assumption broke for the overwhelmingly common
    // `default_value == 0` case: `clamp_selected(0, count)` is `0` both
    // before and after registration, so `selected()`'s own *value* never
    // changes when items register, and (since `item_ids` below is read
    // through `.peek()`, deliberately not a tracked dependency) this
    // effect was never scheduled to re-run once they did. Its very first
    // execution -- mount, before any `CarouselItem` has registered --
    // early-returned with `is_first` still `true`, and nothing ran this
    // effect again until the user's first real navigation, which then
    // found a valid id for the first time and inherited that stale
    // `instant: true` by mistake. Reproduced live (round5,
    // `$S/round5/carousel-drag/item1-repro-{main,rtl}.log`): clicking
    // Next from slide 1 sent `{instant: true, behavior: 'auto'}` while
    // every later click sent `{instant: false, behavior: 'smooth'}`, on
    // an unmodified tree, with the test browser reporting
    // `prefers-reduced-motion: no-preference` and `.dx-carousel-content`
    // itself computing `scroll-behavior: auto` throughout (the global
    // `html { scroll-behavior: smooth }` rule in
    // `preview/assets/main.css` never reaches it -- `scroll-behavior` is
    // not an inherited property, and this element is its own independent
    // scroll container) -- ruling out both cheap alternative
    // explanations before accepting this one.
    //
    // Fixed by construction, not by timing: `count()` below is read with
    // tracked syntax (unlike the `item_ids.peek()` a few lines down), so
    // registration completing re-runs this effect the instant it does,
    // even though `selected()`'s own clamped value stayed the same --
    // and `is_first` is consumed unconditionally the moment `count() > 0`
    // on *that* run, regardless of whether this particular index's id
    // happens to resolve this same tick. So the flag can never survive
    // past the first post-registration run of this effect -- which,
    // since registration is itself effect-driven and completes
    // synchronously within the same initial render/effect-flush wave
    // that mounts this component (long before a user could physically
    // interact with it), can never coincide with a genuine navigation.
    // This subsumes both the `default_value == 0` case above (the leak
    // this construction closes) and the already-correct nonzero-
    // `default_value` case (still instant, still consumed at the same
    // conceptual point, just via a value change rather than a `count()`
    // retry).
    let mut is_first = use_signal(|| true);
    use_effect(move || {
        let index = selected();
        let has_items = count() > 0;
        let first = is_mount_settle(*is_first.peek(), has_items);
        if has_items {
            is_first.set(false);
        }
        let Some(id) = item_ids.peek().get(index).cloned() else {
            return;
        };
        // Tracked (like `count()` above), for the identical reason: a
        // scroller id that only arrives *after* this effect's first
        // post-registration run must still re-run this effect once it
        // does, rather than being silently missed the way an untracked
        // `.peek()` would miss it. In practice this never actually races
        // `has_items` becoming true -- `CarouselContent` publishes
        // `content_id` from its own mount effect, which (unlike item
        // registration) depends on nothing any child does, so it
        // completes no later than the child `CarouselItem` registrations
        // that make `has_items` true, within the same initial
        // render/effect-flush wave this effect's own long comment above
        // already establishes for item registration.
        let scroller_id = content_id();
        if scroller_id.is_empty() {
            return;
        }
        let orientation_str = orientation().as_str().to_string();
        let align_str = align().as_str().to_string();
        let eval = document::eval(CAROUSEL_SCROLL_TO_JS);
        let _ = eval.send((
            scroller_id,
            id,
            orientation_str,
            first,
            CAROUSEL_SNAP_RESTORE_FALLBACK_MS,
            align_str,
        ));
    });

    // Focus safety (module doc §3 / `loop-a11y-guidance.md` §7's "Focus
    // handling on rotation"): whenever `selected` changes, the slide that
    // was current a moment ago is about to (or, since the DOM patch that
    // sets its `inert` attribute is already applied by the time this
    // effect runs, may already) drop out of the visible/interactive set.
    // Per the HTML spec, an element becoming `inert` while it contains the
    // focused element unfocuses it and falls back to the document itself
    // (`<body>`) -- exactly the fallback this must never be seen to do.
    //
    // `focused_item` (each `CarouselItem`'s own plain, synchronous
    // `onfocusin`/`onfocusout`, no `document::eval` involved) is read here
    // as a snapshot of "was a slide focused right before this transition,
    // and if so which one" -- captured independently of whichever DOM
    // mutation already ran, so it still answers correctly even though a
    // `.peek()` of `document.activeElement` at this point could not (the
    // browser's own auto-blur has already fired by the time any effect,
    // this one included, gets to run -- effects only ever see the DOM
    // *after* the patch that triggered them, the same ordering every other
    // post-render `document::eval` in this module already relies on for
    // reading fresh geometry).
    //
    // The redirect target is [`CarouselContent`]'s own scroller element
    // (`content_id`) -- already unconditionally focusable
    // (`tabindex="0"`), never `inert`, and explicitly sanctioned as a
    // "stable control" by the contract ("the carousel's own focusable
    // region/content element, *or* the Next/Previous button that caused
    // the move"). A single, source-agnostic target rather than
    // special-casing which button (if any) caused the move keeps this one
    // effect correct for every transition source (buttons, the root
    // keyboard handler, `CarouselTab`, `CarouselApi::scroll_to`, and a
    // drag/wheel/trackpad settle that changes `selected` with no button
    // involved at all) without threading "who caused this" through every
    // one of them.
    //
    // Deliberately not gated more finely than "the previously-focused
    // slide is not the new `selected`": knowing whether that slide
    // *remains* inside the new multi-per-view visible window without
    // actually performing the redirect would need the post-scroll geometry
    // this effect cannot see yet (the visible-range settle above is still
    // in flight at this exact point) -- so a focused slide other than the
    // new current one is always treated as "about to need a fixup" even in
    // the rare multi-per-view case where it happens to still be visible
    // afterward. Safety over precision: an unnecessary refocus onto the
    // scroller (still inside the carousel, still a sensible place for
    // focus to be) is a far smaller cost than the alternative -- focus
    // silently dropping to `<body>`.
    let prev_selected = use_previous(selected.into());
    use_effect(move || {
        let new_selected = selected();
        let old_selected = prev_selected();
        if old_selected == new_selected {
            return;
        }
        let Some(focused_idx) = *focused_item.peek() else {
            return;
        };
        if focused_idx == new_selected {
            return;
        }
        let target = content_id.peek().clone();
        if target.is_empty() {
            return;
        }
        // Unconditional, not gated on re-reading `document.activeElement`
        // first: measured live (this session) that this effect's own
        // `document::eval` dispatch can run *before* Dioxus's own DOM patch
        // has applied the new `inert` attribute -- `document.activeElement`
        // at that moment still reports the slide's own descendant, still
        // genuinely focused, so a "only redirect if focus already looks
        // lost" check silently no-ops here and then never gets a second
        // chance once the patch (and the browser's own auto-blur-to-`body`
        // it triggers) actually lands a moment later. `focused_item` is
        // this component's own Rust-side judgment call, captured
        // synchronously and independently of DOM-patch timing (each
        // `CarouselItem`'s own `onfocusin`/`onfocusout`), so it is trusted
        // outright: whichever slide held focus right before this
        // transition is not the new current one, so a redirect is due
        // regardless of whether the browser has already carried out its
        // own fallback-to-`body` by the time this runs.
        document::eval(&format!(
            "var root = document.getElementById('{target}'); \
             if (root) {{ root.focus({{ preventScroll: true }}); }}"
        ));
    });

    let onkeydown = move |event: Event<KeyboardData>| {
        let key = event.key();
        let Some(intent) = carousel_key_intent(&key, orientation(), direction) else {
            return;
        };
        let loop_now = loop_enabled();
        match intent {
            HorizontalNav::Prev => set_selected.call(step_prev(selected(), count(), loop_now)),
            HorizontalNav::Next => set_selected.call(step_next(selected(), count(), loop_now)),
        }
        autoplay.note_interaction();
        event.prevent_default();
    };

    // Autoplay pause-on-hover/pause-on-focus -- always wired (cheap plain
    // signal writes; a harmless no-op with no `CarouselAutoplay` mounted,
    // since `autoplay_rotating` above can never be `true` without one).
    // See `AutoplayContext`'s own doc for the hover-vs-focus asymmetry.
    let onmouseenter = move |_| autoplay_hover.set(true);
    let onmouseleave = move |_| autoplay_hover.set(false);
    let onfocusin = move |_| {
        autoplay_focus_within.set(true);
        autoplay_resume_blocked.set(true);
    };
    let onfocusout = move |_| autoplay_focus_within.set(false);

    if !has_own_accessible_name(&props.attributes) {
        tracing::warn!(
            "Carousel: no aria-label or aria-labelledby was supplied. The APG Carousel \
             pattern requires the carousel region to have an accessible name that does not \
             itself contain the word \"carousel\" \
             (dev-docs/research/carousel-2026-09-19.md \u{a7}1.2)."
        );
    }

    // Merged (caller-wins for anything presentational, then component-owned
    // state wins last), not a literal beside a bare `..props.attributes`
    // spread: `role`/`aria_roledescription`/the two `data-*` attributes have
    // no typed field in `CarouselProps` claiming their names, so SSR would
    // otherwise emit both this literal and a same-named caller override,
    // which the HTML parser and a hydrating client resolve to opposite
    // values (`scripts/check-attr-spread-collision.sh`,
    // `dev-docs/issues/duplicate-attribute-root-cause.md`). These are all
    // structural/ARIA state this component must control, so they go last
    // (owned-wins) -- preserves today's production (SSR+hydrate) behavior
    // exactly, since the browser's own parser already resolves a duplicate
    // to the first (library) value.
    let attributes = merge_attributes(vec![
        props.attributes,
        attributes!(div {
            role: "region",
            aria_roledescription: "carousel",
            "data-orientation": orientation().as_str(),
            "data-direction": direction.as_str(),
        }),
    ]);

    rsx! {
        div {
            dir: direction.as_str(),
            onkeydown,
            onmouseenter,
            onmouseleave,
            onfocusin,
            onfocusout,
            ..attributes,

            {props.children}
        }
    }
}

/// The props for the [`CarouselContent`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CarouselContentProps {
    /// The ID of the carousel content element.
    pub id: ReadSignal<Option<String>>,

    /// Whether a mouse/pen pointer drag on the track pages the carousel --
    /// see this component's own "Pointer drag" doc. Defaults to `true`;
    /// touch is never affected either way (it already scrolls natively).
    #[props(default = ReadSignal::new(Signal::new(true)))]
    pub draggable: ReadSignal<bool>,

    /// Additional attributes to apply to the carousel content element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The [`CarouselItem`] children of the carousel content.
    pub children: Element,
}

/// # CarouselContent
///
/// The scroll-snap track: a native `overflow-{x,y}: auto;
/// scroll-snap-type` element containing every [`CarouselItem`]. This
/// element itself already clips its own overflowing children (that is
/// what `overflow: auto` means), so -- unlike an embla-style engine,
/// which translates a strip via `transform` and therefore needs a
/// *separate* `overflow: hidden` clipping wrapper -- no extra wrapper
/// element is needed here at all.
///
/// This must be used inside a [`Carousel`] component.
///
/// Carries `tabindex="0"`, unconditionally: like [`crate::scroll_area::ScrollArea`]'s
/// own doc puts it, "if you don't have any focusable content within the
/// scroll area, you should make the scroll area focusable" -- a
/// [`CarouselItem`] is not itself focusable unless its own content
/// happens to be, and axe's `scrollable-region-focusable` rule (WCAG
/// 2.1.1/2.1.3) confirms this by execution: without it, a keyboard user
/// who has no pointer at all has no way to reach this element's own
/// native scroll (arrow keys/Page Up/Page Down once focused) at all --
/// [`CarouselPrevious`]/[`CarouselNext`] page one slide at a time, but
/// axe is still right that the scrollable region itself should be
/// independently reachable, e.g. for finer scroll control than a full
/// slide-step in the `multiple`-per-view layout.
///
/// ## Pointer drag
///
/// A mouse or pen drag anywhere on the track pages the carousel, the same
/// way shadcn/embla-style carousels do -- layered on this element's own
/// scroll-snap track (a `scrollBy` per pointer move, never a second,
/// competing animation engine), and disableable per instance via
/// [`CarouselContentProps::draggable`] if a caller needs to opt out.
/// **Touch is never intercepted** -- it already scrolls this track
/// natively (`touch-action` is never set to `none` here), so a drag
/// gesture only ever starts for `pointerType === 'mouse' | 'pen'`.
///
/// Dragging is direct manipulation (the track tracks the pointer 1:1), so
/// under [`CarouselOrientation::Horizontal`] the physical direction that
/// reveals the next slide mirrors under `dir="rtl"` -- swipe-left-for-next
/// in LTR, swipe-right-for-next in RTL, the same familiar split a
/// right-to-left photo gallery or story viewer already has, and the same
/// direction [`Carousel`]'s own root-level `ArrowLeft`/`ArrowRight`
/// keyboard handling already swaps under RTL. No direction branch exists
/// in the drag code itself for this -- see `CAROUSEL_DRAG_JS`'s own doc
/// for why none is needed.
///
/// A drag has to move at least a few pixels from its origin before it
/// takes over -- below that threshold nothing happens at all, so a plain
/// click on a link/button inside a [`CarouselItem`] still works exactly
/// like a click. Once a real drag is recognized, the synthetic `click`
/// that would otherwise fire on release is suppressed (a capture-phase
/// listener on this element, ahead of both a native default action like
/// link navigation and any caller's own `onclick`), text selection is
/// suppressed for the gesture's duration, and pointer capture keeps the
/// drag tracking correctly even if the pointer leaves this element's
/// bounds mid-gesture.
///
/// On release, `endDrag` explicitly settles the track on whichever slide
/// is numerically nearest -- see this component's own "Release settle"
/// section below for the mechanism and why it is no longer simply
/// "restore `scroll-snap-type` and let the browser re-snap." That settle
/// (like every other native scroll) is what `use_carousel_scroll_tracking`
/// (already attached to this same element) picks up to update `selected`,
/// so a drag never invents a second source of truth for the index: the
/// Previous/Next buttons' `disabled` state, the "N of M" slide labels, and
/// a custom picker built on [`use_carousel`] all stay correct through the
/// exact same bridge a click or keypress already uses.
///
/// This gesture reads pointer state maintained entirely on the JS side
/// (see `CAROUSEL_DRAG_JS`'s own doc) and writes only a scroll position
/// per pointer move -- never a synchronous layout *read* in the same
/// handler (`dev-docs/recommended-implementations.md` §11 Rule 1); the one
/// read this construction needs (the container's own starting scroll
/// offset) is implicit in `scrollBy`'s own *relative* delta, so it is
/// never taken at all, on either side of the bridge. `endDrag` itself does
/// take a one-time `getBoundingClientRect` read once the pointer is
/// already released, to compute the settle target -- a discrete,
/// once-per-gesture event, not the continuous per-pointer-move path Rule 1
/// is about; see "Release settle" below.
///
/// ## Release settle
///
/// `endDrag` finds the slide nearest the track's raw (possibly unsnapped)
/// position at release -- the identical `getBoundingClientRect`-based
/// geometry `use_carousel_scroll_tracking`'s own `settle()` already uses
/// on this same element (a fresh `document::eval` string cannot import a
/// shared JS helper, so this is duplicated rather than called, but it is
/// the same technique, not a different one) -- and explicitly `scrollBy`s
/// this element (the scroller itself, never `nearest.scrollIntoView()`)
/// by the physical delta to that slide, with `behavior: 'smooth'` (or
/// `'auto'` under `prefers-reduced-motion: reduce`, the exact policy
/// [`CAROUSEL_SCROLL_TO_JS`] already applies to every other paging
/// path). This reuses that same scroller-only `scrollBy`-by-delta
/// mechanism this crate already pages with everywhere else -- not a
/// second, hand-rolled animation engine, and (see that constant's own
/// doc) not `scrollIntoView`, which would walk and scroll every
/// scrollable ancestor including the page.
///
/// This replaced simply restoring `scroll-snap-type` and trusting the
/// browser's own re-snap to both choose the destination *and* animate the
/// way there (round5's original construction, and still what
/// `onPointerMove`'s own "why suspend scroll-snap-type" comment
/// describes). Measured (the carousel-feel lane's own regression): it
/// never animated.
/// Sampling `.dx-carousel-content`'s own `scrollLeft` across a release
/// yielded exactly two distinct values -- the position mid-drag and the
/// final slide -- for drags of very different lengths, both landing
/// instantly. Setting `scrollSnapType` back to `'x mandatory'` on a
/// position that is already stationary is not treated by this engine
/// (Chromium) as "a scrolling operation resuming through skipped snap
/// points"; it is a synchronous, static re-evaluation, and nothing
/// animates a style recalculation. Explicitly `scrollBy`-ing this element
/// to the destination slide's delta with `behavior: 'smooth'` sidesteps
/// that path entirely: it is a genuine, browser-animated scroll operation,
/// the same kind "## scroll-snap-stop" below documents as actually
/// constrained/animated by this engine.
///
/// Restoring `scroll-snap-type` is **deferred**, not synchronous, for the
/// same reason: doing it immediately, right after starting the
/// `scrollBy` call, reintroduces the exact bug above the instant the
/// browser notices the position is (still, momentarily) not yet at a snap
/// point. So it waits for this specific scroll to actually finish
/// (`scrollend`, falling back to a fixed timeout where that event is
/// unsupported -- the same feature detection `use_carousel_scroll_tracking`
/// already uses, and the reason `CAROUSEL_SNAP_RESTORE_FALLBACK_MS`
/// exists) before touching the property again, so there is never a
/// stationary, unsnapped position for the browser to instantly correct
/// while this animation is still in flight. A fresh `pointerdown` cancels
/// any still-pending restore from a previous release rather than letting
/// it fire mid-*new*-drag and re-enable snapping while that drag's own
/// `scrollBy` calls are still running -- see `onPointerDown`'s own
/// comment.
///
/// This does not change *which* slide a long or fast drag lands on --
/// see "## scroll-snap-stop" below, "Known limitation," which still
/// applies, now for a restated reason.
///
/// ## Edge rubber-band (mode B3)
///
/// Dragging (mouse/pen) or scrolling (wheel/trackpad) past either end now
/// produces a damped elastic bounce, ported from a closed design (five
/// mechanisms benched side by side, `dev-docs/research/carousel-overscroll-2026-09-23.md`,
/// `dev-docs/backlog.md` row 102) rather than reimplemented from scratch
/// here. The full mechanism, its measurements, and the eight binding port
/// invariants live in that doc's §3-§6; this section records only the
/// shape of the port and what stays true of it.
///
/// **THE RULE this is built on:** at any moment during a wheel/trackpad
/// gesture on a scroll-snap container, the scroll position has three
/// would-be writers -- the compositor's own momentum animation (where
/// `preventDefault` is only advisory once momentum is live), scroll
/// snapping on its own schedule, and this crate's own code -- and two of
/// the three are neither observable nor sequenceable from JS. So where the
/// browser is already the scroller, this crate never writes the scroll
/// position and never disables snapping mid-gesture; the overdrag is
/// expressed purely as a `transform` on [`CarouselContent`]'s own element,
/// which the browser never writes and therefore never contests
/// (`CAROUSEL_WHEEL_BOUNCE_JS`, above). A mouse/pen drag has no native
/// scrolling behind it, so that same rule permits `CAROUSEL_DRAG_JS`'s own
/// pointer path to keep owning the scroll position exactly as it already
/// did before this port (`feedOverdrag`, which asks the scroller for the
/// whole of the requested motion and treats whatever it refuses as the
/// overdrag) -- the negative evidence behind this asymmetry (the pointer
/// path stayed clean for 20+ bench revisions untouched while the wheel
/// path glitched nearly every time) is measured, not assumed; see the
/// research doc §2.
///
/// **Why the transform lands on [`CarouselContent`]'s own *scroller*,
/// never the [`Carousel`] root.** The bench's own mode C (a whole-element
/// damped transform) felt closest to native but created a z-index
/// problem: a transform establishes a new containing block, and
/// translating the *root* -- an ancestor of
/// [`CarouselPrevious`]/[`CarouselNext`] -- would have dragged their own
/// absolutely-positioned placement along with it. The scroller is a
/// *sibling* of those buttons under [`Carousel`] (by way of the clipping
/// viewport wrapper described next), so transforming it alone leaves them
/// untouched; this is what dissolves that problem rather than working
/// around it (research doc §5).
///
/// **Correction, filed [2026-09-25](../../dev-docs/research/carousel-overscroll-2026-09-23.md):
/// this element does *not* self-clip a translated overdrag, and an earlier
/// version of this doc claimed it did.** An element's own `overflow`
/// clips relative to *its own* box, and a `transform` moves that box (and
/// its clip region) as one unit -- it does not clip the box against where
/// it would have painted before the transform. So translating this
/// element by its own overdrag moves its clip boundary along with its
/// content, and nothing clips the result: on real hardware a trackpad
/// overdrag pushed the whole visible slide track outside the carousel,
/// unclipped, overlapping whatever sat beside it on the page. The fix is
/// the bench's own construction, restored rather than reinvented: a
/// *stationary* wrapper -- `data-slot="carousel-viewport"`, `overflow:
/// clip` -- around this element, so the element inside it is what
/// translates while the clip boundary around it never moves.
/// `overflow: clip`, not `overflow: hidden`: a `hidden` box is still a
/// scroll container, and this module's paging/drag-release-settle paths
/// no longer risk scrolling it even so -- both now call `scrollBy()` on
/// the scroller element directly rather than `scrollIntoView()` on a
/// slide (see [`CAROUSEL_SCROLL_TO_JS`]'s own doc for why, and for the
/// incident that made that a hard requirement rather than a nice-to-have)
/// -- but `clip` establishes no scrollport at all regardless, so nothing
/// here (or added later) can ever scroll this wrapper by accident, the
/// same defense-in-depth reasoning as never giving it an `id`/ARIA for a
/// stray selector to latch onto. See that wrapper's own doc comment,
/// right above this component's `rsx!` body, for the full construction.
///
/// **Hydration parity.** Neither bridge ever touches a Dioxus-rendered
/// attribute: both mutate `element.style.transform` as a plain DOM write,
/// the same escape hatch `CAROUSEL_DRAG_JS`'s own `scroll-snap-type`
/// suspend/restore already uses and for the same reason (`document::eval`
/// is outside the vdom entirely). So first-render/SSR markup is
/// unaffected by construction, not by a special case: there is no signal,
/// no conditional render, nothing for hydration to reconcile.
///
/// **`prefers-reduced-motion`** drops the whole effect on both paths
/// (checked fresh per gesture/event, not cached at mount) rather than
/// merely skipping the spring-back animation -- this is cosmetic feedback
/// layered on scrolling that already happens regardless, so "no bounce" is
/// the correct reading of that preference, not a faster one.
///
/// **What this does not close.** [`CarouselPrevious`]/[`CarouselNext`]
/// remain genuinely `disabled` at the ends regardless (module doc,
/// "Accessibility") -- the bounce is additional physical feedback for a
/// drag/scroll gesture, not a replacement for that signal. Headless
/// synthetic wheel events (`playwright/carousel.spec.ts`'s own "edge
/// rubber-band" tests, via `page.mouse.wheel`) can exercise the edge-limit
/// branch and the spring-back, but cannot reproduce a real trackpad's
/// momentum/decay feel -- that remains a real-hardware check, as it was
/// for the bench itself (research doc's own header).
///
/// ## scroll-snap-stop
///
/// Every [`CarouselItem`] declares `scroll-snap-stop: always` as a plain
/// inline `style` property, right next to its own `scroll-snap-align`
/// (see that component's own source) -- matching every other
/// layout-critical property on this track/its slides (`scroll-snap-type`
/// here, `scroll-snap-align`/the flex layout there), this module's own
/// "works with zero theme CSS" convention (module doc, "Engine" section).
///
/// ### What this is for
///
/// `scroll-snap-stop: always` requires this scroll container to stop at
/// the *first* snap position a scroll operation would otherwise pass,
/// rather than skipping over several to settle on whichever is numerically
/// nearest -- the user's own explicit call: express embla-carousel's
/// default (non-`dragFree`, non-`skipSnaps`) "never skip past more than one
/// slide" outcome natively, without reimplementing its velocity model, and
/// accepting that a native-scroll engine will not *feel* identical to a
/// transform-based one. Measured (not assumed) to cap a genuine
/// browser-animated scroll operation -- a caller's own
/// `el.scrollBy({ left, behavior: 'smooth' })`, and by extension a native
/// wheel/trackpad fling, since both are the same "smooth scrolling
/// operation" the CSS Scroll Snap spec's own `scroll-snap-stop` language
/// describes -- to exactly one slide of travel regardless of the
/// requested distance.
///
/// ### Why a plain inline declaration, not `@supports`
///
/// An earlier version of this gated the property behind a scoped
/// `<style>` tag rendered as this element's own preceding sibling --
/// `@supports (scroll-snap-stop: always) { #id > * { ... } }` -- reasoning
/// that "only if the browser supports it" (the user's own explicit
/// requirement) has no equivalent inside a plain `style="..."` attribute,
/// since `@supports` is a stylesheet at-rule. That construction shipped
/// broken, and its own regression test was a false green. Dioxus emits
/// hydration marker comments *inside* a `<style>` element's own text
/// content (`<!--node-id413-->...<!--#-->`); `<!--`/`-->` are CDO/CDC
/// tokens that a stylesheet's top level normally ignores, but the ident
/// immediately following the opening one (`node-id413`) begins a
/// qualified rule whose prelude then swallows the real `@supports` rule
/// and takes its own `{...}` block as that rule's block, so the browser's
/// CSS parser drops the whole thing. Measured live against a hydrated
/// release SSG build: the `<style>` tag was present with the correct
/// `id`, matching the live element, and yet
/// `styleElement.sheet.cssRules.length` read `0` and
/// `getComputedStyle(slide).scrollSnapStop` read `"normal"` on every
/// slide -- a hydration-marker/CSS-parser hazard, not an id-scoping or
/// timing bug. (This would have worked in pure CSR, where Dioxus emits no
/// hydration markers at all -- very likely why it was originally believed
/// to work.) The test that shipped alongside it scrolled 2.5 slide widths
/// and asserted landing on slide 2, which nearest-snap settling alone
/// already gives regardless of whether the property applies at all, so it
/// never had the power to catch this.
///
/// No conditional gating is actually needed to satisfy "only if the
/// browser supports it": an unrecognized property inside a `style="..."`
/// attribute is simply dropped by the browser's own CSS parser by
/// definition, so a browser without `scroll-snap-stop` support silently
/// keeps today's behavior -- the same fallback `@supports` would have
/// produced, with no stylesheet, no id scoping, and no hydration-marker
/// hazard to get wrong.
///
/// ### Known limitation, measured rather than assumed: this crate's own
/// pointer-drag path does *not* get this guarantee
///
/// `CAROUSEL_DRAG_JS`'s mouse/pen drag moves the track with many
/// discrete `scrollBy({ behavior: 'instant' })` calls while
/// `scroll-snap-type` is suspended for the gesture's own duration (see
/// that constant's own doc for why), so a long or fast drag's raw
/// position at release can land several slide-widths from the origin with
/// no scrolling operation having passed through the intervening snap
/// points at all -- each jump was independently instant and unsnapped.
/// Originally (round5), the release simply restored `scroll-snap-type` and
/// trusted the browser's own re-snap to choose the destination: that
/// restore was not itself treated as "a scrolling operation resuming past
/// skipped snap points," but a fresh, static re-evaluation of an
/// already-stationary position, which this property does not appear to
/// constrain on this engine (Chromium) -- confirmed live, isolated from
/// this crate's own JS entirely (raw `el.scrollBy({ behavior: 'instant'
/// })` in a loop with `scroll-snap-type` suspended, then restored, on an
/// element whose child already computes `scroll-snap-stop: always`) --
/// the settle still landed on whichever slide was numerically nearest the
/// raw position, identical to what happens with the property absent
/// altogether. A same-tick zero-distance and 1px `behavior: 'smooth'`
/// nudge issued immediately after restoring `scroll-snap-type`, tried as
/// a cheap way to route the correction through the constrained "smooth
/// scroll" path instead, did not change this.
///
/// "## Release settle" above replaced that mechanism entirely -- release
/// no longer relies on restoring `scroll-snap-type` to pick a destination
/// at all, it explicitly targets whichever slide its own geometry search
/// finds nearest -- which is why release now visibly *animates* there
/// (this section's own regression, item 2, is fixed). But the
/// *destination* that search finds is still "whichever slide is
/// numerically nearest the raw drag position," with no notion of "cap to
/// one slide from the origin" -- `scroll-snap-stop`'s own guarantee is
/// about a scroll *operation* the browser resolves against intervening
/// snap points, and a plain nearest-neighbor geometry search never asks
/// the browser to resolve anything, so it was never a candidate to
/// inherit that guarantee either. So this specific gap -- a long or fast
/// drag can still land several slides from the origin -- is unchanged
/// today, now for this restated reason rather than the "static
/// re-evaluation" one above. This remains a real, open gap for this
/// crate's own pointer-drag feature specifically -- not for wheel/
/// trackpad/native scrolling on the same track, and not for a caller's
/// own `scrollBy`, both of which this property does correctly constrain.
///
/// Also not covered, and unreachable by construction rather than merely
/// unimplemented: a short, fast flick that settles back to its origin
/// instead of advancing is a velocity-shaped symptom, and no CSS property
/// can observe release speed. Closing it would mean a JS
/// distance-or-speed threshold -- i.e. reintroducing the velocity model
/// under another name -- which the user explicitly declined; see
/// `dev-docs/backlog.md` row 96 for the fuller history.
///
/// ## Styling
///
/// The [`CarouselContent`] component defines the following data
/// attributes you can use to control styling:
/// - `data-orientation`: `horizontal` or `vertical`, matching the parent
///   [`Carousel`]'s own.
/// - `data-draggable`: `true` or `false`, matching
///   [`CarouselContentProps::draggable`].
/// - `data-dragging`: present (`"true"`) only while an active mouse/pen
///   drag has crossed the movement threshold above; this is what the
///   themed package's own `cursor: grabbing` styling keys off.
///
/// The scroller is wrapped in a presentational `div
/// [data-slot="carousel-viewport"]` (see this component's "Edge
/// rubber-band" doc, "Correction" paragraph, above) that a caller's own
/// stylesheet can target for purely visual purposes; it carries none of
/// this component's own attributes/ARIA and is not part of the public
/// props surface.
#[component]
pub fn CarouselContent(props: CarouselContentProps) -> Element {
    let mut ctx: CarouselContext = use_context();
    let uuid = use_unique_id();
    let id = use_id_or(uuid, props.id);

    // Publish this element's own rendered id into the shared context so
    // `CarouselPrevious`/`CarouselNext` can point `aria-controls` at it --
    // the same construction `SelectTrigger`/`DropdownMenuTrigger` already
    // use for `trigger_id` (`dev-docs/conformance-harness.md` tier-2 Rule
    // 14), one layer in reverse (content publishing to its controls,
    // rather than a trigger publishing to its content).
    use_effect(move || {
        ctx.content_id.set(id());
    });

    use_carousel_scroll_tracking(
        id,
        ctx.orientation,
        ctx.align,
        ctx.set_selected,
        ctx.count,
        ctx.visible_range,
        None,
    );
    use_carousel_drag(id, ctx.orientation, ctx.align, props.draggable);
    use_carousel_wheel_bounce(id, ctx.orientation);

    let orientation = (ctx.orientation)();
    let draggable = (props.draggable)();

    // `aria-live` -- only when a `CarouselAutoplay` is actually present
    // (APG's own "optional" framing, `carousel-pattern.html`'s Roles
    // table -- a manually-paged carousel has no rotation to announce
    // around, so this stays absent exactly like it did before autoplay
    // existed at all, matching every already-shipped non-autoplay
    // variant/test). `"off"` while rotating (so a screen reader is not
    // interrupted every few seconds), `"polite"` once stopped (so
    // activating a tab/pressing Previous/Next while stopped is announced)
    // -- `dev-docs/research/carousel-2026-09-19.md` §1.2/§1.3 point 2:
    // deliberately no `aria-atomic`, matching the tested reference rather
    // than its own (self-contradicting) prose.
    let aria_live = (ctx.autoplay.present)().then(|| {
        if (ctx.autoplay.rotating)() {
            "off"
        } else {
            "polite"
        }
    });

    let (caller_style, rest_attrs) = fold_style_attributes(props.attributes);
    let gap_margin = content_gap_margin(orientation);
    let axis_style = match orientation {
        CarouselOrientation::Horizontal => {
            "display:flex;flex-direction:row;overflow-x:auto;overflow-y:hidden;\
             scroll-snap-type:x mandatory;"
        }
        CarouselOrientation::Vertical => {
            "display:flex;flex-direction:column;overflow-y:auto;overflow-x:hidden;\
             scroll-snap-type:y mandatory;"
        }
    };
    let style = format!(
        "{axis_style}{gap_margin}{}",
        caller_style.map(|s| format!(" {s}")).unwrap_or_default()
    );

    // Merged (owned-wins), spreading only the merged result -- never a
    // literal `style`/`tabindex`/`data-*` beside a raw `..rest_attrs`
    // spread (`scripts/check-attr-spread-collision.sh`). `rest_attrs`
    // already excludes any caller `style` (`fold_style_attributes` above
    // folded it into `style`, which is why it is safe to place `style`
    // itself in the owned group rather than concatenate it again here);
    // `tabindex`/the two `data-*` attributes are structural state this
    // component must keep correct (the scroll region's own focusability,
    // and the axis/drag-opt-out this crate's own CSS keys off), so they go
    // last, same as `Carousel`'s own root.
    let attributes = merge_attributes(vec![
        rest_attrs,
        attributes!(div {
            style: style.clone(),
            tabindex: "0",
            aria_live: aria_live,
            "data-orientation": orientation.as_str(),
            "data-draggable": draggable,
        }),
    ]);

    rsx! {
        // Clipping viewport (module doc's "Engine" section, and this
        // component's own "Edge rubber-band" doc above, have the full
        // reasoning): a stationary wrapper whose OWN box clips, so
        // translating the scroller inside it moves the content without
        // moving the clip boundary along with it. `overflow: clip`, never
        // `overflow: hidden` -- `hidden` is still a scroll container (the
        // CSSOM View spec's own "scrolling box" definition keys off any
        // `overflow` value other than `visible`/`clip`); the paging and
        // drag-release-settle paths never scroll THIS element either way
        // (both call `scrollBy()` on the scroller below directly, never
        // `scrollIntoView()` on a slide -- see `CAROUSEL_SCROLL_TO_JS`'s
        // own doc), but `clip` never creates a scrollport at all, so
        // nothing here or added later has anything to
        // scroll. Presentational only -- no `role`, no `id`, no ARIA, and
        // none of the caller's own `props.attributes` land here (every one
        // of those still goes on the scroller below, unchanged): this
        // element carries nothing hydration needs to reconcile and nothing
        // a caller can already reach through this component's own props,
        // so it needs no `attributes!`/`merge_attributes` construction
        // either -- there is no spread onto it for a literal to collide
        // with (`scripts/check-attr-spread-collision.sh`'s own class).
        // Present unconditionally on every render (SSR and CSR alike, and
        // never toggled by an effect), so hydration sees the identical
        // tree both times.
        div {
            "data-slot": "carousel-viewport",
            style: "overflow: clip;",

            div {
                id,
                ..attributes,

                {props.children}
            }
        }
    }
}

/// The props for the [`CarouselItem`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CarouselItemProps {
    /// The index of this slide (0-based). Determines both its default
    /// `"{n} of {m}"` accessible name and where it counts toward
    /// [`CarouselApi::count`] -- siblings must use contiguous indices
    /// starting at `0`, the same convention `tabs.rs`'s `TabTrigger`/
    /// `TabContent` already require of their own `index` prop.
    pub index: ReadSignal<usize>,

    /// The ID of the carousel item element.
    pub id: ReadSignal<Option<String>>,

    /// Additional attributes to apply to the carousel item element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the carousel item -- the slide's own content.
    pub children: Element,
}

/// # CarouselItem
///
/// A single slide: `role="group"` + `aria-roledescription="slide"`, with
/// a default `aria-label` of `"{n} of {m}"` unless the caller supplies
/// its own `aria-label`/`aria-labelledby`.
///
/// This must be used inside a [`CarouselContent`] component.
///
/// ## SSR stability
///
/// The `"{n} of {m}"` count is only known once every sibling
/// [`CarouselItem`] has registered itself, which -- like `tabs.rs`'s own
/// `tab_content_ids` -- happens via each item's own effect, and effects
/// do not run during SSR. So an SSR (or a client's own pre-hydration
/// first) render always sees `count == 0` and renders **no** `aria-label`
/// at all, filling it in once mounted. Server and the client's own first
/// render therefore always agree (both start from the same empty
/// registry) -- nothing here changes what SSR produced out from under
/// hydration; the label simply arrives, correctly, a moment later, the
/// same already-shipped tradeoff `tabs.rs`'s `aria-controls` makes.
///
/// ## Visible slides / `inert`
///
/// Every [`CarouselItem`] the a11y contract's own "visible" set does not
/// include (`dev-docs/research/carousel-loop-2026-09-25/loop-a11y-guidance.md`
/// §7, cross-checked against every a11y-layer-having carousel library's
/// own converged construction in `loop-libraries.md`) renders `inert` --
/// removed from both the Tab order and the accessibility tree in one
/// attribute, the Chrome "make accessible carousels" recipe's own
/// mechanism. "Visible" is the current slide (always, regardless of the
/// bridges below -- a safety net, not the primary signal: never leaves
/// the slide the rest of this component's own state calls "selected"
/// stranded `inert` by a stale/lagging geometry read) unioned with
/// [`CarouselContext::visible_range`], the `[start, end]` item indices
/// [`use_carousel_scroll_tracking`]/[`use_carousel_drag`] last measured
/// actually inside [`CarouselContent`]'s own viewport at rest -- which for
/// a single-slide-per-view layout is always just the current slide again,
/// and for a multi-per-view one (e.g. `flex-basis: 33%`) additionally
/// covers however many full neighbours are genuinely on screen.
///
/// **First render / SSR is deterministic.** [`CarouselContext::selected`]
/// is forced to `0` until every sibling has registered (this component's
/// own "SSR stability" doc above), and [`CarouselContext::visible_range`]
/// starts at its own matching default, `(0, 0)` -- so server and a
/// client's own pre-hydration first render both render only index `0` as
/// non-`inert`, agreeing without either needing a special case for the
/// other. A multi-per-view layout's true visible count is not knowable
/// from Rust-only state at all (it depends on `flex-basis`/gap CSS this
/// component never reads) -- widening past that single slide is real, but
/// always happens through a later effect's own signal write
/// ([`use_carousel_scroll_tracking`]'s own `ResizeObserver`, whose first
/// callback fires for the scroller's *current* box the moment it attaches
/// -- reached even for an already-aligned `default_value == 0` mount that
/// never scrolls on its own at all -- see that constant's own doc), never
/// inside this component's own first render.
///
/// **Frozen while moving.** [`use_carousel_scroll_tracking`]'s own settle
/// only ever writes `visible_range` at a genuine rest point -- on
/// `scrollend` (or the debounced-`scroll` fallback) or a `ResizeObserver`
/// firing, explicitly skipped for as long as `data-dragging` is set -- never
/// mid-drag, mid-momentum, or mid-smooth-page (the same rule
/// `dev-docs/research/carousel-overscroll-2026-09-23.md` §3/§6 already
/// binds every other scroll-position write in this module to). A drag's own
/// release settle reaches this only indirectly: `data-dragging` is removed
/// before `endDrag`'s own settle `scrollBy` runs, so the resulting native
/// scroll/scrollend event is what this bridge actually observes.
///
/// ## Styling
///
/// The [`CarouselItem`] component defines the following data attribute
/// you can use to control styling:
/// - `data-selected`: `true` when this is the currently selected slide,
///   `false` otherwise.
#[component]
pub fn CarouselItem(props: CarouselItemProps) -> Element {
    let mut ctx: CarouselContext = use_context();
    let uuid = use_unique_id();
    let id = use_id_or(uuid, props.id);
    let index = props.index;

    // Grow-only registration by index, mirroring `tabs.rs`'s
    // `tab_content_ids` exactly (see this component's "SSR stability"
    // doc above for why that shape is what keeps this SSR-stable).
    use_effect(move || {
        let idx = index();
        let mut ids = ctx.item_ids.write();
        while ids.len() <= idx {
            ids.push(String::new());
        }
        ids[idx] = id();
    });

    let selected = (ctx.selected)();
    let count = (ctx.count)();
    let is_selected = selected == index();
    let has_name = has_own_accessible_name(&props.attributes);
    let default_label = (!has_name && count > 0).then(|| slide_label(index(), count));

    // See this component's own "Visible slides / `inert`" doc above.
    //
    // `Option<&'static str>` (`None` when visible, never a bare `false`
    // `bool`) is load-bearing, not a style preference: `dioxus-interpreter-js`
    // 0.7.9's `set_attribute.ts` special-cases which HTML attributes are
    // "boolean" (present/absent) via its own hardcoded `isBoolAttr`
    // allowlist -- `disabled`/`hidden`/`checked`/etc. are on it, `inert` is
    // not (confirmed live: a raw `inert: bool` here rendered
    // `inert="false"` as a PRESENT attribute on the web target, which HTML
    // boolean-attribute semantics read as inert regardless of the string
    // value -- every slide came back inert, including the selected one).
    // A `bool`-typed dynamic attribute always carries a value (true or
    // false) as far as the vdom is concerned, so Dioxus can only ever emit
    // a `set_attribute` mutation for it -- through exactly that buggy
    // allowlist. An `Option`-typed one one that is genuinely absent when
    // `None`, so Dioxus instead emits a `remove_attribute` mutation
    // (`unified_bindings.rs`'s own `remove_attribute`, a plain
    // `node.removeAttribute(field)` for any field with no special case),
    // which never touches `isBoolAttr` at all -- and `Some("true")` still
    // reaches `setAttributeDefault`, but `truthy("true")` is `true`, so the
    // (buggy, allowlist-gated) removal branch is simply never reached
    // either way. This sidesteps the whole allowlist rather than trying to
    // special-case around it, and is the same `Option<&str>`-for-omission
    // construction this component's own `default_label`/`CarouselPrevious`'s
    // `aria_controls` already use for the identical "must be entirely
    // absent, not merely falsy" reason -- see `dev-docs/backlog.md` row 91's
    // dated addendum for the live repro this was found from.
    let (visible_start, visible_end) = (ctx.visible_range)();
    let in_visible_range = index() >= visible_start && index() <= visible_end;
    let is_visible = is_selected || in_visible_range;
    let inert: Option<&'static str> = (!is_visible).then_some("true");

    // Plain, synchronous native focus tracking -- no `document::eval`
    // involved (unlike almost everything else in this module): this is
    // read back, synchronously, by `Carousel`'s own focus-redirect effect
    // (see that effect's own doc for why a JS-side `document.activeElement`
    // read cannot answer the same question after the fact).
    let mut focused_item = ctx.focused_item;
    let onfocusin = move |_| focused_item.set(Some(index()));
    let onfocusout = move |_| {
        if (focused_item)() == Some(index()) {
            focused_item.set(None);
        }
    };

    // `role="tabpanel"` (in lieu of `group`) once a `CarouselTabList` has
    // registered -- the APG tabbed style, `carousel-2-tablist.html`'s own
    // markup. `aria-roledescription="slide"` stays regardless: the
    // vendored tabbed example's own markup keeps it alongside `tabpanel`
    // (`<div ... role="tabpanel" aria-roledescription="slide" ...>`),
    // contradicting the pattern page's own (unfollowed) prose that a
    // tabpanel "does not have the aria-roledescription property" --
    // `dev-docs/research/carousel-2026-09-19.md` §1.3 point 1 already
    // recommends following the tested example over the abstract prose.
    let role = if (ctx.tablist_present)() {
        "tabpanel"
    } else {
        "group"
    };

    // `scroll-snap-stop:always` -- see `CarouselContent`'s own doc, "##
    // scroll-snap-stop" section, for what this is, its known limitations,
    // and why this is a plain inline declaration rather than a stylesheet
    // gated by `@supports`.
    let gap_padding = item_gap_padding((ctx.orientation)());
    let basis = item_basis_style();
    let align_str = (ctx.align)().as_str();
    let (caller_style, rest_attrs) = fold_style_attributes(props.attributes);
    let style = format!(
        "{basis}scroll-snap-align:{align_str};scroll-snap-stop:always;min-width:0;min-height:0;{gap_padding}{}",
        caller_style.map(|s| format!(" {s}")).unwrap_or_default()
    );

    // Merged (owned-wins for structural/ARIA state; `aria_label` is a
    // presentational *default* -- caller-overridable, per `default_label`'s
    // own `has_own_accessible_name` gating above, which already never
    // computes a `Some` once the caller supplied their own name -- so it
    // is placed first rather than last, same as `CarouselPrevious`/
    // `CarouselNext`'s own default label below). Never a literal beside a
    // bare `..rest_attrs` spread (`scripts/check-attr-spread-collision.sh`).
    let attributes = merge_attributes(vec![
        attributes!(div {
            aria_label: default_label
        }),
        rest_attrs,
        attributes!(div {
            role: role,
            aria_roledescription: "slide",
            style: style.clone(),
            "data-selected": is_selected,
            // The stable data index and this child's logical position --
            // see `use_carousel_scroll_tracking`'s own doc for why every
            // JS bridge reads these rather than a child's array position.
            // Equal here (the plain children API has no window), so this
            // is the identical wire shape `CarouselVirtualContent` renders
            // for a real virtualised window -- one code path serves both.
            "data-index": index(),
            "data-position": index(),
            inert,
        }),
    ]);

    rsx! {
        div {
            id,
            onfocusin,
            onfocusout,
            ..attributes,

            {props.children}
        }
    }
}

/// The props for the [`CarouselVirtualContent`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CarouselVirtualContentProps<T: Clone + PartialEq + 'static> {
    /// The ID of the carousel content element.
    pub id: ReadSignal<Option<String>>,

    /// The full data set, in true order. Rendered in full (no window, no
    /// wrap) on the server and on the client's own first render -- see
    /// this component's own "SSR / first render" doc -- narrowed to a
    /// `radius`-wide window around the current slide in a post-mount
    /// effect once virtualisation is active.
    pub items: ReadSignal<Vec<T>>,

    /// Renders one slide's content, given its stable data index and
    /// value.
    pub render_item: Callback<(usize, T), Element>,

    /// Slides kept mounted on each side of the current one once
    /// virtualisation is active -- the window is `2 * radius + 1` slides
    /// wide. Defaults to `2`.
    #[props(default = 2usize)]
    pub radius: usize,

    /// Whether to virtualise at all. `None` (the default) auto-decides:
    /// virtualise only once `items.len() > 2 * radius + 1` (there is
    /// something to save by not rendering every slide). `Some(false)`
    /// always renders the full list (find-in-page/browse-mode reach every
    /// slide, at the cost of the DOM holding all of them). `Some(true)`
    /// always virtualises, even for a small data set.
    #[props(default)]
    pub virtualize: Option<bool>,

    /// Whether a mouse/pen pointer drag on the track pages the carousel --
    /// see [`CarouselContent`]'s own "Pointer drag" doc. Defaults to
    /// `true`; touch is never affected either way.
    #[props(default = ReadSignal::new(Signal::new(true)))]
    pub draggable: ReadSignal<bool>,

    /// Additional attributes to apply to the carousel content element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
}

/// # CarouselVirtualContent
///
/// A data-driven, virtualised drop-in for [`CarouselContent`] +
/// [`CarouselItem`]s: used INSIDE the same [`Carousel`] root, in place of
/// them, whenever the slides come from a plain `Vec<T>` rather than a
/// fixed set of children (`dev-docs/backlog.md` row 91's dated addendum
/// has the approved construction this implements).
///
/// Publishes its own count into [`CarouselContext`] the same way
/// [`CarouselItem`]'s own registration effect does, so
/// [`CarouselPrevious`]/[`CarouselNext`]/[`CarouselTabList`]/[`CarouselTab`]/
/// [`CarouselAutoplay`]/[`CarouselRotationControl`]/the root's own arrow
/// keys/[`use_carousel`] all keep working completely unchanged -- none of
/// them know this component exists; every one of them only ever reads or
/// writes [`CarouselContext`].
///
/// This must be used inside a [`Carousel`] component, in place of
/// [`CarouselContent`] (never alongside it).
///
/// ## SSR / first render
///
/// Renders every item, in true data order (`position == data index`, no
/// window, no wrap), on the server and on the client's own first render --
/// identical markup both times, so hydration has nothing to reconcile.
/// Virtualising (below) only ever begins in a post-mount effect,
/// client-side -- the same "first client render == SSR" property
/// [`CarouselItem`]'s own "SSR stability" doc already relies on for
/// `selected`/`visible_range`.
///
/// ## Virtualisation
///
/// Once mounted, if `virtualize` resolves to `true` (explicit, or the
/// auto default once `items.len() > 2 * radius + 1`), only a window of
/// `2 * radius + 1` logical POSITIONS around an internal `anchor` renders,
/// via [`crate::r#virtual::window`] (`wrap` = [`CarouselProps::r#loop`] AND
/// at least two slides -- one slide never loops, per this component's own
/// doc, "N=1 disables loop"). Each rendered slide carries both its stable
/// `data-index` (`position.rem_euclid(N)`, may repeat across positions
/// once `N < 2 * radius + 1`) and its own unique `data-position` (the
/// value every slide is keyed and DOM-id'd by, so a position that survives
/// an anchor move keeps its own DOM node -- and [`Self::render_item`]'s
/// own component state along with it).
///
/// With virtualisation inactive (a small data set, or
/// `virtualize: Some(false)`), [`CarouselProps::r#loop`] is the existing
/// APG **rewind** style, completely unchanged -- `position == data index`
/// throughout, and this component publishes a real per-data-index DOM id
/// into [`CarouselContext::item_ids`] exactly like [`CarouselItem`] does,
/// so [`Carousel`]'s own built-in mount/selected `scrollBy`-by-id paging
/// effect (and drag, wheel bounce, scroll tracking) drive it without any
/// bespoke code in this component at all.
///
/// ## Seamless loop
///
/// Only reached once virtualisation is active AND [`CarouselProps::r#loop`]
/// is on: `position` is allowed to run arbitrarily far past `[0, N)`
/// (mapped down to a data index only via `.rem_euclid(N)` for rendering),
/// so paging forward past the last data index does not rewind visibly
/// across every intervening slide the way the non-virtualised loop above
/// does -- it keeps sliding physically forward, one already-mounted window
/// slide at a time. [`CarouselContext::item_ids`] stays empty in this mode
/// (a data index can label more than one position at once, so there is no
/// single DOM id to publish per data index) -- the built-in paging effect
/// therefore harmlessly no-ops, and this component owns the reaction to
/// [`CarouselContext::selected`] changing itself:
///
/// - A change of exactly one data index -- [`CarouselPrevious`]/
///   [`CarouselNext`]/the root's arrow keys/autoplay all step by exactly
///   one -- resolves to the nearest matching POSITION (`anchor \u{b1} 1`)
///   and is animated there with the very same scroller-only
///   `scrollBy`-by-id helper ([`CAROUSEL_SCROLL_TO_JS`]) every other
///   paging path in this module already uses. The target position is
///   already a rendered window slide (`radius >= 1`), so this alone never
///   touches the window.
/// - A change to any other data index (a distant [`CarouselTab`] click, a
///   controlled `value` jump) resolves to whichever POSITION nearest the
///   current anchor carries that data index, sets `anchor` to it directly
///   -- an instant re-render around the new centre, so it is simply the
///   current slide the moment this runs, never a visible scroll across
///   the intervening ones -- and instantly re-aligns the scroller to it.
/// - Once a physical scroll genuinely settles (a real drag release, a
///   wheel/trackpad gesture, or the animated paging call above finishing
///   -- see [`use_carousel_scroll_tracking`]'s own doc for what "settles"
///   means), the settle's own reported POSITION (not just its data index)
///   becomes the new `anchor` directly, covering a native gesture that
///   moved the track by more than one slide with no extra arithmetic.
///
/// Either way, `anchor` changing re-renders the window keyed by
/// `position` (so every surviving slide keeps its DOM node across the
/// move), and a dedicated effect immediately re-aligns the scroller to
/// the new centre with one more instant [`CAROUSEL_SCROLL_TO_JS`] call --
/// needed because the window's own far edge changing (one position
/// dropped, one added) shifts every later flex sibling's physical layout
/// offset by one slide's width, which a plain DOM patch does not
/// otherwise compensate for.
///
/// **Deviation from the bench's own microtask-timed correction, recorded
/// rather than silently substituted:** the bench prototype
/// (`dev-docs/research/carousel-overscroll-bench.html`, `seamlessLoop()`)
/// runs this same re-alignment from a `MutationObserver` callback -- a
/// microtask, guaranteed to run before the browser paints the DOM patch
/// that triggered it. This component instead runs it from a plain
/// `use_effect`, scheduled after the triggering render has already
/// committed; whether that is early enough to still land before paint on
/// every engine is an empirical question this lane's own Playwright
/// per-`requestAnimationFrame` sampling test is what actually answers, not
/// an assumption -- see this lane's own report for the measured result,
/// and upgrade to a `MutationObserver` if it is not.
///
/// ## Non-loop, virtualised
///
/// With virtualisation active but [`CarouselProps::r#loop`] off, the
/// window still clamps at the ends (`wrap = false` -- position never
/// leaves `[0, N)`, so it equals data index throughout), and the existing
/// edge rubber-band bridges apply exactly as they do for the plain
/// children API.
///
/// ## Dynamic `N`, and the degenerate counts
///
/// `N == 0` renders no slides at all (the scroller shell still mounts, an
/// empty scroll region, matching [`CarouselContent`]'s own shape with zero
/// children). `N == 1` disables looping regardless of
/// [`CarouselProps::r#loop`] -- there is nothing to wrap to. A dynamic
/// change in `N` reclamps [`CarouselContext::selected`] the same way it
/// already does for the plain children API (`clamp_selected`); this
/// component's own `anchor` degrades gracefully too, since
/// `anchor.rem_euclid(N)` is well-defined for any `N > 0` regardless of
/// how large `anchor` has drifted.
#[component]
pub fn CarouselVirtualContent<T: Clone + PartialEq + 'static>(
    props: CarouselVirtualContentProps<T>,
) -> Element {
    let mut ctx: CarouselContext = use_context();
    let uuid = use_unique_id();
    let id = use_id_or(uuid, props.id);
    let items = props.items;
    let render_item = props.render_item;
    let radius = props.radius.max(1);
    let explicit_virtualize = props.virtualize;

    use_effect(move || {
        ctx.content_id.set(id());
    });

    let count = use_memo(move || items().len());

    // SSR / first render always renders the full list -- see this
    // component's own "SSR / first render" doc. `mounted` flips exactly
    // once, client-side, in a post-mount effect.
    let mut mounted = use_signal(|| false);
    use_effect(move || {
        mounted.set(true);
    });

    // Recomputed (not cached) inside every effect below that needs it, so
    // each one tracks `count()`/`mounted()` directly rather than trusting
    // that a fresh closure alone reruns it -- see this component's own
    // lane report for why that distinction matters here.
    let virtualize_active_now = move || {
        let n = count();
        n > 0 && mounted() && explicit_virtualize.unwrap_or_else(|| n > 2 * radius + 1)
    };

    let mut anchor = use_signal(|| (ctx.selected)() as isize);
    // The position the next physical step should be computed relative to
    // -- see the paging effect's own doc, "Load-bearing guard" and the
    // dedicated doc just above it, for why this must be tracked
    // separately from `anchor` (which only ever moves once a physical
    // scroll has genuinely settled).
    let mut intended = use_signal(|| (ctx.selected)() as isize);

    // Publish `count` into the shared context, and either a real
    // per-data-index id (small/non-virtualised -- the existing built-in
    // paging effect just works, see "Virtualisation" doc) or an empty
    // registry (seamless loop -- see "Seamless loop" doc for why this
    // component owns paging itself in that mode instead).
    use_effect(move || {
        let scroller_id = id();
        let n = count();
        let active = virtualize_active_now();
        let mut ids = ctx.item_ids.write();
        ids.clear();
        for i in 0..n {
            ids.push(if active {
                String::new()
            } else {
                format!("{scroller_id}-p{i}")
            });
        }
    });

    // A settle (native drag/wheel/trackpad, or one of this component's own
    // animated paging calls below finishing) reports the slide it actually
    // rested on -- `anchor` (drives the render window) and `intended`
    // (the paging effect's own "what to compute the next step from")
    // both snap to it together, so the two can never drift apart once a
    // physical scroll has genuinely finished.
    let on_settle_position = use_callback(move |position: isize| {
        anchor.set(position);
        intended.set(position);
    });
    use_carousel_scroll_tracking(
        id,
        ctx.orientation,
        ctx.align,
        ctx.set_selected,
        ctx.count,
        ctx.visible_range,
        Some(on_settle_position),
    );
    use_carousel_drag(id, ctx.orientation, ctx.align, props.draggable);
    use_carousel_wheel_bounce(id, ctx.orientation);

    // Translate a `selected` (data index) change into a physical move --
    // see "Seamless loop" doc. A no-op whenever virtualisation is
    // inactive: the built-in `Carousel`-level effect (real per-data-index
    // ids, published above) already handles that case.
    //
    // **Two load-bearing fixes, both found live (this lane's own dx-serve
    // verification against the real, running dev server -- not merely
    // reasoned through):**
    //
    // 1. This must compute the next target from `intended` (the last
    //    position THIS effect itself has already asked for), never from
    //    `anchor` (the last position a physical scroll actually SETTLED
    //    on). `anchor` only updates once `on_settle_position` above fires
    //    -- which for an animated (not instant) step happens only after
    //    the browser's own `scrollend` -- so a second `selected` change
    //    arriving before the first step has finished animating (found
    //    live: consecutive `CarouselAutoplay` ticks close enough together)
    //    would otherwise still read the PRE-first-step anchor, compute the
    //    same target the first step already asked for, and silently
    //    swallow one entire step (measured: ticks 1..12 with `delay_ms:
    //    1200` skipped index 6 outright, landing on 5 then 7). Reading
    //    `intended` instead -- updated immediately, in the same tick this
    //    effect runs, in both branches below -- means a rapid-fire second
    //    change always computes relative to where the first one is
    //    already headed, not where the scroller has physically gotten to
    //    yet.
    // 2. Skip entirely once `intended` already denotes `new_sel`: a settle
    //    calls `ctx.set_selected` and `on_settle_position` together for
    //    the SAME reported slide, so by the time this effect observes
    //    that `selected` change, `intended` (like `anchor`) may already
    //    agree with it. Recomputing a jump from `old_sel` regardless (an
    //    earlier version of this effect did exactly that, before
    //    `intended` existed at all) treated the settle's own report as a
    //    fresh navigation request layered on the *previous* selected
    //    value, landing on a second, wrong position and triggering
    //    another settle -- an unbounded feedback loop, measured live: the
    //    anchor drifted continuously in one direction for as long as the
    //    page was left open, entirely without user interaction. Only a
    //    change nothing has already reconciled (a button/key/autoplay/
    //    tab/`scroll_to` call, none of which ever touch `intended`
    //    themselves) reaches the jump logic below.
    let prev_selected = use_previous(ctx.selected.into());
    let orientation = ctx.orientation;
    let align = ctx.align;
    let content_id = ctx.content_id;
    use_effect(move || {
        let new_sel = (ctx.selected)();
        let old_sel = prev_selected();
        let n = count();
        if !virtualize_active_now() || n == 0 || old_sel == new_sel {
            return;
        }
        let cur_intended = *intended.peek();
        if n > 0 && cur_intended.rem_euclid(n as isize) as usize == new_sel {
            // `intended` already agrees with the new selected value -- see
            // this effect's own doc, fix 2, for why this must be a no-op.
            return;
        }
        let wrapped = shortest_signed_delta(old_sel, new_sel, n);
        if wrapped == 0 {
            return;
        }
        let target_position = cur_intended + wrapped;
        let scroller_id = content_id.peek().clone();
        if scroller_id.is_empty() {
            return;
        }
        let orientation_str = orientation().as_str().to_string();
        // **Third load-bearing fix, also found live.** Whether the target
        // is "already a rendered window slide" must be measured against
        // `anchor` (the last SETTLED position -- what the render window
        // is actually keyed from), never against `intended`'s own step
        // size. `intended` moves the instant this effect runs (fix 1,
        // above), but the window only re-centres once a settle actually
        // lands -- which, for an animated (non-instant) step, only
        // happens after the browser's own `scrollend`, real wall-clock
        // time later. A second (or third) `selected` change arriving
        // before that -- measured live: two `CarouselNext` clicks a few
        // hundred ms apart, comfortably human-paced, well inside a
        // `radius: 2` window's own margin -- can walk `intended` further
        // from `anchor` than `radius` even though each individual STEP is
        // still exactly one. Using `wrapped` (the step size) here instead
        // of this distance let that second step's own animated call
        // target a position outside the still-`anchor`-centred window --
        // an id with no element behind it -- so the `scrollBy` silently
        // no-opped (`CAROUSEL_SCROLL_TO_JS`'s own null-target guard) and
        // the click was lost outright, with the *original* (first) step's
        // own scroll finishing normally afterward and its genuinely
        // correct settle then overwriting `intended` back down to its own
        // target -- confirmed live via a temporary trace: "old_sel=2
        // new_sel=3" (the second click, computed correctly) followed
        // moments later by "settle position=2" (the first click's own,
        // late-arriving, entirely valid settle silently reverting the
        // second one). Measuring against `anchor` instead means a target
        // that has drifted outside the true rendered window takes the
        // instant re-anchor branch below -- which needs no pre-rendered
        // element at all -- rather than silently failing to reach an
        // element that was never going to exist.
        let cur_anchor = *anchor.peek();
        if (target_position - cur_anchor).unsigned_abs() as usize <= radius {
            // Already a rendered window slide -- animate the existing
            // scroller-only paging helper to it. `intended` moves right
            // away (fix 1); `anchor` (and the window it drives) only
            // moves once `on_settle_position` reports this scroll has
            // actually finished.
            intended.set(target_position);
            let target_id = format!("{scroller_id}-p{target_position}");
            let align_str = align().as_str().to_string();
            let eval = document::eval(CAROUSEL_SCROLL_TO_JS);
            let _ = eval.send((
                scroller_id,
                target_id,
                orientation_str,
                false,
                CAROUSEL_SNAP_RESTORE_FALLBACK_MS,
                align_str,
            ));
        } else {
            // Outside the current window (a distant jump, OR `intended`
            // has drifted further from the last settle than `radius`
            // allows) -- re-anchor both instantly; the alignment effect
            // below does the matching instant scroll once this render has
            // committed.
            intended.set(target_position);
            anchor.set(target_position);
        }
    });

    // Re-align the scroller to `anchor`'s own slide -- covers the
    // instant-jump branch above, every settle-driven move (paging, drag,
    // wheel/trackpad) reported back by `use_carousel_scroll_tracking`,
    // AND the one-time transition from full-list to windowed rendering
    // (`virtualize_active` flipping `false` -> `true`): the window's own
    // radius-worth of leading slides only exist once that flip happens,
    // so the scroller's physical position (wherever it happened to rest
    // showing the full list) needs the identical realignment even though
    // `anchor`'s own numeric value does not change at that moment.
    // `align_key` bundles both triggers into one comparison rather than
    // two separate effects racing each other over the same scroller.
    // Always instant: by the time this runs, the physical "this should
    // feel like a slide" motion has already happened, either via the
    // browser's own native scroll or via the animated
    // `CAROUSEL_SCROLL_TO_JS` call above -- this call only ever
    // compensates the window's own re-centring (or its own initial
    // appearance), which must not itself be seen to animate.
    let align_key = use_memo(move || (anchor(), virtualize_active_now()));
    let prev_align_key = use_previous(align_key.into());
    use_effect(move || {
        let (a, active) = align_key();
        let (pa, pactive) = prev_align_key();
        if !active || (a == pa && active == pactive) {
            return;
        }
        let scroller_id = content_id.peek().clone();
        if scroller_id.is_empty() {
            return;
        }
        let target_id = format!("{scroller_id}-p{a}");
        let orientation_str = orientation().as_str().to_string();
        let align_str = align().as_str().to_string();
        let eval = document::eval(CAROUSEL_SCROLL_TO_JS);
        let _ = eval.send((
            scroller_id,
            target_id,
            orientation_str,
            true,
            CAROUSEL_SNAP_RESTORE_FALLBACK_MS,
            align_str,
        ));
    });

    let n = count();
    let virtualize_active = virtualize_active_now();
    let loop_now = (ctx.loop_enabled)() && n >= 2;

    let win: Vec<crate::r#virtual::WindowItem> = if n == 0 {
        Vec::new()
    } else if virtualize_active {
        crate::r#virtual::window(n, anchor(), anchor(), radius, loop_now)
    } else {
        crate::r#virtual::window(n, 0, n as isize - 1, 0, false)
    };

    let selected = (ctx.selected)();
    let (visible_start, visible_end) = (ctx.visible_range)();
    let tablist_present = (ctx.tablist_present)();
    let mut focused_item = ctx.focused_item;
    let items_now = items();
    let scroller_id_now = id();

    let orientation_now = (ctx.orientation)();
    let align_now = (ctx.align)();
    let draggable = (props.draggable)();
    let aria_live = (ctx.autoplay.present)().then(|| {
        if (ctx.autoplay.rotating)() {
            "off"
        } else {
            "polite"
        }
    });

    let (caller_style, rest_attrs) = fold_style_attributes(props.attributes);
    let gap_margin = content_gap_margin(orientation_now);
    let axis_style = match orientation_now {
        CarouselOrientation::Horizontal => {
            "display:flex;flex-direction:row;overflow-x:auto;overflow-y:hidden;\
             scroll-snap-type:x mandatory;"
        }
        CarouselOrientation::Vertical => {
            "display:flex;flex-direction:column;overflow-y:auto;overflow-x:hidden;\
             scroll-snap-type:y mandatory;"
        }
    };
    let style = format!(
        "{axis_style}{gap_margin}{}",
        caller_style.map(|s| format!(" {s}")).unwrap_or_default()
    );

    let attributes = merge_attributes(vec![
        rest_attrs,
        attributes!(div {
            style: style.clone(),
            tabindex: "0",
            aria_live: aria_live,
            "data-orientation": orientation_now.as_str(),
            "data-draggable": draggable,
        }),
    ]);

    let slides = win.into_iter().map(move |item| {
        let data_index = item.data_index;
        let position = item.position;
        let is_selected = data_index == selected;
        let in_visible_range = data_index >= visible_start && data_index <= visible_end;
        let is_visible = is_selected || in_visible_range;
        let inert: Option<&'static str> = (!is_visible).then_some("true");
        let role = if tablist_present { "tabpanel" } else { "group" };
        let label = slide_label(data_index, n);
        let slide_id = format!("{scroller_id_now}-p{position}");
        let value = items_now[data_index].clone();
        let content = render_item.call((data_index, value));
        let item_style = format!(
            "{}scroll-snap-align:{};scroll-snap-stop:always;min-width:0;min-height:0;{}",
            item_basis_style(),
            align_now.as_str(),
            item_gap_padding(orientation_now)
        );
        rsx! {
            div {
                key: "{position}",
                id: slide_id,
                role,
                aria_roledescription: "slide",
                aria_label: label,
                style: item_style,
                "data-selected": is_selected,
                "data-index": data_index,
                "data-position": position,
                inert,
                onfocusin: move |_| focused_item.set(Some(data_index)),
                onfocusout: move |_| {
                    if (focused_item)() == Some(data_index) {
                        focused_item.set(None);
                    }
                },
                {content}
            }
        }
    });

    rsx! {
        // Clipping viewport -- see `CarouselContent`'s own "Edge
        // rubber-band" doc for the full construction; this wrapper is
        // identical to that one's.
        div {
            "data-slot": "carousel-viewport",
            style: "overflow: clip;",

            div {
                id,
                ..attributes,

                {slides}
            }
        }
    }
}

/// The props shared by [`CarouselPrevious`] and [`CarouselNext`].
#[derive(Props, Clone, PartialEq)]
pub struct CarouselPreviousProps {
    /// Additional attributes to apply to the button element.
    #[props(extends = GlobalAttributes)]
    #[props(extends = button)]
    pub attributes: Vec<Attribute>,

    /// The children of the button -- its visible content. The accessible
    /// name comes from `aria-label` (defaulted to `"Previous slide"`)
    /// regardless of what is rendered here, so this can be purely
    /// decorative (an icon, an arrow glyph).
    pub children: Element,
}

/// # CarouselPrevious
///
/// A native `<button type="button">` that moves to the previous slide,
/// genuinely `disabled` (not merely `aria-disabled`) when already at the
/// first slide -- unless [`CarouselProps::r#loop`] is enabled, in which
/// case this is never `disabled` at all (it wraps to the last slide
/// instead). Default accessible name `"Previous slide"`, overridable with
/// your own `aria-label`/`aria-labelledby`. `aria-controls` points at the
/// sibling [`CarouselContent`]'s own id.
///
/// This must be used inside a [`Carousel`] component.
#[component]
pub fn CarouselPrevious(props: CarouselPreviousProps) -> Element {
    let ctx: CarouselContext = use_context();
    let selected = (ctx.selected)();
    let loop_enabled = (ctx.loop_enabled)();
    let disabled = !loop_enabled && !can_scroll_prev(selected);
    let default_label = (!has_own_accessible_name(&props.attributes)).then_some("Previous slide");
    let content_id = (ctx.content_id)();
    let aria_controls = (!content_id.is_empty()).then_some(content_id);

    // Merged (caller-wins for the presentational defaults, then
    // component-owned state wins last), not a literal beside a bare
    // `..props.attributes` spread (`scripts/check-attr-spread-collision.sh`).
    // `type`/`aria_label` are overridable defaults (a caller wanting
    // `type="submit"` or their own label still can); `aria_controls`/
    // `disabled` are ARIA-relationship and real-boundary state this
    // component must keep correct, so they go last -- owned-wins preserves
    // today's production (SSR+hydrate) behavior exactly, since a caller
    // could otherwise (attempt to) re-enable a button already correctly
    // disabled at a real boundary.
    let attributes = merge_attributes(vec![
        attributes!(button {
            r#type: "button",
            aria_label: default_label
        }),
        props.attributes,
        attributes!(button {
            aria_controls,
            disabled
        }),
    ]);

    rsx! {
        button {
            onclick: move |_| {
                let loop_now = (ctx.loop_enabled)();
                ctx.set_selected.call(step_prev((ctx.selected)(), (ctx.count)(), loop_now));
                ctx.autoplay.note_interaction();
            },
            ..attributes,

            {props.children}
        }
    }
}

/// # CarouselNext
///
/// The mirror of [`CarouselPrevious`]: moves to the next slide, `disabled`
/// at the last slide unless [`CarouselProps::r#loop`] is enabled (in which
/// case it wraps to the first slide instead and is never `disabled`),
/// default accessible name `"Next slide"`.
///
/// This must be used inside a [`Carousel`] component.
#[component]
pub fn CarouselNext(props: CarouselPreviousProps) -> Element {
    let ctx: CarouselContext = use_context();
    let selected = (ctx.selected)();
    let count = (ctx.count)();
    let loop_enabled = (ctx.loop_enabled)();
    let disabled = !loop_enabled && !can_scroll_next(selected, count);
    let default_label = (!has_own_accessible_name(&props.attributes)).then_some("Next slide");
    let content_id = (ctx.content_id)();
    let aria_controls = (!content_id.is_empty()).then_some(content_id);

    // See `CarouselPrevious`'s identical construction, just above, for the
    // full precedence rationale.
    let attributes = merge_attributes(vec![
        attributes!(button {
            r#type: "button",
            aria_label: default_label
        }),
        props.attributes,
        attributes!(button {
            aria_controls,
            disabled
        }),
    ]);

    rsx! {
        button {
            onclick: move |_| {
                let loop_now = (ctx.loop_enabled)();
                ctx.set_selected.call(step_next((ctx.selected)(), (ctx.count)(), loop_now));
                ctx.autoplay.note_interaction();
            },
            ..attributes,

            {props.children}
        }
    }
}

/// A read-only snapshot of a [`Carousel`]'s state, plus the one action
/// ([`Self::scroll_to`]) that does not already have a dedicated button
/// component -- returned by [`use_carousel`]. Mirrors shadcn's own
/// `CarouselApi` shape (`selected`/`count`/`can_scroll_prev`/
/// `can_scroll_next`) closely enough that a caller who has used shadcn's
/// Carousel (or `dignifiedquire/dx-components`' own `CarouselApi`,
/// `use_carousel`) will recognize it immediately.
#[derive(Clone, Copy, PartialEq)]
pub struct CarouselApi {
    /// The currently selected slide's index (0-based).
    pub selected: usize,
    /// The number of currently-registered [`CarouselItem`]s.
    pub count: usize,
    /// Whether [`Self::scroll_to`] to `selected - 1` would move anywhere.
    pub can_scroll_prev: bool,
    /// Whether [`Self::scroll_to`] to `selected + 1` would move anywhere.
    pub can_scroll_next: bool,
    scroll_to: Callback<usize>,
    autoplay: AutoplayContext,
}

impl CarouselApi {
    /// Move directly to the given slide index (clamped into range the
    /// same way every other path in this module is). For building a
    /// custom picker (e.g. a row of dot indicators) alongside
    /// [`CarouselPrevious`]/[`CarouselNext`], which only ever step by one.
    /// Counts as "interaction" for [`CarouselAutoplayProps::stop_on_interaction`]
    /// the same way clicking Previous/Next does.
    pub fn scroll_to(&self, index: usize) {
        self.autoplay.note_interaction();
        self.scroll_to.call(index);
    }
}

/// Read the nearest ancestor [`Carousel`]'s current state and reach its
/// `scroll_to` action, for building custom controls (e.g. dot indicators)
/// alongside or instead of [`CarouselPrevious`]/[`CarouselNext`].
///
/// Panics (via [`use_context`]) if called outside a [`Carousel`].
///
/// ## Example
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::carousel::{Carousel, CarouselContent, CarouselItem, use_carousel};
/// #[component]
/// fn Indicators() -> Element {
///     let api = use_carousel();
///     rsx! {
///         for i in 0..api.count {
///             button {
///                 "aria-label": "Go to slide {i + 1}",
///                 "data-active": i == api.selected,
///                 onclick: move |_| api.scroll_to(i),
///             }
///         }
///     }
/// }
/// ```
pub fn use_carousel() -> CarouselApi {
    let ctx: CarouselContext = use_context();
    let selected = (ctx.selected)();
    let count = (ctx.count)();
    let loop_enabled = (ctx.loop_enabled)();
    CarouselApi {
        selected,
        count,
        can_scroll_prev: (loop_enabled && count > 0) || can_scroll_prev(selected),
        can_scroll_next: (loop_enabled && count > 0) || can_scroll_next(selected, count),
        scroll_to: ctx.set_selected,
        autoplay: ctx.autoplay,
    }
}

/// One-shot: reads `window.matchMedia('(prefers-reduced-motion: reduce)').matches`
/// and reports it back. The one required JS read even under this module's
/// "almost no JS" posture (`dev-docs/research/carousel-2026-09-19.md` §5.2)
/// that has to be observed from the *Rust* side rather than only ever
/// branched on inside a script -- [`CarouselAutoplay`] needs the answer to
/// decide whether to start its own timer at all, a decision that lives in
/// Rust. Mirrors the reduced-motion checks already inline inside
/// `CAROUSEL_DRAG_JS`/`CAROUSEL_WHEEL_BOUNCE_JS`, just surfaced instead of
/// only ever consulted in place.
const CAROUSEL_REDUCED_MOTION_JS: &str =
    "dioxus.send(window.matchMedia('(prefers-reduced-motion: reduce)').matches);";

/// The props for the [`CarouselAutoplay`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CarouselAutoplayProps {
    /// Milliseconds between automatic slide advances. Mirrors
    /// `embla-carousel-autoplay`'s own `delay` option, including its
    /// default (`4000`).
    #[props(default = ReadSignal::new(Signal::new(4000)))]
    pub delay_ms: ReadSignal<u64>,

    /// Whether rotation stops for good (until [`CarouselRotationControl`]
    /// is explicitly clicked again) after a manual paging action --
    /// Previous/Next, the root keyboard handler, a [`CarouselTab`], or a
    /// custom picker's own [`CarouselApi::scroll_to`]. Mirrors
    /// `embla-carousel-autoplay`'s own `stopOnInteraction` option and its
    /// default (`true`).
    ///
    /// **Scope note (v1):** a native pointer-drag or wheel/trackpad scroll
    /// does **not** count as "interaction" here -- only the discrete
    /// paging actions above do. Wiring the drag/wheel bridges too is a
    /// fast-follow, not a silent gap: see [`AutoplayContext::note_interaction`]'s
    /// own doc for exactly which call sites are wired.
    #[props(default = ReadSignal::new(Signal::new(true)))]
    pub stop_on_interaction: ReadSignal<bool>,

    /// Whether hovering anywhere in the carousel pauses rotation --
    /// resuming once the pointer leaves, unless focus is *also* currently
    /// holding it paused (see [`AutoplayContext`]'s own "pause/resume
    /// state machine" doc for the hover-vs-focus asymmetry, taken directly
    /// from the vendored tabbed reference's own accessibility prose).
    /// Mirrors `embla-carousel-autoplay`'s own `stopOnMouseEnter` option
    /// and its default (`true`).
    #[props(default = ReadSignal::new(Signal::new(true)))]
    pub stop_on_mouse_enter: ReadSignal<bool>,

    /// Whether this carousel should start rotating on mount at all --
    /// before the one-shot `prefers-reduced-motion` check below, which
    /// always forces it off regardless of this value (APG's own reference
    /// does the same: `dev-docs/research/carousel-2026-09-19.md` §1.3
    /// point 4). Defaults to `true`, matching `embla-carousel-autoplay`'s
    /// own default of starting immediately.
    #[props(default = ReadSignal::new(Signal::new(true)))]
    pub default_playing: ReadSignal<bool>,
}

/// # CarouselAutoplay
///
/// Installs the APG "auto-rotating" carousel's timer behavior: ticks every
/// [`CarouselAutoplayProps::delay_ms`], paging through the same [`step_next`]
/// path [`CarouselNext`] uses (so [`CarouselProps::r#loop`] applies
/// identically -- without `loop`, rotation simply stops once it reaches
/// the last slide, rather than ticking forever against a no-op the way
/// `embla-carousel-autoplay`'s own documented behavior does; with `loop`,
/// it rewinds and keeps going). Renders no DOM of its own -- pair it with
/// a [`CarouselRotationControl`] (placed **first**, before any other
/// focusable content, per APG's own requirement) to give the user a way to
/// toggle it, and see [`AutoplayContext`]'s own doc for the full
/// pause/resume state machine (hover, focus, and the sticky
/// "does not auto-resume after focus" rule).
///
/// `aria-live` on [`CarouselContent`] (`"off"` while rotating, `"polite"`
/// once stopped) and the rotation control's own toggling label are both
/// driven by this component's presence -- neither exists at all in a
/// [`Carousel`] with no [`CarouselAutoplay`] mounted, preserving every
/// already-shipped non-autoplay variant's markup exactly.
///
/// `prefers-reduced-motion: reduce` always forces rotation off at mount,
/// regardless of [`CarouselAutoplayProps::default_playing`] -- checked
/// once, client-side only, via a post-mount effect that starts from the
/// same deterministic value SSR and the client's own pre-hydration first
/// render already agree on, so there is no hydration mismatch (mirrors
/// [`Carousel`]'s own `is_first`/mount-settle construction).
///
/// This must be used inside a [`Carousel`] component.
#[component]
pub fn CarouselAutoplay(props: CarouselAutoplayProps) -> Element {
    let ctx: CarouselContext = use_context();
    let autoplay = ctx.autoplay;

    // Publish presence + the two opt-outs every render -- plain signal
    // writes, cheap even when unchanged (Dioxus signals already no-op a
    // same-value write), the same "publish into a shared context slot"
    // idiom `CarouselContent`'s own `content_id` publish above uses.
    use_effect(move || {
        let mut present = autoplay.present;
        let mut stop_on_interaction = autoplay.stop_on_interaction;
        let mut stop_on_mouse_enter = autoplay.stop_on_mouse_enter;
        present.set(true);
        stop_on_interaction.set((props.stop_on_interaction)());
        stop_on_mouse_enter.set((props.stop_on_mouse_enter)());
    });

    // Mount-time default + the one-shot `prefers-reduced-motion` read,
    // consumed exactly once -- mirrors `Carousel`'s own `is_first`
    // construction (see that effect's own doc): SSR and the client's
    // pre-hydration first render both start `playing` from the same
    // deterministic value (`default_playing`), so there is no hydration
    // mismatch; this effect then corrects it once, client-side only, if
    // the platform prefers reduced motion.
    let mut checked_reduced_motion = use_signal(|| false);
    use_effect(move || {
        if *checked_reduced_motion.peek() {
            return;
        }
        checked_reduced_motion.set(true);
        let mut playing = autoplay.playing;
        playing.set((props.default_playing)());
        spawn(async move {
            let mut eval = document::eval(CAROUSEL_REDUCED_MOTION_JS);
            if let Ok(true) = eval.recv::<bool>().await {
                playing.set(false);
            }
        });
    });

    // The ticking loop. Restarts fresh (no partial-elapsed carry-over
    // across a pause/resume -- a deliberate v1 simplification) every time
    // `rotating` flips, cancelling whatever was previously running first
    // -- mirrors `menu_sub.rs`'s own `DelayedAction::schedule` cancel-
    // before-reschedule idiom, this crate's established precedent for a
    // cancellable spawned timer. Always torn down: both on every flip to
    // `false` (this effect's own re-run, via the `take()` below) and on
    // unmount (`use_effect_cleanup`), so no timer ever outlives the
    // component or a single "rotating" span -- `Task::cancel()` is
    // immediate, not cooperative.
    let mut running: Signal<Option<Task>> = use_signal(|| None);
    let count = ctx.count;
    let selected = ctx.selected;
    let loop_enabled = ctx.loop_enabled;
    let set_selected = ctx.set_selected;
    use_effect(move || {
        let active = (autoplay.rotating)();
        if let Some(task) = running.write().take() {
            task.cancel();
        }
        if !active {
            return;
        }
        let delay = Duration::from_millis((props.delay_ms)().max(1));
        let task = spawn(async move {
            loop {
                sleep(delay).await;
                let count_now = *count.peek();
                if count_now == 0 {
                    continue;
                }
                let loop_now = *loop_enabled.peek();
                let current = *selected.peek();
                // Stops rather than ticking forever against a no-op once a
                // non-looping carousel reaches its last slide -- see this
                // component's own doc, "pick shadcn's behavior and
                // document it": `embla-carousel-autoplay`'s own documented
                // behavior keeps its interval alive indefinitely once
                // `scrollNext()` becomes a permanent no-op at the end;
                // this construction instead cancels the timer outright,
                // which is observably identical (nothing further ever
                // advances) but does not spin a no-op interval forever.
                if !loop_now && current + 1 >= count_now {
                    break;
                }
                set_selected.call(step_next(current, count_now, loop_now));
            }
        });
        running.set(Some(task));
    });
    use_effect_cleanup(move || {
        if let Some(task) = running.write().take() {
            task.cancel();
        }
    });

    rsx! {}
}

/// The props for the [`CarouselRotationControl`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CarouselRotationControlProps {
    /// Additional attributes to apply to the button element.
    #[props(extends = GlobalAttributes)]
    #[props(extends = button)]
    pub attributes: Vec<Attribute>,

    /// The children of the button -- its visible content (e.g. a
    /// play/pause icon). The accessible name comes from `aria-label`
    /// (defaulted to the two toggling strings below) regardless of what is
    /// rendered here.
    pub children: Element,
}

/// # CarouselRotationControl
///
/// A native `<button type="button">` that toggles [`CarouselAutoplay`]'s
/// rotation on and off. Accessible name toggles between `"Start automatic
/// slide show"`/`"Stop automatic slide show"` (overridable) -- deliberately
/// **no** `aria-pressed`: the changing label itself is the state (APG's
/// own explicit rotation-control contract, `dev-docs/research/carousel-2026-09-19.md`
/// §1.2/§1.3). Activating it never moves focus (native `<button>`
/// semantics), so a user can toggle rotation repeatedly without losing
/// their place.
///
/// **Composition:** place this **first** among [`Carousel`]'s children --
/// before [`CarouselPrevious`]/[`CarouselTabList`]/[`CarouselContent`] --
/// so it is the first focusable element in the carousel, matching APG's
/// own explicit requirement ("Rotation control ... precede\[s\] the slide
/// content in the Tab sequence"). This component cannot enforce that
/// order itself, only document it -- the same tradeoff [`Carousel`]'s own
/// doc already makes for [`CarouselPrevious`]/[`CarouselNext`] preceding
/// [`CarouselContent`].
///
/// This must be used inside a [`Carousel`] alongside a [`CarouselAutoplay`]
/// -- clicking it before one has mounted is a harmless no-op (there is
/// nothing to toggle yet).
#[component]
pub fn CarouselRotationControl(props: CarouselRotationControlProps) -> Element {
    let ctx: CarouselContext = use_context();
    let mut autoplay = ctx.autoplay;
    let playing = (autoplay.playing)();
    let default_label = (!has_own_accessible_name(&props.attributes)).then_some(if playing {
        "Stop automatic slide show"
    } else {
        "Start automatic slide show"
    });

    // Merged (caller-wins for the overridable label default), not a
    // literal beside a bare `..props.attributes` spread
    // (`scripts/check-attr-spread-collision.sh`).
    let attributes = merge_attributes(vec![
        attributes!(button {
            r#type: "button",
            aria_label: default_label
        }),
        props.attributes,
    ]);

    rsx! {
        button {
            onclick: move |_| {
                autoplay.playing.set(!playing);
                // An explicit click always re-arms resume, whichever
                // direction it toggles -- "if a user activates the
                // rotation control button ... it is assumed the user
                // wants auto-rotation to start immediately" (the vendored
                // tabbed reference's own accessibility-features prose) --
                // so a later focus-driven pause never inherits a stale
                // block left over from before this click.
                autoplay.resume_blocked.set(false);
            },
            // Stops its own `focusin`/`focusout` from reaching `Carousel`'s
            // root (the `focus_within`/`resume_blocked` pause) -- note
            // `focusin`/`focusout` specifically, the bubbling pair the root
            // actually listens for, not `focus`/`blur` (a non-bubbling
            // pair whose own propagation is a separate dispatch entirely;
            // stopping *that* one does nothing to the other, confirmed by
            // execution: an earlier version of this stopped `focus`/`blur`
            // and the very failure below persisted unchanged). A
            // narrowly-scoped exemption, NOT the vendored reference's own
            // broader "ignore hover/focus once explicitly started" quirk
            // (this module's own doc already declines to port that class
            // of behavior, `dev-docs/research/carousel-2026-09-19.md` §1.3
            // point 3): a mouse click on this very button moves DOM focus
            // onto it (standard Chromium `<button>` behavior), and without
            // this, `focus_within` would immediately re-pause the very
            // rotation this click just started, defeating a mouse click's
            // own visible effect -- confirmed by execution
            // (`playwright/carousel.spec.ts`'s own "autoplay" describe
            // block was red without this). Keyboard/pointer focus on every
            // OTHER focusable piece of the carousel (Previous/Next, the
            // content track, a `CarouselTab`) still pauses normally.
            onfocusin: move |event: Event<FocusData>| event.stop_propagation(),
            onfocusout: move |event: Event<FocusData>| event.stop_propagation(),
            ..attributes,

            {props.children}
        }
    }
}

/// Shared roving-focus state for one [`CarouselTabList`], consumed by its
/// [`CarouselTab`] children -- the tablist's *own* [`CollectionState`],
/// entirely separate from [`CarouselContext`]'s own `selected`/`count`
/// (which stay the single source of truth for which slide is current;
/// this collection only tracks *focus* among the tab buttons themselves,
/// the same separation `tabs.rs`'s own `TabsContext`/`CollectionState`
/// pair keeps).
#[derive(Clone, Copy)]
struct CarouselTabListContext {
    focus: CollectionState,
    /// Reused verbatim from [`CarouselContext::direction`] -- see that
    /// field's own doc for why a fresh `use_direction(None)` call here
    /// would risk disagreeing with the [`Carousel`] root's own resolution.
    direction: Direction,
}

/// The props for the [`CarouselTabList`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CarouselTabListProps {
    /// Additional attributes to apply to the tablist element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The [`CarouselTab`] children.
    pub children: Element,
}

/// # CarouselTabList
///
/// The slide-picker row for the APG **tabbed** carousel style:
/// `role="tablist"` containing one [`CarouselTab`] per slide. Composing
/// this on this crate's own `crate::collection` roving-focus machinery
/// (the same module [`crate::tabs::Tabs`] itself is built on) rather than
/// [`crate::tabs::Tabs`]/`TabTrigger`/`TabContent` directly was a
/// deliberate choice, not an oversight -- two concrete reasons, found by
/// trying the literal composition first:
///
/// 1. **`TabContent` hides its inactive panels** (`hidden: !selected()`).
///    A carousel's slides must all stay simultaneously present in
///    [`CarouselContent`]'s own scroll-snap track -- "showing" a slide
///    here means scrolling to it, never toggling `hidden` -- so
///    `TabContent` is structurally the wrong shape regardless of any
///    other consideration. [`CarouselItem`] already exists and is reused
///    unchanged (just switching its own `role` to `tabpanel`, see that
///    component's own doc).
/// 2. **`TabTrigger`'s own arrow-key handling is manual activation**
///    (`tabs.rs`: arrow keys only move focus; only `Enter`/`Space`/click
///    actually switches the active tab, via `TabTrigger`'s `onclick`).
///    APG's tabbed carousel requires **automatic** activation instead --
///    "Right Arrow: Moves focus to the next tab ... Shows the slide
///    associated with the newly focused tab" (no `Enter` needed,
///    `examples/carousel-2-tablist.html`'s own keyboard table). Building
///    [`CarouselTab`] directly on `crate::collection` (below) makes this
///    one line (`onfocus` calling [`CarouselContext::set_selected`]
///    directly) instead of a manual-activation component fighting its own
///    contract.
///
/// What *is* reused, per the brief's own "compose, don't reimplement"
/// recommendation: the exact same `collection_item`/`use_item`/
/// `CollectionState` roving-tabindex machinery `TabTrigger` itself is
/// built on (`crate::collection`, `pub(crate)` and already shared across
/// this crate) -- so this is still composition on `tabs.rs`'s own
/// underlying focus engine, one layer below the `Tabs`/`TabTrigger`
/// components themselves, not a hand-rolled reimplementation of roving
/// focus.
///
/// Arrow-key wrap (`ArrowLeft`/`ArrowRight` -- RTL-aware, reusing
/// [`Carousel`]'s own resolved [`Direction`] -- wraps at both ends
/// unconditionally, per APG's own tablist keyboard table ("If focus is on
/// the last tab, moves focus to the first tab"). This is the *tablist's*
/// own always-circular navigation, independent of [`CarouselProps::r#loop`]
/// (which governs [`CarouselPrevious`]/[`CarouselNext`]/the root keyboard
/// instead, and defaults to `false`).
///
/// This must be used inside a [`Carousel`] component.
#[component]
pub fn CarouselTabList(props: CarouselTabListProps) -> Element {
    let ctx: CarouselContext = use_context();
    let mut tablist_present = ctx.tablist_present;

    use_effect(move || {
        tablist_present.set(true);
    });

    // Always wraps (`ReadSignal::new(Signal::new(true))`) -- see this
    // component's own doc, "Arrow-key wrap."
    let focus = use_collection_provider(ReadSignal::new(Signal::new(true)));
    use_context_provider(|| CarouselTabListContext {
        focus,
        direction: ctx.direction,
    });

    let has_name = has_own_accessible_name(&props.attributes);
    // "Choose slide to display" -- this crate's own design sketch's
    // suggested name (`dev-docs/research/carousel-2026-09-19.md` §5.4);
    // the vendored reference itself uses the shorter `aria-label="Slides"`
    // (`examples/carousel-2-tablist.html`) -- APG requires *a* meaningful
    // name, not this exact string, so either is conformant; overridable
    // either way.
    let default_label = (!has_name).then_some("Choose slide to display");

    let attributes = merge_attributes(vec![
        attributes!(div {
            aria_label: default_label
        }),
        props.attributes,
        attributes!(div { role: "tablist" }),
    ]);

    rsx! {
        div {
            ..attributes,
            {props.children}
        }
    }
}

/// The props for the [`CarouselTab`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CarouselTabProps {
    /// The index of the slide this tab controls (0-based) -- the same
    /// convention [`CarouselItem::index`] and `tabs.rs`'s own
    /// `TabTrigger`/`TabContent` `index` props use.
    pub index: ReadSignal<usize>,

    /// Additional attributes to apply to the tab button element.
    #[props(extends = GlobalAttributes)]
    #[props(extends = button)]
    pub attributes: Vec<Attribute>,

    /// The children of the tab -- its visible content (e.g. a dot/thumbnail
    /// icon). The accessible name comes from `aria-label` (defaulted to
    /// `"Slide {n}"`, matching the vendored reference's own
    /// `aria-label="Slide X"`) regardless of what is rendered here.
    pub children: Element,
}

/// # CarouselTab
///
/// One slide-picker button: `role="tab"`, roving `tabindex`, `aria-selected`,
/// `aria-controls` pointing at the matching [`CarouselItem`]'s own id.
/// Focusing a tab -- by `Tab`/`Shift+Tab` landing on it (already the
/// selected one, roving tabindex) or by `ArrowLeft`/`ArrowRight`/`Home`/`End`
/// moving focus onto it -- **automatically** selects and scrolls to its
/// slide (no `Enter`/click needed), matching APG's own tabbed-style
/// automatic-activation contract. See [`CarouselTabList`]'s own doc for
/// why this is a purpose-built component on `crate::collection` rather
/// than [`crate::tabs::TabTrigger`] directly (manual activation there is
/// the wrong contract for this pattern).
///
/// This must be used inside a [`CarouselTabList`] component, with indices
/// matching the sibling [`CarouselItem`]s one-to-one.
#[component]
pub fn CarouselTab(props: CarouselTabProps) -> Element {
    let carousel_ctx: CarouselContext = use_context();
    let tablist_ctx: CarouselTabListContext = use_context();
    let index = props.index;

    let is_selected = use_memo(move || (carousel_ctx.selected)() == index());

    let item =
        use_item(collection_item(tablist_ctx.focus, index).selected(move || is_selected.cloned()));
    let onmounted = item.onmounted();
    let tabindex = item.tabindex;

    let default_label =
        (!has_own_accessible_name(&props.attributes)).then(|| format!("Slide {}", index() + 1));
    let aria_controls = (carousel_ctx.item_ids)()
        .get(index())
        .cloned()
        .filter(|s| !s.is_empty());

    // Merged (caller-wins for the overridable label default, then
    // component-owned ARIA/roving state last), not a literal beside a
    // bare `..props.attributes` spread (`scripts/check-attr-spread-collision.sh`).
    let attributes = merge_attributes(vec![
        attributes!(button {
            r#type: "button",
            aria_label: default_label
        }),
        props.attributes,
        attributes!(button {
            role: "tab",
            tabindex: tabindex,
            aria_selected: is_selected(),
            aria_controls: aria_controls,
        }),
    ]);

    rsx! {
        button {
            onmounted,
            onfocus: move |_| {
                let mut focus = tablist_ctx.focus;
                focus.set_focus(Some(index()));
                carousel_ctx.set_selected.call(index());
                carousel_ctx.autoplay.note_interaction();
            },
            onkeydown: move |event: Event<KeyboardData>| {
                let key = event.key();
                let mut focus = tablist_ctx.focus;
                let mut prevent_default = true;
                match key {
                    Key::ArrowLeft | Key::ArrowRight => {
                        match tablist_ctx.direction.resolve_horizontal(&key) {
                            Some(HorizontalNav::Prev) => focus.focus_prev(),
                            Some(HorizontalNav::Next) => focus.focus_next(),
                            None => {}
                        }
                    }
                    Key::Home => focus.focus_first(),
                    Key::End => focus.focus_last(),
                    _ => prevent_default = false,
                }
                if prevent_default {
                    event.prevent_default();
                }
            },
            ..attributes,

            {props.children}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_selected_within_bounds_is_unchanged() {
        assert_eq!(clamp_selected(1, 3), 1);
    }

    #[test]
    fn clamp_selected_clamps_to_last_index() {
        assert_eq!(clamp_selected(99, 3), 2);
    }

    #[test]
    fn clamp_selected_with_zero_count_is_zero() {
        assert_eq!(clamp_selected(0, 0), 0);
        assert_eq!(clamp_selected(5, 0), 0);
    }

    #[test]
    fn clamp_visible_range_within_bounds_is_unchanged() {
        assert_eq!(clamp_visible_range((0, 2), 5), (0, 2));
    }

    #[test]
    fn clamp_visible_range_clamps_end_past_last_index() {
        assert_eq!(clamp_visible_range((1, 99), 3), (1, 2));
    }

    #[test]
    fn clamp_visible_range_clamps_start_past_last_index_too() {
        assert_eq!(clamp_visible_range((50, 60), 3), (2, 2));
    }

    #[test]
    fn clamp_visible_range_with_zero_count_is_zero() {
        assert_eq!(clamp_visible_range((0, 0), 0), (0, 0));
        assert_eq!(clamp_visible_range((4, 9), 0), (0, 0));
    }

    #[test]
    fn can_scroll_prev_false_at_first_slide() {
        assert!(!can_scroll_prev(0));
    }

    #[test]
    fn can_scroll_prev_true_after_first_slide() {
        assert!(can_scroll_prev(1));
    }

    #[test]
    fn can_scroll_next_true_before_last_slide() {
        assert!(can_scroll_next(0, 3));
        assert!(can_scroll_next(1, 3));
    }

    #[test]
    fn can_scroll_next_false_at_last_slide() {
        assert!(!can_scroll_next(2, 3));
    }

    #[test]
    fn can_scroll_next_false_with_zero_slides() {
        assert!(!can_scroll_next(0, 0));
    }

    #[test]
    fn next_selected_advances_by_one() {
        assert_eq!(next_selected(0, 3), 1);
        assert_eq!(next_selected(1, 3), 2);
    }

    #[test]
    fn next_selected_stops_at_last_slide_without_wrapping() {
        assert_eq!(next_selected(2, 3), 2);
    }

    #[test]
    fn next_selected_with_zero_slides_is_zero() {
        assert_eq!(next_selected(0, 0), 0);
    }

    #[test]
    fn prev_selected_retreats_by_one() {
        assert_eq!(prev_selected(2), 1);
        assert_eq!(prev_selected(1), 0);
    }

    #[test]
    fn prev_selected_stops_at_first_slide_without_wrapping() {
        assert_eq!(prev_selected(0), 0);
    }

    #[test]
    fn step_prev_without_loop_matches_prev_selected() {
        assert_eq!(step_prev(0, 3, false), prev_selected(0));
        assert_eq!(step_prev(2, 3, false), prev_selected(2));
    }

    #[test]
    fn step_prev_with_loop_wraps_from_first_to_last() {
        assert_eq!(step_prev(0, 3, true), 2);
    }

    #[test]
    fn step_prev_with_loop_mid_range_steps_back_by_one() {
        assert_eq!(step_prev(2, 3, true), 1);
    }

    #[test]
    fn step_prev_with_loop_and_zero_slides_stays_zero() {
        assert_eq!(step_prev(0, 0, true), 0);
    }

    #[test]
    fn step_next_without_loop_matches_next_selected() {
        assert_eq!(step_next(0, 3, false), next_selected(0, 3));
        assert_eq!(step_next(2, 3, false), next_selected(2, 3));
    }

    #[test]
    fn step_next_with_loop_wraps_from_last_to_first() {
        assert_eq!(step_next(2, 3, true), 0);
    }

    #[test]
    fn step_next_with_loop_mid_range_steps_forward_by_one() {
        assert_eq!(step_next(0, 3, true), 1);
    }

    #[test]
    fn step_next_with_loop_and_zero_slides_stays_zero() {
        assert_eq!(step_next(0, 0, true), 0);
    }

    #[test]
    fn slide_label_is_one_based_n_of_m() {
        assert_eq!(slide_label(0, 6), "1 of 6");
        assert_eq!(slide_label(5, 6), "6 of 6");
    }

    // `is_mount_settle` -- round5's fix for "slide 1 -> 2 jumps instead of
    // animating" (see the function's own doc, and `Carousel`'s
    // mount-time effect, for the construction). The pre-fix bug was
    // exactly the case `mount_settle_is_false_before_any_items_have_registered`
    // below combined with `mount_settle_is_true_the_instant_items_exist`
    // *not* being reachable before a real navigation -- gating on
    // `has_items` rather than "did this particular id lookup succeed" is
    // what makes `mount_settle_is_false_on_a_real_navigation_after_mount`
    // hold unconditionally.
    #[test]
    fn mount_settle_is_false_before_any_items_have_registered() {
        // The mount effect's own early runs, before `CarouselItem`
        // registration completes -- `is_first` is still `true`, but there
        // is nothing to scroll to yet, so this must not consume it.
        assert!(!is_mount_settle(true, false));
    }

    #[test]
    fn mount_settle_is_true_the_instant_items_exist() {
        // The first run where `has_items` flips `true` -- whether that is
        // this effect's very first execution (items already registered
        // by the time it runs) or a later retry once registration
        // completes (the common `default_value == 0` case, where
        // `selected()`'s own value never changes) -- this is always the
        // mount-time settle, and must be instant.
        assert!(is_mount_settle(true, true));
    }

    #[test]
    fn mount_settle_is_false_on_a_real_navigation_after_mount() {
        // Once the caller has set `is_first` to `false` (which it does on
        // every run where `has_items` is `true`, per this function's own
        // doc), no later run -- including every genuine user navigation --
        // can ever read `true` again, regardless of `has_items`.
        assert!(!is_mount_settle(false, true));
        assert!(!is_mount_settle(false, false));
    }

    #[test]
    fn carousel_key_intent_horizontal_ltr_matches_physical_keys() {
        assert_eq!(
            carousel_key_intent(
                &Key::ArrowLeft,
                CarouselOrientation::Horizontal,
                Direction::Ltr
            ),
            Some(HorizontalNav::Prev)
        );
        assert_eq!(
            carousel_key_intent(
                &Key::ArrowRight,
                CarouselOrientation::Horizontal,
                Direction::Ltr
            ),
            Some(HorizontalNav::Next)
        );
    }

    #[test]
    fn carousel_key_intent_horizontal_rtl_swaps_physical_keys() {
        assert_eq!(
            carousel_key_intent(
                &Key::ArrowLeft,
                CarouselOrientation::Horizontal,
                Direction::Rtl
            ),
            Some(HorizontalNav::Next)
        );
        assert_eq!(
            carousel_key_intent(
                &Key::ArrowRight,
                CarouselOrientation::Horizontal,
                Direction::Rtl
            ),
            Some(HorizontalNav::Prev)
        );
    }

    #[test]
    fn carousel_key_intent_vertical_ignores_direction() {
        for direction in [Direction::Ltr, Direction::Rtl] {
            assert_eq!(
                carousel_key_intent(&Key::ArrowUp, CarouselOrientation::Vertical, direction),
                Some(HorizontalNav::Prev)
            );
            assert_eq!(
                carousel_key_intent(&Key::ArrowDown, CarouselOrientation::Vertical, direction),
                Some(HorizontalNav::Next)
            );
        }
    }

    #[test]
    fn carousel_key_intent_vertical_ignores_horizontal_arrow_keys() {
        assert_eq!(
            carousel_key_intent(
                &Key::ArrowLeft,
                CarouselOrientation::Vertical,
                Direction::Ltr
            ),
            None
        );
        assert_eq!(
            carousel_key_intent(
                &Key::ArrowRight,
                CarouselOrientation::Vertical,
                Direction::Ltr
            ),
            None
        );
    }

    #[test]
    fn carousel_key_intent_ignores_unrelated_keys() {
        for orientation in [
            CarouselOrientation::Horizontal,
            CarouselOrientation::Vertical,
        ] {
            assert_eq!(
                carousel_key_intent(&Key::Home, orientation, Direction::Ltr),
                None
            );
            assert_eq!(
                carousel_key_intent(&Key::Enter, orientation, Direction::Ltr),
                None
            );
        }
    }

    #[test]
    fn orientation_as_str_matches_the_data_orientation_tokens() {
        assert_eq!(CarouselOrientation::Horizontal.as_str(), "horizontal");
        assert_eq!(CarouselOrientation::Vertical.as_str(), "vertical");
    }

    #[test]
    fn orientation_default_is_horizontal() {
        assert_eq!(
            CarouselOrientation::default(),
            CarouselOrientation::Horizontal
        );
    }

    #[test]
    fn shortest_signed_delta_zero_count_is_zero() {
        assert_eq!(shortest_signed_delta(0, 0, 0), 0);
    }

    #[test]
    fn shortest_signed_delta_same_index_is_zero() {
        assert_eq!(shortest_signed_delta(3, 3, 12), 0);
    }

    #[test]
    fn shortest_signed_delta_forward_step_is_plus_one() {
        assert_eq!(shortest_signed_delta(4, 5, 12), 1);
    }

    #[test]
    fn shortest_signed_delta_backward_step_is_minus_one() {
        assert_eq!(shortest_signed_delta(5, 4, 12), -1);
    }

    #[test]
    fn shortest_signed_delta_wraps_forward_at_the_end() {
        // step_next's own rewind wrap: last -> first is +1 in position
        // space, not -(count - 1).
        assert_eq!(shortest_signed_delta(11, 0, 12), 1);
    }

    #[test]
    fn shortest_signed_delta_wraps_backward_at_the_start() {
        assert_eq!(shortest_signed_delta(0, 11, 12), -1);
    }

    #[test]
    fn shortest_signed_delta_picks_the_nearer_direction_for_a_distant_jump() {
        // 12 slides, jumping from 1 to 7 (6 apart either way) -- either
        // signed direction is equally short; the formula's own tie-break
        // (`wrapped > n/2`, strict) keeps the positive/forward one rather
        // than flipping to `-6`.
        assert_eq!(shortest_signed_delta(1, 7, 12), 6);
        // A jump that is unambiguously nearer forward.
        assert_eq!(shortest_signed_delta(1, 4, 12), 3);
        // ...and unambiguously nearer backward.
        assert_eq!(shortest_signed_delta(1, 10, 12), -3);
    }

    #[test]
    fn shortest_signed_delta_small_count_never_exceeds_half() {
        for count in 2..8 {
            for old in 0..count {
                for new in 0..count {
                    if old == new {
                        continue;
                    }
                    let d = shortest_signed_delta(old, new, count);
                    assert!(d != 0);
                    assert!(d.unsigned_abs() as usize <= count / 2 + 1);
                    // Applying it must land back on `new`, modulo `count`.
                    let landed = ((old as isize + d).rem_euclid(count as isize)) as usize;
                    assert_eq!(landed, new);
                }
            }
        }
    }
}

#[cfg(test)]
mod ssr_tests {
    use super::*;

    #[component]
    fn ThreeSlideCarousel() -> Element {
        rsx! {
            Carousel { aria_label: "Featured photos",
                CarouselPrevious { "Previous" }
                CarouselNext { "Next" }
                CarouselContent {
                    CarouselItem { index: 0usize, "One" }
                    CarouselItem { index: 1usize, "Two" }
                    CarouselItem { index: 2usize, "Three" }
                }
            }
        }
    }

    #[component]
    fn CarouselWithoutAccessibleName() -> Element {
        rsx! {
            Carousel {
                CarouselContent {
                    CarouselItem { index: 0usize, "One" }
                }
            }
        }
    }

    #[component]
    fn CarouselItemWithOwnLabel() -> Element {
        rsx! {
            Carousel { aria_label: "Featured photos",
                CarouselContent {
                    CarouselItem { index: 0usize, aria_label: "The Eiffel Tower at dusk", "One" }
                }
            }
        }
    }

    fn render(component: fn() -> Element) -> String {
        let mut dom = VirtualDom::new(component);
        dom.rebuild_in_place();
        dioxus_ssr::render(&dom)
    }

    /// The opening tag of the `occurrence`-th (0-based) `<button>` in
    /// `html`, in document order. Deliberately not "find the text
    /// 'Previous'/'Next' and walk backward to the nearest `<button`" --
    /// that text also appears inside each button's own `aria-label`
    /// attribute value (`aria-label="Previous slide"`), which a naive
    /// `str::find` matches *before* the button's visible children text,
    /// truncating the "opening tag" slice before it ever reaches
    /// `disabled`. Locating by structural position (`<button` occurrence
    /// count, matching `Carousel`'s own documented DOM order --
    /// `CarouselPrevious` then `CarouselNext`, both before
    /// `CarouselContent`) has no such ambiguity.
    fn button_tag(html: &str, occurrence: usize) -> &str {
        let tag_start = html
            .match_indices("<button")
            .nth(occurrence)
            .map(|(i, _)| i)
            .unwrap();
        let tag_end = html[tag_start..].find('>').unwrap() + tag_start;
        &html[tag_start..=tag_end]
    }

    #[test]
    fn root_has_region_role_and_roledescription() {
        let html = render(ThreeSlideCarousel);
        assert!(html.contains(r#"role="region""#));
        assert!(html.contains(r#"aria-roledescription="carousel""#));
        assert!(html.contains(r#"aria-label="Featured photos""#));
    }

    #[test]
    fn root_defaults_to_horizontal_ltr() {
        let html = render(ThreeSlideCarousel);
        assert!(html.contains(r#"data-orientation="horizontal""#));
        assert!(html.contains(r#"data-direction="ltr""#));
        assert!(html.contains(r#"dir="ltr""#));
    }

    #[test]
    fn renders_without_an_accessible_name_supplied() {
        // Only a `tracing::warn!`, never a panic or missing markup --
        // this just proves the warning path doesn't also break rendering.
        let html = render(CarouselWithoutAccessibleName);
        assert!(html.contains(r#"role="region""#));
    }

    #[test]
    fn item_has_group_role_and_slide_roledescription() {
        let html = render(ThreeSlideCarousel);
        assert!(html.contains(r#"role="group""#));
        assert!(html.contains(r#"aria-roledescription="slide""#));
    }

    #[test]
    fn item_has_no_default_label_on_first_render_before_registration_effects_run() {
        // See `CarouselItem`'s own "SSR stability" doc: `count` is only
        // known once every sibling's registration effect has run, and
        // `rebuild_in_place` alone never runs effects at all -- so this
        // is also exactly what a real SSR render (and a client's own
        // pre-hydration first paint) produces. A `"1 of 0"`-shaped label
        // leaking through here would be the regression this guards.
        let html = render(ThreeSlideCarousel);
        assert!(!html.contains("of 0"));
        assert!(!html.contains("1 of 3"));
    }

    #[test]
    fn item_zero_is_not_inert_on_first_render_before_registration_effects_run() {
        // Hydration parity (`CarouselItem`'s own "Visible slides / `inert`"
        // doc): `selected`/`visible_range` are both forced to their
        // deterministic default (`0`/`(0, 0)`) until every sibling's own
        // registration effect has run, and (like the label test above)
        // `rebuild_in_place` alone never runs effects at all -- so this is
        // also exactly what a real SSR render (and a client's own
        // pre-hydration first paint) produces regardless of how many
        // slides actually exist. Slide 0's own opening `<div ... role="group"`
        // tag (the first one in this fixture's DOM order) must never carry
        // `inert` -- it is always the deterministic first-render "visible"
        // slide.
        let html = render(ThreeSlideCarousel);
        let tag_start = html.find(r#"role="group""#).unwrap();
        let open_start = html[..tag_start].rfind("<div").unwrap();
        let open_end = html[open_start..].find('>').unwrap() + open_start;
        assert!(!html[open_start..=open_end].contains("inert"));
    }

    #[test]
    fn items_after_zero_are_inert_on_first_render_before_registration_effects_run() {
        // The mirror of the test above: slides 1 and 2 are neither
        // `selected` (forced to `0`) nor inside `visible_range` (forced to
        // `(0, 0)`) on this same first render, so both must be `inert`
        // -- the a11y contract's "only visible slides are reachable"
        // (`dev-docs/research/carousel-loop-2026-09-25/loop-a11y-guidance.md`
        // §7) already holding before a single effect has run, not
        // merely once the client catches up.
        let html = render(ThreeSlideCarousel);
        let group_tags: Vec<&str> = html
            .match_indices(r#"role="group""#)
            .map(|(tag_start, _)| {
                let open_start = html[..tag_start].rfind("<div").unwrap();
                let open_end = html[open_start..].find('>').unwrap() + open_start;
                &html[open_start..=open_end]
            })
            .collect();
        assert_eq!(group_tags.len(), 3);
        assert!(!group_tags[0].contains("inert"));
        assert!(group_tags[1].contains("inert"));
        assert!(group_tags[2].contains("inert"));
    }

    #[test]
    fn item_with_own_aria_label_is_not_overridden() {
        let html = render(CarouselItemWithOwnLabel);
        assert!(html.contains(r#"aria-label="The Eiffel Tower at dusk""#));
        assert!(!html.contains("1 of"));
    }

    #[test]
    fn previous_button_is_disabled_at_the_first_slide() {
        let html = render(ThreeSlideCarousel);
        // `CarouselPrevious` is the first `<button>` in this fixture's own
        // documented DOM order (`CarouselPrevious`, `CarouselNext`, then
        // `CarouselContent`'s slides).
        let opening_tag = button_tag(&html, 0);
        assert!(opening_tag.contains("disabled"));
        assert!(opening_tag.contains(r#"aria-label="Previous slide""#));
    }

    #[test]
    fn next_button_is_disabled_before_registration_effects_run() {
        // Unlike `CarouselPrevious` (whose `disabled` never depends on
        // `count`), `CarouselNext`'s does -- and on this first,
        // pre-effect render `count == 0` (see `CarouselItem`'s "SSR
        // stability" doc: registration is effect-driven, and
        // `rebuild_in_place` alone never runs effects). So this button is
        // conservatively disabled too here, consistent with every other
        // count-derived default in this module ("N of M" included) --
        // not a bug. `next_button_becomes_enabled_once_items_have_registered`
        // below is the same fixture after effects run.
        let html = render(ThreeSlideCarousel);
        let opening_tag = button_tag(&html, 1);
        assert!(opening_tag.contains("disabled"));
        assert!(opening_tag.contains(r#"aria-label="Next slide""#));
    }

    #[test]
    fn next_button_becomes_enabled_once_items_have_registered() {
        // `rebuild_in_place` alone never runs an effect (`use_animated_open`'s
        // own doc: "tasks will not be polled") -- `render_immediate`
        // additionally drains the effect queue and re-diffs every
        // now-dirty scope synchronously, which is what actually runs
        // each `CarouselItem`'s registration effect and, once `count`
        // is no longer `0`, re-renders `CarouselNext` with the correct
        // `disabled` state. No async executor is needed here (unlike
        // `tooltip.rs`'s own `render_immediate` test): every effect this
        // triggers (`CarouselItem`'s registration, `Carousel`'s own
        // scroll-into-view effect) is a plain synchronous `use_effect`
        // body, not one that must await a spawned task's own result to
        // finish applying its state.
        let mut dom = VirtualDom::new(ThreeSlideCarousel);
        dom.rebuild_in_place();
        // One `render_immediate` runs each `CarouselItem`'s registration
        // effect (which updates `item_ids`); a *second* is what actually
        // re-diffs `CarouselNext` (and every "N of M" label) against the
        // resulting, now-nonzero `count` -- a downstream `Memo` read
        // becoming dirty from an effect's write is its own scheduling
        // wave, not folded into the same pass that ran the effect
        // (confirmed by execution: a single call here still showed
        // `CarouselNext` disabled). Looping a few times rather than
        // hardcoding "exactly two" keeps this test from being brittle to
        // a future Dioxus scheduling change.
        for _ in 0..4 {
            dom.render_immediate(&mut dioxus::core::NoOpMutations);
        }
        let html = dioxus_ssr::render(&dom);

        let opening_tag = button_tag(&html, 1);
        assert!(!opening_tag.contains("disabled"));
        assert!(html.contains("1 of 3"));
    }

    #[component]
    fn RtlCarousel() -> Element {
        rsx! {
            Carousel {
                aria_label: "Featured photos",
                dir: Direction::Rtl,
                CarouselContent {
                    CarouselItem { index: 0usize, "One" }
                }
            }
        }
    }

    #[test]
    fn dir_prop_renders_rtl_on_the_root() {
        let html = render(RtlCarousel);
        assert!(html.contains(r#"dir="rtl""#));
        assert!(html.contains(r#"data-direction="rtl""#));
    }

    #[component]
    fn CarouselWithDragDisabled() -> Element {
        rsx! {
            Carousel { aria_label: "Featured photos",
                CarouselContent { draggable: false,
                    CarouselItem { index: 0usize, "One" }
                }
            }
        }
    }

    #[test]
    fn content_is_draggable_by_default() {
        // The live pointer-drag *behavior* (threshold, scrollBy, click
        // suppression) needs a real browser -- `playwright/carousel.spec.ts`'s
        // own "pointer drag" describe block -- but the opt-out prop's own
        // wiring into this styling hook is plain, SSR-checkable markup.
        // Unquoted `true`/`false` (not `"true"`/`"false"`) is how a `bool`
        // data attribute actually renders -- matching `calendar.rs`'s own
        // `data-selected=true`/`data-disabled=false` assertions, confirmed
        // by execution (the quoted form fails against this same markup).
        let html = render(ThreeSlideCarousel);
        assert!(html.contains("data-draggable=true"));
    }

    #[test]
    fn content_draggable_can_be_opted_out() {
        let html = render(CarouselWithDragDisabled);
        assert!(html.contains("data-draggable=false"));
    }

    #[test]
    fn slide_declares_scroll_snap_stop_always_inline() {
        // `scroll-snap-stop: always` used to be applied via a scoped
        // `<style>` tag rendered as `CarouselContent`'s own preceding
        // sibling, gated by `@supports` -- removed; see `CarouselContent`'s
        // own doc, "scroll-snap-stop" section ("Why a plain inline
        // declaration, not `@supports`"), for the full write-up. Measured
        // live against a hydrated release SSG build, that construction
        // never took effect at all: Dioxus's own hydration marker comments
        // inside the `<style>` tag's text content corrupted the CSS parse,
        // so `styleElement.sheet.cssRules.length` was `0` and every slide
        // computed `scroll-snap-stop: normal` regardless. It is now a
        // plain inline declaration on each slide, alongside
        // `scroll-snap-align`, matching every other layout-critical
        // property this module already sets this way. This is the
        // SSR-checkable half of that fix; the computed-style/behavioral
        // half needs a real browser -- `playwright/carousel.spec.ts`'s own
        // "scroll-snap-stop" describe block.
        let html = render(ThreeSlideCarousel);
        assert!(html.contains("scroll-snap-stop:always"));
        // The earlier, broken `<style>`-tag construction is gone entirely,
        // not just replaced with a working version of itself.
        assert!(!html.contains("<style>"));
    }

    // -- Gap model (backlog row 91's shadcn-parity addendum) -----------

    #[component]
    fn VerticalThreeSlideCarousel() -> Element {
        rsx! {
            Carousel { aria_label: "Featured photos", orientation: CarouselOrientation::Vertical,
                CarouselPrevious { "Previous" }
                CarouselNext { "Next" }
                CarouselContent {
                    CarouselItem { index: 0usize, "One" }
                    CarouselItem { index: 1usize, "Two" }
                    CarouselItem { index: 2usize, "Three" }
                }
            }
        }
    }

    #[test]
    fn horizontal_item_reserves_gap_as_leading_inline_padding() {
        // The gap lives INSIDE each item's own border-box (a leading-edge
        // padding), never a real flex `gap` between items -- see
        // `item_gap_padding`'s own doc for why a real `gap` would overflow
        // a fractional `flex-basis` track. `var(..., 0px)` is the
        // raw-primitive default (a themed stylesheet supplies the actual
        // token, e.g. `--dx-space-4`) -- so zero theme CSS still renders a
        // valid, harmless (zero-width) declaration rather than an
        // invalid/dropped one.
        let html = render(ThreeSlideCarousel);
        assert!(html.contains("padding-inline-start:var(--dx-carousel-gap, 0px)"));
        assert!(!html.contains("padding-block-start:var(--dx-carousel-gap"));
    }

    #[test]
    fn vertical_item_reserves_gap_as_leading_block_padding() {
        let html = render(VerticalThreeSlideCarousel);
        assert!(html.contains("padding-block-start:var(--dx-carousel-gap, 0px)"));
        assert!(!html.contains("padding-inline-start:var(--dx-carousel-gap"));
    }

    #[test]
    fn horizontal_content_compensates_with_a_negative_inline_margin() {
        // The exact mirror of the item's own leading padding -- see
        // `content_gap_margin`'s own doc for why this keeps the first
        // item's visible content flush with the viewport's own clip
        // boundary instead of shifting every slide by one gap's worth.
        let html = render(ThreeSlideCarousel);
        assert!(html.contains("margin-inline-start:calc(var(--dx-carousel-gap, 0px) * -1)"));
        assert!(!html.contains("margin-block-start:calc(var(--dx-carousel-gap"));
        // Never a real flex `gap` any more -- that would be additive to a
        // fractional `flex-basis` and overflow the track (the "~2.3
        // slides visible" incident this construction closes).
        assert!(!html.contains("gap:var(--dx-carousel-gap"));
    }

    #[test]
    fn vertical_content_compensates_with_a_negative_block_margin() {
        let html = render(VerticalThreeSlideCarousel);
        assert!(html.contains("margin-block-start:calc(var(--dx-carousel-gap, 0px) * -1)"));
        assert!(!html.contains("margin-inline-start:calc(var(--dx-carousel-gap"));
    }

    // -- Sizes: whole slides by default, opt-in peek --------------------

    #[test]
    fn item_basis_defaults_to_the_per_view_peek_calc() {
        // `--dx-carousel-per-view`/`--dx-carousel-peek` default to `1`/`0%`
        // in the calc's own fallback, which is byte-identical to the old
        // hardcoded `flex:0 0 100%` -- see `item_basis_style`'s own doc.
        let html = render(ThreeSlideCarousel);
        assert!(html.contains(
            "flex:0 0 calc((100% - var(--dx-carousel-peek, 0%)) / var(--dx-carousel-per-view, 1));"
        ));
        assert!(!html.contains("flex:0 0 100%;"));
    }

    #[test]
    fn virtual_content_item_basis_uses_the_same_calc() {
        let html = render(VirtualCarousel12Loop);
        assert!(html.contains(
            "flex:0 0 calc((100% - var(--dx-carousel-peek, 0%)) / var(--dx-carousel-per-view, 1));"
        ));
    }

    // -- `align` ---------------------------------------------------------

    #[test]
    fn item_scroll_snap_align_defaults_to_start() {
        // Matches shadcn's own default (`opts={{ align: "start" }}` in
        // every demo that sets it explicitly) -- unchanged from before
        // `align` existed as a prop at all.
        let html = render(ThreeSlideCarousel);
        assert!(html.contains("scroll-snap-align:start;"));
    }

    #[component]
    fn CenterAlignedCarousel() -> Element {
        rsx! {
            Carousel { aria_label: "Featured photos", align: CarouselAlign::Center,
                CarouselContent {
                    CarouselItem { index: 0usize, "One" }
                    CarouselItem { index: 1usize, "Two" }
                }
            }
        }
    }

    #[component]
    fn EndAlignedCarousel() -> Element {
        rsx! {
            Carousel { aria_label: "Featured photos", align: CarouselAlign::End,
                CarouselContent {
                    CarouselItem { index: 0usize, "One" }
                    CarouselItem { index: 1usize, "Two" }
                }
            }
        }
    }

    #[test]
    fn align_center_sets_scroll_snap_align_center_on_every_item() {
        let html = render(CenterAlignedCarousel);
        assert!(!html.contains("scroll-snap-align:start;"));
        assert_eq!(html.matches("scroll-snap-align:center;").count(), 2);
    }

    #[test]
    fn align_end_sets_scroll_snap_align_end_on_every_item() {
        let html = render(EndAlignedCarousel);
        assert!(!html.contains("scroll-snap-align:start;"));
        assert_eq!(html.matches("scroll-snap-align:end;").count(), 2);
    }

    #[test]
    fn align_as_str_matches_the_scroll_snap_align_keywords() {
        assert_eq!(CarouselAlign::Start.as_str(), "start");
        assert_eq!(CarouselAlign::Center.as_str(), "center");
        assert_eq!(CarouselAlign::End.as_str(), "end");
    }

    #[test]
    fn align_default_is_start() {
        assert_eq!(CarouselAlign::default(), CarouselAlign::Start);
    }

    #[component]
    fn CarouselWithBasisOverride() -> Element {
        rsx! {
            Carousel { aria_label: "Featured photos",
                CarouselContent {
                    CarouselItem { index: 0usize, style: "flex-basis: 40%;", "One" }
                }
            }
        }
    }

    #[test]
    fn an_inline_flex_basis_override_still_wins_over_the_new_default_calc() {
        // The pre-existing "override flex-basis per item via inline style"
        // escape hatch (docs.md's own "Sizing slides" section, predating
        // this feature) must keep working unchanged: a caller-supplied
        // `flex-basis` still simply appears later in the same `style`
        // attribute and wins, the same as it always has.
        let html = render(CarouselWithBasisOverride);
        assert!(html.contains("flex-basis: 40%;"));
    }

    // -- `loop` -------------------------------------------------------

    #[component]
    fn LoopCarousel() -> Element {
        rsx! {
            Carousel { aria_label: "Featured photos", r#loop: true,
                CarouselPrevious { "Previous" }
                CarouselNext { "Next" }
                CarouselContent {
                    CarouselItem { index: 0usize, "One" }
                    CarouselItem { index: 1usize, "Two" }
                    CarouselItem { index: 2usize, "Three" }
                }
            }
        }
    }

    #[test]
    fn loop_enabled_previous_is_never_disabled_even_before_registration() {
        // Even on the very first, pre-effect render (`count == 0`, the
        // same moment `next_button_is_disabled_before_registration_effects_run`
        // asserts the NON-loop `CarouselNext` conservatively disables) --
        // `loop` makes both buttons unconditionally not-disabled, per the
        // approved decision (backlog row 91): they wrap instead of ever
        // reaching a real boundary.
        let html = render(LoopCarousel);
        let previous_tag = button_tag(&html, 0);
        let next_tag = button_tag(&html, 1);
        assert!(!previous_tag.contains("disabled"));
        assert!(!next_tag.contains("disabled"));
    }

    #[test]
    fn loop_enabled_previous_stays_enabled_once_items_have_registered() {
        let mut dom = VirtualDom::new(LoopCarousel);
        dom.rebuild_in_place();
        for _ in 0..4 {
            dom.render_immediate(&mut dioxus::core::NoOpMutations);
        }
        let html = dioxus_ssr::render(&dom);
        let previous_tag = button_tag(&html, 0);
        let next_tag = button_tag(&html, 1);
        assert!(!previous_tag.contains("disabled"));
        assert!(!next_tag.contains("disabled"));
    }

    // -- Autoplay + rotation control -----------------------------------

    #[component]
    fn AutoplayCarousel() -> Element {
        rsx! {
            Carousel { aria_label: "Featured photos",
                CarouselRotationControl { "Toggle" }
                CarouselAutoplay {}
                CarouselContent {
                    CarouselItem { index: 0usize, "One" }
                    CarouselItem { index: 1usize, "Two" }
                }
            }
        }
    }

    #[test]
    fn no_aria_live_without_a_carousel_autoplay() {
        // Regression guard: a `Carousel` with no `CarouselAutoplay` at all
        // must render byte-for-byte the same as it always has -- no
        // `aria-live` anywhere (APG's own "optional" framing: a
        // manually-paged carousel has nothing to announce around).
        let html = render(ThreeSlideCarousel);
        assert!(!html.contains("aria-live"));
    }

    #[test]
    fn no_aria_live_before_carousel_autoplay_has_mounted() {
        // `CarouselAutoplay` publishes `autoplay.present` via its own
        // effect, which (like every other registration effect in this
        // module) never runs during a bare `rebuild_in_place` -- so SSR
        // and the client's pre-hydration first render both still agree on
        // "no aria-live yet," the same hydration-parity shape
        // `item_has_no_default_label_on_first_render_before_registration_effects_run`
        // already establishes for "N of M" labels.
        let html = render(AutoplayCarousel);
        assert!(!html.contains("aria-live"));
    }

    #[component]
    fn AutoplayCarouselNotPlaying() -> Element {
        // `default_playing: false` -- deliberately never `true` in this
        // fixture. The `rotating=true` (ticking) case needs a real
        // `sleep().await` registration, which `dioxus_sdk_time::sleep`
        // asserts requires "a Tokio 1.x runtime" (confirmed by execution,
        // panicking `render_immediate` under a plain `#[test]`, no reactor
        // present) -- a synchronous SSR unit test has none, and adding one
        // just to spawn-and-immediately-abandon a timer would test the
        // runtime, not this component. The actual tick -> advance ->
        // aria-live="off" round trip is exercised live instead,
        // `playwright/carousel.spec.ts`'s own "autoplay" describe block,
        // where a real browser's own JS timer backs it.
        rsx! {
            Carousel { aria_label: "Featured photos",
                CarouselRotationControl { "Toggle" }
                CarouselAutoplay { default_playing: false }
                CarouselContent {
                    CarouselItem { index: 0usize, "One" }
                    CarouselItem { index: 1usize, "Two" }
                }
            }
        }
    }

    #[test]
    fn aria_live_polite_and_rotation_label_once_autoplay_has_mounted_and_is_not_playing() {
        let mut dom = VirtualDom::new(AutoplayCarouselNotPlaying);
        dom.rebuild_in_place();
        for _ in 0..4 {
            dom.render_immediate(&mut dioxus::core::NoOpMutations);
        }
        let html = dioxus_ssr::render(&dom);
        // `present` is now `true` (published by `CarouselAutoplay`'s own
        // effect) but `playing` is `false` (`default_playing: false`), so
        // `rotating` is `false` and the live region is `"polite"` -- and
        // the timer never spawns at all (this component's own doc: the
        // ticking effect's `if !active { return; }` branch), which is
        // exactly why this fixture -- unlike a default-playing one -- is
        // safe to drive through `render_immediate` in a plain `#[test]`.
        assert!(html.contains(r#"aria-live="polite""#));
        assert!(html.contains(r#"aria-label="Start automatic slide show""#));
    }

    // -- Tablist variant ------------------------------------------------

    #[component]
    fn TablistCarousel() -> Element {
        rsx! {
            Carousel { aria_label: "Featured photos",
                CarouselTabList {
                    CarouselTab { index: 0usize, "1" }
                    CarouselTab { index: 1usize, "2" }
                }
                CarouselContent {
                    CarouselItem { index: 0usize, "One" }
                    CarouselItem { index: 1usize, "Two" }
                }
            }
        }
    }

    #[test]
    fn item_is_group_role_before_a_carousel_tab_list_has_mounted() {
        // Same hydration-parity shape as autoplay's own "before mount"
        // test above: `CarouselTabList`'s presence publish is effect-driven,
        // so a bare `rebuild_in_place` (SSR, and the client's own
        // pre-hydration first render) still sees the ordinary `group` role.
        let html = render(ThreeSlideCarousel);
        assert!(html.contains(r#"role="group""#));
        assert!(!html.contains(r#"role="tabpanel""#));
    }

    #[test]
    fn tablist_wires_role_tablist_tab_and_tabpanel_once_mounted() {
        let mut dom = VirtualDom::new(TablistCarousel);
        dom.rebuild_in_place();
        for _ in 0..4 {
            dom.render_immediate(&mut dioxus::core::NoOpMutations);
        }
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains(r#"role="tablist""#));
        assert!(html.contains(r#"role="tab""#));
        assert!(html.contains(r#"role="tabpanel""#));
        // `aria-roledescription="slide"` stays on the tabpanel too --
        // deliberately following the tested example over the pattern
        // page's own contradicting prose (see `CarouselItem`'s own doc).
        assert!(html.contains(r#"aria-roledescription="slide""#));
        // Unquoted `true`/`false`, not `"true"`/`"false"` -- a plain
        // `bool` value renders that way regardless of attribute name
        // (confirmed by execution; matches `data-draggable=true`'s own
        // established convention elsewhere in this file's SSR tests, not
        // an aria-specific special case).
        assert!(html.contains("aria-selected=true"));
        assert!(html.contains("aria-selected=false"));
        assert!(html.contains(r#"aria-label="Slide 1""#));
        assert!(html.contains(r#"aria-label="Slide 2""#));
        // Roving tabindex: only the selected tab is a page tab stop.
        assert!(html.contains(r#"tabindex="0""#));
        assert!(html.contains(r#"tabindex="-1""#));
    }

    // -- CarouselVirtualContent ------------------------------------------

    fn string_items(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("Slide {i}")).collect()
    }

    #[component]
    fn VirtualCarousel12Loop() -> Element {
        rsx! {
            Carousel { aria_label: "Featured photos", r#loop: true,
                CarouselPrevious { "Previous" }
                CarouselNext { "Next" }
                CarouselVirtualContent::<String> {
                    items: string_items(12),
                    render_item: move |(_idx, value): (usize, String)| rsx! { span { "{value}" } },
                }
            }
        }
    }

    #[component]
    fn VirtualCarouselN(n: usize) -> Element {
        rsx! {
            Carousel { aria_label: "Featured photos",
                CarouselVirtualContent::<String> {
                    items: string_items(n),
                    render_item: move |(_idx, value): (usize, String)| rsx! { span { "{value}" } },
                }
            }
        }
    }

    #[test]
    fn virtual_content_ssr_renders_every_item_in_order_before_any_effect_has_run() {
        // First render (SSR, and a client's own pre-hydration paint) never
        // windows -- see `CarouselVirtualContent`'s own "SSR / first
        // render" doc. All 12 slides, in true order, must be present.
        let html = render(VirtualCarousel12Loop);
        for i in 0..12 {
            assert!(
                html.contains(&format!("Slide {i}")),
                "missing Slide {i} in SSR output"
            );
        }
        // "k of 12" labels are correct at first render too (unlike the
        // plain children API, `CarouselVirtualContent` already knows `N`
        // from `items.len()` without waiting for per-child registration
        // effects).
        assert!(html.contains(r#"aria-label="1 of 12""#));
        assert!(html.contains(r#"aria-label="12 of 12""#));
    }

    #[test]
    fn virtual_content_ssr_only_slide_zero_is_visible() {
        let html = render(VirtualCarousel12Loop);
        let group_tags: Vec<&str> = html
            .match_indices(r#"role="group""#)
            .map(|(tag_start, _)| {
                let open_start = html[..tag_start].rfind("<div").unwrap();
                let open_end = html[open_start..].find('>').unwrap() + open_start;
                &html[open_start..=open_end]
            })
            .collect();
        assert_eq!(group_tags.len(), 12);
        assert!(!group_tags[0].contains("inert"));
        for tag in &group_tags[1..] {
            assert!(tag.contains("inert"));
        }
    }

    #[test]
    fn virtual_content_position_equals_data_index_before_any_window_is_active() {
        let html = render(VirtualCarousel12Loop);
        for i in 0..12 {
            assert!(
                html.contains(&format!(r#""data-index":{i}"#))
                    || html.contains(&format!("data-index={i}"))
            );
        }
    }

    #[component]
    fn VirtualCarouselZero() -> Element {
        rsx! {
            Carousel { aria_label: "Featured photos",
                CarouselVirtualContent::<String> {
                    items: Vec::<String>::new(),
                    render_item: move |(_idx, value): (usize, String)| rsx! { span { "{value}" } },
                }
            }
        }
    }

    #[test]
    fn virtual_content_n_zero_renders_no_slides() {
        let html = render(VirtualCarouselZero);
        assert!(!html.contains(r#"role="group""#));
    }

    #[test]
    fn virtual_content_n_one_renders_one_slide_labeled_1_of_1() {
        let html = dioxus_ssr::render(&{
            let mut dom =
                VirtualDom::new_with_props(VirtualCarouselN, VirtualCarouselNProps { n: 1 });
            dom.rebuild_in_place();
            dom
        });
        assert!(html.contains(r#"aria-label="1 of 1""#));
        assert_eq!(html.matches(r#"role="group""#).count(), 1);
    }

    #[test]
    fn virtual_content_n_two_renders_two_distinct_slides() {
        let mut dom = VirtualDom::new_with_props(VirtualCarouselN, VirtualCarouselNProps { n: 2 });
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains(r#"aria-label="1 of 2""#));
        assert!(html.contains(r#"aria-label="2 of 2""#));
    }

    #[test]
    fn virtual_content_n_three_renders_three_distinct_slides() {
        let mut dom = VirtualDom::new_with_props(VirtualCarouselN, VirtualCarouselNProps { n: 3 });
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains(r#"aria-label="1 of 3""#));
        assert!(html.contains(r#"aria-label="2 of 3""#));
        assert!(html.contains(r#"aria-label="3 of 3""#));
    }

    #[test]
    fn window_math_small_n_duplicates_data_index_with_unique_positions() {
        // N=3 < 2R+1 (radius 2 -> window width 5): the shared window math
        // (`primitives/src/virtual/window.rs`, read-only to this lane) is
        // what `CarouselVirtualContent` reuses, and it already guarantees
        // unique positions with a repeating data index -- pinned here
        // against this component's own actual radius default (2) rather
        // than only against that module's own generic tests.
        let items = crate::r#virtual::window(3, 0, 0, 2, true);
        let mut positions: Vec<isize> = items.iter().map(|i| i.position).collect();
        positions.sort_unstable();
        positions.dedup();
        assert_eq!(positions.len(), items.len());
        let data_indices: Vec<usize> = items.iter().map(|i| i.data_index).collect();
        assert_eq!(data_indices, vec![1, 2, 0, 1, 2]);
    }

    #[component]
    fn VirtualCarouselCenterAligned() -> Element {
        rsx! {
            Carousel { aria_label: "Featured photos", align: CarouselAlign::Center,
                CarouselVirtualContent::<String> {
                    items: string_items(12),
                    render_item: move |(_idx, value): (usize, String)| rsx! { span { "{value}" } },
                }
            }
        }
    }

    #[test]
    fn virtual_content_item_scroll_snap_align_follows_the_align_prop() {
        let html = render(VirtualCarouselCenterAligned);
        assert!(!html.contains("scroll-snap-align:start;"));
        assert!(html.contains("scroll-snap-align:center;"));
    }

    #[test]
    fn virtual_content_slides_reserve_gap_the_same_way_plain_items_do() {
        // `CarouselVirtualContent`'s own rendered slide markup duplicates
        // (rather than shares) `CarouselItem`'s inline style construction --
        // see that component's own doc for why a fresh string is built here
        // instead of calling into `CarouselItem` -- so the gap model must be
        // pinned on this path too, not just the plain children API.
        let html = render(VirtualCarousel12Loop);
        assert!(html.contains("padding-inline-start:var(--dx-carousel-gap, 0px)"));
        assert!(html.contains("margin-inline-start:calc(var(--dx-carousel-gap, 0px) * -1)"));
    }
}
