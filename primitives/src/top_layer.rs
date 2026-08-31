//! Top-layer wiring for non-modal overlays — the `popover` attribute.
//!
//! Concept ported from `dignifiedquire/dx-components` (MIT OR Apache-2.0),
//! `primitives/src/top_layer.rs` @ `5af3cc292559a0e8d73c7b9a827c4ca08ef34d99`
//! (`use_top_layer`, `TopLayerKind`). Adapted: rewritten against this
//! crate's `document::eval` house pattern (see `use_form_reset_listener`,
//! `lib.rs`) instead of `wasm_bindgen`/`web_sys` — this crate has neither as
//! a dependency, and `lifting-from-forks.md` §4 says not to take dependency
//! edits when a lift can avoid them. Narrowed to the Popover API only; the
//! fork's `TopLayerKind::DialogModal` arm is dropped, since native
//! `<dialog>` + `showModal()` is Phase 4.2's job (docs/plan.md), not this
//! slice's.
//!
//! Two WHATWG HTML spec sections define everything below:
//! - The `popover` attribute and its `auto`/`manual` states:
//!   <https://html.spec.whatwg.org/multipage/popover.html#the-popover-attribute>
//! - Light dismiss (Escape, outside pointerdown, closing sibling `auto`
//!   popovers) for `popover="auto"`:
//!   <https://html.spec.whatwg.org/multipage/popover.html#popover-light-dismiss>
//! - `showPopover()`/`hidePopover()` and the `toggle` event fired on every
//!   state change (browser- or script-driven alike):
//!   <https://html.spec.whatwg.org/multipage/popover.html#dom-showpopover>,
//!   <https://html.spec.whatwg.org/multipage/popover.html#dom-hidepopover>

#[cfg(target_family = "wasm")]
use dioxus::document;
#[cfg(target_family = "wasm")]
use dioxus::prelude::*;

/// Which `popover` dismissal behaviour an element declares.
/// WHATWG HTML §the-popover-attribute (see module docs for the link).
///
/// Only referenced from the `#[cfg(target_family = "wasm")]` leaf render
/// functions in `tooltip.rs`/`hover_card.rs`/`popover.rs` -- the native
/// (Blitz) arm never renders the `popover` attribute at all (see
/// `use_popover_sync`'s doc) -- so this whole type is `#[cfg]`-gated too,
/// rather than degrading to a dead stub on that target.
#[cfg(target_family = "wasm")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PopoverKind {
    /// `popover="auto"` — light-dismisses on Escape and on a pointerdown
    /// outside the popover (and any popovers it is nested in); showing one
    /// auto popover also closes unrelated open ones. Used by the non-modal
    /// [`crate::popover`] arm, whose existing `use_outside_dismiss`/
    /// `use_global_escape_listener` wiring this replaces on the web arm.
    Auto,
    /// `popover="manual"` — no light-dismiss; only an explicit
    /// `hidePopover()` (here, driven by our own `open` signal going false)
    /// closes it. Used by [`crate::tooltip`] and [`crate::hover_card`],
    /// which already own their entire open/close lifecycle via hover/focus
    /// and gain nothing from -- and would fight with -- native light
    /// dismiss (a `HoverCardContent` mouseenter re-opens across what would
    /// otherwise be an outside click if `auto` briefly closed it first).
    Manual,
}

#[cfg(target_family = "wasm")]
impl PopoverKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Manual => "manual",
        }
    }
}

/// Drive `showPopover()`/`hidePopover()` on the element with the given `id`
/// from `open`, and sync the browser's own `toggle` event — fired on
/// light-dismiss (`auto`), Escape (`auto`), or any other script/UA-driven
/// close — back into `set_open`, so the Rust signal never strands. This is
/// the same defect class `docs/recommended-implementations.md` Caveat 1
/// documents for `<dialog>`'s one-way `showModal()`/`close()` binding
/// (the upstream regression that motivated PR #47); the eval-channel
/// pattern that fixes it here is the same one already used by
/// `use_form_reset_listener` (`lib.rs`).
///
/// The show/hide effect checks `:popover-open` before calling either
/// method, mirroring the native-`<dialog>` spike's `dialog.open` guard
/// (`docs/phase4-spike-findings.md`, "An `.open` guard") — without it, a
/// redundant call (the signal already agrees with the browser's state)
/// throws `InvalidStateError`.
///
/// # Renderer-support note
///
/// Only a real browser or webview implements the Popover API. Blitz
/// (`dioxus-native`) implements neither the `popover` attribute nor
/// `document::eval` (the latter is a no-op there) —
/// `docs/recommended-implementations.md` Caveat 2's capability table shows
/// both at zero. This hook must never run on that target: calling it there
/// would be a silent no-op at best (`eval` swallowed) and, worse, its
/// *caller* rendering the `popover` attribute unconditionally would be
/// actively harmful — Blitz's UA stylesheet borrows Firefox's
/// `[popover]:not(:popover-open) { display: none }` rule without
/// implementing `:popover-open` itself, so any element carrying the
/// attribute there would always match `:not(:popover-open)` and render
/// permanently hidden. See each call site (`tooltip.rs`, `hover_card.rs`,
/// `popover.rs`) for the matching `#[cfg]` split on the attribute itself.
///
/// It is gated on `target_family = "wasm"` for now because that is the only
/// axis this repo's CI (web build + host `cargo check`) can build and check
/// both sides of. Per `docs/phase4-spike-findings.md` Construction B, the
/// *correct* production axis is a renderer Cargo feature mirroring this
/// crate's own `web` feature — `dioxus-desktop` is a non-wasm binary with a
/// real, working webview `eval`, and belongs on this same web-style arm,
/// not on the native/no-op one `not(target_family = "wasm")` would route it
/// to. Swap the `cfg` predicate here (and at every call site) for that
/// feature once one exists; nothing else about this hook's shape needs to
/// change.
#[cfg(target_family = "wasm")]
pub(crate) fn use_popover_sync(
    id: String,
    open: impl Readable<Target = bool> + Copy + 'static,
    set_open: Callback<bool>,
) {
    // Browser -> signal.
    let id_for_listener = id.clone();
    crate::use_effect_with_cleanup(move || {
        let mut eval = document::eval(
            "const id = await dioxus.recv();
            const el = document.getElementById(id);
            const onToggle = (e) => dioxus.send(e.newState === 'open');
            el.addEventListener('toggle', onToggle);
            await dioxus.recv();
            el.removeEventListener('toggle', onToggle);",
        );
        let _ = eval.send(id_for_listener.clone());
        spawn(async move {
            while let Ok(is_open) = eval.recv::<bool>().await {
                set_open.call(is_open);
            }
        });
        move || {
            let _ = eval.send(true);
        }
    });

    // Signal -> browser.
    use_effect(move || {
        let want_open = open.cloned();
        let id = id.clone();
        document::eval(&format!(
            "const el = document.getElementById('{id}');
            if (!el) return;
            const isOpen = el.matches(':popover-open');
            if ({want_open} && !isOpen) el.showPopover();
            if (!{want_open} && isOpen) el.hidePopover();"
        ));
    });
}

// No native (Blitz) counterpart: every call site
// (`tooltip.rs`/`hover_card.rs`/`popover.rs`) is itself inside a
// `#[cfg(target_family = "wasm")]`-only component -- the modal/non-modal
// and mount/unmount boundaries in those files are real child-component
// boundaries specifically so the *hook call* can differ per arm without a
// conditional-hook-call hazard (see each file's comment), so a same-named
// no-op stub here would never be called and is dead code by construction,
// like `PopoverKind` and `position_anchor_style` above.

/// CSS Anchor Positioning wiring — *not* part of the Popover API itself, but
/// required for it to be usable at all here, and included in this slice for
/// that reason rather than as extra scope.
///
/// Moving an element into the top layer changes its containing block to the
/// initial containing block (viewport) regardless of its own `position`
/// value — confirmed by execution against this repo's actual preview build
/// (see this slice's PR/session notes): a `Tooltip` given `popover="manual"`
/// with no other change kept `position: absolute` but its rendered box
/// jumped hundreds of pixels from its trigger, because "absolute, relative
/// to the nearest positioned ancestor" no longer applies once the element
/// is promoted. Every existing `[data-side]` rule in the preview
/// stylesheets (`tooltip`/`hover_card`/`popover` `style.css`) assumes the
/// old, DOM-relative containing block, so without a fix every non-modal
/// overlay this slice touches would render detached from its trigger.
///
/// CSS Anchor Positioning is the platform's own, still-CSS answer (no
/// `floating-ui`, no JS-computed geometry — that stays out of scope; see
/// `docs/plan.md` Phase 5, "Collision detection", which this is not): an
/// `anchor-name` declared on the trigger and referenced from the content
/// via `position-anchor` lets `top`/`left`/etc. in the *stylesheet* use
/// `anchor()` to mean "relative to that named anchor" even from the top
/// layer. Verified working in this environment's Chromium
/// (`CSS.supports('anchor-name: --x')` true) — see this slice's stylesheet
/// changes for the `anchor()`-based rules this pairs with, `@supports`-
/// gated so an engine without Anchor Positioning (not yet Firefox/WebKit as
/// of this writing) falls back to the pre-existing rules rather than an
/// invalid declaration.
///
/// Each overlay instance gets its own anchor name, built from its own
/// unique id (already threaded through every one of these components for
/// ARIA), so unrelated instances on the same page never collide. Only
/// meaningful alongside `popover`, so it degrades to an inert empty `style`
/// value off the web arm rather than being `#[cfg]`-gated at every call
/// site.
#[cfg(target_family = "wasm")]
pub(crate) fn anchor_name_style(id: &str) -> String {
    format!("anchor-name: --dxa-{id};")
}

/// See [`anchor_name_style`]. Set on the content element, referencing the
/// same id the trigger's `anchor_name_style` was built from. Only called
/// from the web-arm leaf render functions (the native arm never promotes
/// content to the top layer, so it has no containing-block problem to
/// solve), so -- like [`PopoverKind`] -- this has no native counterpart.
#[cfg(target_family = "wasm")]
pub(crate) fn position_anchor_style(id: &str) -> String {
    format!("position-anchor: --dxa-{id};")
}

/// No-op on every non-wasm target -- see [`anchor_name_style`]'s doc. Unlike
/// [`position_anchor_style`], this one *is* called unconditionally (every
/// trigger sets it, regardless of target), so it needs a real native stub
/// rather than being `#[cfg]`-gated away entirely.
#[cfg(not(target_family = "wasm"))]
pub(crate) fn anchor_name_style(_id: &str) -> String {
    String::new()
}
