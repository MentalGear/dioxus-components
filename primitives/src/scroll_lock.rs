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
//! ## Known limitation (carried over from the dq base)
//! No iOS momentum-scroll handling and no scrollbar-gap compensation --
//! Radix delegates both to `react-remove-scroll`, which this crate has no
//! equivalent of. See docs/recommended-implementations.md §5.

use std::cell::RefCell;
use std::rc::Rc;

use dioxus::document;
use dioxus::prelude::*;

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
