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

#[cfg(feature = "web")]
use dioxus::document;
#[cfg(feature = "web")]
use dioxus::prelude::*;

#[cfg(feature = "web")]
use std::cell::Cell;

#[cfg(feature = "web")]
thread_local! {
    /// Whether [`ensure_anchor_positioning_styles`]'s `document::eval` has
    /// already been scheduled once in this WASM instance -- same idempotency
    /// guard shape as `scroll_lock.rs`'s
    /// `SCROLLBAR_GUTTER_BASELINE_INSTALLED`.
    static ANCHOR_POSITIONING_STYLES_INSTALLED: Cell<bool> = const { Cell::new(false) };
}

/// Installs the shared, engine-level anchor-positioning stylesheet described
/// below, at most once per WASM instance -- mirrors
/// `scroll_lock::ensure_scrollbar_gutter_baseline`'s idempotent
/// style-tag-injection shape exactly (a Rust-side `Cell` guard, plus a
/// JS-side `getElementById` check as a second guard that survives a
/// hot-reload resetting Rust statics but leaving the already-injected tag in
/// the live DOM).
///
/// ## The bug class this closes (docs/backlog.md item 1, 2026-09-01)
///
/// Before this function existed, the `@supports (anchor-name: --a)` block
/// (per-`[data-side]`/`[data-align]` `anchor()` rules, `position-try-
/// fallbacks: flip-block, flip-inline`) and the `[popover]` UA-stylesheet
/// reset (`margin: 0; inset: auto;` -- undoing `[popover] { inset: 0; margin:
/// auto; }`, which otherwise invokes CSS's auto-margin centering algorithm
/// and lands content far from its trigger; see `use_anchor_position_fallback`'s
/// doc above) had to be hand-copied into *every* consuming page's own
/// `#[css_module]` stylesheet, because `manganis-core`'s `css_module_parser`
/// does not scope classes referenced only inside `@supports`
/// (`docs/issues/css-module-supports-scoping.md`) -- so each copy had to be a
/// plain, unhashed `dx-anchor-*` marker class rather than a `Styles::`-routed
/// one. Two consuming pages (`color_picker`, `date_picker`) shipped without
/// their copy and silently fell back to un-anchored positioning until the
/// gap was noticed and hand-patched (see the "Wiring-gap fix" comments this
/// change removes from both files' `style.css`) -- a bug class, not a
/// one-off: every *future* `dx-anchor-*`-marked overlay would have to
/// remember to duplicate this same block into its own page's stylesheet, or
/// fail the identical way. Injecting this once, engine-side, from the same
/// Rust module that defines the marker-class contract
/// (`anchor_name_style`/`position_anchor_style` below) instead makes anchor
/// positioning "just work" for `dx-anchor-tooltip`/`dx-anchor-hover-card`/
/// `dx-anchor-popover`/`dx-anchor-dropdown-menu`/`dx-anchor-menubar`/
/// `dx-anchor-navbar`/`dx-anchor-select`/`dx-anchor-combobox` (Migration A
/// slice 3/3, `select/components/list.rs`'s `SelectListRendered` and
/// `combobox/components/list.rs`'s `ComboboxListRendered`; `dx-anchor-navbar`
/// added 2026-09-03, finding C -- `navbar.rs`'s `NavbarContentRendered`)
/// content everywhere in the app, with no per-page CSS to remember.
///
/// Call this once per overlay content mount -- [`use_anchor_position_fallback`]
/// does, since every current `dx-anchor-*` consumer (`tooltip.rs`,
/// `hover_card.rs`, `popover.rs`, `dropdown_menu.rs`, `menubar.rs`,
/// `navbar.rs`, `select/components/list.rs`, `combobox/components/list.rs`,
/// and anything built on `crate::popover` like `ColorPicker`/`DatePicker`)
/// already calls that hook. A future
/// `dx-anchor-*` consumer that doesn't call that hook for some reason should
/// call this directly instead.
///
/// `:where(...)` (zero specificity) is used for the `[popover]` reset so it
/// never has to out-specificity-fight a component's own rules -- author
/// origin already beats the UA stylesheet regardless of specificity (see
/// `tooltip.rs`'s call site... actually see the removed `../tooltip/
/// style.css` comment history for the full citation), so zero specificity
/// here is exactly enough to win against the *UA* default while leaving any
/// component-authored override free to still take precedence if one is ever
/// needed.
///
/// `border`/`overflow` resets are deliberately **not** included here, unlike
/// the `margin`/`inset` ones: those two are genuine per-component style
/// choices (`Tooltip` wants no border and never scrolls; `HoverCard`/
/// `Popover` already declare a real border of their own, which -- being
/// author-origin -- already beats the UA default with no reset needed), not
/// a single correct answer every anchored overlay shares the way the
/// centering-trap fix is. Each component's own stylesheet keeps whatever
/// `border`/`overflow` reset it individually needs.
#[cfg(feature = "web")]
pub(crate) fn ensure_anchor_positioning_styles() {
    if ANCHOR_POSITIONING_STYLES_INSTALLED.with(|installed| installed.replace(true)) {
        return;
    }
    let eval = document::eval(&anchor_positioning_inject_js());
    let _ = eval;
}

/// The idempotent (JS-side `getElementById` guarded, so safe to run more
/// than once) style-tag-injection statement [`ensure_anchor_positioning_styles`]
/// dispatches on its own, factored out into a plain JS-snippet-returning
/// function rather than only living inside that eval call, so
/// [`use_anchor_position_fallback`] below can prepend the *exact same*
/// statement to the front of its own measurement script.
///
/// That prepending matters, and is not merely a convenience: this function's
/// statement and a measurement's `getBoundingClientRect()` call dispatched
/// from two *separate* `document::eval()` calls have no guaranteed relative
/// ordering against each other from Rust's point of view (confirmed by
/// execution: an earlier version of this fix called
/// `ensure_anchor_positioning_styles()` from a `use_effect` declared just
/// before `use_anchor_position_fallback`'s own measurement effect in the
/// same function, relying on Dioxus's normal same-component effect-ordering
/// guarantee -- and on a fresh page load, before this stylesheet had ever
/// been injected, the measurement's `reposition()` still ran before the
/// browser had applied the freshly-appended `<style>` tag's rules, so its
/// `matches()` check saw the *pre-anchor* position and wrongly concluded
/// this engine's CSS Anchor Positioning integration had failed, taking over
/// with an inline-style override it never needed to make --
/// `playwright/oracle/tier2-html/top-layer.spec.ts`'s Rule 8 ColorPicker
/// case caught this on the very first cold load of that page). Two
/// statements inside *one* `document::eval()` call, by contrast, are one
/// synchronous JS script as far as the browser's engine is concerned:
/// `appendChild`ing the `<style>` tag synchronously invalidates style for
/// matching elements, and the very next statement's `getBoundingClientRect()`
/// forces a synchronous style/layout recalc that is guaranteed to already
/// see it -- no ordering assumption about *separate* `document::eval()`
/// dispatches required at all.
#[cfg(feature = "web")]
fn anchor_positioning_inject_js() -> String {
    format!(
        r#"
        if (!document.getElementById('dx-anchor-positioning-styles')) {{
            const style = document.createElement('style');
            style.id = 'dx-anchor-positioning-styles';
            style.textContent = {css};
            document.head.appendChild(style);
        }}
        "#,
        css = ANCHOR_POSITIONING_CSS_JS_LITERAL
    )
}

/// The shared anchor-positioning stylesheet's CSS text, as a JS backtick
/// template-literal source (i.e. this string itself already includes the
/// wrapping backticks) -- see [`ensure_anchor_positioning_styles`]'s doc for
/// the bug class this closes and why `border`/`overflow` resets are
/// deliberately not part of it.
///
/// Every `.dx-anchor-popover[popover]` selector below has a
/// `.dx-anchor-popover:modal` sibling (native-dialog engine migration,
/// two-engine overlay architecture completion): a modal `Popover`'s web arm
/// is a real `<dialog>` opened with `showModal()`, so it carries the
/// `dx-anchor-popover` marker class the same way the non-modal arm's
/// `<dialog popover="auto">` does, but never the `popover` attribute itself
/// (`showModal()` and the Popover API are two different top-layer
/// mechanisms -- see `popover.rs`'s module doc). `:modal` is the CSS
/// pseudo-class a `<dialog>` matches for as long as it is shown via
/// `showModal()` -- the natural selector to key the exact same reset/
/// `anchor()` treatment off for that arm, mirroring `[popover]`'s role for
/// the Popover-API arms without needing a second marker class.
#[cfg(feature = "web")]
const ANCHOR_POSITIONING_CSS_JS_LITERAL: &str = r#"`
:where(.dx-anchor-tooltip[popover], .dx-anchor-hover-card[popover], .dx-anchor-popover[popover], .dx-anchor-popover:modal, .dx-anchor-dropdown-menu[popover],
  .dx-anchor-menubar[popover], .dx-anchor-navbar[popover], .dx-anchor-select[popover], .dx-anchor-combobox[popover]) {
  margin: 0;
  inset: auto;
}

@supports (anchor-name: --a) {
  .dx-anchor-tooltip[popover],
  .dx-anchor-popover[popover],
  .dx-anchor-popover:modal,
  .dx-anchor-dropdown-menu[popover],
  .dx-anchor-menubar[popover],
  .dx-anchor-navbar[popover],
  .dx-anchor-select[popover],
  .dx-anchor-combobox[popover] {
    position: fixed;
    margin: 0;
    inset: auto;
    transform: none;
    position-try-fallbacks: flip-block, flip-inline;
  }

  .dx-anchor-tooltip[popover][data-side="top"],
  .dx-anchor-popover[popover][data-side="top"],
  .dx-anchor-popover:modal[data-side="top"],
  .dx-anchor-dropdown-menu[popover][data-side="top"],
  .dx-anchor-menubar[popover][data-side="top"],
  .dx-anchor-navbar[popover][data-side="top"],
  .dx-anchor-select[popover][data-side="top"],
  .dx-anchor-combobox[popover][data-side="top"] {
    bottom: anchor(top);
    left: anchor(center);
    margin-bottom: 8px;
    transform: translateX(-50%);
  }

  .dx-anchor-tooltip[popover][data-side="right"],
  .dx-anchor-popover[popover][data-side="right"],
  .dx-anchor-popover:modal[data-side="right"],
  .dx-anchor-dropdown-menu[popover][data-side="right"],
  .dx-anchor-menubar[popover][data-side="right"],
  .dx-anchor-navbar[popover][data-side="right"],
  .dx-anchor-select[popover][data-side="right"],
  .dx-anchor-combobox[popover][data-side="right"] {
    top: anchor(center);
    left: anchor(right);
    margin-left: 8px;
    transform: translateY(-50%);
  }

  .dx-anchor-tooltip[popover][data-side="bottom"],
  .dx-anchor-popover[popover][data-side="bottom"],
  .dx-anchor-popover:modal[data-side="bottom"],
  .dx-anchor-dropdown-menu[popover][data-side="bottom"],
  .dx-anchor-menubar[popover][data-side="bottom"],
  .dx-anchor-navbar[popover][data-side="bottom"],
  .dx-anchor-select[popover][data-side="bottom"],
  .dx-anchor-combobox[popover][data-side="bottom"] {
    top: anchor(bottom);
    left: anchor(center);
    margin-top: 8px;
    transform: translateX(-50%);
  }

  .dx-anchor-tooltip[popover][data-side="left"],
  .dx-anchor-popover[popover][data-side="left"],
  .dx-anchor-popover:modal[data-side="left"],
  .dx-anchor-dropdown-menu[popover][data-side="left"],
  .dx-anchor-menubar[popover][data-side="left"],
  .dx-anchor-navbar[popover][data-side="left"],
  .dx-anchor-select[popover][data-side="left"],
  .dx-anchor-combobox[popover][data-side="left"] {
    top: anchor(center);
    right: anchor(left);
    margin-right: 8px;
    transform: translateY(-50%);
  }

  .dx-anchor-hover-card[popover] {
    position: fixed;
    margin: 0;
    inset: auto;
    position-try-fallbacks: flip-block, flip-inline;
  }

  .dx-anchor-hover-card[popover][data-side="top"] {
    bottom: anchor(top);
    left: anchor(center);
    margin-bottom: 10px;
    transform: translateX(-50%);
  }

  .dx-anchor-hover-card[popover][data-side="right"] {
    top: anchor(center);
    left: anchor(right);
    margin-left: 10px;
    transform: translateY(-50%);
  }

  .dx-anchor-hover-card[popover][data-side="bottom"] {
    top: anchor(bottom);
    left: anchor(center);
    margin-top: 10px;
    transform: translateX(-50%);
  }

  .dx-anchor-hover-card[popover][data-side="left"] {
    top: anchor(center);
    right: anchor(left);
    margin-right: 10px;
    transform: translateY(-50%);
  }

  .dx-anchor-tooltip[popover][data-side="top"][data-align="start"],
  .dx-anchor-hover-card[popover][data-side="top"][data-align="start"],
  .dx-anchor-popover[popover][data-side="top"][data-align="start"],
  .dx-anchor-popover:modal[data-side="top"][data-align="start"],
  .dx-anchor-dropdown-menu[popover][data-side="top"][data-align="start"],
  .dx-anchor-menubar[popover][data-side="top"][data-align="start"],
  .dx-anchor-navbar[popover][data-side="top"][data-align="start"],
  .dx-anchor-select[popover][data-side="top"][data-align="start"],
  .dx-anchor-combobox[popover][data-side="top"][data-align="start"],
  .dx-anchor-tooltip[popover][data-side="bottom"][data-align="start"],
  .dx-anchor-hover-card[popover][data-side="bottom"][data-align="start"],
  .dx-anchor-popover[popover][data-side="bottom"][data-align="start"],
  .dx-anchor-popover:modal[data-side="bottom"][data-align="start"],
  .dx-anchor-dropdown-menu[popover][data-side="bottom"][data-align="start"],
  .dx-anchor-menubar[popover][data-side="bottom"][data-align="start"],
  .dx-anchor-navbar[popover][data-side="bottom"][data-align="start"],
  .dx-anchor-select[popover][data-side="bottom"][data-align="start"],
  .dx-anchor-combobox[popover][data-side="bottom"][data-align="start"] {
    left: anchor(left);
    transform: none;
  }

  .dx-anchor-tooltip[popover][data-side="top"][data-align="center"],
  .dx-anchor-hover-card[popover][data-side="top"][data-align="center"],
  .dx-anchor-popover[popover][data-side="top"][data-align="center"],
  .dx-anchor-popover:modal[data-side="top"][data-align="center"],
  .dx-anchor-dropdown-menu[popover][data-side="top"][data-align="center"],
  .dx-anchor-menubar[popover][data-side="top"][data-align="center"],
  .dx-anchor-navbar[popover][data-side="top"][data-align="center"],
  .dx-anchor-select[popover][data-side="top"][data-align="center"],
  .dx-anchor-combobox[popover][data-side="top"][data-align="center"],
  .dx-anchor-tooltip[popover][data-side="bottom"][data-align="center"],
  .dx-anchor-hover-card[popover][data-side="bottom"][data-align="center"],
  .dx-anchor-popover[popover][data-side="bottom"][data-align="center"],
  .dx-anchor-popover:modal[data-side="bottom"][data-align="center"],
  .dx-anchor-dropdown-menu[popover][data-side="bottom"][data-align="center"],
  .dx-anchor-menubar[popover][data-side="bottom"][data-align="center"],
  .dx-anchor-navbar[popover][data-side="bottom"][data-align="center"],
  .dx-anchor-select[popover][data-side="bottom"][data-align="center"],
  .dx-anchor-combobox[popover][data-side="bottom"][data-align="center"] {
    left: anchor(center);
    transform: translateX(-50%);
  }

  .dx-anchor-tooltip[popover][data-side="top"][data-align="end"],
  .dx-anchor-hover-card[popover][data-side="top"][data-align="end"],
  .dx-anchor-popover[popover][data-side="top"][data-align="end"],
  .dx-anchor-popover:modal[data-side="top"][data-align="end"],
  .dx-anchor-dropdown-menu[popover][data-side="top"][data-align="end"],
  .dx-anchor-menubar[popover][data-side="top"][data-align="end"],
  .dx-anchor-navbar[popover][data-side="top"][data-align="end"],
  .dx-anchor-select[popover][data-side="top"][data-align="end"],
  .dx-anchor-combobox[popover][data-side="top"][data-align="end"],
  .dx-anchor-tooltip[popover][data-side="bottom"][data-align="end"],
  .dx-anchor-hover-card[popover][data-side="bottom"][data-align="end"],
  .dx-anchor-popover[popover][data-side="bottom"][data-align="end"],
  .dx-anchor-popover:modal[data-side="bottom"][data-align="end"],
  .dx-anchor-dropdown-menu[popover][data-side="bottom"][data-align="end"],
  .dx-anchor-menubar[popover][data-side="bottom"][data-align="end"],
  .dx-anchor-navbar[popover][data-side="bottom"][data-align="end"],
  .dx-anchor-select[popover][data-side="bottom"][data-align="end"],
  .dx-anchor-combobox[popover][data-side="bottom"][data-align="end"] {
    left: anchor(right);
    transform: translateX(-100%);
  }

  .dx-anchor-tooltip[popover][data-side="left"][data-align="start"],
  .dx-anchor-hover-card[popover][data-side="left"][data-align="start"],
  .dx-anchor-popover[popover][data-side="left"][data-align="start"],
  .dx-anchor-popover:modal[data-side="left"][data-align="start"],
  .dx-anchor-dropdown-menu[popover][data-side="left"][data-align="start"],
  .dx-anchor-menubar[popover][data-side="left"][data-align="start"],
  .dx-anchor-navbar[popover][data-side="left"][data-align="start"],
  .dx-anchor-select[popover][data-side="left"][data-align="start"],
  .dx-anchor-combobox[popover][data-side="left"][data-align="start"],
  .dx-anchor-tooltip[popover][data-side="right"][data-align="start"],
  .dx-anchor-hover-card[popover][data-side="right"][data-align="start"],
  .dx-anchor-popover[popover][data-side="right"][data-align="start"],
  .dx-anchor-popover:modal[data-side="right"][data-align="start"],
  .dx-anchor-dropdown-menu[popover][data-side="right"][data-align="start"],
  .dx-anchor-menubar[popover][data-side="right"][data-align="start"],
  .dx-anchor-navbar[popover][data-side="right"][data-align="start"],
  .dx-anchor-select[popover][data-side="right"][data-align="start"],
  .dx-anchor-combobox[popover][data-side="right"][data-align="start"] {
    top: anchor(top);
    transform: none;
  }

  .dx-anchor-tooltip[popover][data-side="left"][data-align="center"],
  .dx-anchor-hover-card[popover][data-side="left"][data-align="center"],
  .dx-anchor-popover[popover][data-side="left"][data-align="center"],
  .dx-anchor-popover:modal[data-side="left"][data-align="center"],
  .dx-anchor-dropdown-menu[popover][data-side="left"][data-align="center"],
  .dx-anchor-menubar[popover][data-side="left"][data-align="center"],
  .dx-anchor-navbar[popover][data-side="left"][data-align="center"],
  .dx-anchor-select[popover][data-side="left"][data-align="center"],
  .dx-anchor-combobox[popover][data-side="left"][data-align="center"],
  .dx-anchor-tooltip[popover][data-side="right"][data-align="center"],
  .dx-anchor-hover-card[popover][data-side="right"][data-align="center"],
  .dx-anchor-popover[popover][data-side="right"][data-align="center"],
  .dx-anchor-popover:modal[data-side="right"][data-align="center"],
  .dx-anchor-dropdown-menu[popover][data-side="right"][data-align="center"],
  .dx-anchor-menubar[popover][data-side="right"][data-align="center"],
  .dx-anchor-navbar[popover][data-side="right"][data-align="center"],
  .dx-anchor-select[popover][data-side="right"][data-align="center"],
  .dx-anchor-combobox[popover][data-side="right"][data-align="center"] {
    top: anchor(center);
    transform: translateY(-50%);
  }

  .dx-anchor-tooltip[popover][data-side="left"][data-align="end"],
  .dx-anchor-hover-card[popover][data-side="left"][data-align="end"],
  .dx-anchor-popover[popover][data-side="left"][data-align="end"],
  .dx-anchor-popover:modal[data-side="left"][data-align="end"],
  .dx-anchor-dropdown-menu[popover][data-side="left"][data-align="end"],
  .dx-anchor-menubar[popover][data-side="left"][data-align="end"],
  .dx-anchor-navbar[popover][data-side="left"][data-align="end"],
  .dx-anchor-select[popover][data-side="left"][data-align="end"],
  .dx-anchor-combobox[popover][data-side="left"][data-align="end"],
  .dx-anchor-tooltip[popover][data-side="right"][data-align="end"],
  .dx-anchor-hover-card[popover][data-side="right"][data-align="end"],
  .dx-anchor-popover[popover][data-side="right"][data-align="end"],
  .dx-anchor-popover:modal[data-side="right"][data-align="end"],
  .dx-anchor-dropdown-menu[popover][data-side="right"][data-align="end"],
  .dx-anchor-menubar[popover][data-side="right"][data-align="end"],
  .dx-anchor-navbar[popover][data-side="right"][data-align="end"],
  .dx-anchor-select[popover][data-side="right"][data-align="end"],
  .dx-anchor-combobox[popover][data-side="right"][data-align="end"] {
    top: anchor(bottom);
    transform: translateY(-100%);
  }
}
`"#;

/// Which `popover` dismissal behaviour an element declares.
/// WHATWG HTML §the-popover-attribute (see module docs for the link).
///
/// Only referenced from the `#[cfg(feature = "web")]` leaf render
/// functions in `tooltip.rs`/`hover_card.rs`/`popover.rs` -- the native
/// (Blitz) arm never renders the `popover` attribute at all (see
/// `use_popover_sync`'s doc) -- so this whole type is `#[cfg]`-gated too,
/// rather than degrading to a dead stub on that target.
#[cfg(feature = "web")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PopoverKind {
    /// `popover="auto"` — light-dismisses on Escape and on a pointerdown
    /// outside the popover (and any popovers it is nested in); showing one
    /// auto popover also closes unrelated open ones. Used by the non-modal
    /// [`crate::popover`] arm, whose existing `use_outside_dismiss`/
    /// `use_global_escape_listener` wiring this replaces on the web arm;
    /// also `dropdown_menu.rs`, `menubar.rs`, and `select/components/
    /// list.rs`'s `SelectListRendered` (Migration A slices 1-3), each of
    /// which layers light dismiss on top of -- not instead of -- their own
    /// existing blur-driven dismissal as a backstop; see each's own doc for
    /// why `auto` is safe there specifically.
    Auto,
    /// `popover="manual"` — no light-dismiss; only an explicit
    /// `hidePopover()` (here, driven by our own `open` signal going false)
    /// closes it. Used by [`crate::tooltip`] and [`crate::hover_card`],
    /// which already own their entire open/close lifecycle via hover/focus
    /// and gain nothing from -- and would fight with -- native light
    /// dismiss (a `HoverCardContent` mouseenter re-opens across what would
    /// otherwise be an outside click if `auto` briefly closed it first);
    /// also `context_menu.rs` (a point-opened menu with no persistent
    /// trigger for `auto` to reason about) and `combobox/components/
    /// list.rs`'s `ComboboxListRendered` (Migration A slice 3/3, reversed
    /// from an initial `auto` attempt by execution -- see that component's
    /// doc for the concrete regression that decision caught).
    Manual,
}

#[cfg(feature = "web")]
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
/// It is gated on `feature = "web"` (a renderer Cargo feature this crate
/// already defines), not `target_family = "wasm"` — `dioxus-desktop` is a
/// non-wasm binary with a real, working webview `eval`, and belongs on this
/// same web-style arm, not on the native/no-op one `target_family = "wasm"`
/// would wrongly exclude it from. `target_family = "wasm"` was the original
/// gate here; it was corrected to `feature = "web"` after a 2026-09-01
/// production incident (`docs/recommended-implementations.md` Caveat 1)
/// showed the fullstack SSG prerender — a host (non-wasm) binary built with
/// this feature on — rendered the native-arm markup instead, because the
/// old gate excluded it. See `docs/recommended-implementations.md` Caveat 1
/// for the corrected rule: markup/attribute choice keys off this renderer
/// feature; only genuinely wasm-only *execution* internals (none exist in
/// this hook's body — `document::eval` itself compiles and runs inertly on
/// any renderer with no document context) would still key off
/// `target_family`.
#[cfg(feature = "web")]
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

/// Variant of [`use_popover_sync`] for content whose *own* `data-state`
/// (Radix-style) animates through a "closed but still mounted" phase --
/// [`use_animated_open`]-driven content, unlike every other
/// `use_popover_sync` call site. Forwards native/browser closes to
/// `on_native_close` exactly like [`use_popover_sync`]; the difference is
/// entirely on the "signal -> browser" side: `showPopover()` still runs
/// reactively, every time `open` becomes `true` (not once, at mount -- see
/// "Two execution-confirmed bugs" below for why once is not enough), but
/// **`hidePopover()` is never called on our own closing path at all** --
/// only a real DOM removal (this component's caller unmounting it once its
/// own exit animation settles) ever takes this content out of the top
/// layer.
///
/// ## Why this exists (Migration A slice 3/3): two execution-confirmed bugs
///
/// [`use_popover_sync`]'s own "signal -> browser" effect calls
/// `hidePopover()` the instant `open` goes `false`. That is exactly right
/// for content with no exit animation (every `use_popover_sync` call site
/// before this slice: `Tooltip`/`HoverCard`/`Popover`/`DropdownMenu`/
/// `ContextMenu`/`Menubar`, none of which keep their content mounted
/// through a CSS animate-out -- Radix-style `data-state="closed"`
/// animations on those are cosmetic fades that finish well after the
/// element would already be gone). `SelectList`/`ComboboxList` are
/// different: both render through [`use_animated_open`], which
/// deliberately keeps the content mounted with `data-state="closed"` for
/// its whole exit animation (plus a settle hold) before actually
/// unmounting it -- `combobox.spec.ts`'s "keeps filtered options during
/// keyboard close animation" test asserts exactly this, that the
/// `data-state="closed"` element is still visible mid-animation.
///
/// **Bug 1 (animation race).** Confirmed by execution: an earlier version
/// of this migration called [`use_popover_sync`] directly with the raw
/// `open` signal (the same shape every prior `dx-anchor-*` consumer uses),
/// and that test went red -- `hidePopover()` fired on the same tick `open`
/// went `false`, which (per the UA popover stylesheet's
/// `[popover]:not(:popover-open) { display: none }`) sets `display: none`
/// on the content *before* [`use_animated_open`]'s own rAF-deferred
/// `getAnimations()` check ever ran, so it always observed zero running
/// animations and finished the close cycle immediately -- the exit
/// animation was skipped outright, and Playwright's `toBeVisible()` on the
/// still-should-be-animating `data-state="closed"` element failed. Never
/// calling `hidePopover()` from the closing path at all -- only from a real
/// unmount, once [`use_animated_open`] itself has already decided the
/// animation (and its settle hold) is done -- sidesteps this entirely: the
/// exit animation plays out undisturbed on an element that is still very
/// much `:popover-open` the whole time.
///
/// **Bug 2 (reopen-while-still-mounted, `auto` only).** Confirmed by
/// execution against `combobox.spec.ts`'s "filters and selects with the
/// keyboard" test (intermittently red -- see this session's report):
/// `auto`'s native light dismiss has no notion of "this input is the
/// trigger that owns me" (`ComboboxInput`/`SelectTrigger` are never
/// declared as this popover's invoker -- confirmed by execution that
/// neither a `popovertarget` attribute nor `preventDefault()` on the
/// trigger's own `pointerdown`/`click` stops it; WHATWG light dismiss
/// treats *any* pointerdown outside the popover's own DOM subtree as
/// "outside," full stop). So: select an option (closes, `open` -> `false`,
/// content stays mounted mid-exit-animation per Bug 1's fix above):, then
/// -- while that same instance is *still* mounted, `:popover-open` still
/// `true` -- click the trigger again to reopen. The trigger's own
/// `pointerdown` is classified as "outside" by native light dismiss, which
/// closes the *still-showing* popover for real (a genuine
/// `showPopover`/`hidePopover` pair, not merely a `data-state` flip) --
/// forwarded correctly back into `on_native_close`, setting `open` to
/// `false` -- **immediately followed**, same gesture, by the trigger's own
/// `onclick` reading `open` and calling `set_open(true)` to reopen it. A
/// one-time, mount-only `showPopover()` call (this function's first,
/// insufficient version) never re-fires for this already-mounted instance,
/// so `open` becomes `true` again in Rust/`data-state` while the actual
/// DOM element stays hidden (`:not(:popover-open)`) -- a real, user-visible
/// "second click does nothing" defect. Reacting to `open` on every
/// transition (not just the first) fixes it the same way
/// [`use_popover_sync`]'s own show effect already always has: whenever
/// `open` is `true` and the element is not yet `:popover-open`,
/// `showPopover()` runs again, regardless of *why* it had stopped being
/// shown.
///
/// A *native* close (Escape / outside pointerdown, `auto` only) still
/// bypasses the exit animation on its own path -- the browser's own hide
/// algorithm sets `display: none` synchronously, before Rust ever learns
/// about it -- but that is an accepted, well-known limit of the plain
/// Popover API without `@starting-style`/`transition-behavior:
/// allow-discrete` (out of scope here), not something this slice's oracle
/// requires animated; the test Bug 1's fix targets only exercises a
/// script-driven (Enter-key) close, exactly the path that fix keeps
/// animatable.
#[cfg(feature = "web")]
pub(crate) fn use_popover_shown_while_mounted(
    id: String,
    open: impl Readable<Target = bool> + Copy + 'static,
    on_native_close: Callback<bool>,
) {
    // Browser -> signal: identical in shape to `use_popover_sync`'s own --
    // forwards every native `toggle` for this element's whole mounted
    // lifetime, unconditionally.
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
                on_native_close.call(is_open);
            }
        });
        move || {
            let _ = eval.send(true);
        }
    });

    // Signal -> browser, show-only: reacts to the real `open` signal (not a
    // one-shot mount effect -- see "Bug 2" above for why a reopen needs
    // this to run again) so `showPopover()` fires every time `open` is
    // `true` and the browser doesn't already agree; `hidePopover()` is
    // deliberately never called here at all -- see "Bug 1" above -- only a
    // real unmount ever closes this content.
    use_effect(move || {
        if !open.cloned() {
            return;
        }
        let id = id.clone();
        document::eval(&format!(
            "const el = document.getElementById('{id}');
            if (el && !el.matches(':popover-open')) el.showPopover();"
        ));
    });
}

/// JS-measured static positioning fallback/self-check for engines where CSS
/// Anchor Positioning doesn't land the content next to its trigger. Confirmed
/// by execution to be *required*, not merely a nice-to-have: once an element
/// is promoted to the top layer via `popover`, its containing block becomes
/// the viewport regardless of its own `position` value (see
/// [`position_anchor_style`]'s doc) -- which means the pre-4.4 `[data-side]`
/// rules in `tooltip`/`hover_card`/`popover`'s `style.css` (written for the
/// old DOM-relative `position: absolute`) do not gracefully degrade into
/// "roughly right, just not collision-aware" on an engine that can't resolve
/// `anchor()`; they degrade into detached from the trigger entirely -- the
/// "tooltip not working" symptom this fixes.
///
/// Deliberately does *not* gate on `CSS.supports('anchor-name: --a')` alone,
/// unlike the CSS `@supports` blocks this pairs with (those stay -- they're
/// what makes Chromium's positioning correct from the very first layout,
/// before this effect's synchronous correction below even runs). Confirmed
/// by execution against a real, current-at-time-of-writing Firefox build:
/// `CSS.supports('anchor-name: --a')` reports `true` there (the property
/// parses), yet a popover-promoted element's `anchor()`-based `top`/`left`
/// still failed to resolve against its trigger -- a real, engine-specific
/// popover/anchor-positioning integration gap that a syntax-support check
/// cannot see. So instead this *measures the outcome*: it compares the
/// content's actual position to the trigger-relative position this
/// component's own `[data-side]`/`[data-align]` contract promises, and
/// only overrides with inline styles when they disagree by more than a
/// couple of pixels (rounding/subpixel noise). On an engine where `anchor()`
/// already resolved correctly (this repo's Chromium, confirmed by
/// execution), the computed and actual positions match and nothing is
/// touched -- no extra reflow, no risk of fighting a working anchor
/// resolution.
///
/// ## The flip contract (docs/backlog.md row 10)
///
/// The stylesheets this pairs with (`tooltip`/`hover_card`/`popover`
/// `style.css`, `@supports` blocks) now declare
/// `position-try-fallbacks: flip-block, flip-inline` on the anchored
/// `[popover]` rule -- the CSS Anchor Positioning spec's own collision-
/// avoidance primitive
/// (<https://www.w3.org/TR/css-anchor-position-1/#fallback-var>). On an
/// engine that implements it, a preferred placement that doesn't fit the
/// viewport can legitimately paint at the *opposite* side on the relevant
/// axis instead -- e.g. `[data-side="bottom"]` rendering physically above
/// its trigger. That is intentional disagreement with the naive "actual
/// equals `[data-side]`'s own formula" check this function used to make
/// unconditionally, and this function runs after that flip has already
/// painted (see the mount-order/timing note further down) -- so this
/// function computes **both** candidate positions (the primary side's own
/// formula, and the same formula with only the relevant axis's side
/// swapped -- top/bottom sides flip on the block axis, left/right sides on
/// the inline axis, matching `flip-block`/`flip-inline` exactly) and
/// accepts the painted position if it matches *either* one. Only when it
/// matches **neither** -- meaning this engine's anchor-positioning
/// integration failed outright, the pre-existing case this function was
/// written for -- does it override with inline styles at all. This is the
/// one load-bearing rule change here: naively re-checking only the primary
/// formula would see a legitimate CSS flip as "wrong" and stomp it back to
/// the off-viewport primary position, fighting the platform's own fix.
///
/// When it does override (no working anchor-positioning engine at all), it
/// now makes the *same* flip decision itself, from plain
/// `getBoundingClientRect()`/`window.inner{Width,Height}` viewport math:
/// if the primary placement would overflow the relevant viewport edge, it
/// uses the flipped placement instead. This closes the "legacy engines get
/// no flip at all" gap docs/backlog.md row 10 notes alongside the CSS
/// addition. Resizing to fit stays out of scope (that remainder of Phase 5
/// is still open) -- but see "Inline-axis shift (2026-09-03)" below for the
/// one piece of shift this function *does* now do.
///
/// ## Inline-axis shift (2026-09-03, user device report)
///
/// User report (iOS 18 Safari, no CSS Anchor Positioning there at all --
/// confirmed by the report itself, and the exact engine this whole function
/// exists for): the `ColorPicker` popup on the home page's widget masonry
/// clips against the viewport's edge on a small screen, with no attempt to
/// reposition it. Reproduced in this sandbox's Chromium via the same
/// no-anchor-engine simulation `top-layer.spec.ts` Rule 11 already uses
/// (`stripAnchorSupportsBlock`, `MOBILE_VIEWPORT`) -- confirmed by
/// execution: the masonry `ColorPicker`'s content landed at
/// `left: -2.97px` (a few pixels past the *left* edge in this sandbox's
/// exact layout -- the reported direction depends only on which side of a
/// narrow viewport the trigger happens to sit on, not on anything specific
/// to "right"), and two of this file's own `edge-bottom-*` fixture cases
/// (`Tooltip`, non-modal `Popover`) landed at `left: -192px`/`-194px` --
/// far enough that `flip-block` (the only fallback this contract declared
/// before this fix) could never have been the cause: `side="bottom"` only
/// ever flips to `side="top"`, which changes `top`, never `left`.
///
/// Root cause: flip (both the CSS `position-try-fallbacks` primitive this
/// function mirrors, and its own pre-existing viewport-math equivalent just
/// above) only ever swaps `side` to its axis opposite -- block for
/// top/bottom, inline for left/right (the `opposite` map above). For a
/// `side="top"`/`side="bottom"` placement, the horizontal position comes
/// entirely from `align` (start/center/end, computed once against the
/// trigger's own rect in `place()` above) and was never once checked
/// against the viewport at all -- not by CSS (this crate's `@supports`
/// block declares no shift primitive), and not by this function. A
/// center-aligned overlay wider than the room its trigger happens to have
/// on one side had no correction of any kind, on either engine.
///
/// Fixed *only* on this JS-fallback path (below, in the "neither matches"
/// branch): after the existing flip decision, `target.left` is clamped into
/// `[EDGE_MARGIN, vw - EDGE_MARGIN - cw]` (falling back to flush against the
/// left edge if the content is wider than the viewport has room for at
/// all -- resizing to fit stays out of scope, matching the flip-only note
/// above). Deliberately not mirrored into the CSS `@supports` contract in
/// this round: no `position-try-fallbacks` keyword shifts along an axis the
/// way this function's clamp does (CSS's own analogous primitive,
/// `position-area`/`margin: auto`-driven "shift", is a materially larger
/// change this fix does not need to make in order to close the reported
/// device's gap) -- so a genuinely CSS-Anchor-Positioning-conforming engine
/// (this sandbox's Chromium among them) is untouched by this fix, exactly
/// as the existing `matches()` early-return above already keeps it for
/// flip. That is not a gap for the *reported* device, though: iOS Safari
/// (per the report) has no CSS Anchor Positioning support at all, so it
/// always takes this exact fallback path regardless. `top-layer.spec.ts`
/// Rule 12 is the regression oracle, reusing Rule 11's own no-anchor-engine
/// simulation.
///
/// ## Scroll/resize tracking (decision 2026-09-01, revised 2026-09-02)
///
/// Originally a one-shot measurement taken when `open` becomes true, not a
/// live-tracking listener -- reasoned at the time as static positioning
/// parity with the pre-4.4 behavior, with scroll-following deliberately
/// deferred to `docs/plan.md` Phase 5. The repository owner reported, from
/// live-site testing, an anchored overlay (`ColorPicker`'s popup) floating
/// viewport-fixed while the page scrolled underneath it -- detached from its
/// trigger -- and explicitly decided scroll-following is now in scope,
/// superseding that deferral. As first built (2026-09-01), tracking was
/// installed **only** while an overlay was open **and** this function's own
/// measurement had judged CSS Anchor Positioning non-conforming at open time
/// and taken over with inline styles (the "neither matches" branch below);
/// see "iOS keyboard (2026-09-02)" further down for why that condition was
/// removed the very next day -- tracking listeners now install
/// unconditionally for the life of every open overlay, not gated on whether
/// the very first measurement happened to need the fallback. The function
/// still tracks its own outcome in the `usingFallback` flag closed over by
/// `reposition()`; once a call sets it, every subsequent call -- including
/// ones driven by the listeners added below -- skips the `matches()` check
/// and unconditionally recomputes and re-applies the inline position. When
/// CSS Anchor Positioning is (still) placing the content correctly,
/// `usingFallback` never flips and `reposition()` keeps returning early
/// after one extra `getBoundingClientRect()` pair per event -- the
/// platform's own `anchor()` re-resolution does the real work there, this
/// is just confirming it still is.
///
/// When tracking is active (now: always, for the reason above), capture-
/// phase (so scroll inside a nested scroll container is seen too -- `scroll`
/// does not bubble, only capture reaches it from `window`), passive
/// `scroll` and a plain `resize` listener on `window` re-run the same
/// `reposition()` function, rAF-throttled (a `rafScheduled` flag drops any
/// additional event that fires before the next paint) so a fast scroll
/// gesture doesn't queue up more synchronous layout work than one
/// measurement per frame. The flip decision is re-evaluated on every
/// re-measure (`reposition()` always recomputes `primary`/`flipped` from
/// the current, live `getBoundingClientRect()`/viewport size), so a resize
/// that changes which side fits gets the same flip treatment the initial
/// placement does. All listeners are removed -- via the same
/// `dioxus.recv()`-gated teardown every other eval-channel hook in this
/// crate uses (see `use_popover_sync` above) -- when `open` goes false or
/// this component unmounts; nothing is left listening on `window` (or
/// `visualViewport`, added 2026-09-02) past the overlay's own lifetime.
/// Deliberately narrow: no `ResizeObserver` on the trigger or content
/// elements -- only `window` (and, since 2026-09-02, `visualViewport`)
/// `scroll`/`resize`, which is all a *position* (not size) correction
/// needs. Re-running `reposition()` on every tracked event re-applies the
/// inline-axis shift (2026-09-03, see that section below) exactly as it
/// re-applies flip -- both live inside the same function, so a scroll or
/// resize that changes how much room the trigger has re-clamps the same way
/// the initial placement does, with no separate wiring needed.
///
/// ## iOS keyboard (2026-09-02)
///
/// User report (main page, iOS Safari): the `Combobox` options list renders
/// on top of its own search input after focusing/typing. Investigation
/// (this session) found two gaps in the 2026-09-01 construction above, both
/// closed together here:
///
/// 1. **No visual-viewport awareness.** `reposition()` read only
///    `window.innerWidth`/`innerHeight` plus `getBoundingClientRect()`. iOS
///    Safari's on-screen keyboard shrinks and offsets
///    `window.visualViewport` while leaving `window.inner{Width,Height}`
///    completely unchanged -- so a keyboard-driven layout change was
///    invisible to every fit/flip computation in this function, regardless
///    of whether anything re-ran it. Fixed by `viewportMetrics()` below:
///    every `vw`/`vh` read, and the trigger/content rects compared against
///    them, now go through `window.visualViewport` (falling back to
///    `window.inner*` when absent, i.e. unchanged behavior on an engine
///    without it) -- see that function's own doc for the coordinate-space
///    bookkeeping this requires (`position: fixed` stays relative to the
///    *layout* viewport even once the *visual* one is what the fit
///    decision needs to reason about).
/// 2. **Tracking installed only when already conforming-fallback at open.**
///    The 2026-09-01 construction attached the scroll/resize listeners only
///    inside the `if (usingFallback)` branch, i.e. only when the very first
///    `reposition()` call had already judged CSS Anchor Positioning
///    non-conforming. An overlay that CSS Anchor Positioning placed
///    correctly at *open* time got no listener at all for the rest of that
///    open -- reasoned at the time as "an engine that gets here already
///    re-resolves `anchor()` on scroll for free," true for scroll, but
///    silently assuming the set of contract-legal positions itself could
///    never change again for the life of that one open. It can: an iOS
///    keyboard appearing is exactly such a change, and Combobox is the one
///    anchored overlay whose opening reliably summons one (a focused text
///    input) -- consistent with only it being reported. Fixed by moving the
///    listener installation out of that `if` entirely (see below): tracking
///    is now unconditional for the life of every open overlay, and
///    `reposition()`'s own `matches()` check (unchanged) is what keeps a
///    still-conforming engine's re-checks a no-op.
///
/// Falsified along the way, so as not to be re-tried: neither gap is a
/// "plain no anchor support" problem -- `use_anchor_position_fallback`
/// already self-corrects fully and immediately for an overlay with no
/// CSS-anchor support *from open*, byte-identical to native placement (see
/// "The flip contract" above); and a failed trigger lookup (the
/// `document.querySelector` returning `null`) lands the content off-screen
/// entirely, not on top of its own trigger -- neither symptom matches the
/// report. `playwright/oracle/tier2-html/top-layer.spec.ts` Rule 11 is the
/// regression oracle for both gaps, including the specific "conforming at
/// open, CSS Anchor Positioning support removed mid-open" shape gap 2 is
/// about (that file's own header doc records which of the seven
/// `use_anchor_position_fallback` consumers this sandbox's Chromium can
/// actually be driven into that exact shape for, and why the rest are
/// marked as needing a real device instead).
///
/// `anchor_id` must be the exact same string [`anchor_name_style`] was
/// built from for this overlay's trigger (every call site threads its
/// content's own id through both, so they always match by construction --
/// see `PopoverCtx::content_id`'s doc in `popover.rs` for the bug this
/// guards against for that component specifically). The trigger element
/// itself is found by matching that id's `anchor-name` back out of its
/// inline `style` attribute (a plain substring match -- cheap, and every
/// trigger always sets this style whether or not it ends up used) rather
/// than requiring a second id to be threaded through separately.
///
/// The query includes the closing `;` [`anchor_name_style`] always ends
/// with (`anchor-name: --dxa-{id};`), not just `--dxa-{id}` bare --
/// confirmed by execution to be required, not cosmetic: this repo's
/// `use_unique_id` ids are plain incrementing integers rendered as decimal
/// strings, so an id like `dxc-4` is a literal string *prefix* of `dxc-40`,
/// `dxc-41`, etc. Migration A slice 3/3's `Combobox` page is the first
/// consumer of this fallback with enough sibling anchored instances on one
/// page (several demo variants, each contributing its own `ComboboxInput`)
/// to actually hit this: without the trailing `;`, `document.querySelector`
/// for anchor id `dxc-4`'s substring pattern matched a *different* input's
/// `anchor-name: --dxa-dxc-40;` first in DOM order, measuring the wrong
/// trigger's position entirely and landing the listbox hundreds of pixels
/// off-screen (`combobox.spec.ts`'s "controlled value and controlled open
/// stay in sync" test timing out clicking an option it could never reach).
/// The `;` is never itself ambiguous across ids: it is not part of any
/// digit sequence, so `dxc-4;` cannot appear inside `dxc-40;`.
#[cfg(feature = "web")]
pub(crate) fn use_anchor_position_fallback(
    id: String,
    anchor_id: String,
    open: impl Readable<Target = bool> + Copy + 'static,
    side: crate::ContentSide,
    align: crate::ContentAlign,
    gap_px: u32,
) {
    // Best-effort early install for repeat opens in this WASM instance --
    // once this has run once (from any overlay), it's a same-tick Rust-side
    // Cell check on every later call, nothing more. It is *not* what makes
    // the very first, cold-load open correct, though: see
    // `anchor_positioning_inject_js`'s doc for why that guarantee instead
    // comes from prepending the identical injection statement onto the
    // measurement eval's own script below.
    use_effect(ensure_anchor_positioning_styles);

    crate::use_effect_with_cleanup(move || -> Box<dyn FnOnce()> {
        if !open.cloned() {
            return Box::new(|| {});
        }
        let id = id.clone();
        let anchor_id = anchor_id.clone();
        let side = side.as_str();
        let align = align.as_str();
        // Prepended into the same `document::eval()` script as the
        // measurement below -- see `anchor_positioning_inject_js`'s doc for
        // why this specific pairing (one script, not two separate `eval()`
        // calls) is what actually guarantees the stylesheet is in effect
        // before `reposition()`'s first `getBoundingClientRect()` call.
        let inject = anchor_positioning_inject_js();
        // Synchronous up to and including the first `reposition()` call
        // below, not deferred to `requestAnimationFrame`: this runs from an
        // effect declared (and therefore, on Dioxus's normal mount-order
        // guarantee, run) after `use_popover_sync`'s own effect in every
        // call site, so `showPopover()` has already been dispatched -- and
        // `getBoundingClientRect()`/`offsetWidth` force a synchronous layout
        // in every engine regardless of paint timing, so there is nothing to
        // wait a frame for. Confirmed by execution to matter, not just
        // simpler: an earlier version of this function wrapped its body in
        // `requestAnimationFrame(() => {{ ... }})` and it silently never ran
        // at all on a real Firefox build -- an eval's `document::eval` call
        // is a bare, uncaptured expression whose returned handle can be
        // dropped at the end of a synchronous closure with no ill effect
        // (the same fire-and-forget shape `use_popover_sync`'s own
        // synchronous `document::eval` calls use throughout this file), but
        // dropping it before a callback deferred to a later task/frame ever
        // fires appears to tear down the JS side before that callback runs.
        // A synchronous body has no such window -- which is exactly why,
        // below, the eval handle *is* kept alive (not dropped) for as long
        // as tracking might need it: unlike the one-shot original, this
        // version's `await dioxus.recv()` tail is a real pending callback.
        let eval = document::eval(&format!(
            r#"
            {inject}
            const content = document.getElementById('{id}');
            const trigger = document.querySelector(
                '[style*="anchor-name: --dxa-{anchor_id};"]'
            );
            const align = '{align}';
            const gap = {gap_px};
            const side = '{side}';
            // The only side `position-try-fallbacks: flip-block,
            // flip-inline` can ever swap this one to: top/bottom flip
            // on the block axis, left/right on the inline axis -- each
            // of the four sides has exactly one relevant flip axis, so
            // there is exactly one opposite candidate, not a set of
            // four.
            const opposite = {{ top: 'bottom', bottom: 'top', left: 'right', right: 'left' }}[side];

            // Sticky once flipped true by a `reposition()` call below --
            // see this function's doc, "Scroll/resize tracking": once this
            // engine's anchor-positioning integration has been judged
            // non-conforming, every later re-measure (scroll/resize-driven
            // or not) skips the `matches()` check and unconditionally
            // re-applies the inline position, rather than re-litigating
            // whether CSS anchoring "started working" mid-overlay.
            let usingFallback = false;

            // 2026-09-02 iOS keyboard fix (see this function's doc, "iOS
            // keyboard (2026-09-02)"): visual-viewport-aware metrics.
            // `window.innerWidth`/`innerHeight` do NOT shrink when iOS
            // Safari's on-screen keyboard appears; `window.visualViewport`'s
            // `width`/`height` do -- so every fit/flip decision below reads
            // through here instead of `window.inner*` directly, and this is
            // the one thing that makes the fix keyboard-*aware* rather than
            // merely keyboard-*re-triggered* (gap 2, fixed separately below,
            // is what makes it re-run at all). `offsetLeft`/`offsetTop` is
            // the visual viewport's own origin offset from the *layout*
            // viewport's (mainly a pinch-zoom-panning concept, included for
            // completeness): `getBoundingClientRect()` always reports
            // layout-viewport-relative coordinates regardless of which
            // viewport is visually showing, and the `position: fixed` this
            // function's override sets is likewise positioned relative to
            // the layout viewport, not the visual one -- so trigger/content
            // rects are shifted into visual-viewport space by this offset
            // before comparison, and shifted back out of it when writing
            // the final inline `top`/`left`. Falls back to
            // `window.inner` width/height and `0` on an engine with no
            // `visualViewport` at all, i.e. exactly today's behavior there.
            function viewportMetrics() {{
                const vv = window.visualViewport;
                return {{
                    width: vv ? vv.width : window.innerWidth,
                    height: vv ? vv.height : window.innerHeight,
                    offsetLeft: vv ? vv.offsetLeft : 0,
                    offsetTop: vv ? vv.offsetTop : 0,
                }};
            }}

            function reposition() {{
                if (!content || !trigger) return;
                const t = trigger.getBoundingClientRect();
                const c = content.getBoundingClientRect();
                const cw = content.offsetWidth;
                const ch = content.offsetHeight;
                const vp = viewportMetrics();
                const vw = vp.width;
                const vh = vp.height;
                // See `viewportMetrics()`'s doc just above: re-express the
                // two rects `getBoundingClientRect()` gave in layout-
                // viewport coordinates as visual-viewport-relative ones, so
                // they compare like-for-like against `vw`/`vh` (also
                // visual-viewport-relative) in every formula below.
                const tTop = t.top - vp.offsetTop;
                const tBottom = t.bottom - vp.offsetTop;
                const tLeft = t.left - vp.offsetLeft;
                const tRight = t.right - vp.offsetLeft;
                const cTop = c.top - vp.offsetTop;
                const cLeft = c.left - vp.offsetLeft;

                // Same trigger-relative formula the `[data-side]`/
                // `[data-align]` CSS contract promises, parameterised over
                // *which* side is currently being placed -- called once for
                // the primary side and once for its flip-axis opposite (see
                // this function's doc, "The flip contract"). Operates
                // entirely in visual-viewport space (the `t*` locals above),
                // like everything else in this function.
                function place(effectiveSide) {{
                    let top;
                    let left;
                    if (effectiveSide === 'top') {{
                        top = tTop - gap - ch;
                    }} else if (effectiveSide === 'bottom') {{
                        top = tBottom + gap;
                    }} else if (align === 'start') {{
                        top = tTop;
                    }} else if (align === 'end') {{
                        top = tBottom - ch;
                    }} else {{
                        top = tTop + (tBottom - tTop) / 2 - ch / 2;
                    }}
                    if (effectiveSide === 'left') {{
                        left = tLeft - gap - cw;
                    }} else if (effectiveSide === 'right') {{
                        left = tRight + gap;
                    }} else if (align === 'start') {{
                        left = tLeft;
                    }} else if (align === 'end') {{
                        left = tRight - cw;
                    }} else {{
                        left = tLeft + (tRight - tLeft) / 2 - cw / 2;
                    }}
                    return {{ top, left }};
                }}

                const primary = place(side);
                const flipped = place(opposite);

                // Tolerance for rounding/subpixel noise -- see this
                // function's doc for why this checks the *outcome* rather
                // than trusting a feature-detection flag.
                function matches(pos) {{
                    return Math.abs(cTop - pos.top) <= 2 && Math.abs(cLeft - pos.left) <= 2;
                }}

                if (!usingFallback && (matches(primary) || matches(flipped))) {{
                    // A working anchor-positioning engine placed this at
                    // one of the two contract-legal spots -- its own
                    // preferred side, or the CSS `position-try-fallbacks`
                    // flip of it. Never override: doing so here would fight
                    // a legitimate flip and stomp it back off-viewport (see
                    // this function's doc). Tracking listeners are
                    // installed unconditionally below regardless of this
                    // branch (2026-09-02 fix, gap 2) -- this only decides
                    // whether *this one call* overrides, not whether future
                    // calls get a chance to.
                    return;
                }}

                // Neither matches (or a previous call already committed to
                // this branch -- see `usingFallback` above): this engine's
                // anchor-positioning integration failed outright (the
                // pre-existing case this function exists for), or CSS
                // Anchor Positioning support disappeared out from under an
                // already-open overlay (2026-09-02 fix, gap 2 -- see this
                // function's doc). Make the same flip decision from plain
                // viewport math, so a non-anchor engine gets flip parity
                // with the CSS path.
                usingFallback = true;
                let target = primary;
                if (side === 'top' && primary.top < 0) {{
                    target = flipped;
                }} else if (side === 'bottom' && primary.top + ch > vh) {{
                    target = flipped;
                }} else if (side === 'left' && primary.left < 0) {{
                    target = flipped;
                }} else if (side === 'right' && primary.left + cw > vw) {{
                    target = flipped;
                }}

                // 2026-09-03 inline shift/clamp (user device report, iOS 18
                // Safari, no CSS Anchor Positioning there at all -- see this
                // function's doc, "Inline-axis shift (2026-09-03)"): flip
                // alone only ever swaps `side` to its opposite on the *same*
                // axis (block for top/bottom, inline for left/right -- the
                // `opposite` map above). It does nothing for the *other*
                // axis's placement -- concretely, a `side="bottom"`/
                // `side="top"` placement's horizontal position comes only
                // from `align` (start/center/end, computed once against the
                // trigger in `place()` above) and never once considers the
                // viewport at all, flip or otherwise. A center-aligned
                // overlay wider than the room its trigger happens to have on
                // one side (a trigger near a viewport edge, exactly the
                // reported ColorPicker case on a narrow screen) always
                // overflowed that edge with no correction of any kind.
                // Clamping `target.left` into the viewport, with a small
                // edge margin, closes that gap the same way a `position-
                // try-fallbacks: ..., shift-inline` CSS primitive would for
                // a conforming engine (not declared in this crate's CSS
                // contract today -- see that doc section for why this stays
                // JS-fallback-only for now, matching the reported engine
                // exactly: iOS Safari has no CSS Anchor Positioning at all,
                // so it always runs this exact path). Applied after the
                // flip decision above, on whichever `target` that decision
                // already picked -- shift is a final safety net on top of
                // flip, never a replacement for it.
                const EDGE_MARGIN = 4;
                if (target.left < EDGE_MARGIN) {{
                    target = {{ top: target.top, left: EDGE_MARGIN }};
                }} else if (target.left + cw > vw - EDGE_MARGIN) {{
                    // `Math.max`, not a bare subtraction: on a viewport too
                    // narrow for the content at all (`cw` alone exceeds
                    // `vw - 2 * EDGE_MARGIN`), the two clamp branches
                    // disagree about which edge to honor -- resizing to fit
                    // is out of scope (shift only, not size, matching the
                    // existing flip-only scope note this replaces), so the
                    // left edge wins and the content simply runs past the
                    // right edge rather than past both.
                    target = {{ top: target.top, left: Math.max(EDGE_MARGIN, vw - EDGE_MARGIN - cw) }};
                }}

                content.style.position = 'fixed';
                content.style.margin = '0';
                content.style.inset = 'auto';
                content.style.transform = 'none';
                // Shift back out of visual-viewport space (see
                // `viewportMetrics()`'s doc above): `position: fixed` is
                // relative to the layout viewport.
                content.style.top = (target.top + vp.offsetTop) + 'px';
                content.style.left = (target.left + vp.offsetLeft) + 'px';
            }}

            reposition();

            // Settle loop, deferred across animation frames, *only* once the
            // synchronous call just above has already committed to the
            // fallback (`usingFallback`): confirmed by execution (this
            // session's diagnosis, `date-picker.spec.ts`'s "anchors the
            // popup" case) that the synchronous call above can run *before*
            // this content's own children have finished growing into their
            // final size -- `DatePickerPopover`'s calendar grid, populated
            // by a nested component's own reactive render, was still
            // measuring an intermediate 352x178px box (narrower-grid/
            // shorter-before-rows placeholder shape) at the exact tick this
            // effect ran, rather than its final 276x278px, throwing off
            // `place()`'s `cw`/`ch`-dependent math (here, the align="center"
            // horizontal centering) by however much the box still had left
            // to grow -- 38px off center on that box's ~76px of subsequent
            // width growth. A **content-size** race, not a scroll/layout-
            // timing one: `getBoundingClientRect()`/`offsetWidth` already
            // force a synchronous style/layout recalc (this function's doc
            // above), so the geometry read back is never stale relative to
            // the DOM as it stood at that instant -- the DOM itself just
            // wasn't finished changing shape yet. A single extra
            // `requestAnimationFrame` call was tried first and confirmed by
            // execution (console-instrumented) to still observe the
            // pre-grow 352px box even several real frames and 300ms later:
            // whatever later render pass grows the grid does not land
            // within one, or even a handful of, animation frames of this
            // effect's own -- so instead of guessing a frame count, this
            // polls `content.offsetWidth`/`offsetHeight` once per frame and
            // keeps re-running `reposition()` until two consecutive frames
            // read the *same* box size (the settle signal: nothing is still
            // resizing), capped at `MAX_SETTLE_FRAMES` frames as a backstop
            // against a pathological page that never stabilizes.
            //
            // Gated on `usingFallback` -- *not* run unconditionally -- for a
            // reason confirmed by execution the hard way (`top-layer.spec.ts`
            // Rule 8's ColorPicker case, a real regression this exact
            // unconditional shape caused first): once the first call has
            // judged CSS Anchor Positioning conforming, every extra
            // `reposition()` call in a naive unconditional loop re-runs the
            // *same* `matches()` decision from scratch, and each of those
            // re-checks is one more chance for one transient frame --
            // mid-fade-in-animation, mid this-same content-growth window,
            // anything that momentarily moves the content's painted rect
            // off the anchor-computed one by more than the 2px tolerance --
            // to be wrongly read as "the engine failed," latching
            // `usingFallback` true *permanently* (it is deliberately sticky,
            // this function's doc above) for a case that was never actually
            // broken. Only entering this loop when the first call already
            // committed to the fallback removes that risk entirely: from
            // that point on every `reposition()` call skips the `matches()`
            // check outright (`usingFallback` short-circuits the `if` below)
            // and unconditionally re-applies fresh inline geometry -- the
            // decision to accept the fallback's placement is never
            // relitigated, only its geometry is refined. A CSS-Anchor-
            // Positioning engine that placed this correctly on the first
            // call is instead left completely alone, exactly as before this
            // loop existed.
            if (usingFallback) {{
                const MAX_SETTLE_FRAMES = 30; // ~0.5s at 60fps
                let lastW = content.offsetWidth;
                let lastH = content.offsetHeight;
                let stableStreak = 0;
                let frame = 0;
                const settle = () => {{
                    frame++;
                    reposition();
                    const w = content.offsetWidth;
                    const h = content.offsetHeight;
                    stableStreak = (w === lastW && h === lastH) ? stableStreak + 1 : 0;
                    lastW = w;
                    lastH = h;
                    if (stableStreak < 2 && frame < MAX_SETTLE_FRAMES) {{
                        requestAnimationFrame(settle);
                    }}
                }};
                requestAnimationFrame(settle);
            }}

            // 2026-09-02 iOS keyboard fix, gap 2 -- see this function's doc,
            // "iOS keyboard (2026-09-02)": tracking listeners now install
            // UNCONDITIONALLY, for the life of every open overlay, not only
            // once `usingFallback` is already `true` at this point in the
            // script. Before this fix, an overlay that CSS Anchor
            // Positioning placed correctly at open time got no listener at
            // all -- reasoned at the time (see the removed comment this
            // replaces, preserved in this function's git history) as "an
            // engine that gets here already re-resolves `anchor()` on
            // scroll for free" -- which is true for scroll, but silently
            // assumed the *set of contract-legal positions* itself never
            // changes for the rest of that open. An iOS Safari on-screen
            // keyboard breaks exactly that assumption mid-open (shrinking
            // `visualViewport` out from under an already-placed overlay),
            // and Combobox is the one anchored overlay whose opening
            // reliably summons a keyboard (a focused text input) --
            // consistent with only it being reported. `reposition()` itself
            // still only overrides when `matches()` fails (or `usingFallback`
            // is already sticky-true), so a still-conforming CSS-anchor
            // engine re-running `reposition()` on every scroll/resize costs
            // one extra `getBoundingClientRect()` pair and returns
            // immediately -- not a behavior change for the common case,
            // only for the one this fix exists to close.
            //
            // `scroll` is added on `window` with `capture: true` because
            // the event does not bubble -- capture on `window` is the only
            // way to observe a scroll on a nested scroll container, not
            // just the document itself. `visualViewport`'s own `resize`
            // (fires when the keyboard opens/closes, or on pinch-zoom) and
            // `scroll` (fires when the visual viewport pans, e.g. while a
            // focused input is scrolled into view above the keyboard) are
            // added too, when present, alongside the existing `window`
            // listeners -- `window`'s own `resize` does not fire for a
            // visual-viewport-only change, since `window.inner` width/
            // height do not change when the keyboard appears (this
            // function's doc, gap 1). rAF-throttled: `rafScheduled`
            // collapses any number of scroll/resize events firing before
            // the next paint (from either source) into a single
            // `reposition()` call. Removed on close/unmount exactly as
            // every other eval-channel listener in this file -- the
            // `await dioxus.recv()` tail below is unconditional now for the
            // same reason: something always needs cleaning up.
            let rafScheduled = false;
            const onTrack = () => {{
                if (rafScheduled) return;
                rafScheduled = true;
                requestAnimationFrame(() => {{
                    rafScheduled = false;
                    reposition();
                }});
            }};
            window.addEventListener('scroll', onTrack, {{ capture: true, passive: true }});
            window.addEventListener('resize', onTrack, {{ passive: true }});
            const vv = window.visualViewport;
            if (vv) {{
                vv.addEventListener('resize', onTrack, {{ passive: true }});
                vv.addEventListener('scroll', onTrack, {{ passive: true }});
            }}
            await dioxus.recv();
            window.removeEventListener('scroll', onTrack, true);
            window.removeEventListener('resize', onTrack);
            if (vv) {{
                vv.removeEventListener('resize', onTrack);
                vv.removeEventListener('scroll', onTrack);
            }}
            "#
        ));
        Box::new(move || {
            let _ = eval.send(true);
        })
    });
}

// No native (Blitz) counterpart: every call site
// (`tooltip.rs`/`hover_card.rs`/`popover.rs`) is itself inside a
// `#[cfg(feature = "web")]`-only component -- the modal/non-modal
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
#[cfg(feature = "web")]
pub(crate) fn anchor_name_style(id: &str) -> String {
    format!("anchor-name: --dxa-{id};")
}

/// See [`anchor_name_style`]. Set on the content element, referencing the
/// same id the trigger's `anchor_name_style` was built from. Only called
/// from the web-arm leaf render functions (the native arm never promotes
/// content to the top layer, so it has no containing-block problem to
/// solve), so -- like [`PopoverKind`] -- this has no native counterpart.
#[cfg(feature = "web")]
pub(crate) fn position_anchor_style(id: &str) -> String {
    format!("position-anchor: --dxa-{id};")
}

/// No-op whenever this crate's `web` feature is off -- see
/// [`anchor_name_style`]'s doc. Unlike [`position_anchor_style`], this one
/// *is* called unconditionally (every trigger sets it, regardless of
/// build), so it needs a real native stub rather than being `#[cfg]`-gated
/// away entirely.
#[cfg(not(feature = "web"))]
pub(crate) fn anchor_name_style(_id: &str) -> String {
    String::new()
}
