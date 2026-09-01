//! Body scroll lock -- keeps the page from scrolling behind an open modal
//! surface.
//!
//! Ported from dignifiedquire/dx-components (MIT OR Apache-2.0),
//! `primitives/src/scroll_lock.rs` @ 5af3cc292559a0e8d73c7b9a827c4ca08ef34d99.
//! Adapted:
//! 1. That file coordinates nested locks with a `window.__dxScrollLockCount`
//!    global read and written from inside `document::eval`. This version
//!    keeps its refcounting and its restore-the-original-state behavior,
//!    but moves the counter into Rust -- a lazily-provided context shared
//!    by every call site in one nesting chain, the same pattern
//!    `EscapeListenerStack` (`lib.rs`) uses for nested Escape-key handling --
//!    so "is this the outermost lock/unlock" is decided synchronously in
//!    Rust *before* any `document::eval` runs, per docs/plan.md Phase 3.2.
//!    That reproduces the guard sarendipitee/dx-components' `overlay.rs`
//!    uses against the unlock-flash (@ ~L625-655: only unlock once the
//!    shared count reaches zero) without adopting that fork's
//!    `OverlayCtx`/`OverlayManager` architecture -- see
//!    docs/recommended-implementations.md §5.
//! 2. That file's unmount cleanup guards on `*active.peek()` being true,
//!    i.e. it only releases the lock if `active` is *still* true at drop
//!    time. In dq's own tree that's fine, because their `DialogContent`
//!    always renders (a native `<dialog>`'s `showModal()`/`close()` toggle
//!    visibility, not mount/unmount) so their `active.peek()` genuinely
//!    reflects "was this modal open when the whole `Dialog` disappeared".
//!    In this crate's tree, the components that call this hook unmount
//!    *because* `active` (`is_modal() && open()`) just became false -- so by
//!    drop time `active.peek()` is always false, and porting that guard
//!    verbatim meant the lock was acquired but never released. This version
//!    instead tracks, per hook instance, whether *this instance* actually
//!    incremented the counter, and releases based on that -- independent of
//!    `active`'s value at unmount.
//!
//! ## How this hook actually blocks scrolling: three generations, two
//! ## execution-falsified along the way
//! Every generation below was verified (or falsified) by running real
//! assertions against a real Chromium, not assumed from reading Radix's or
//! `react-remove-scroll`'s source. The history matters because the failure
//! modes are non-obvious and a future change to this file risks silently
//! reintroducing one of them.
//!
//! **Generation 1 (the dq port): `overflow: hidden` on `<html>` and
//! `<body>`.** Reliably blocks wheel/keyboard scroll (confirmed by
//! execution, `docs/phase4-spike-findings.md` "Round 2"), but never
//! compensates for a classic (non-overlay) scrollbar's width: removing the
//! scrollbar without reserving its space shifts the whole page horizontally
//! by that width the instant the lock engages. [`ensure_scrollbar_gutter_baseline`]
//! (below) exists to fix exactly that, independently of which generation
//! actually blocks scrolling.
//!
//! **Generation 2: same `overflow: hidden`, narrowed to `<html>`'s
//! `overflow-y` only** (once the gutter fix made touching `<body>`
//! unnecessary). This reintroduced the exact shift the gutter baseline was
//! built to prevent, for a different reason: `scrollbar-gutter`'s
//! reservation on `<html>`, in this environment's Chromium
//! (`chromium-1194`), requires `overflow-y` to remain **exactly** `auto`
//! (or `scroll`) at every instant -- switching it to `hidden`, even
//! momentarily, drops the reservation for as long as it stays `hidden`,
//! confirmed by execution with an isolated repro page (`scrollbar-gutter:
//! stable` alone, `overflow-y: hidden` from the very start, no toggle
//! involved at all: `clientWidth` still comes back as the full,
//! unreserved viewport width). `playwright/oracle/tier3-radix/scroll-lock.spec.ts`'s
//! "scrollbar-gutter reservation survives the lock itself" test was written
//! red against precisely this.
//!
//! **Generation 3 (tried, execution-falsified, not shipped): a compensated
//! `<body>` freeze** -- `position: fixed; top: -{scrollY}px; left: 0;
//! right: 0;` on `<body>`, `<html>`'s `overflow` left completely untouched.
//! Ported by construction from `docs/phase4-spike-findings.md` "Round 2",
//! Construction A / `spike_native_dialog.rs`'s `use_compensated_scroll_lock`
//! (spike branch `spike/native-dialog`, never merged), going further than
//! that spike by never touching `overflow-y` at all (the spike's own version
//! still toggled it, which generation 2 above shows is itself sufficient to
//! reproduce the defeat). Blocking scroll this way relies on a real,
//! confirmed mechanism -- taking `<body>` out of flow via `position: fixed`
//! collapses `<html>`'s own scrollable content to nothing, and `<html>`'s
//! `scrollTop` clamps to `0` immediately as a structural consequence, not
//! an event-interception race.
//!
//! **This was verified against the live app under a forced classic
//! scrollbar (`playwright/xvfb.local.config.ts`) and found to *still*
//! reproduce the exact same defeat** (`clientWidth` 1265 -> 1280 unlocked
//! -> locked, byte-for-byte the same numbers as generation 2's defect), even
//! though `overflow-y` never changed. Isolated further with a minimal
//! repro page, independent of this codebase entirely: `scrollbar-gutter:
//! stable` plus `overflow-y: auto` (never touched) on `<html>`, with `<body>`
//! shrunk so it no longer overflows `<html>` at all -- whether via
//! `position: fixed` or by simply reducing its `height` in normal flow, no
//! position trick needed -- *also* drops the gutter reservation in this
//! Chromium version. Confirmed by contrast: the identical `scrollbar-gutter:
//! stable` on an ordinary (non-root) scrolling `<div>` correctly keeps its
//! reservation even after its content shrinks below its own height. So this
//! Chromium version's `scrollbar-gutter: stable` support for the *root*
//! scroller specifically requires the root to keep genuinely overflowing --
//! `overflow-y: auto` alone is necessary but not sufficient -- which a
//! `<body>` freeze (by design) prevents, by removing all of `<body>`'s
//! contribution to `<html>`'s scrollable content. **Generation 3 solves
//! generation 2's specific defect (touching `overflow-y`) while
//! reintroducing the same observable bug through a different mechanism the
//! spike's own test suite did not catch** (their probe measured a
//! `position: fixed` marker's on-screen edge, which a `scrollbar-gutter`
//! regression on the *root* also happens to move, but they never asserted
//! `document.documentElement.clientWidth` directly against the *root*
//! scroller in isolation the way this repo's oracle does).
//!
//! **Generation 4 (this version): pure event interception, `<html>` and
//! `<body>` never touched in any way.** If the actual defect is "anything
//! that changes `<html>`'s computed `overflow` *or* reduces its genuine
//! overflow amount defeats `scrollbar-gutter: stable` on this engine," the
//! only remaining lever that cannot trip either failure mode is to never
//! pull it at all -- block the scroll *input* instead of the scroll
//! *capability*. A single pair of `window`-level, capturing (`{ capture:
//! true, passive: false }`) `wheel` and `keydown` listeners, installed once
//! per WASM instance and toggled only via a plain boolean flag
//! (`window.__dxScrollLocked`), call `preventDefault()` on an event that
//! would otherwise reach the document root -- and nothing else. Confirmed
//! by execution (both an isolated repro and the real app under
//! `xvfb.local.config.ts`): `clientWidth` stays bit-for-bit constant through
//! a full lock/unlock cycle, because nothing about `<html>`/`<body>`'s
//! style ever changes; wheel and the keyboard's Page/Home/End/Arrow scroll
//! keys are fully blocked while locked and fully restored on unlock.
//!
//! This is closer to Radix's own reference implementation than any earlier
//! generation here: `@radix-ui/react-dialog` et al. delegate to
//! `react-remove-scroll`, which itself blocks scroll via wheel/touch/key
//! *event interception*, not `overflow: hidden` -- this crate's earlier
//! generations were a simplification of that, not a faithful port, and it
//! took this engine-specific `scrollbar-gutter` defeat to make the
//! simplification's cost (an unfixable-on-this-engine layout regression)
//! outweigh its simplicity.
//!
//! **Not blocked unconditionally -- and deliberately so, confirmed
//! necessary by reading this crate's own components, not assumed:**
//! - **A scroll/key event whose target has a scrollable ancestor that can
//!   still consume it** (walked from `event.target` up to, but not
//!   including, `<body>`/`<html>`, checking `overflow-y` computes to
//!   `auto`/`scroll`/`overlay`, `scrollHeight > clientHeight`, and the
//!   relevant edge hasn't already been reached) is let through untouched.
//!   Confirmed by execution: wheeling over an unrelated `position: fixed`,
//!   independently `overflow-y: auto` scrollable box (the shape every
//!   `popover`-attributed element in this crate's web arm has by the
//!   WHATWG UA popover stylesheet -- `Tooltip`/`HoverCard`/non-modal
//!   `Popover`/menu content) scrolls *that* box and leaves
//!   `window.scrollY` at `0`, both before and after. Without this check, a
//!   long dropdown menu or popover would lose the ability to scroll its own
//!   overflowing content the instant its own lock engaged.
//! - **A key event whose target is a text-editable control** (`<input>`,
//!   `<textarea>`, `<select>`, or `[contenteditable]`) is never touched --
//!   Home/End/arrow keys have their ordinary cursor-movement meaning there,
//!   which must keep working inside any locked surface's own form fields.
//! - **A key event whose target (or an ancestor, via the scrollable-ancestor
//!   walk stopping short) carries an ARIA `role` attribute is never
//!   touched.** Confirmed necessary by reading, not assumed: every
//!   keyboard-navigable widget in this crate that could plausibly sit
//!   inside a locked surface -- `RadioGroup` (`role="radio"`), `Tabs`
//!   (`role="tab"`), `Slider` (`role="slider"`), `Menubar`/`DropdownMenu`/
//!   `ContextMenu` (`role="menuitem"`/`"option"`), `TagGroup`
//!   (`role="gridcell"`), `Toolbar` (`role="toolbar"`) -- implements its own
//!   Page/Home/End/arrow-key handling on an element that carries a `role`.
//!   This hook's listeners never call `stopPropagation()`, so those
//!   widgets' own handlers always still run regardless; the `role` check
//!   only decides whether *this* listener additionally suppresses the
//!   browser's own default action for that key on that element, which for
//!   a `role`-bearing custom widget is never scrolling the page (their own
//!   handler already does whatever the key should do) and skipping it here
//!   avoids ever second-guessing a widget's own keyboard model.
//! - **The Space key is deliberately never included** in the blocked key
//!   set at all (unlike Page/Home/End/arrow keys). Space's browser-default
//!   action is context-dependent -- "scroll the page" only when focus is on
//!   `<body>` or a plain, non-interactive element, but "activate the
//!   focused control" (a real `.click()`) when focus is on a button or
//!   similar. Every surface that calls this hook holds a focus trap while
//!   open, so focus inside a locked surface is essentially never on `<body>`
//!   -- meaning Space's only realistic default action while locked is
//!   activation, which must never be suppressed. Confirmed as a real hazard
//!   by reading `docs/phase4-spike-findings.md` (Round 1, experiment 4's
//!   note on a dialog's own focused Close button), not by hitting it here.
//!
//! **What this generation does not attempt:** no `padding-right`
//! compensation (falsified in `docs/phase4-spike-findings.md`, Round 2, for
//! exactly the `position: fixed` marker shape this crate's own overlays and
//! a realistic right-aligned navbar action both have); no iOS
//! momentum-scroll handling (see "Known limitation" below); touch is
//! handled by the same scroll-chain-aware check on `touchmove` (blocks a
//! drag gesture that would otherwise pan the background), but a fling's
//! post-release inertial scrolling on iOS Safari is a platform behavior no
//! `preventDefault()` placement reaches once the gesture has already ended,
//! same limitation `react-remove-scroll` documents for itself.
//!
//! ## Permanent `scrollbar-gutter: stable` baseline (scrollbar-gap fix)
//! Backported from `docs/phase4-spike-findings.md`, "Round 2 -- solved by
//! construction", Construction A/C. The defect: naively removing scroll
//! capability never compensates for a classic (non-overlay) scrollbar's
//! width on its own, so doing so shifts everything horizontally by that
//! width the instant a lock engages -- confirmed 15px under a forced real
//! scrollbar (`playwright/xvfb.local.config.ts`), invisible under this
//! repo's default headless Chromium, which renders 0-width overlay
//! scrollbars regardless of how tall the page is. Most users' actual
//! browsers (Windows/Linux Chrome and Firefox defaults, and other engines)
//! render classic, space-reserving scrollbars, so this is a real, live
//! regression despite being silent in the default test run -- see
//! `playwright/oracle/tier3-radix/scroll-lock.spec.ts`'s
//! `assertNoHorizontalShift`.
//!
//! [`ensure_scrollbar_gutter_baseline`] installs `scrollbar-gutter: stable`
//! on `<html>` **permanently** -- once, idempotently, the first time any
//! scroll-lock-capable primitive mounts on the page -- and never toggles it
//! again, in either direction, for any reason. Generation 4 above (the
//! actual lock/unlock mechanism) never touches `<html>`/`<body>` at all, so
//! there is no longer any moment in the lock's lifecycle where this
//! baseline's own precondition (`overflow-y` computing to `auto`/`scroll`
//! on `<html>`, *and* `<html>` genuinely still overflowing -- both
//! confirmed necessary above) could stop holding. That call is made from
//! each primitive's *root* component (`DialogRoot`, `AlertDialogRoot`,
//! `PopoverRoot`, `DropdownMenu`, `ContextMenu`), not only from
//! [`use_scroll_lock`] here: those roots mount as soon as the primitive
//! appears at all, while the `*Content`/`ScrollLockGuard` components that
//! call `use_scroll_lock` mount lazily, only once the surface first opens --
//! installing the baseline that late would make the very first open on a
//! page double as the moment the gutter reservation appears, i.e. exactly
//! the one-time shift a *permanent* baseline exists to avoid (confirmed by
//! execution: measuring before that first open, then after, shows precisely
//! this). Two constructions were tried and falsified by execution before
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
//! alone.
//!
//! **Old-engine note:** `scrollbar-gutter` is unsupported on older
//! WebKit/Safari. There, the injected rule is inert (an unrecognized
//! property is dropped), so those engines simply keep today's shipped
//! behavior -- no scrollbar-gap compensation, exactly as before this fix --
//! never worse. Generation 4's scroll-blocking mechanism does not depend on
//! `scrollbar-gutter` at all, so scroll blocking keeps working there
//! regardless.
//!
//! ## Known limitation (carried over from the dq base)
//! No iOS momentum-scroll handling: this hook can block the touch drag that
//! *initiates* a scroll gesture, but a fling's post-release inertial
//! scrolling on iOS Safari is a platform behavior no event listener reaches
//! once the gesture has ended. Radix delegates this same case to
//! `react-remove-scroll`, which this crate has no equivalent of. See
//! docs/recommended-implementations.md §5.
//!
//! ## Scroll-position capture and restore, and `use_early_scroll_capture`
//! This hook has always captured `window.scrollX`/`scrollY` before locking
//! and restored them on unlock -- this is unrelated to *how* scrolling is
//! blocked (every generation above has kept it) and exists for a separate,
//! real reason: opening a native `<dialog>` (`DialogContent`/
//! `AlertDialogContent`'s modal arm, `crate::dialog`) reliably shifts
//! `window.scrollY` by several hundred pixels the instant `showModal()`
//! runs -- confirmed by execution under a forced classic scrollbar
//! (`playwright/xvfb.local.config.ts`) on this repo's own dialog demo page,
//! both in headless-adjacent Chromium and in a real Firefox build --
//! almost certainly the browser's own default scroll-into-view-on-focus
//! behavior for the dialog's autofocus target. Because this hook's own
//! lock-acquisition effect and the dialog's `showModal()`-driving effect
//! (`crate::use_dialog_open_driver`) are two separate effects that, per
//! Dioxus's normal same-commit effect ordering, may run in either order
//! from this hook's point of view, [`use_early_scroll_capture`] is called
//! from the *root* component (`DialogRoot`/`AlertDialogRoot`) -- an
//! already-mounted component reacting to the same `open` flip that
//! `*Content` mounting-and-effects-running is itself a *consequence* of --
//! so its capture is dispatched, and therefore executes, before
//! `showModal()` gets a chance to move anything.
//!
//! [`use_scroll_lock`]'s own lock-acquisition effect still captures a
//! scroll position too (`get_or_insert`, never clobbering an earlier,
//! more-trustworthy capture from [`use_early_scroll_capture`]) -- both so
//! every other caller of this hook (`Popover`, `DropdownMenu`,
//! `ContextMenu`, none of which have a native `showModal()` of their own to
//! race) has a capture at all, and as the value restored on unlock.
//! Restoring to a value that could itself already reflect the dialog's own
//! post-jump position (possible only in the specific race where
//! `use_early_scroll_capture` hasn't resolved yet and this hook's own
//! fallback capture reads a jumped `scrollY`) is still strictly no worse
//! than not restoring at all, and does not affect `Popover`/`DropdownMenu`/
//! `ContextMenu`, none of which have any focus-driven scroll of their own to
//! correct for. Actually preventing the dialog's own initial jump is
//! separate, real follow-up work, not done here.

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

    /// Same idempotency guard as above, for the [`use_scroll_lock`]
    /// event-blocking listeners (module docs, "Generation 4").
    static SCROLL_BLOCK_LISTENERS_INSTALLED: Cell<bool> = const { Cell::new(false) };
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

/// Installs the permanent, capturing `wheel`/`keydown`/`touchmove` listeners
/// described in the module docs ("Generation 4"), at most once per WASM
/// instance. The listeners themselves check `window.__dxScrollLocked` (a
/// plain boolean, flipped by [`use_scroll_lock`]'s lock/unlock effects) on
/// every event, so installing them once, permanently, and simply toggling
/// that flag is equivalent to adding/removing them per lock cycle, without
/// the bookkeeping (or hot-reload edge cases) of tracking listener
/// identities across many `use_scroll_lock` instances sharing one
/// `ScrollLockState`.
fn ensure_scroll_block_listeners_installed() {
    if SCROLL_BLOCK_LISTENERS_INSTALLED.with(|installed| installed.replace(true)) {
        return;
    }
    let eval = document::eval(
        r#"
        if (!window.__dxScrollBlockInstalled) {
            window.__dxScrollBlockInstalled = true;
            window.__dxScrollLocked = false;

            const dxHasRole = (el) => !!(el && el.getAttribute && el.getAttribute('role'));
            const dxIsFormControl = (el) => {
                if (!el) return false;
                const tag = el.tagName;
                return tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT' || el.isContentEditable;
            };
            // Walks from `el` up to (but not including) <body>/<html>
            // looking for a scroll container that can still consume `delta`
            // in its own overflow-y direction -- see the module docs'
            // "Not blocked unconditionally" section for why this exists.
            const dxFindScrollableAncestor = (el, delta) => {
                while (el && el !== document.body && el !== document.documentElement) {
                    if (el.nodeType === 1) {
                        const style = getComputedStyle(el);
                        if (/(auto|scroll|overlay)/.test(style.overflowY) && el.scrollHeight > el.clientHeight) {
                            if (delta < 0 && el.scrollTop > 0) return el;
                            if (delta > 0 && el.scrollTop + el.clientHeight < el.scrollHeight - 1) return el;
                        }
                    }
                    el = el.parentElement;
                }
                return null;
            };

            window.addEventListener('wheel', (e) => {
                if (!window.__dxScrollLocked) return;
                if (dxFindScrollableAncestor(e.target, e.deltaY)) return;
                e.preventDefault();
            }, { passive: false, capture: true });

            // Space is deliberately excluded -- see the module docs'
            // "Not blocked unconditionally" section.
            const DX_SCROLL_KEYS = { PageUp: -1, PageDown: 1, Home: -1, End: 1, ArrowUp: -1, ArrowDown: 1 };
            window.addEventListener('keydown', (e) => {
                if (!window.__dxScrollLocked) return;
                if (!(e.key in DX_SCROLL_KEYS)) return;
                if (dxIsFormControl(e.target) || dxHasRole(e.target)) return;
                if (dxFindScrollableAncestor(e.target, DX_SCROLL_KEYS[e.key])) return;
                e.preventDefault();
            }, { passive: false, capture: true });

            let dxTouchStartY = null;
            window.addEventListener('touchstart', (e) => {
                dxTouchStartY = (e.touches && e.touches.length === 1) ? e.touches[0].clientY : null;
            }, { passive: true, capture: true });
            window.addEventListener('touchmove', (e) => {
                if (!window.__dxScrollLocked || dxTouchStartY === null) return;
                if (!e.touches || e.touches.length !== 1) return;
                const delta = dxTouchStartY - e.touches[0].clientY;
                if (dxFindScrollableAncestor(e.target, delta)) return;
                e.preventDefault();
            }, { passive: false, capture: true });
        }
        "#,
    );
    let _ = eval;
}

/// Per-nesting-chain lock count, and the page scroll position captured just
/// before the outermost lock in the chain engaged.
#[derive(Default)]
struct ScrollLockInner {
    count: usize,
    original_scroll: Option<(f64, f64)>,
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

/// Captures the page's scroll position *before* a modal surface has any
/// chance to move it, for [`use_scroll_lock`]'s restore-on-unlock mitigation
/// (see the module docs, "Scroll-position capture and restore") to have a
/// pre-jump baseline to restore in the one case that most needs it: a native
/// `<dialog>`'s `showModal()` call, which reliably scrolls the page on its
/// own (confirmed by execution) before `use_scroll_lock`'s *own* capture --
/// which runs from `DialogContent`, mounted and effects-run only as a
/// consequence of the same `open` flip -- would otherwise get a chance to.
///
/// Call this from the *root* component (`DialogRoot`, `AlertDialogRoot`),
/// passing the same `open` the content components key off of: roots are
/// already-mounted, existing components reacting to `open` changing, while
/// `*Content` is a brand-new mount conditioned on that same change --
/// Dioxus's normal top-down effect ordering within one commit means this
/// root-level capture's `document::eval` (which reads `window.scrollX`/
/// `scrollY` the instant it runs, independent of when its result is later
/// received) is dispatched, and therefore executes, before the content's own
/// `showModal()`-driving effect gets a chance to move anything.
///
/// A no-op if a lock is already engaged (nested dialogs: the outer one's
/// captured position is the one worth keeping) or if this cycle's position
/// was already captured (guards a redundant capture racing with
/// [`use_scroll_lock`]'s own, which only captures when this hasn't already
/// run -- see there). Popover/DropdownMenu/ContextMenu have no equivalent
/// call: none of them have a native `showModal()` of their own to race, so
/// [`use_scroll_lock`]'s own capture is early enough for them as-is.
pub(crate) fn use_early_scroll_capture(open: Memo<bool>) {
    let state = use_scroll_lock_state();
    use_effect(move || {
        if !open.cloned() {
            return;
        }
        {
            let inner = state.0.borrow();
            if inner.count > 0 || inner.original_scroll.is_some() {
                return;
            }
        }
        let state = state.clone();
        spawn(async move {
            let mut eval = document::eval("dioxus.send([window.scrollX, window.scrollY]);");
            if let Ok(position) = eval.recv::<(f64, f64)>().await {
                let mut inner = state.0.borrow_mut();
                // Belt-and-suspenders re-check: `use_scroll_lock`'s own
                // capture (or a concurrent instance of this same effect for
                // a sibling root) may have raced ahead while this awaited.
                if inner.original_scroll.is_none() {
                    inner.original_scroll = Some(position);
                }
            }
        });
    });
}

/// Prevents the page from scrolling while `active` is true, via the
/// event-interception mechanism described in the module docs ("Generation
/// 4") -- `<html>`/`<body>` are never touched in any way, so the permanent
/// `scrollbar-gutter: stable` baseline ([`ensure_scrollbar_gutter_baseline`])
/// keeps reserving the same width before, during, and after every lock
/// cycle, on an engine where that reservation depends on `<html>` never
/// having its `overflow` changed *or* its genuine overflow amount reduced.
///
/// Nested locks -- e.g. a dialog that opens a second, nested dialog -- share
/// one counter (see module docs): only the count's 0 -> 1 transition
/// engages the block, and only its 1 -> 0 transition releases it. That
/// means a still-open outer modal is never affected by an inner one
/// closing, and there is no window in which the page is briefly, visibly
/// unlocked between an inner modal's close and an outer modal's
/// already-locked state.
///
/// Call this from a component that mounts exactly while the lock should be
/// held and unmounts when it shouldn't (this crate's `*Content` components,
/// which are conditionally rendered by `use_animated_open`) -- the lock is
/// acquired in this hook's effect and released in its unmount cleanup.
pub(crate) fn use_scroll_lock(active: Memo<bool>) {
    let state = use_scroll_lock_state();

    // Permanent baseline and listeners (see module docs) -- unconditional,
    // run once per WASM instance, and never re-run or reverted by the
    // lock/unlock toggle below.
    use_effect(ensure_scrollbar_gutter_baseline);
    use_effect(ensure_scroll_block_listeners_installed);

    // Whether *this* hook instance is the one that incremented the shared
    // counter -- tracked separately from `active` because by the time this
    // component unmounts, `active` has typically already flipped to false
    // (see adaptation note 2 above), so it can't be used to decide whether
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
                    r#"
                    dioxus.send([window.scrollX, window.scrollY]);
                    window.__dxScrollLocked = true;
                    "#,
                );
                if let Ok((x, y)) = eval.recv::<(f64, f64)>().await {
                    let mut inner = lock_state.0.borrow_mut();
                    // Don't clobber an earlier, pre-jump capture --
                    // `use_early_scroll_capture` (called from `DialogRoot`/
                    // `AlertDialogRoot`) may already have run by now with a
                    // more trustworthy value; see its doc.
                    inner.original_scroll.get_or_insert((x, y));
                }
            });
        }
    });

    // Cleanup only runs when this hook's owning component unmounts.
    // `acquired` (not `active`) records whether this instance still owes a
    // decrement -- see adaptation note 2 above.
    crate::use_effect_cleanup(move || {
        if !*acquired.peek() {
            return;
        }
        let restore = {
            let mut inner = state.0.borrow_mut();
            inner.count = inner.count.saturating_sub(1);
            (inner.count == 0).then(|| inner.original_scroll.take())
        };
        if let Some(original_scroll) = restore {
            let eval = document::eval(
                r#"window.__dxScrollLocked = false;
                const scroll = await dioxus.recv();
                if (scroll) {
                    const [x, y] = scroll;
                    // Restores whatever page position was in effect just
                    // before the lock engaged -- see the module docs
                    // ("Scroll-position capture and restore") for why this
                    // exists: a real, measured page-scroll jump (confirmed
                    // on both Chromium and Firefox, most visibly a native
                    // `<dialog>`'s own `showModal()` autofocus/scroll-into-
                    // view behavior) otherwise persists silently through the
                    // lock and is never undone once it releases, leaving the
                    // page permanently scrolled away from where the user
                    // left it -- e.g. its sticky top nav rendering
                    // off-screen. This is a mitigation for that symptom, not
                    // a fix for whatever causes the scroll to move in the
                    // first place while locked.
                    window.scrollTo(x, y);
                }"#,
            );
            let _ = eval.send(original_scroll);
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
