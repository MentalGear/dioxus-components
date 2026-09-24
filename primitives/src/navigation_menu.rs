//! Defines the [`NavigationMenu`] component and its sub-components.
//!
//! # Why this is a new primitive, not a `Navbar` reskin
//!
//! [`crate::navbar`] implements the APG **Menu and Menubar** pattern:
//! `role="menubar"`/`role="menu"`/`role="menuitem"` (via
//! `crate::menu_semantics`), roving `tabindex`, and the full
//! ArrowLeft/Right/Up/Down/Home/End menu keyboard contract -- and this
//! crate's docs site depends on that exact contract
//! (`playwright/navbar.spec.ts`,
//! `playwright/oracle/tier1-apg/keyboard-matrix.spec.ts`). Radix/shadcn's
//! `NavigationMenu` is deliberately a *different* APG pattern: **Disclosure
//! (Show/Hide) Navigation** -- a `nav` of plain links and
//! `button[aria-expanded][aria-controls]` disclosure triggers, with **no
//! menu role anywhere** (`content/patterns/disclosure/disclosure-pattern.html`,
//! its "WAI-ARIA Roles, States, and Properties" section: only `button`,
//! `aria-expanded`, optionally `aria-controls` -- no `aria-haspopup`, no
//! `role="menu"`; `content/patterns/disclosure/examples/
//! disclosure-navigation.html`'s own "Important" advisement is explicit:
//! "this implementation of site navigation does not use the menu role
//! because it does not provide the complex functionality that assistive
//! technologies expect in a widget that has the menu role. Typical site
//! navigation does not need all the keyboard interactions specified by the
//! menu and menubar pattern." `disclosure-navigation-hybrid.html` is the
//! same pattern with top-level links alongside the disclosure buttons,
//! cited below for its own optional-arrow-key keyboard table, which this
//! module also implements).
//!
//! A different ARIA contract is a real reason for a distinct primitive,
//! not a duplicate: this is **not** the duplication
//! `crate::menu_root`'s own module doc describes for row 53's
//! `DropdownMenu`/`ContextMenu`/`Menubar` consolidation (three hosts
//! implementing the *same* Menu-and-Menubar pattern, with identical
//! plumbing literally copy-pasted between them). `Navbar` and
//! `NavigationMenu` implement two *different, APG-distinguished* patterns
//! on purpose -- forcing them onto one shared component would either
//! smuggle menu semantics into a widget the pattern explicitly forbids
//! them from, or strip Navbar's real menubar contract out from under the
//! specs that already pin it. `navbar.rs` is still the right file to read
//! for this crate's house idioms (context struct shape, `use_id_or`,
//! `merge_attributes`, `attributes!`, `use_animated_open`, the
//! `#[cfg(feature = "web")]` `*ContentRendered` leaf split via
//! `crate::top_layer`), and this module mirrors those idioms closely --
//! just never `crate::menu_semantics`, and never a roving-tabindex
//! collection for keyboard focus (see below).
//!
//! # Tab order: native, not roving
//!
//! The disclosure-navigation example's own keyboard table is explicit that
//! Tab/Shift+Tab move through *every* top-level button/link and, once a
//! panel is open, through its links too, "in the page tab sequence" --
//! unlike a menu/menubar, "the buttons and links are not contained by an
//! element with a widget role ... that is expected to occupy only one stop
//! in the page tab sequence and manage focus for all its descendants."
//! So every [`NavigationMenuTrigger`]/[`NavigationMenuLink`] keeps its
//! element's native default `tabindex` (never `-1`) -- this crate's
//! existing precedent for "use `crate::collection`'s ordering purely to
//! move real DOM focus on an explicit key press, never to drive a roving
//! `tabindex`" is [`crate::accordion::AccordionTrigger`], which already
//! ignores its own collection item's `tabindex` and hard-codes `"0"`
//! instead; this module does the same.
//!
//! Left/Right between top-level items, Home/End to the first/last top-level
//! item, and Up/Down within an open panel's own list of links are all
//! optional in APG and all implemented here
//! (`disclosure-navigation-hybrid.html`'s own keyboard table includes every
//! one of them) -- even though Tab alone already reaches every one of
//! those same targets, exactly as this section's own citation above
//! describes. Not implemented: Home/End *within* an open panel (the
//! reference table's own Home/End rows cover this too, but this round's
//! own brief scoped Home/End to top-level items only) and treating
//! ArrowUp/ArrowDown on a top-level item as aliases for Left/Right (the
//! reference table conflates them one row apiece; kept separate here so
//! ArrowDown on a top-level trigger unambiguously means "open/enter this
//! panel," never "also move to the next item" -- see
//! [`NavigationMenuTrigger`]'s own doc for that choice). Both are cheap,
//! cited, backward-compatible follow-ups for a future round or a user
//! report, not implemented this round.
//!
//! # Hover intent and the pointer contract
//!
//! Opening on pointer hover (after a short delay) is, like `crate::menu_sub`'s
//! submenu hover-intent, a widely-implemented convention (Radix, native OS
//! menus) rather than an APG requirement -- this module's own choice of
//! delay, not a citation. `crate::menu_sub::DelayedAction`/
//! `crate::menu_sub::use_delayed_action` (already `pub(crate)`, already
//! used by `DropdownMenuSub`/`ContextMenuSub`'s identical
//! hover-open/hover-close-grace timers) is reused as-is rather than
//! reinvented a third time.
//!
//! # Detecting "focus left the nav" without `relatedTarget`
//!
//! The disclosure-navigation example's own accessibility-features note
//! ("Moving focus out of the navigation region also closes an open
//! dropdown ... necessary to meet WCAG 2.1 1.4.13") is implemented in its
//! reference JS (`js/disclosureMenu.js`'s `onBlur`) by reading
//! `event.relatedTarget` and checking `rootNode.contains(...)`. Confirmed
//! by reading `dioxus-html` 0.7.9's own event source
//! (`dioxus-html-0.7.9/src/events/focus.rs`): this framework version's
//! `FocusData` exposes no `related_target` at all, so that check is not
//! available here -- consistent with no existing component in this crate
//! reading it either (`rg relatedTarget primitives/src` finds nothing).
//! Every existing multi-item collection in this crate (`Navbar`,
//! `Accordion`, `Toolbar`, `ToggleGroup`, `Tabs`) sidesteps the same gap
//! with a *roving*-`tabindex` invariant: only one item is ever a real Tab
//! stop, so any blur it doesn't itself cause is, by construction, focus
//! leaving the widget. That invariant does not hold here (every item is
//! independently tabbable, on purpose -- see above), so this module
//! instead debounces: every focusable element's `onblur` schedules a
//! `DelayedAction` that closes the open panel, and every focusable
//! element's `onfocus` cancels it. A real browser dispatches a moving
//! focus's `blur`/`focusout` on the old element and `focus`/`focusin` on
//! the new one back-to-back in the same task, so a Tab/click that lands on
//! *any* other element this module wires (a sibling trigger, a sibling
//! link, or a link inside the panel that's opening) cancels the scheduled
//! close before `FOCUS_LEAVE_CLOSE_DELAY` elapses; a Tab/click that
//! leaves the whole `nav` has nothing left to cancel it, so the close goes
//! through.
//!
//! # Top layer
//!
//! [`NavigationMenuContent`]'s web-arm leaf promotes to the top layer
//! broadly the way `NavbarContent` does --
//! `crate::top_layer::anchored_content_attributes` with a
//! `dx-anchor-navigation-menu` marker class (added next to every existing
//! `dx-anchor-navbar` entry in `top_layer.rs`'s shared anchor-positioning
//! stylesheet) + `crate::top_layer::use_anchor_position_fallback` -- except
//! `popover="manual"`, not `"auto"`: this module already owns its entire
//! open/close lifecycle (hover-intent timers, the focus-leave debounce
//! above, Escape, click), the same reasoning `crate::tooltip`/
//! `crate::hover_card` document for `Manual` (`top_layer.rs`'s
//! `PopoverKind::Manual` doc) -- native `auto` light-dismiss firing
//! instantly on outside focus/click would fight this module's own delayed,
//! debounced open/close instead of the two ever agreeing. Unlike
//! `NavbarContent`, this content's own popover lifecycle sync is
//! `crate::top_layer::use_popover_shown_while_mounted`, not
//! `crate::top_layer::use_popover_sync` -- see
//! `NavigationMenuContentRendered`'s own doc for why (this content, like
//! `NavbarContent`, renders through `use_animated_open` and needs its exit
//! animation not to be cut short by an instant `hidePopover()`).

use std::time::Duration;

use dioxus::prelude::*;
use dioxus_attributes::attributes;

use crate::collection::{
    collection_item, use_collection_provider, use_item, CollectionOptions, CollectionState,
};
use crate::menu_sub::{use_delayed_action, DelayedAction};
use crate::{
    has_own_accessible_name, merge_attributes, use_animated_open, use_id_or, use_unique_id,
};

/// Hover-intent delay before a pointer hovering a trigger opens its panel.
/// Numerically the same as this theme's `--dx-motion-duration-base` token
/// (`preview/assets/dx-components-theme.css`) -- not read from it (this is
/// a plain Rust constant, not CSS), but deliberately kept in step with it.
const HOVER_OPEN_INTENT_DELAY: Duration = Duration::from_millis(150);

/// Grace delay before a pointer leaving a trigger or its content (without
/// entering the other) closes the panel. Numerically the same as this
/// theme's `--dx-motion-duration-slower` token, for the same reason as
/// [`HOVER_OPEN_INTENT_DELAY`].
const HOVER_CLOSE_GRACE_DELAY: Duration = Duration::from_millis(300);

/// Debounce before a blur that nothing inside the nav cancels closes the
/// open panel -- see this module's doc, "Detecting 'focus left the nav'
/// without `relatedTarget`". Deliberately tiny but nonzero, so it is never
/// perceptible as a delay yet still gives a same-task compensating
/// `onfocus` elsewhere in the nav a chance to cancel it first.
const FOCUS_LEAVE_CLOSE_DELAY: Duration = Duration::from_millis(1);

#[derive(Clone, Copy)]
struct NavigationMenuContext {
    /// The currently open item's index, if any. One open at a time, the
    /// same shape as `NavbarContext::open_nav`.
    open: Signal<Option<usize>>,
    set_open: Callback<Option<usize>>,
    disabled: ReadSignal<bool>,
    /// Top-level items only (every [`NavigationMenuTrigger`] and every
    /// top-level [`NavigationMenuLink`]), keyed by [`NavigationMenuItem`]'s
    /// own `index` -- Left/Right's move target and the mount-ref registry
    /// Escape uses to return focus to a trigger. Never drives `tabindex`
    /// (see this module's doc).
    focus: CollectionState,
    /// Hover-intent timers. One pending action of each kind is enough: a
    /// pointer can only be hovering one element at a time.
    hover_open: DelayedAction,
    hover_close: DelayedAction,
    /// The "focus left the nav" debounce -- see this module's doc.
    blur_close: DelayedAction,
}

/// The props for the [`NavigationMenu`] component.
#[derive(Props, Clone, PartialEq)]
pub struct NavigationMenuProps {
    /// Whether the whole navigation menu is disabled.
    #[props(default)]
    pub disabled: ReadSignal<bool>,

    /// Additional attributes to apply to the root `nav` element. An
    /// accessible name (`aria_label`/`aria_labelledby`) is required by the
    /// disclosure-navigation pattern's own accessibility-features note --
    /// pass one of these.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
    /// The children of the navigation menu -- typically a
    /// [`NavigationMenuList`] of [`NavigationMenuItem`]s.
    pub children: Element,
}

/// # NavigationMenu
///
/// The root of a disclosure-pattern site navigation: a `nav` landmark
/// containing top-level links and disclosure triggers whose panels expand
/// below them. See this module's doc for why this is a distinct primitive
/// from [`crate::navbar::Navbar`] (a different APG pattern, not a
/// duplicate).
///
/// ## Example
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::navigation_menu::{
///     NavigationMenu, NavigationMenuContent, NavigationMenuItem, NavigationMenuLink,
///     NavigationMenuList, NavigationMenuTrigger,
/// };
///
/// #[component]
/// fn Demo() -> Element {
///     rsx! {
///         NavigationMenu { aria_label: "Main",
///             NavigationMenuList {
///                 NavigationMenuItem { index: 0usize,
///                     NavigationMenuTrigger { "Components" }
///                     NavigationMenuContent {
///                         NavigationMenuLink { href: "/calendar", "Calendar" }
///                         NavigationMenuLink { href: "/slider", "Slider" }
///                     }
///                 }
///                 NavigationMenuItem { index: 1usize,
///                     NavigationMenuLink { href: "/docs", "Docs" }
///                 }
///             }
///         }
///     }
/// }
/// ```
///
/// ## Styling
///
/// The [`NavigationMenu`] component defines the following data attributes
/// you can use to control styling:
/// - `data-disabled`: Indicates if the navigation menu is disabled. Values
///   are `true` or `false`.
#[component]
pub fn NavigationMenu(props: NavigationMenuProps) -> Element {
    let mut open = use_signal(|| None);
    let set_open = use_callback(move |idx| open.set(idx));

    // Non-looping (matches the reference implementation's clamped
    // `Math.max`/`Math.min` indices, `js/disclosureMenu.js`'s
    // `controlFocusByKey` -- Left/Right stop at the first/last item rather
    // than wrapping).
    let focus = use_collection_provider(ReadSignal::new(Signal::new(false)));

    // Each `use_delayed_action()` call is itself a hook (it wraps
    // `use_signal`, `menu_sub.rs`), so -- like `open`/`focus` above -- it
    // must run directly in this component's own body, never nested inside
    // another hook's closure. Calling it inline inside
    // `use_context_provider`'s initializer below produced a `BorrowMutError`
    // ("The hook list is already borrowed ... hook inside a hook") on every
    // real render, confirmed by live reproduction against a running dev
    // server (the wasm panic never surfaced in `cargo check`/`clippy`/
    // `cargo test`, since Dioxus's Rules of Hooks are enforced at runtime,
    // not by the type system).
    let hover_open = use_delayed_action();
    let hover_close = use_delayed_action();
    let blur_close = use_delayed_action();

    use_context_provider(|| NavigationMenuContext {
        open,
        set_open,
        disabled: props.disabled,
        focus,
        hover_open,
        hover_close,
        blur_close,
    });

    // Owned by this component -- disabled state must win over a caller's
    // own attributes (`docs/backlog.md` row 93's duplicate-attribute
    // hazard).
    let attributes = merge_attributes(vec![
        props.attributes,
        attributes!(nav {
            "data-disabled": (props.disabled)(),
        }),
    ]);

    rsx! {
        nav {
            ..attributes,
            {props.children}
        }
    }
}

/// The props for the [`NavigationMenuList`] component.
#[derive(Props, Clone, PartialEq)]
pub struct NavigationMenuListProps {
    /// Additional attributes to apply to the list element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
    /// The children of the list -- [`NavigationMenuItem`]s.
    pub children: Element,
}

/// # NavigationMenuList
///
/// The `ul` that holds a [`NavigationMenu`]'s top-level
/// [`NavigationMenuItem`]s -- matches the disclosure-navigation example's
/// own list structure ("the top-level set ... is in a list ... and the set
/// of links controlled by each button is contained within a list nested
/// inside the item with the controlling button"). Purely structural: this
/// component holds no state of its own.
///
/// This must be used inside a [`NavigationMenu`] component.
#[component]
pub fn NavigationMenuList(props: NavigationMenuListProps) -> Element {
    rsx! {
        ul { ..props.attributes, {props.children} }
    }
}

#[derive(Clone, Copy)]
struct NavigationMenuItemContext {
    index: ReadSignal<usize>,
    is_open: Memo<bool>,
    disabled: ReadSignal<bool>,
    /// This item's own [`NavigationMenuContent`]'s current element id, kept
    /// in sync by that component -- mirrors `NavbarNavContext::content_id`
    /// (`navbar.rs`): `NavigationMenuTrigger`'s `anchor-name`/`aria-controls`
    /// must key off this item's own content, not another item's.
    content_id: Signal<String>,
    /// This item's own [`NavigationMenuTrigger`] element id, so
    /// [`NavigationMenuContent`] can label itself `aria-labelledby` from it,
    /// and so Escape-from-content can return focus to it. Mirrors
    /// `NavbarNavContext::trigger_id`.
    trigger_id: Signal<String>,
    /// Set by the trigger's own ArrowDown handler when it opens a
    /// previously-closed panel (or finds one already open); consumed once
    /// `content_focus` has at least one link registered -- see
    /// [`NavigationMenuContent`]'s own consumer effect. This is the whole
    /// mechanism behind ArrowDown jumping into the panel's first link, for
    /// either case.
    focus_first_link: Signal<bool>,
    /// Roving-focus collection for this item's own OPEN content's links --
    /// ArrowUp/ArrowDown move within them
    /// (`disclosure-navigation-hybrid.html`'s own keyboard table, "Up
    /// Arrow"/"Down Arrow" rows: "moves focus to the previous/next link"),
    /// and it is also how `focus_first_link` above finds and focuses the
    /// first one (`content_focus.first_available_index()` +
    /// `content_focus.set_focus`, the exact mechanism Home/Left/Right
    /// already use on the top-level collection below). Distinct from
    /// `NavigationMenuContext::focus` (this item's own top-level
    /// Left/Right/Home/End collection, shared across every
    /// [`NavigationMenuItem`]) -- this one is scoped to a single item's own
    /// panel, one instance per [`NavigationMenuItem`]. Never drives
    /// `tabindex` (this module's own "Tab order: native, not roving" doc);
    /// [`NavigationMenuLink`] reads only the focus-moving half of
    /// `crate::collection::use_item`'s return value, exactly like the
    /// top-level collection already does.
    content_focus: CollectionState,
}

/// The props for the [`NavigationMenuItem`] component.
#[derive(Props, Clone, PartialEq)]
pub struct NavigationMenuItemProps {
    /// The index of this item within the [`NavigationMenu`]. Used for
    /// Left/Right keyboard navigation between top-level items.
    pub index: ReadSignal<usize>,

    /// Whether this item is disabled.
    #[props(default)]
    pub disabled: ReadSignal<bool>,

    /// Additional attributes to apply to the `li` element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
    /// The children of the item -- either a [`NavigationMenuTrigger`] +
    /// [`NavigationMenuContent`] pair, or a single top-level
    /// [`NavigationMenuLink`].
    pub children: Element,
}

/// # NavigationMenuItem
///
/// One top-level entry: either a disclosure trigger with its panel, or a
/// plain top-level link. Provides the context its children key off.
///
/// This must be used inside a [`NavigationMenu`] (or [`NavigationMenuList`])
/// component.
///
/// ## Styling
///
/// The [`NavigationMenuItem`] component defines the following data
/// attributes you can use to control styling:
/// - `data-state`: Indicates if this item's panel is open or closed.
///   Values are `open` or `closed`.
/// - `data-disabled`: Indicates if the item is disabled. Values are `true`
///   or `false`.
#[component]
pub fn NavigationMenuItem(props: NavigationMenuItemProps) -> Element {
    let ctx: NavigationMenuContext = use_context();
    let is_open = use_memo(move || (ctx.open)() == Some(props.index.cloned()));
    let content_id = use_unique_id();
    let trigger_id = use_unique_id();

    // Not `use_collection_provider` (which also provides `CollectionState`
    // as its own bare context type): nothing here needs `content_focus`
    // reachable that way, only as this struct's own field, and this item's
    // panel content is unmounted/remounted far more often than this
    // component itself, so the collection's identity must survive that
    // (`use_hook`, exactly like `NavigationMenuLink`'s own pre-existing
    // per-instance collection construction below).
    let content_focus = use_hook(|| {
        CollectionState::new(
            ReadSignal::new(Signal::new(false)),
            CollectionOptions::default(),
        )
    });

    let mut item_ctx = use_context_provider(|| NavigationMenuItemContext {
        index: props.index,
        is_open,
        disabled: props.disabled,
        content_id,
        trigger_id,
        focus_first_link: Signal::new(false),
        content_focus,
    });

    use_effect(move || {
        if !is_open() {
            item_ctx.focus_first_link.set(false);
        }
    });

    // Owned by this component -- open/disabled state must win over a
    // caller's own attributes (`docs/backlog.md` row 93's
    // duplicate-attribute hazard).
    let attributes = merge_attributes(vec![
        props.attributes,
        attributes!(li {
            "data-state": if is_open() { "open" } else { "closed" },
            "data-disabled": (ctx.disabled)() || (props.disabled)(),
        }),
    ]);

    rsx! {
        li { ..attributes, {props.children} }
    }
}

/// Provided only by [`NavigationMenuContent`], so a [`NavigationMenuLink`]
/// can tell whether it is a top-level item or a link inside an open panel
/// (context-type presence, the same technique `NavbarItem` uses
/// `try_use_context::<NavbarNavContext>()` for -- `navbar.rs`). Carries no
/// data of its own -- a content link's shared state
/// (`content_focus`/`focus_first_link`) already lives on
/// [`NavigationMenuItemContext`], reachable from both the trigger and every
/// link regardless of nesting; this type exists purely so its *presence*
/// (not any field on it) answers "am I nested in content."
#[derive(Clone, Copy)]
struct NavigationMenuContentContext;

/// The props for the [`NavigationMenuTrigger`] component.
#[derive(Props, Clone, PartialEq)]
pub struct NavigationMenuTriggerProps {
    /// The ID of the trigger button. If not provided, an internally
    /// generated ID is used and kept in sync with a caller override (the
    /// same `use_id_or` construction as `NavbarTrigger`, `navbar.rs`).
    pub id: ReadSignal<Option<String>>,

    /// Additional attributes to apply to the trigger element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
    /// The children of the trigger.
    pub children: Element,
}

/// # NavigationMenuTrigger
///
/// A disclosure button (`aria-expanded`, `aria-controls`) that toggles its
/// sibling [`NavigationMenuContent`] on click/Enter/Space, and on pointer
/// hover after a short delay (see this module's doc).
///
/// This must be used inside a [`NavigationMenuItem`] component.
#[component]
pub fn NavigationMenuTrigger(props: NavigationMenuTriggerProps) -> Element {
    let mut ctx: NavigationMenuContext = use_context();
    let item_ctx: NavigationMenuItemContext = use_context();
    let index = item_ctx.index;
    let disabled = move || (ctx.disabled)() || (item_ctx.disabled)();
    let is_open = item_ctx.is_open;

    // The trigger is the focusable element, so it registers this item in
    // the top-level collection -- mirrors `NavbarTrigger` registering
    // `nav_ctx.index` in `ctx.focus`. Its `tabindex` is never read (see
    // this module's doc): this button keeps its native default tabindex.
    let item = use_item(collection_item(ctx.focus, index).disabled(disabled));
    let onmounted = item.onmounted();

    let id = use_id_or(item_ctx.trigger_id, props.id);

    // Merged (caller-wins, deduped) rather than a bare literal alongside
    // `..props.attributes` -- see `NavbarTrigger`'s identical construction
    // (`navbar.rs`) for the duplicate-attribute hazard this avoids.
    let attributes = merge_attributes(vec![
        attributes!(button { id: id.cloned() }),
        props.attributes,
    ]);
    // Ties this trigger to the web-arm content's `position-anchor` once
    // promoted to the top layer -- see `crate::top_layer::anchor_name_style`
    // (inert off the web arm). Previously a bare `style: anchor_name_style(
    // ..)` literal directly on this element, beside `..attributes` below: a
    // caller supplying their own `style` then produced two `style`
    // attributes on this same tag (`docs/conformance-harness.md`
    // hydration-parity Rule 4; caught live by
    // `playwright/oracle/attr-synth`'s `navigation_menu:trigger:style`
    // case). Folding it into `attributes` here instead, via
    // `top_layer::anchored_trigger_attributes`, fixes that and (unlike
    // plain `merge_attributes`, which only folds `class`) keeps the anchor
    // binding even when the caller's `style` is present; see that
    // function's own doc.
    let attributes =
        crate::top_layer::anchored_trigger_attributes(&item_ctx.content_id.cloned(), attributes);
    // Owned by this component -- disclosure semantics (`aria-expanded`/
    // `aria-controls`) and state must win over a caller's own attributes
    // (`docs/backlog.md` row 93's duplicate-attribute hazard).
    let attributes = merge_attributes(vec![
        attributes,
        attributes!(button {
            type: "button",
            aria_expanded: is_open(),
            aria_controls: item_ctx.content_id.cloned(),
            "data-state": if is_open() { "open" } else { "closed" },
            "data-disabled": disabled(),
        }),
    ]);

    rsx! {
        button {
            onmounted,
            onclick: move |_| {
                if disabled() {
                    return;
                }
                ctx.hover_open.cancel();
                ctx.hover_close.cancel();
                let want_open = !is_open.cloned();
                ctx.set_open.call(want_open.then(|| index.cloned()));
            },
            onmouseenter: move |_| {
                if disabled() {
                    return;
                }
                ctx.hover_close.cancel();
                if (ctx.open)() == Some(index.cloned()) {
                    return;
                }
                let target = index.cloned();
                ctx.hover_open.schedule(HOVER_OPEN_INTENT_DELAY, move || {
                    ctx.set_open.call(Some(target));
                });
            },
            onmouseleave: move |_| {
                ctx.hover_open.cancel();
                if (ctx.open)() != Some(index.cloned()) {
                    return;
                }
                let target = index.cloned();
                ctx.hover_close.schedule(HOVER_CLOSE_GRACE_DELAY, move || {
                    if (ctx.open)() == Some(target) {
                        ctx.set_open.call(None);
                    }
                });
            },
            onfocus: move |_| {
                ctx.blur_close.cancel();
                // Sync native focus (Tab, a click, or a screen reader's own
                // navigation -- anything that isn't this collection's own
                // `focus_next`/`focus_prev`/`set_focus` call) back into
                // `ctx.focus`'s own `recent`/`focused` state -- the same
                // construction `toolbar.rs`/`tabs.rs` already use on their
                // own identical collections (`ctx.focus.set_focus(Some(
                // (props.index)()))`). Without this, `focus_next()`/
                // `focus_prev()` compute their "current position" from
                // whatever this collection last *itself* set focus to,
                // which can go stale the moment focus arrives some other
                // way -- confirmed by execution: Tab into a top-level item,
                // then Left/Right, and the first press was silently a
                // same-spot no-op (`next_index_after`/`prev_index_before`
                // treat an unknown/`None` current position as "before the
                // first item", so the first press only catches the
                // collection up to where focus already was).
                ctx.focus.set_focus(Some(index.cloned()));
            },
            onblur: move |_| {
                ctx.blur_close.schedule(FOCUS_LEAVE_CLOSE_DELAY, move || {
                    ctx.set_open.call(None);
                });
            },
            onkeydown: move |event: Event<KeyboardData>| {
                match event.key() {
                    // "If a dropdown is open ... pressing Esc will close the
                    // dropdown" -- disclosure-navigation.html's own
                    // accessibility-features note. Closes whatever is open
                    // (mirroring the reference JS's own
                    // `toggleExpand(this.openIndex, false)`), not only when
                    // it is this trigger's own panel, matching
                    // `onButtonKeyDown` exactly.
                    Key::Escape if (ctx.open)().is_some() => {
                        ctx.set_open.call(None);
                    }
                    Key::ArrowLeft => ctx.focus.focus_prev(),
                    Key::ArrowRight => ctx.focus.focus_next(),
                    // disclosure-navigation-hybrid.html's own keyboard
                    // table, Home/End rows: "moves focus to the
                    // first/last item" -- this collection is already
                    // non-looping (see `NavigationMenu`'s own doc comment
                    // on `focus`), so these are exactly `focus_first`/
                    // `focus_last`, the same pair `NavigationMenuLink`'s
                    // own top-level branch below (and, elsewhere in this
                    // crate, `context_menu.rs`'s top-level Home/End) already
                    // use for an identical collection.
                    Key::Home => ctx.focus.focus_first(),
                    Key::End => ctx.focus.focus_last(),
                    // "if focus is on a button and its dropdown is
                    // expanded, moves focus to the first link in the
                    // dropdown" -- disclosure-navigation-hybrid.html's
                    // keyboard table. Extended here, past that one
                    // sentence, to also OPEN a still-*closed* panel first
                    // -- this module's own choice, not the reference
                    // table's literal text for a collapsed trigger (that
                    // row instead sends a collapsed trigger's ArrowDown to
                    // the *next* top-level item, same as ArrowRight;
                    // deliberately not mirrored, since ArrowRight already
                    // covers that and doing both would make the two keys
                    // redundant with each other for half of what ArrowDown
                    // does). A keyboard user pressing ArrowDown on a
                    // collapsed trigger far more plausibly means "open this
                    // and take me into it" than "skip it, take me to
                    // whatever's next" -- the same one-key affordance a
                    // native `<select>`/combobox already gives ArrowDown.
                    Key::ArrowDown => {
                        if !is_open.cloned() {
                            ctx.hover_open.cancel();
                            ctx.hover_close.cancel();
                            ctx.set_open.call(Some(index.cloned()));
                        }
                        item_ctx.focus_first_link.clone().set(true);
                    }
                    _ => return,
                }
                event.prevent_default();
            },

            ..attributes,
            {props.children}
        }
    }
}

/// The props for the [`NavigationMenuContent`] component.
#[derive(Props, Clone, PartialEq)]
pub struct NavigationMenuContentProps {
    /// The id of the content element.
    pub id: ReadSignal<Option<String>>,
    /// Additional attributes to apply to the content element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
    /// The children of the content -- typically a grid of
    /// [`NavigationMenuLink`]s.
    pub children: Element,
}

/// # NavigationMenuContent
///
/// The panel a [`NavigationMenuTrigger`] discloses: anchored below its
/// trigger and, on the web arm, promoted to the top layer (see this
/// module's doc). Never carries a menu role (R4: this pattern has none).
///
/// This must be used inside a [`NavigationMenuItem`] component, after its
/// [`NavigationMenuTrigger`].
///
/// ## Styling
///
/// The [`NavigationMenuContent`] component defines the following data
/// attributes you can use to control styling:
/// - `data-state`: Indicates if this panel is open or closed. Values are
///   `open` or `closed`.
#[component]
pub fn NavigationMenuContent(props: NavigationMenuContentProps) -> Element {
    let mut item_ctx: NavigationMenuItemContext = use_context();

    let unique_id = use_unique_id();
    let id = use_id_or(unique_id, props.id);

    // Keep `item_ctx.content_id` in sync -- mirrors `NavbarContent`'s
    // identical `nav_ctx.content_id.set(id())` (`navbar.rs`).
    use_effect(move || item_ctx.content_id.set(id()));

    let render = use_animated_open(id, item_ctx.is_open);

    // Consume a pending "focus the first link" request (the trigger's own
    // ArrowDown) once `content_focus` has at least one link registered --
    // see `NavigationMenuItemContext::focus_first_link`'s doc. Moves focus
    // through `content_focus.set_focus`, the exact mechanism Home/Left/
    // Right already use to move focus on the top-level collection
    // (`use_item`'s own `control_mount_focus`), not a hand-rolled
    // `MountedData` tracker -- an earlier version of this used exactly
    // that (`first_content_link`/`register_first_link`, "whichever content
    // link mounts first, tracked via a `Signal::peek()`-then-`set()`
    // guard") and found by execution that it was not reliable: a real,
    // multi-link panel opened via ArrowDown always focused its *last*
    // link, never its first. Root-caused to an assumption, not a race:
    // that guard's "first-write-wins" logic implicitly assumed a content
    // panel's children mount in source order, and confirmed by execution
    // (a diagnostic build exposing each link's own assigned position) that
    // they do not, for this shape -- reproducibly the exact *reverse* of
    // source order, not a one-off flake. `content_focus`'s own
    // `first_available_index()` sidesteps this entirely: it sorts by each
    // registered item's own `index`, and `NavigationMenuLink`'s `index`
    // for a content link is `props.content_index` -- a plain value fixed
    // by the caller before any rendering happens (see that prop's own
    // doc), not anything computed from mount order at all.
    use_effect(move || {
        if !(item_ctx.focus_first_link)() {
            return;
        }
        let mut content_focus = item_ctx.content_focus;
        let Some(first_index) = content_focus.first_available_index() else {
            return;
        };
        item_ctx.focus_first_link.set(false);
        content_focus.set_focus(Some(first_index));
    });

    use_context_provider(|| NavigationMenuContentContext);

    rsx! {
        if render() {
            NavigationMenuContentRendered {
                id: id.cloned(),
                attributes: props.attributes,
                children: props.children,
            }
        }
    }
}

/// Web arm: promote to the top layer via `popover="manual"`, anchored to
/// this item's own trigger -- see this module's doc, "Top layer", for why
/// `manual` (not `auto`, unlike `NavbarContent`).
///
/// # Exit animation: `use_popover_shown_while_mounted`, not `use_popover_sync`
///
/// This content renders through [`use_animated_open`] (in
/// [`NavigationMenuContent`] above), which deliberately keeps the element
/// mounted with `data-state="closed"` for its whole exit transition (plus a
/// settle hold) before actually unmounting it -- exactly the shape
/// `crate::top_layer::use_popover_shown_while_mounted`'s own doc describes
/// for `SelectList`/`ComboboxList`/`Toast`/`Popover`/`HoverCard`/`Tooltip`.
/// An earlier version of this function used the plain
/// `crate::top_layer::use_popover_sync` instead (mirroring `NavbarContent`'s
/// own call, which does not need the animated-exit variant... except
/// `NavbarContent` *also* renders through `use_animated_open` and has the
/// same latent gap -- out of this module's scope to fix, see the module doc)
/// -- that hook's "signal -> browser" effect calls `hidePopover()` the
/// instant `open` goes `false`, which (per the UA popover stylesheet's
/// `[popover]:not(:popover-open) { display: none }`) sets `display: none`
/// on the content *before* `use_animated_open`'s own rAF-deferred
/// `getAnimations()` check ever ran, so that check always observed zero
/// running animations and finished the close cycle immediately -- the exit
/// fade/scale was skipped outright, confirmed by execution (a live
/// `dx serve` instance: the panel vanished within one 40ms sample of its
/// `data-state` flipping to `"closed"`, instead of playing the ~150ms
/// fade-and-scale-down this file's stylesheet declares, plus its ~250ms
/// settle hold). This is `top_layer.rs`'s own documented "Bug 1 (animation
/// race)" on `use_popover_shown_while_mounted`'s doc -- the exact bug class
/// that hook exists to close. Switching to it here fixes the same way it
/// already does for every other `use_animated_open`-rendered, `popover`-
/// promoted consumer: `hidePopover()` is never called on this component's
/// own closing path at all, only a real DOM removal (once
/// `use_animated_open` itself has decided the animation and its hold are
/// done) ever takes this content out of the top layer, so the exit
/// animation plays out undisturbed on an element that is still very much
/// `:popover-open` the whole time.
#[cfg(feature = "web")]
#[component]
fn NavigationMenuContentRendered(
    id: String,
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let mut ctx: NavigationMenuContext = use_context();
    let item_ctx: NavigationMenuItemContext = use_context();
    let open = item_ctx.is_open;
    let index = item_ctx.index;

    crate::top_layer::use_popover_shown_while_mounted(
        id.clone(),
        open,
        Callback::new(move |is_open: bool| {
            if !is_open && (ctx.open)() == Some(index.cloned()) {
                ctx.set_open.call(None);
            }
        }),
    );
    // JS-measured static positioning fallback for engines without CSS
    // Anchor Positioning -- anchored below the trigger, left-aligned,
    // matching `NavbarContentRendered`'s identical bottom/start convention.
    crate::top_layer::use_anchor_position_fallback(
        id.clone(),
        id.clone(),
        open,
        crate::ContentSide::Bottom,
        crate::ContentAlign::Start,
        8,
    );

    // An APG-adjacent panel still needs an accessible name; labelled by its
    // own trigger unless the caller already named it some other way --
    // mirrors `MenubarContent`/`NavbarContent`'s identical construction.
    let labelledby: Vec<Attribute> = if has_own_accessible_name(&attributes) {
        Vec::new()
    } else {
        attributes!(div {
            aria_labelledby: "{item_ctx.trigger_id}"
        })
    };
    let attributes = merge_attributes(vec![
        attributes,
        attributes!(div {
            class: "dx-anchor-navigation-menu"
        }),
        labelledby,
        // Owned by this component -- top-layer wiring + state must win
        // over a caller's own attributes (`docs/backlog.md` row 93's
        // duplicate-attribute hazard).
        attributes!(div {
            popover: crate::top_layer::PopoverKind::Manual.as_str(),
            "data-state": if open() { "open" } else { "closed" },
        }),
    ]);
    let attributes = crate::top_layer::anchored_content_attributes(&id, attributes);

    rsx! {
        div {
            id: id.clone(),
            onmouseenter: move |_| {
                ctx.hover_close.cancel();
            },
            onmouseleave: move |_| {
                let target = index.cloned();
                ctx.hover_close.schedule(HOVER_CLOSE_GRACE_DELAY, move || {
                    if (ctx.open)() == Some(target) {
                        ctx.set_open.call(None);
                    }
                });
            },
            onkeydown: move |event: Event<KeyboardData>| {
                // "closes it and sets focus on the button that controls
                // that dropdown" -- disclosure-navigation-hybrid.html's
                // keyboard table, Escape row. Mirrors the reference JS's
                // `onMenuKeyDown`: focus the trigger via the top-level
                // collection (registered by `NavigationMenuTrigger`).
                if event.key() == Key::Escape {
                    ctx.set_open.call(None);
                    // `clear_focus()` immediately before `set_focus` --
                    // not merely `set_focus(Some(index.cloned()))` alone --
                    // because `ctx.focus`'s own `focused` field can already
                    // equal this trigger's index from *before* focus ever
                    // left it for `content_focus` (a link inside the panel
                    // never touches `ctx.focus` at all, per this module's
                    // two-collection design -- see this module's own doc,
                    // "Tab order"), and `CollectionState::set_focus` only
                    // notifies/re-drives DOM focus when the value actually
                    // *changes* (its own doc: "A redundant clear ... must
                    // not wake effects"). Setting the same already-current
                    // value is therefore a silent no-op that leaves real
                    // DOM focus exactly where it was -- confirmed by
                    // execution: with real DOM focus on a link inside the
                    // panel, `ctx.focus.set_focus(Some(trigger_index))`
                    // alone never re-focused the trigger at all (focus fell
                    // through to `<body>` once the closing content was
                    // actually removed from the DOM, several hundred ms
                    // later, past this animated exit's own hold). Clearing
                    // first forces a real `None -> Some` transition, so the
                    // very next line's `set_focus` is never a no-op.
                    ctx.focus.clear_focus();
                    ctx.focus.set_focus(Some(index.cloned()));
                    event.prevent_default();
                }
            },

            ..attributes,
            {children}
        }
    }
}

/// Native (Blitz) arm: a plain positioned `div` -- Blitz has no popover-API
/// support, matching `NavbarContentRendered`'s native arm.
#[cfg(not(feature = "web"))]
#[component]
fn NavigationMenuContentRendered(
    id: String,
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let mut ctx: NavigationMenuContext = use_context();
    let item_ctx: NavigationMenuItemContext = use_context();
    let index = item_ctx.index;

    let labelledby: Vec<Attribute> = if has_own_accessible_name(&attributes) {
        Vec::new()
    } else {
        attributes!(div {
            aria_labelledby: "{item_ctx.trigger_id}"
        })
    };
    let attributes = merge_attributes(vec![
        attributes,
        labelledby,
        // Owned by this component -- see the web arm's identical
        // construction above (`docs/backlog.md` row 93).
        attributes!(div {
            "data-state": if (item_ctx.is_open)() { "open" } else { "closed" },
        }),
    ]);

    rsx! {
        div {
            id,
            onmouseenter: move |_| {
                ctx.hover_close.cancel();
            },
            onmouseleave: move |_| {
                let target = index.cloned();
                ctx.hover_close.schedule(HOVER_CLOSE_GRACE_DELAY, move || {
                    if (ctx.open)() == Some(target) {
                        ctx.set_open.call(None);
                    }
                });
            },
            onkeydown: move |event: Event<KeyboardData>| {
                if event.key() == Key::Escape {
                    ctx.set_open.call(None);
                    // `clear_focus()` before `set_focus` -- see the web
                    // arm's identical Escape handler, just above in this
                    // file, for the full account of why `set_focus` alone
                    // can silently no-op here.
                    ctx.focus.clear_focus();
                    ctx.focus.set_focus(Some(index.cloned()));
                    event.prevent_default();
                }
            },

            ..attributes,
            {children}
        }
    }
}

/// The props for the [`NavigationMenuLink`] component.
#[derive(Props, Clone, PartialEq)]
pub struct NavigationMenuLinkProps {
    /// Whether this link represents the current page. Reflected as
    /// `data-active` and, per the disclosure-navigation example's own
    /// `aria-current="page"` row, as `aria-current`.
    #[props(default)]
    pub active: bool,

    /// Whether this link is disabled.
    #[props(default)]
    pub disabled: ReadSignal<bool>,

    /// This link's own position within its panel, for ArrowUp/ArrowDown to
    /// move between links in the right order (this module's own doc, "Tab
    /// order: native, not roving"). Ignored for a top-level link (a direct
    /// child of [`NavigationMenuItem`], no [`NavigationMenuContent`]
    /// ancestor) -- those instead key off their own [`NavigationMenuItem`]'s
    /// `index` prop for Left/Right/Home/End, so leaving this at its default
    /// there is correct, not merely tolerated. **Required** (in the same
    /// sense [`NavigationMenuItemProps::index`] is -- no runtime check
    /// enforces it) for a link nested inside [`NavigationMenuContent`]: set
    /// it to that link's own position among its panel's siblings, `0`-based
    /// in reading order. Deliberately not inferred from mount order --
    /// confirmed by execution that Dioxus does not mount a content panel's
    /// children in source order for this shape (a `use_hook`-assigned,
    /// monotonic-counter version of this index was tried first; every
    /// panel's links came back numbered in the exact *reverse* of their
    /// visual order, reproducibly, not as a one-off race), so nothing
    /// computed at a content link's own mount time can be trusted for this
    /// -- only a value fixed by the caller, before any rendering happens,
    /// the same way the top-level collection's own indices already are.
    #[props(default)]
    pub content_index: ReadSignal<usize>,

    /// Called when the link is activated (after this primitive's own
    /// close-the-open-panel handling runs).
    pub onclick: Option<EventHandler<MouseEvent>>,

    /// Additional attributes to apply to the `a` element -- pass `href`
    /// here. This module renders a plain anchor rather than
    /// `dioxus_router::Link`, so it needs no `router` feature.
    #[props(extends = GlobalAttributes)]
    #[props(extends = a)]
    pub attributes: Vec<Attribute>,
    /// The children of the link.
    pub children: Element,
}

/// # NavigationMenuLink
///
/// A plain `a[href]`, usable either as a top-level item (a direct child of
/// [`NavigationMenuItem`], no [`NavigationMenuTrigger`]/[`NavigationMenuContent`]
/// pair) or nested inside a [`NavigationMenuContent`] panel.
///
/// This must be used inside a [`NavigationMenuItem`] component.
///
/// ## Styling
///
/// The [`NavigationMenuLink`] component defines the following data
/// attributes you can use to control styling:
/// - `data-active`: Indicates if this link represents the current page.
///   Values are `true` or `false`.
/// - `data-disabled`: Indicates if the link is disabled. Values are `true`
///   or `false`.
#[component]
pub fn NavigationMenuLink(props: NavigationMenuLinkProps) -> Element {
    let mut ctx: NavigationMenuContext = use_context();
    let mut item_ctx: NavigationMenuItemContext = use_context();
    let content_ctx: Option<NavigationMenuContentContext> = try_use_context();
    let is_top_level = content_ctx.is_none();
    let disabled = move || (ctx.disabled)() || (item_ctx.disabled)() || (props.disabled)();

    // `use_item` is always called (Rules of Hooks: this component's hook
    // order must never depend on a runtime value) -- but only a top-level
    // link is meant to participate in the top-level Left/Right/Home/End
    // collection (`ctx.focus`, shared across every top-level item, keyed by
    // its own `NavigationMenuItem`'s `index` prop). A content link instead
    // participates in *this item's own* `content_focus` collection
    // (ArrowUp/ArrowDown, and the trigger's own ArrowDown-to-first-link --
    // see `NavigationMenuItemContext::content_focus`'s doc), keyed by
    // `props.content_index` (see that prop's own doc for why this is a
    // plain, caller-supplied value rather than anything computed from
    // mount order). `is_top_level` cannot change for a mounted instance
    // (it is fixed by where this component sits in the tree), so which of
    // the two ends up used is itself stable across this instance's
    // re-renders.
    let (collection, own_index) = if is_top_level {
        (ctx.focus, item_ctx.index)
    } else {
        (item_ctx.content_focus, props.content_index)
    };
    let item = use_item(collection_item(collection, own_index).disabled(disabled));
    let mut onmounted_item = item.onmounted();

    // Owned by this component -- active/disabled state must win over a
    // caller's own attributes (`docs/backlog.md` row 93's
    // duplicate-attribute hazard).
    let attributes = merge_attributes(vec![
        props.attributes,
        attributes!(a {
            aria_current: props.active.then_some("page"),
            // `<a>` has no native disabled semantics (unlike
            // `<button disabled>`), so `data-disabled` alone would look,
            // to assistive tech, like a perfectly normal active link --
            // same fix as `NavbarItem`/`DropdownMenuItem` (docs/backlog.md
            // row 39).
            aria_disabled: disabled(),
            "data-active": props.active,
            "data-disabled": disabled(),
        }),
    ]);

    rsx! {
        a {
            onmounted: move |evt: MountedEvent| {
                onmounted_item(evt.clone());
            },
            onclick: move |event: MouseEvent| {
                if disabled() {
                    event.prevent_default();
                    return;
                }
                ctx.hover_open.cancel();
                ctx.hover_close.cancel();
                ctx.set_open.call(None);
                if let Some(onclick) = props.onclick {
                    onclick.call(event);
                }
            },
            onfocus: move |_| {
                ctx.blur_close.cancel();
                // Sync native focus back into whichever collection this
                // link belongs to -- see `NavigationMenuTrigger`'s
                // identical addition for the full account (Tab landing on
                // a link never otherwise informs `content_focus`/`ctx.focus`
                // that it is now the "current" item, so ArrowUp/ArrowDown
                // -- or Left/Right for a top-level link -- would otherwise
                // waste their first press catching the collection up
                // instead of moving).
                let mut collection = collection;
                collection.set_focus(Some(own_index.cloned()));
            },
            onblur: move |_| {
                ctx.blur_close.schedule(FOCUS_LEAVE_CLOSE_DELAY, move || {
                    ctx.set_open.call(None);
                });
            },
            onkeydown: move |event: Event<KeyboardData>| {
                if is_top_level {
                    match event.key() {
                        Key::ArrowLeft => ctx.focus.focus_prev(),
                        Key::ArrowRight => ctx.focus.focus_next(),
                        // disclosure-navigation-hybrid.html's own keyboard
                        // table, Home/End rows: "moves focus to the
                        // first/last item" -- mirrors
                        // `NavigationMenuTrigger`'s identical addition (a
                        // top-level item is a trigger or a plain link
                        // interchangeably, and both share this same
                        // collection).
                        Key::Home => ctx.focus.focus_first(),
                        Key::End => ctx.focus.focus_last(),
                        _ => return,
                    }
                } else {
                    match event.key() {
                        // disclosure-navigation-hybrid.html's own keyboard
                        // table, "Up Arrow"/"Down Arrow" rows: "If focus is
                        // on a link within an expanded dropdown, and it is
                        // not the first/last link, moves focus to the
                        // previous/next link" -- explicitly conditional on
                        // "not the first/last link", i.e. non-looping
                        // (stops at the ends) rather than wrapping, the
                        // same choice (and the same citation) this
                        // module's top-level collection already makes --
                        // see `NavigationMenu`'s own doc comment on
                        // `focus`. `content_focus` is constructed
                        // non-looping for the identical reason.
                        Key::ArrowUp => item_ctx.content_focus.focus_prev(),
                        Key::ArrowDown => item_ctx.content_focus.focus_next(),
                        _ => return,
                    }
                }
                event.prevent_default();
            },

            ..attributes,
            {props.children}
        }
    }
}
