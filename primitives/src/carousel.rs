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
//! `scroll-snap-align` children. Paging (buttons, keyboard) calls the
//! target item's own `Element.scrollIntoView()` -- the browser computes
//! the offset, so there is no manual pixel/offset math to get wrong, and
//! (per research §2.2's RTL paragraph) no sign-flip step either: DOM
//! order never changes, so "the next slide" is always "the next
//! sibling," and `scrollIntoView` already resolves the correct physical
//! position under `dir="rtl"`. A second, independent JS bridge
//! (`use_carousel_scroll_tracking`, a private helper) keeps `CarouselContext`'s own `selected`
//! correct after a *native* drag/wheel/trackpad scroll the caller's own
//! buttons/keyboard never drove. A third (`use_carousel_drag`, also
//! private -- see [`CarouselContent`]'s own "Pointer drag" doc) adds a
//! mouse/pen drag-to-scroll gesture on top of the *same* track, moving it
//! with plain `scrollBy` calls rather than a parallel transform-based
//! engine; every `scrollBy` it issues is native scrolling as far as the
//! browser (and the second bridge above) is concerned, so dragging is
//! never a second source of truth for `selected`. All three are
//! `document::eval` call sites, and all three no-op harmlessly off a real
//! document (native/Blitz, or a plain `cargo test`), the same as every
//! other un-gated `document::eval` helper in `crate::lib` (`use_outside_dismiss`,
//! `use_form_reset_listener`, ...): this module's rendered markup never
//! branches on `feature = "web"` at all, so unlike the native-`<dialog>`
//! overlays there is nothing here for a `feature = "web"` gate to protect.
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
    direction::{use_direction, Direction, HorizontalNav},
    fold_style_attributes, has_own_accessible_name, use_controlled, use_id_or, use_unique_id,
};
use dioxus::prelude::*;

/// The axis a [`Carousel`] pages along.
///
/// Matches [`crate::resizable::ResizableDirection`]'s own
/// horizontal/vertical shape and `data-orientation` convention. Vertical
/// support fell out of the horizontal construction for free (the same
/// component, an axis-swapped CSS declaration and `scrollIntoView`
/// option, and `ArrowUp`/`ArrowDown` in place of the RTL-aware
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

/// The default accessible name for a slide with no name of its own:
/// `"{one-based index} of {count}"`. APG's own sanctioned exception to
/// "don't encode position/size in an accessible name" -- `role="group"`
/// supports neither `aria-setsize` nor `aria-posinset`
/// (dev-docs/research/carousel-2026-09-19.md §1.2).
fn slide_label(index: usize, count: usize) -> String {
    format!("{} of {}", index + 1, count)
}

/// Whether [`Carousel`]'s own mount-time scroll-into-view effect (below,
/// in its component body) should ask [`CAROUSEL_SCROLL_INTO_VIEW_JS`] for
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

/// Fire-and-forget: scroll the given slide element into view along the
/// carousel's own axis only. `inline`/`block` are chosen so a horizontal
/// carousel's `scrollIntoView` can never also nudge the page's *vertical*
/// scroll position, and vice versa for a vertical one -- the standard
/// technique for using `scrollIntoView` on one axis of a
/// multi-directionally-scrollable page. `instant` forces `behavior:
/// 'auto'` on the very first call (mount), so a non-zero `default_value`
/// does not visibly animate in on page load; every later call additionally
/// respects `prefers-reduced-motion` the same way. No response is read
/// back -- [`CAROUSEL_SCROLL_TRACKING_JS`] is what keeps `selected` in
/// sync with wherever the scroll position actually ends up.
const CAROUSEL_SCROLL_INTO_VIEW_JS: &str = "\
    const [id, orientation, instant] = await dioxus.recv();
    const behavior = (instant || window.matchMedia('(prefers-reduced-motion: reduce)').matches)
        ? 'auto' : 'smooth';
    const el = document.getElementById(id);
    if (el) {
        el.scrollIntoView({
            behavior,
            inline: orientation === 'horizontal' ? 'start' : 'nearest',
            block: orientation === 'horizontal' ? 'nearest' : 'start',
        });
    }";

/// Long-lived (mount-to-unmount): translate a [`CarouselContent`]
/// element's *actual* scroll position into a slide index, so `selected`
/// stays correct after a native drag/wheel/trackpad scroll -- one this
/// crate never drove via [`CAROUSEL_SCROLL_INTO_VIEW_JS`] itself.
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
    const [id, orientation] = await dioxus.recv();
    const container = document.getElementById(id);
    if (!container) {
        await dioxus.recv();
        return;
    }
    let lastSent = null;
    const settle = () => {
        const children = Array.from(container.children);
        if (children.length === 0) {
            return;
        }
        const containerRect = container.getBoundingClientRect();
        const containerStart = orientation === 'horizontal' ? containerRect.left : containerRect.top;
        let nearest = 0;
        let nearestDist = Infinity;
        children.forEach((child, i) => {
            const rect = child.getBoundingClientRect();
            const childStart = orientation === 'horizontal' ? rect.left : rect.top;
            const dist = Math.abs(childStart - containerStart);
            if (dist < nearestDist) {
                nearestDist = dist;
                nearest = i;
            }
        });
        if (nearest !== lastSent) {
            lastSent = nearest;
            dioxus.send(nearest);
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
    await dioxus.recv();
    container.removeEventListener('scroll', onScroll);
    if (supportsScrollEnd) {
        container.removeEventListener('scrollend', settle);
    }
    clearTimeout(debounceTimer);";

/// Attach [`CAROUSEL_SCROLL_TRACKING_JS`] to the element with the given
/// `id` for as long as the calling component stays mounted, forwarding
/// every reported index to `set_selected`. Mirrors `crate::lib`'s
/// `use_outside_dismiss`/`use_form_reset_listener` shape exactly: an
/// initial `eval.send(..)` of the (id, orientation) the script's own
/// `await dioxus.recv()` unpacks, a spawned task looping on
/// `eval.recv()` for as long as the script keeps sending, and a cleanup
/// closure that sends a teardown value so the script's own trailing
/// `await dioxus.recv()` can resolve and remove its listeners before the
/// element is gone.
fn use_carousel_scroll_tracking(
    id: impl Readable<Target = String> + Copy + 'static,
    orientation: ReadSignal<CarouselOrientation>,
    set_selected: Callback<usize>,
) {
    crate::use_effect_with_cleanup(move || {
        let id = id.cloned();
        let orientation_str = orientation().as_str().to_string();
        let mut eval = document::eval(CAROUSEL_SCROLL_TRACKING_JS);
        let _ = eval.send((id, orientation_str));
        spawn(async move {
            while let Ok(index) = eval.recv::<usize>().await {
                set_selected.call(index);
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

/// Long-lived (mount-to-unmount): a mouse/pen drag-to-scroll gesture on
/// [`CarouselContent`]'s own element, layered on the *same* scroll-snap
/// track [`CAROUSEL_SCROLL_INTO_VIEW_JS`]/[`CAROUSEL_SCROLL_TRACKING_JS`]
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
/// starts (after the movement threshold, alongside `data-dragging`) and
/// restores the exact value this element is always rendered with on
/// release -- which is also what makes the release "settle on a slide"
/// at all: once snapping is back on, the browser glides to the nearest
/// slide by itself, the same as it would after releasing a real
/// touch/trackpad drag, with no hand-rolled offset math here to get
/// wrong.
const CAROUSEL_DRAG_JS: &str = "\
    const [id, orientation, thresholdPx, enabled] = await dioxus.recv();
    const el = document.getElementById(id);
    if (!el || !enabled) {
        await dioxus.recv();
        return;
    }
    const thresholdSq = thresholdPx * thresholdPx;
    let pointerId = null;
    let originX = 0;
    let originY = 0;
    let lastX = 0;
    let lastY = 0;
    let dragging = false;
    let suppressNextClick = false;

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
            // duration lets these calls move the track freely; restoring
            // the exact value this element is always rendered with (never
            // just clearing it -- see `endDrag` below) on release is what
            // makes the browser glide to the nearest slide by itself, the
            // same native settle a real trackpad/touch drag gets, with no
            // hand-rolled offset math of this module's own.
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
        // (`0 -> -153`) the Next button's own `scrollIntoView` produces.
        if (orientation === 'horizontal') {
            el.scrollBy({ left: -dx, behavior: 'instant' });
        } else {
            el.scrollBy({ top: -dy, behavior: 'instant' });
        }
    };

    const endDrag = (e) => {
        if (e.pointerId !== pointerId) {
            return;
        }
        if (dragging) {
            suppressNextClick = true;
            el.removeAttribute('data-dragging');
            try { el.releasePointerCapture(pointerId); } catch (err) {}
            // Restore -- never just clear -- the exact snap-type this
            // element is always rendered with (see onPointerMove's own
            // comment on why it was suspended); this is what makes the
            // browser glide to the nearest slide on its own the instant
            // dragging stops, and it must happen synchronously here
            // rather than waiting for Rust to notice the settled scroll
            // position and re-render, or the browser would have nothing
            // to snap *with* the moment the gesture actually ends.
            el.style.scrollSnapType = orientation === 'horizontal' ? 'x mandatory' : 'y mandatory';
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
/// own doc for why: every `scrollBy` it issues is picked up by
/// [`use_carousel_scroll_tracking`]'s own listener on the same element).
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
    enabled: ReadSignal<bool>,
) {
    crate::use_effect_with_cleanup(move || {
        let id = id.cloned();
        let orientation_str = orientation().as_str().to_string();
        let eval = document::eval(CAROUSEL_DRAG_JS);
        // No spawned receive loop here, unlike `use_carousel_scroll_tracking`
        // just above -- this script never `dioxus.send`s anything back (see
        // this function's own doc), so there is nothing to poll for until
        // the cleanup closure's own teardown `send` below, which needs no
        // receiver on this side to take effect.
        let _ = eval.send((id, orientation_str, CAROUSEL_DRAG_THRESHOLD_PX, enabled()));
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
}

/// The props for the [`Carousel`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CarouselProps {
    /// The axis the carousel pages along. Defaults to
    /// [`CarouselOrientation::Horizontal`].
    #[props(default)]
    pub orientation: ReadSignal<CarouselOrientation>,

    /// The controlled selected slide index (0-based). `None` for
    /// uncontrolled use (see [`Self::default_value`]).
    pub value: ReadSignal<Option<usize>>,

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

    let item_ids: Signal<Vec<String>> = use_signal(Vec::new);
    let count = use_memo(move || item_ids.len());
    let selected = use_memo(move || clamp_selected(raw_selected(), count()));
    let set_selected = use_callback(move |index: usize| {
        set_raw_selected.call(clamp_selected(index, *count.peek()));
    });
    let content_id: Signal<String> = use_signal(String::new);

    use_context_provider(|| CarouselContext {
        orientation,
        selected,
        set_selected,
        count,
        item_ids,
        content_id,
    });

    // Scroll the selected slide into view whenever it changes, regardless
    // of source (buttons, keyboard, or `use_carousel_scroll_tracking`
    // reporting a native scroll). In the last case the target slide is
    // already at/near its snap point, so this is a harmless no-op --
    // `scrollIntoView` never moves an element that is already fully in
    // view. `is_first` (peeked, then set -- never tracked-read, so this
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
        let orientation_str = orientation().as_str().to_string();
        let eval = document::eval(CAROUSEL_SCROLL_INTO_VIEW_JS);
        let _ = eval.send((id, orientation_str, first));
    });

    let onkeydown = move |event: Event<KeyboardData>| {
        let key = event.key();
        let Some(intent) = carousel_key_intent(&key, orientation(), direction) else {
            return;
        };
        match intent {
            HorizontalNav::Prev => set_selected.call(prev_selected(selected())),
            HorizontalNav::Next => set_selected.call(next_selected(selected(), count())),
        }
        event.prevent_default();
    };

    if !has_own_accessible_name(&props.attributes) {
        tracing::warn!(
            "Carousel: no aria-label or aria-labelledby was supplied. The APG Carousel \
             pattern requires the carousel region to have an accessible name that does not \
             itself contain the word \"carousel\" \
             (dev-docs/research/carousel-2026-09-19.md \u{a7}1.2)."
        );
    }

    rsx! {
        div {
            role: "region",
            aria_roledescription: "carousel",
            dir: direction.as_str(),
            "data-orientation": orientation().as_str(),
            "data-direction": direction.as_str(),

            onkeydown,
            ..props.attributes,

            {props.children}
        }
    }
}

/// The `<style>` tag text [`CarouselContent`] renders as a sibling of its
/// own scroll-snap track, scoped to that track's own `id` alone -- never a
/// shared/global stylesheet, so multiple carousels on the same page each
/// get their own independent rule and none of them can affect another's.
///
/// ## What this is for
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
/// ## Known limitation, measured rather than assumed: this crate's own
/// pointer-drag path does *not* get this guarantee
///
/// [`CAROUSEL_DRAG_JS`]'s mouse/pen drag moves the track with many
/// discrete `scrollBy({ behavior: 'instant' })` calls while
/// `scroll-snap-type` is suspended for the gesture's own duration (see
/// that constant's own doc for why), so a long or fast drag's raw
/// position at release can land several slide-widths from the origin with
/// no scrolling operation having passed through the intervening snap
/// points at all -- each jump was independently instant and unsnapped.
/// Restoring `scroll-snap-type` afterward is not itself treated as "a
/// scrolling operation resuming past skipped snap points"; it is a fresh,
/// static re-evaluation of an already-stationary position, which this
/// property does not appear to constrain on this engine (Chromium):
/// confirmed live, isolated from this crate's own JS entirely (raw
/// `el.scrollBy({ behavior: 'instant' })` in a loop with
/// `scroll-snap-type` suspended, then restored, on an element whose child
/// already computes `scroll-snap-stop: always`) -- the settle still lands
/// on whichever slide is numerically nearest the raw position, identical
/// to what happens with the property absent altogether. A same-tick
/// zero-distance and 1px `behavior: 'smooth'` nudge issued immediately
/// after restoring `scroll-snap-type`, tried as a cheap way to route the
/// correction through the constrained "smooth scroll" path instead, did
/// not change this. This is a real, currently-open gap for this crate's
/// own pointer-drag feature specifically -- not for wheel/trackpad/native
/// scrolling on the same track, and not for a caller's own `scrollBy`,
/// both of which this property does correctly constrain.
///
/// ## Why a scoped `<style>` tag, not a fourth inline declaration
///
/// Every other layout-critical property on this track/its slides
/// (`scroll-snap-type`, `scroll-snap-align`, the flex layout) is a plain
/// inline `style` declaration, matching this module's own "works with
/// zero theme CSS" convention (module doc, "Engine" section).
/// `scroll-snap-stop` is the one property here that needs conditional
/// application -- "only if the browser supports it" was the user's own
/// explicit requirement -- and `@supports` is a stylesheet at-rule with no
/// equivalent inside a plain `style="..."` attribute value at all (there
/// is no syntax for an at-rule there). A scoped `<style>` tag is the
/// minimal construction that can express it while keeping the same
/// "lives on the primitive, works with zero theme CSS" property every
/// other layout-critical rule here already has. Scoped to this element's
/// own `id` -- never a shared marker class or a global, once-per-page
/// injected stylesheet, unlike `top_layer.rs`'s own
/// `@supports (anchor-name: --a)` construction -- because this rule has
/// no cross-component contract to maintain: it only ever needs to reach
/// this one track's own direct children, so the simplest correct scope is
/// used rather than the broadest one.
///
/// `> *` (every direct child of the track), rather than a class or
/// attribute selector naming [`CarouselItem`] specifically: this
/// component's own contract already says its children *are*
/// [`CarouselItem`]s (see [`CarouselContentProps::children`]'s doc), so
/// there is nothing else for `> *` to over-match inside this element in
/// normal use. This tag is rendered as the track's own preceding
/// *sibling*, not the track's child, so `#id > *` (children of the track
/// this selector's `id` names) never matches the `<style>` tag itself.
///
/// No JS, no `document::eval`, no `feature = "web"` branch: a `<style>`
/// tag is plain markup, identical in SSR and CSR, so this needs none of
/// [`CAROUSEL_DRAG_JS`]'s own escape hatches and does not disturb this
/// module's own "rendered markup never branches on `feature = \"web\"`"
/// property (module doc, "Engine" section).
fn no_skip_supports_css(content_id: &str) -> String {
    format!(
        "@supports (scroll-snap-stop: always) {{ #{content_id} > * {{ scroll-snap-stop: always; }} }}"
    )
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
/// On release, the browser's own scroll-snap machinery settles the track
/// on whichever slide is numerically nearest, exactly as it would after a
/// native trackpad/touch scroll -- no separate "settle" step is needed,
/// but also (measured, see this element's own rendered `<style>` tag,
/// `no_skip_supports_css`'s own doc, "Known limitation") no guarantee that
/// "nearest" is close: a drag whose raw distance covers several slide
/// widths can settle several slides from the origin, the same as before
/// this crate declared `scroll-snap-stop: always` on every slide -- that
/// property does correctly cap a wheel/trackpad fling or a caller's own
/// `scrollBy` on this same track to one slide, just not this crate's own
/// pointer-drag release specifically, for the reason that doc explains.
/// That settle (like every other native scroll) is what
/// `use_carousel_scroll_tracking` (already
/// attached to this same element) picks up to update `selected`, so a
/// drag never invents a second source of truth for the index: the
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
/// never taken at all, on either side of the bridge.
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

    use_carousel_scroll_tracking(id, ctx.orientation, ctx.set_selected);
    use_carousel_drag(id, ctx.orientation, props.draggable);

    let orientation = (ctx.orientation)();
    let draggable = (props.draggable)();
    let (caller_style, rest_attrs) = fold_style_attributes(props.attributes);
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
        "{axis_style}{}",
        caller_style.map(|s| format!(" {s}")).unwrap_or_default()
    );
    // See `no_skip_supports_css`'s own doc for what this is and why it is
    // a scoped `<style>` tag rather than a third inline declaration
    // alongside `axis_style` above.
    let no_skip_css = no_skip_supports_css(&id());

    rsx! {
        Fragment {
            // Explicit `Fragment` (mirroring `drag_and_drop_list.rs`'s own
            // multi-root usage), not two bare top-level siblings: found by
            // execution (this component's own doc, "no_skip_supports_css"
            // section, has the write-up) that two un-fragmented roots
            // render correctly on the very first pass (SSR, or a test's
            // own single `rebuild_in_place`) but silently lose the first
            // one -- this `<style>` tag -- the moment any later re-render
            // happens, which every real mount does almost immediately
            // (this component's own registration/tracking effects).
            style { {no_skip_css} }
            div {
                id,
                style,
                tabindex: "0",
                "data-orientation": orientation.as_str(),
                "data-draggable": draggable,
                ..rest_attrs,

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

    let (caller_style, rest_attrs) = fold_style_attributes(props.attributes);
    let style = format!(
        "flex:0 0 100%;scroll-snap-align:start;min-width:0;min-height:0;{}",
        caller_style.map(|s| format!(" {s}")).unwrap_or_default()
    );

    rsx! {
        div {
            id,
            role: "group",
            aria_roledescription: "slide",
            aria_label: default_label,
            style,
            "data-selected": is_selected,
            ..rest_attrs,

            {props.children}
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
/// first slide -- v1 has no `loop`, so this is a real boundary. Default
/// accessible name `"Previous slide"`, overridable with your own
/// `aria-label`/`aria-labelledby`. `aria-controls` points at the sibling
/// [`CarouselContent`]'s own id.
///
/// This must be used inside a [`Carousel`] component.
#[component]
pub fn CarouselPrevious(props: CarouselPreviousProps) -> Element {
    let ctx: CarouselContext = use_context();
    let selected = (ctx.selected)();
    let disabled = !can_scroll_prev(selected);
    let default_label = (!has_own_accessible_name(&props.attributes)).then_some("Previous slide");
    let content_id = (ctx.content_id)();
    let aria_controls = (!content_id.is_empty()).then_some(content_id);

    rsx! {
        button {
            r#type: "button",
            aria_label: default_label,
            aria_controls,
            disabled,

            onclick: move |_| ctx.set_selected.call(prev_selected((ctx.selected)())),
            ..props.attributes,

            {props.children}
        }
    }
}

/// # CarouselNext
///
/// The mirror of [`CarouselPrevious`]: moves to the next slide, `disabled`
/// at the last slide, default accessible name `"Next slide"`.
///
/// This must be used inside a [`Carousel`] component.
#[component]
pub fn CarouselNext(props: CarouselPreviousProps) -> Element {
    let ctx: CarouselContext = use_context();
    let selected = (ctx.selected)();
    let count = (ctx.count)();
    let disabled = !can_scroll_next(selected, count);
    let default_label = (!has_own_accessible_name(&props.attributes)).then_some("Next slide");
    let content_id = (ctx.content_id)();
    let aria_controls = (!content_id.is_empty()).then_some(content_id);

    rsx! {
        button {
            r#type: "button",
            aria_label: default_label,
            aria_controls,
            disabled,

            onclick: move |_| ctx.set_selected.call(next_selected((ctx.selected)(), (ctx.count)())),
            ..props.attributes,

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
}

impl CarouselApi {
    /// Move directly to the given slide index (clamped into range the
    /// same way every other path in this module is). For building a
    /// custom picker (e.g. a row of dot indicators) alongside
    /// [`CarouselPrevious`]/[`CarouselNext`], which only ever step by one.
    pub fn scroll_to(&self, index: usize) {
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
    CarouselApi {
        selected,
        count,
        can_scroll_prev: can_scroll_prev(selected),
        can_scroll_next: can_scroll_next(selected, count),
        scroll_to: ctx.set_selected,
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
    fn no_skip_style_tag_is_present_on_the_very_first_render() {
        let html = render(ThreeSlideCarousel);
        assert!(html.contains("<style>"));
        assert!(html.contains("@supports (scroll-snap-stop: always)"));
        assert!(html.contains("scroll-snap-stop: always"));
    }

    #[test]
    fn no_skip_style_tag_survives_the_post_effect_re_render() {
        // Regression: `CarouselContent` used to render its `<style>` tag
        // and its track `<div>` as two bare top-level rsx siblings with no
        // explicit `Fragment` wrapper. That rendered fine on the very
        // first pass (`rebuild_in_place` alone, no effects flushed --
        // `no_skip_style_tag_is_present_on_the_very_first_render` above
        // still covers exactly that shape) but silently dropped the
        // `<style>` tag the moment any later render happened -- which
        // every real mount does almost immediately, since this
        // component's own registration/tracking effects fire right after
        // mount. Confirmed live in a real browser (a built release SSG
        // page) before finding this: `document.querySelectorAll('style')`
        // found none, and `getComputedStyle` on a slide reported
        // `scroll-snap-stop: normal`, even though this exact same
        // component's bare `rebuild_in_place`-only SSR output (no
        // `render_immediate` loop) already contained the tag correctly --
        // the discrepancy was the give-away, not a CSS problem. Mirrors
        // `next_button_becomes_enabled_once_items_have_registered`'s own
        // `render_immediate` loop shape so this class of "survives a
        // second render" regression has a standing test, not just this
        // one instance of it.
        let mut dom = VirtualDom::new(ThreeSlideCarousel);
        dom.rebuild_in_place();
        for _ in 0..4 {
            dom.render_immediate(&mut dioxus::core::NoOpMutations);
        }
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("<style>"));
        assert!(html.contains("scroll-snap-stop: always"));
    }
}
