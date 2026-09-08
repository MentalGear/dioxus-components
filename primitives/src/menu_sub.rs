//! Shared, type-independent plumbing for the APG "Menu and Menubar"
//! pattern's nested-submenu contract (`DropdownMenu.Sub`/`ContextMenu.Sub`),
//! used by both [`crate::dropdown_menu`] and [`crate::context_menu`].
//!
//! # Why this module exists, and what deliberately stays out of it
//!
//! `dropdown_menu.rs` and `context_menu.rs` each already have their own
//! private root context struct (`DropdownMenuContext`, `ContextMenuCtx`)
//! that a `*SubTrigger`/`*SubItem` needs to read to register itself in the
//! *enclosing* menu's item collection (`ctx.focus`) -- that lookup is
//! necessarily file-local (Rust's module privacy has no way to let this
//! module see either private struct without making them `pub(crate)`, and
//! doing that would widen those files' API surface for no reader benefit).
//! Likewise, how the outermost content of each host is positioned differs:
//! `DropdownMenuContent` anchors to its trigger; `ContextMenuContent`
//! doesn't anchor at all (it's pinned to a raw click point, `position:
//! fixed; left/top`, see that file's own doc). A `*Sub`'s own content,
//! though, anchors to its trigger the same way *either* host's top-level
//! content does -- so that positioning call is identical for both and does
//! belong here in spirit, but the actual `use_anchor_position_fallback`
//! call still has to live in each file's own `#[cfg(feature = "web")]`
//! leaf render function, because `top_layer.rs` (out of scope for this
//! lane -- another lane owns it) hard-codes a fixed list of `dx-anchor-*`
//! marker classes for its shared stylesheet, and both hosts' submenus reuse
//! the existing `dx-anchor-dropdown-menu` entry rather than a class that
//! doesn't exist in that list -- see each `*SubContentRendered`'s own doc
//! for the full reasoning.
//!
//! What *is* identical for both hosts, and lives here:
//! - [`SubMenuState`]: the open/close state, own roving-focus collection,
//!   and trigger/content id pair every `Sub` needs -- mirrors
//!   `DropdownMenuContext`'s own shape at one level of nesting (see that
//!   struct's doc in `dropdown_menu.rs`), with the "which enclosing
//!   collection do I register in" question left to each caller, since only
//!   the caller knows which private root context type answers it.
//! - The hover-intent timer every `*SubTrigger` needs to open on hover, and
//!   every `*SubContent`/`*SubTrigger` needs to close on leaving without
//!   entering the other. This crate's only prior art for a timed
//!   pointer-driven open/close is `context_menu.rs`'s long-press timer
//!   (`LONG_PRESS_DURATION`, `dioxus_sdk_time::sleep` + `spawn`/`Task`) --
//!   [`DelayedAction`] is that same primitive, factored out so it isn't
//!   reinvented a third and fourth time for `DropdownMenuSubTrigger`/
//!   `ContextMenuSubTrigger`.
//!
//! # Scope: one level of nesting
//!
//! A `Sub`'s own content may hold plain items (`DropdownMenuSubItem`/
//! `ContextMenuSubItem`) but not a further nested `Sub` of its own.
//! `DropdownMenuSubTrigger`/`ContextMenuSubTrigger` register in the
//! *root* menu's collection by reading that host's root context type
//! directly (`use_context::<DropdownMenuContext>()`, unconditionally the
//! root, never a `SubMenuState`) -- correct only because there is never
//! more than one level between a sub-trigger and the root. Supporting a
//! `Sub` inside a `Sub` would need that lookup to walk however many
//! `SubMenuState` levels enclose it before finding the true root, which
//! this module's single `SubMenuState` type (identical at every depth,
//! indistinguishable from its neighbours by type alone) cannot resolve on
//! its own. `docs/component-backlog.md` row 68 budgets nested submenus as
//! "L" effort against a `molikto` patch that is "shape only, not
//! cherry-pickable" for exactly this one level -- recursive `Sub`-in-`Sub`
//! is not part of that budget, and is left out here rather than half-built
//! against a type that can't actually support it. See this crate's
//! `menubar.rs` for the precedent: `MenubarMenu` also stops at one level
//! (no nested `MenubarMenu` inside a `MenubarContent`) for the same
//! practical reason.

use std::time::Duration;

use dioxus::prelude::*;
use dioxus_core::Task;
use dioxus_sdk_time::sleep;

use crate::collection::{use_collection_provider, CollectionPlacement, CollectionState};
use crate::use_unique_id;

/// Hover-intent delay before a pointer hovering a sub-trigger opens its
/// submenu.
///
/// APG's Menu and Menubar pattern documents ArrowRight/Enter/Space as the
/// *keyboard* way to open a submenu
/// (`content/patterns/menubar/menu-and-menubar-pattern.html`, "Keyboard
/// Interaction" (h2), pinned commit
/// `7e4034b262bc0d25332e330d8a582aaf34113829` of `w3c/aria-practices` --
/// see `playwright/oracle/reference/README.md`) and is silent on pointer
/// hover entirely -- opening on hover is a widely-implemented UX
/// convention (Radix, native OS menus), not an APG normative requirement,
/// so this delay is this crate's own choice, not a citation (see
/// `docs/component-backlog.md` row 68's "budget as fresh design work
/// against Radix `Sub`/`Portal` semantics"). A short delay, not an instant
/// open, keeps a pointer merely passing over the trigger on its way
/// elsewhere from flashing open every submenu it crosses -- the exact
/// defect an instant hover-open produces.
pub(crate) const SUBMENU_OPEN_INTENT_DELAY: Duration = Duration::from_millis(200);

/// Grace delay before a pointer leaving a sub-trigger (without entering its
/// own submenu content) closes it.
///
/// Without this, the physical gap between a trigger and its anchored
/// content -- however small; see each `*SubContentRendered`'s zero-gap
/// anchor choice, which exists specifically to shrink this window -- would
/// make an ordinary diagonal mouse move from the trigger toward its own
/// submenu register as "the pointer left," closing the submenu out from
/// under the pointer before it ever arrives. Same non-normative status as
/// [`SUBMENU_OPEN_INTENT_DELAY`] above.
pub(crate) const SUBMENU_CLOSE_GRACE_DELAY: Duration = Duration::from_millis(200);

/// A single cancellable delayed action, backed by `dioxus_sdk_time::sleep`
/// + `spawn`/`Task` -- the same primitive `ContextMenuTrigger`'s long-press
/// timer already uses (`context_menu.rs`'s `long_press_task`/
/// `LONG_PRESS_DURATION`), not a new mechanism. Scheduling a new action
/// implicitly cancels whatever this handle was already waiting on, so a
/// trigger can call [`Self::schedule`] on every `onmouseenter` without
/// separately tracking whether a previous timer is still pending.
#[derive(Clone, Copy)]
pub(crate) struct DelayedAction {
    task: Signal<Option<Task>>,
}

/// Create a fresh, unscheduled [`DelayedAction`]. A hook (calls
/// `use_signal`), so -- like every other hook in this crate -- call it
/// unconditionally, once per render.
pub(crate) fn use_delayed_action() -> DelayedAction {
    DelayedAction {
        task: use_signal(|| None),
    }
}

impl DelayedAction {
    /// Cancel a pending action, if any. A no-op if nothing is scheduled.
    pub(crate) fn cancel(&mut self) {
        if let Some(task) = self.task.write().take() {
            task.cancel();
        }
    }

    /// Schedule `action` to run after `delay`, replacing (cancelling)
    /// whatever this handle was already waiting on.
    pub(crate) fn schedule(&mut self, delay: Duration, action: impl FnOnce() + 'static) {
        self.cancel();
        let mut task_slot = self.task;
        let task = spawn(async move {
            sleep(delay).await;
            task_slot.set(None);
            action();
        });
        self.task.set(Some(task));
    }
}

/// The state a `DropdownMenuSub`/`ContextMenuSub` needs, identical for
/// both hosts (see this module's doc for what deliberately stays out of
/// it). Mirrors `DropdownMenuContext`'s own shape (`dropdown_menu.rs`) at
/// one level of nesting: an open/close pair, this submenu's own
/// roving-focus collection (never the enclosing menu's -- keeping them
/// separate is what lets `DropdownMenuSubItem`'s roving `tabindex` stay
/// scoped to just this submenu's items, the same way `MenubarMenu` gives
/// each menu its own `CollectionState` distinct from `Menubar`'s row of
/// triggers), `initial_focus` for the same keyboard-open contract
/// `DropdownMenuContext::open_with_focus` documents, and the trigger/
/// content id pair `aria-controls`/`aria-labelledby`/`anchor-name` key off.
#[derive(Clone, Copy)]
pub(crate) struct SubMenuState {
    pub(crate) open: Memo<bool>,
    pub(crate) set_open: Callback<bool>,
    /// This submenu's own item collection -- registered into by
    /// `*SubItem`, navigated by `*SubContentRendered`'s own
    /// ArrowDown/ArrowUp/Home/End. Never the enclosing menu's collection.
    pub(crate) focus: CollectionState,
    /// Where focus should land once this submenu's content next mounts.
    /// Every opening path (ArrowRight/Enter/Space, or a pointer click) goes
    /// through [`SubMenuState::open_with_focus`], which sets this *before*
    /// flipping `open` -- never the reverse. Hover-intent opens do **not**
    /// go through this: opening on hover must not steal keyboard focus, so
    /// a hover-driven open calls `set_open` directly and leaves this
    /// signal untouched.
    pub(crate) initial_focus: Signal<Option<CollectionPlacement>>,
    /// This submenu's own trigger element id -- `*SubContent` labels
    /// itself `aria-labelledby` from it (APG menu accessible-name
    /// requirement, `docs/backlog.md` row 25's construction, reused here
    /// rather than reintroduced ad hoc), and Escape/ArrowLeft return focus
    /// here.
    pub(crate) trigger_id: Signal<String>,
    /// This submenu's own content element id, kept in sync by
    /// `*SubContent` -- mirrors `DropdownMenuContext::content_id`'s
    /// identical role and the exact bug its doc warns about if trigger and
    /// content ever named different ids (`*SubTrigger`'s `anchor-name` and
    /// `aria-controls` must both key off *this* signal, not
    /// `trigger_id` above).
    pub(crate) content_id: Signal<String>,
}

impl SubMenuState {
    /// The single path every keyboard/pointer-click open goes through:
    /// request `target` as the focus placement once this submenu's content
    /// mounts, then open. Never the other order -- see
    /// `DropdownMenuContext::open_with_focus`'s identical contract
    /// (`dropdown_menu.rs`) for why setting `open` first would leave a
    /// window with no focus request recorded. Deliberately *not* used by a
    /// hover-intent open -- see [`SubMenuState::initial_focus`]'s doc.
    pub(crate) fn open_with_focus(&mut self, target: CollectionPlacement) {
        self.initial_focus.set(Some(target));
        self.set_open.call(true);
    }
}

/// Build a fresh [`SubMenuState`] for one `Sub`. `open`/`set_open` are
/// already resolved by the caller's own `use_controlled` over its own
/// `*SubProps` -- a type this module never needs to know, matching the
/// pattern already set by every other `use_controlled` call site in this
/// crate's menu family.
pub(crate) fn use_sub_menu_state(
    open: Memo<bool>,
    set_open: Callback<bool>,
    roving_loop: ReadSignal<bool>,
) -> SubMenuState {
    SubMenuState {
        open,
        set_open,
        focus: use_collection_provider(roving_loop),
        initial_focus: use_signal(|| None),
        trigger_id: use_unique_id(),
        content_id: use_unique_id(),
    }
}
