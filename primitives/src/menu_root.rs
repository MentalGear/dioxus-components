//! Shared, type-independent plumbing genuinely identical across the three
//! independent top-level "menu button"-shaped hosts -- [`crate::dropdown_menu`],
//! [`crate::context_menu`], and [`crate::menubar`] -- factored out the same
//! way [`crate::menu_sub`] factors the nested-submenu state machine those
//! same three files' own `*Sub` components share. See that module's own doc
//! for the general shape this one follows: identical plumbing lives here,
//! everything host-private (including anything this module's own doc below
//! explicitly declines to generalize) stays exactly where it was.
//!
//! # Scope: `docs/backlog.md` row 53
//!
//! Row 53's own assessment of whether `Navbar` and `Menubar` should go
//! further and share one submenu primitive concluded **extract three narrow
//! bands, do not build one shared component**: the keyboard state machine
//! (`MenubarMenu` concentrates its whole contract in one handler, while
//! `Navbar` splits the identical contract three ways) and the item component
//! (`NavbarItem`'s routing-aware `Link` with an 8+-prop surface vs.
//! `MenubarItem`'s plain interactive `div`) both stay host-private on
//! purpose -- collapsing either would force one side's shape onto the other
//! for no benefit anyone asked for. `Navbar` itself is also **not** a
//! consumer of this module at all: row 53 scoped the extraction to
//! `DropdownMenu`, `ContextMenu`, and `Menubar` only. This module is exactly
//! those three narrow bands and nothing more:
//!
//! - [`use_open_focus_sync`] -- the "open-index container": the effect that
//!   keeps a menu's own `open` boolean in sync with whether its roving-focus
//!   collection currently holds DOM focus, gated on no descendant submenu
//!   being open. Before this extraction this exact effect -- down to the
//!   `.peek()` choice on both reads, so the effect's own reactive dependency
//!   stays `focus.any_focused()` alone -- was duplicated word for word
//!   between `DropdownMenu`'s and `ContextMenu`'s root components. **Not**
//!   used by `Menubar`: that host's analogous effect resyncs a
//!   *which-index-is-currently-open* signal (`Signal<Option<usize>>`), not a
//!   plain boolean, and has no `submenu_open_count` concept at all (`Menubar`
//!   doesn't support `DropdownMenuSub`-style nested submenus) -- forcing the
//!   two shapes into one generic function would either lose the index
//!   `Menubar` needs or invent a translation layer with no existing
//!   duplication to justify it. `Menubar`'s own version stays exactly where
//!   it was, in `Menubar` itself.
//! - [`use_menu_content_lifecycle`] -- the "content lifecycle/positioning
//!   pipeline": id resolution (a caller-overridable id via `use_id_or`) plus
//!   `use_animated_open`'s open/close-hold render gate, identical across all
//!   three hosts' `*Content` components before this extraction. The
//!   *positioning* half of the name deliberately stays host-private, for the
//!   exact reason `crate::menu_sub`'s own doc gives for its own submenu
//!   content: `top_layer.rs` hard-codes a fixed list of `dx-anchor-*` marker
//!   classes for its shared stylesheet, so the actual
//!   `use_anchor_position_fallback`/`use_point_anchor_clamp` call still has
//!   to live in each file's own `#[cfg(feature = "web")]` leaf render
//!   function -- this module only factors the lifecycle gate that runs
//!   *before* that host-private positioning call, never the call itself.
//! - [`content_labelledby_attributes`] -- the "trigger id-plumbing": an APG
//!   menu needs an accessible name (`docs/backlog.md` row 25), supplied by
//!   `aria-labelledby` pointing at the menu's own trigger id unless the
//!   caller already named the content some other way
//!   (`has_own_accessible_name`). Before this extraction this exact
//!   attribute-building block was duplicated across all six
//!   `*ContentRendered` web/native arm pairs (three hosts x two arms).
//!
//! Each function takes only `Copy` context-field *values* (a `Memo`, a
//! `Callback`, a `CollectionState`, a `Signal`, a plain `&str`) rather than a
//! whole context struct -- the same reason `crate::menu_sub`'s doc gives for
//! leaving each host's own private root-context *lookup* file-local: Rust's
//! module privacy has no way to let this module see
//! `DropdownMenuContext`/`ContextMenuCtx`/`MenubarMenuContext` without
//! making them `pub(crate)`, and doing that would widen those files' API
//! surface for no reader benefit. Passing the already-resolved field values
//! sidesteps the question entirely, and it is also why this module cannot
//! (and does not try to) provide a single bundled "root context" struct the
//! way [`crate::menu_sub::SubMenuState`] does for one level of submenu
//! nesting -- the three hosts' root shapes diverge in exactly the ways the
//! three bands above describe, so bundling them into one struct would either
//! paper over that divergence or carry dead fields for whichever host
//! doesn't need them.

use dioxus::prelude::*;
use dioxus_attributes::attributes;

use crate::collection::CollectionState;
use crate::has_own_accessible_name;

/// The "open-index container" band -- see this module's doc for exactly
/// what it does and does not subsume. Keeps `open` in sync with whether
/// `focus`'s roving-focus collection currently holds DOM focus, skipped
/// while `submenu_open_count` is nonzero -- see
/// `crate::dropdown_menu`'s `DropdownMenuContext::submenu_open_count` doc
/// for why a submenu's own item legitimately holding focus must not read as
/// "focus left the whole menu" here.
pub(crate) fn use_open_focus_sync(
    focus: CollectionState,
    open: Memo<bool>,
    set_open: Callback<bool>,
    submenu_open_count: Signal<usize>,
) {
    use_effect(move || {
        let focused = focus.any_focused();
        if *open.peek() != focused && *submenu_open_count.peek() == 0 {
            set_open.call(focused);
        }
    });
}

/// The "content lifecycle/positioning pipeline" band -- see this module's
/// doc for exactly what it does and does not subsume. Resolves this
/// content's own (possibly caller-overridden) element id, then gates its
/// mount/unmount on `open` through `use_animated_open`'s close-animation
/// hold. Returns `(id, render)`; `render()` is what every call site's own
/// `if render() { ...ContentRendered { ... } }` branch keys off, matching
/// every one of the three hosts' `*Content` components unchanged.
///
/// `open` is generic over anything `Readable<Target = bool>` (every current
/// call site passes a plain `Memo<bool>`) so this works unchanged whether a
/// host's own `open` is the menu's own controlled state
/// (`DropdownMenuContent`/`ContextMenuContent`) or a derived per-menu memo
/// (`MenubarContent`'s `menu_ctx.is_open`).
pub(crate) fn use_menu_content_lifecycle(
    id_prop: ReadSignal<Option<String>>,
    open: impl Readable<Target = bool> + Copy + 'static,
) -> (Memo<String>, impl Fn() -> bool + Copy) {
    let unique_id = crate::use_unique_id();
    let id = crate::use_id_or(unique_id, id_prop);
    let render = crate::use_animated_open(id, open);
    (id, render)
}

/// The "trigger id-plumbing" band -- see this module's doc for exactly what
/// it does and does not subsume. An APG menu needs an accessible name
/// (`docs/backlog.md` row 25) -- labelled by its own trigger's id, unless
/// the caller already named the content some other way. Returns the
/// `aria-labelledby` attribute as its own `merge_attributes` input (present
/// only when applicable, an empty `Vec` otherwise), never a bare literal
/// alongside `..attributes`: a caller attribute list can carry a same-named
/// `aria-labelledby`/`aria-label` with an empty/`AttributeValue::None`
/// value, and two entries for one attribute name is exactly the
/// duplicate-attribute hazard `merge_attributes` exists to prevent
/// (`docs/conformance-harness.md` hydration-parity Rule 4; see
/// `has_own_accessible_name`'s own doc).
pub(crate) fn content_labelledby_attributes(
    attributes: &[Attribute],
    trigger_id: &str,
) -> Vec<Attribute> {
    if has_own_accessible_name(attributes) {
        Vec::new()
    } else {
        attributes!(div {
            aria_labelledby: "{trigger_id}"
        })
    }
}
