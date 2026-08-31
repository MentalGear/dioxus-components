//! Body scroll lock -- keeps the page from scrolling behind an open modal
//! surface.
//!
//! Ported from dignifiedquire/dx-components (MIT OR Apache-2.0),
//! `primitives/src/scroll_lock.rs` @ 5af3cc292559a0e8d73c7b9a827c4ca08ef34d99.
//! Adapted:
//! 1. That file coordinates nested locks with a `window.__dxScrollLockCount`
//!    global read and written from inside `document::eval`. This version
//!    keeps its refcounting and its restore-the-original-`overflow`
//!    behavior, but moves the counter into Rust -- a lazily-provided context
//!    shared by every call site in one nesting chain, the same pattern
//!    `EscapeListenerStack` (`lib.rs`) uses for nested Escape-key handling --
//!    so "is this the outermost lock/unlock" is decided synchronously in
//!    Rust *before* any `document::eval` runs, per docs/plan.md Phase 3.2.
//!    That reproduces the guard sarendipitee/dx-components' `overlay.rs`
//!    uses against the unlock-flash (@ ~L625-655: only unlock once the
//!    shared count reaches zero) without adopting that fork's
//!    `OverlayCtx`/`OverlayManager` architecture -- see
//!    docs/recommended-implementations.md §5.
//! 2. That file only sets `overflow: hidden` on `<body>`. Verified against
//!    this repo's `preview/` app: `document.scrollingElement ===
//!    document.documentElement` there, i.e. `<html>` is the element that
//!    actually scrolls, so locking only `<body>` has no effect on
//!    `window.scrollY` (confirmed red in
//!    playwright/oracle/tier3-radix/scroll-lock.spec.ts before this fix,
//!    with `document.body.style.overflow` correctly set to `hidden` and the
//!    page still scrolling). This version locks and restores both
//!    `<html>` and `<body>` `overflow`.
//! 3. That file's unmount cleanup guards on `*active.peek()` being true,
//!    i.e. it only releases the lock if `active` is *still* true at drop
//!    time. In dq's own tree that's fine, because their `DialogContent`
//!    always renders (a native `<dialog>`'s `showModal()`/`close()` toggle
//!    visibility, not mount/unmount) so their `active.peek()` genuinely
//!    reflects "was this modal open when the whole `Dialog` disappeared".
//!    In this crate's tree, the components that call this hook unmount
//!    *because* `active` (`is_modal() && open()`) just became false -- so by
//!    drop time `active.peek()` is always false, and porting that guard
//!    verbatim meant the lock was acquired but never released (caught by
//!    the unlock half of scroll-lock.spec.ts, which stayed red after the
//!    `<html>` fix above until this changed). This version instead tracks,
//!    per hook instance, whether *this instance* actually incremented the
//!    counter, and releases based on that -- independent of `active`'s
//!    value at unmount.
//!
//! ## Permanent `scrollbar-gutter: stable` baseline (scrollbar-gap fix)
//! Backported from `docs/phase4-spike-findings.md`, "Round 2 -- solved by
//! construction", Construction A/C (`primitives/src/spike_native_dialog.rs`'s
//! `use_compensated_scroll_lock`, spike branch `spike/native-dialog`, never
//! merged -- this is a from-scratch port of its *construction*, not a copy of
//! its code). The defect: `overflow: hidden` never compensates for a classic
//! (non-overlay) scrollbar's width, so removing it shifts everything
//! horizontally by that width the instant a lock engages -- confirmed 15px
//! under a forced real scrollbar (`playwright/xvfb.local.config.ts`),
//! invisible under this repo's default headless Chromium, which renders
//! 0-width overlay scrollbars regardless of how tall the page is. Most
//! users' actual browsers (Windows/Linux Chrome and Firefox defaults, and
//! other engines) render classic, space-reserving scrollbars, so this is a
//! real, live regression despite being silent in the default test run --
//! see `playwright/oracle/tier3-radix/scroll-lock.spec.ts`'s
//! `assertNoHorizontalShift`.
//!
//! [`ensure_scrollbar_gutter_baseline`] installs `scrollbar-gutter: stable`
//! on `<html>` **permanently** -- once, idempotently, the first time any
//! scroll-lock-capable primitive mounts on the page -- and never toggles it
//! again. That call is made from each primitive's *root* component
//! (`DialogRoot`, `AlertDialogRoot`, `PopoverRoot`, `DropdownMenu`,
//! `ContextMenu`), not only from [`use_scroll_lock`] here: those roots mount
//! as soon as the primitive appears at all, while the `*Content`/
//! `ScrollLockGuard` components that call `use_scroll_lock` mount lazily,
//! only once the surface first opens -- installing the baseline that late
//! would make the very first open on a page double as the moment the
//! gutter reservation appears, i.e. exactly the one-time shift a
//! *permanent* baseline exists to avoid (confirmed by execution: measuring
//! before that first open, then after, shows precisely this). Locking
//! and unlocking continue to work exactly as before, toggling only
//! `overflow` between its original value and `hidden` (see adaptation notes
//! above); the gutter reservation is independent of that value, so the
//! available layout width is identical before, during, and after every lock
//! cycle. Two constructions were tried and falsified by execution before
//! landing on this one (full detail and measurements in the findings doc):
//! 1. **The textbook `padding-right` recipe** (`react-remove-scroll` and
//!    similar: measure `innerWidth - documentElement.clientWidth`, add it as
//!    `padding-right` on `<body>`). Falsified for `position: fixed`
//!    elements: a fixed-position box (a realistic right-aligned navbar
//!    action, or this crate's own `Popover`/menu content) is positioned
//!    against the *initial containing block*, whose size tracks the true
//!    viewport net of the real scrollbar and is not influenced by any
//!    element's padding. The probe element in the findings doc's Xvfb run
//!    still shifted the full 15px, uncorrected.
//! 2. **A transient `scrollbar-gutter: stable` toggle**, applied together
//!    with `overflow: hidden` only for the duration of the lock. This fixes
//!    the classic-scrollbar case, but Chromium reserves `stable`'s gutter
//!    width unconditionally the instant the property takes effect,
//!    regardless of whether the platform was otherwise showing an overlay
//!    scrollbar -- so on an overlay-scrollbar platform (this repo's default
//!    headless Chromium included), turning it on *at lock time* introduces
//!    exactly the shift being fixed, just newly, on the other kind of
//!    platform, and reverts it again on unlock -- a shift on every open and
//!    every close, oscillating.
//!
//! Applying it *permanently* rather than transiently means overlay-scrollbar
//! platforms pay the same fixed one-time gutter reservation classic-scrollbar
//! platforms always paid anyway (this is `scrollbar-gutter: stable`'s
//! documented, unconditional behavior, not specific to this fix), instead of
//! oscillating on every lock cycle -- confirmed by execution in both regimes
//! in the findings doc.
//!
//! The baseline is installed via a plain (non-`!important`) `:where(html) {
//! scrollbar-gutter: stable; overflow-y: auto; }` rule in an injected
//! `<style>` tag, not an inline style, and specifically through
//! `:where()`'s zero specificity: any author rule touching `html`/`:root` --
//! even a bare element selector -- wins outright, so an app that already
//! declares its own `scrollbar-gutter` or `overflow-y` on the root is left
//! alone. `overflow-y: auto` is part of the baseline (not just
//! `scrollbar-gutter` alone) because `scrollbar-gutter` has no effect while
//! `overflow-y` computes to `visible` (confirmed by execution: `stable`
//! alone, with `overflow-y` untouched, reserved nothing under a forced real
//! scrollbar) -- and it must live in the stylesheet rather than inline
//! specifically so the existing lock/unlock toggle's restore step (which
//! writes the *inline* `overflow` shorthand back to its captured original,
//! typically clearing it) cannot erase it: an inline longhand set once at
//! mount would be wiped by that restore on the very first unlock, since
//! setting the inline `overflow` shorthand to `''` removes both of its
//! longhands, including one set independently through `overflowY`.
//!
//! **Old-engine note:** `scrollbar-gutter` is unsupported on older
//! WebKit/Safari. There, the injected rule is inert (an unrecognized
//! property is dropped), so those engines simply keep today's shipped
//! behavior -- no scrollbar-gap compensation, exactly as before this fix --
//! never worse.
//!
//! ## Known limitation (carried over from the dq base)
//! No iOS momentum-scroll handling. Radix delegates that to
//! `react-remove-scroll`, which this crate has no equivalent of. See
//! docs/recommended-implementations.md §5.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use dioxus::document;
use dioxus::prelude::*;

thread_local! {
    /// Whether [`ensure_scrollbar_gutter_baseline`]'s `document::eval` has
    /// already been scheduled once in this WASM instance. `use_scroll_lock`
    /// is called afresh every time a modal/menu opens (this crate's
    /// `*Content` components mount lazily -- see the component doc below),
    /// so without this guard the baseline install would re-run, harmlessly
    /// but wastefully, on every single open for the rest of the session.
    /// The JS side is *also* idempotent on its own (it checks for its
    /// `<style>` tag by id before creating one) as a second guard, since
    /// this flag alone would not survive e.g. a hot-reload that resets Rust
    /// statics but leaves the already-injected style tag in the live DOM.
    static SCROLLBAR_GUTTER_BASELINE_INSTALLED: Cell<bool> = const { Cell::new(false) };
}

/// Installs the permanent `scrollbar-gutter: stable` baseline described in
/// the module docs, at most once per WASM instance. Deliberately not gated
/// on `active` and not part of the lock/unlock toggle below -- see the
/// module docs for why this must never be toggled per lock.
///
/// Called from an unconditional `use_effect` in each scroll-lock-capable
/// primitive's *root* component (`DialogRoot`, `AlertDialogRoot`,
/// `PopoverRoot`, `DropdownMenu`, `ContextMenu`) rather than only from
/// [`use_scroll_lock`] below: those roots mount as soon as the primitive
/// appears on the page at all, while the `*Content`/`ScrollLockGuard`
/// components that call `use_scroll_lock` mount lazily, only once the
/// surface first opens. Installing only at that later point would make the
/// very first open on a given page load double as the moment the baseline
/// (and therefore its gutter reservation) appears -- a real, if one-time,
/// shift, on overlay-scrollbar platforms, that defeats the purpose of a
/// baseline meant to be there before any interaction. [`use_scroll_lock`]
/// also calls this, as a defensive fallback for any caller that reaches it
/// without going through one of those root components; both call sites are
/// idempotent and race-free (WASM is single-threaded).
pub(crate) fn ensure_scrollbar_gutter_baseline() {
    if SCROLLBAR_GUTTER_BASELINE_INSTALLED.with(|installed| installed.replace(true)) {
        return;
    }
    let eval = document::eval(
        r#"
        if (!document.getElementById('dx-scrollbar-gutter-baseline')) {
            const style = document.createElement('style');
            style.id = 'dx-scrollbar-gutter-baseline';
            style.textContent = ':where(html) { scrollbar-gutter: stable; overflow-y: auto; }';
            document.head.appendChild(style);
        }
        "#,
    );
    let _ = eval;
}

/// Per-nesting-chain lock count, and the `<html>`/`<body>` `overflow` values
/// observed just before the first lock in the chain took effect.
#[derive(Default)]
struct ScrollLockInner {
    count: usize,
    original_overflow: Option<(String, String)>,
}

#[derive(Clone)]
struct ScrollLockState(Rc<RefCell<ScrollLockInner>>);

/// Finds the [`ScrollLockState`] provided by an ancestor in this nesting
/// chain (e.g. an outer modal that is already locked), or becomes the
/// provider itself if none exists yet -- mirrors `EscapeListenerStack`'s
/// `try_consume_context().unwrap_or_else(...)` in `lib.rs`.
fn use_scroll_lock_state() -> ScrollLockState {
    use_hook(|| {
        try_consume_context().unwrap_or_else(|| {
            provide_context(ScrollLockState(Rc::new(RefCell::new(
                ScrollLockInner::default(),
            ))))
        })
    })
}

/// Prevents the page from scrolling while `active` is true.
///
/// Nested locks -- e.g. a dialog that opens a second, nested dialog -- share
/// one counter (see module docs): only the count's 0 -> 1 transition sets
/// `overflow: hidden` on `<html>` and `<body>`, and only its 1 -> 0
/// transition restores their original values. That means a still-open outer
/// modal is never affected by an inner one closing, and there is no window
/// in which the page is briefly, visibly unlocked between an inner modal's
/// close and an outer modal's already-locked state.
///
/// Call this from a component that mounts exactly while the lock should be
/// held and unmounts when it shouldn't (this crate's `*Content` components,
/// which are conditionally rendered by `use_animated_open`) -- the lock is
/// acquired in this hook's effect and released in its unmount cleanup.
pub(crate) fn use_scroll_lock(active: Memo<bool>) {
    let state = use_scroll_lock_state();

    // Permanent baseline (see module docs) -- unconditional, run once per
    // WASM instance, and never re-run or reverted by the lock/unlock toggle
    // below.
    use_effect(ensure_scrollbar_gutter_baseline);

    // Whether *this* hook instance is the one that incremented the shared
    // counter -- tracked separately from `active` because by the time this
    // component unmounts, `active` has typically already flipped to false
    // (see adaptation note 3 above), so it can't be used to decide whether
    // the matching decrement is still owed.
    let mut acquired = use_signal(|| false);

    let lock_state = state.clone();
    use_effect(move || {
        if !active() || *acquired.peek() {
            return;
        }
        acquired.set(true);
        let is_outermost = {
            let mut inner = lock_state.0.borrow_mut();
            inner.count += 1;
            inner.count == 1
        };
        if is_outermost {
            let lock_state = lock_state.clone();
            spawn(async move {
                let mut eval = document::eval(
                    r#"dioxus.send([document.documentElement.style.overflow, document.body.style.overflow]);
                    document.documentElement.style.overflow = 'hidden';
                    document.body.style.overflow = 'hidden';"#,
                );
                if let Ok(original) = eval.recv::<(String, String)>().await {
                    lock_state.0.borrow_mut().original_overflow = Some(original);
                }
            });
        }
    });

    // Cleanup only runs when this hook's owning component unmounts.
    // `acquired` (not `active`) records whether this instance still owes a
    // decrement -- see adaptation note 3 above.
    crate::use_effect_cleanup(move || {
        if !*acquired.peek() {
            return;
        }
        let restore = {
            let mut inner = state.0.borrow_mut();
            inner.count = inner.count.saturating_sub(1);
            (inner.count == 0).then(|| inner.original_overflow.take().unwrap_or_default())
        };
        if let Some(original) = restore {
            let eval = document::eval(
                r#"const [htmlOriginal, bodyOriginal] = await dioxus.recv();
                document.documentElement.style.overflow = htmlOriginal;
                document.body.style.overflow = bodyOriginal;"#,
            );
            let _ = eval.send(original);
        }
    });
}

/// Mount-scoped host for [`use_scroll_lock`], for a `*Content` component
/// that (unlike `DialogContent`/`AlertDialogContent`/
/// `PopoverContentRendered`) does not itself unmount when its surface
/// closes -- `DropdownMenuContent`/`ContextMenuContent` render their own
/// `if render() { div { ... } }` internally rather than being conditionally
/// rendered by a parent, so the *component function* stays mounted for the
/// whole lifetime of the menu, open or closed, and `use_scroll_lock` called
/// directly there would never see its unmount cleanup run. Render this
/// inside that same `if render() { ... }` block instead (as a zero-output
/// child) so *it* mounts and unmounts on the right cycle.
#[component]
pub(crate) fn ScrollLockGuard(active: Memo<bool>) -> Element {
    use_scroll_lock(active);
    rsx! {}
}
