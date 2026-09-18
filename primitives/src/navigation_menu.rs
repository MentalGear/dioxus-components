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
//! Left/Right between top-level items is optional in APG and implemented
//! here (`disclosure-navigation-hybrid.html`'s own keyboard table includes
//! it); Home/End and Up/Down-within-an-open-panel's own list of links are
//! also optional and are **not** implemented, since Tab already reaches
//! every one of those targets and neither is exercised by this round's
//! oracle/smoke coverage -- a future round can add them without changing
//! any existing contract.
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
//! [`NavigationMenuContent`]'s web-arm leaf promotes to the top layer the
//! same way `NavbarContent` does -- `crate::top_layer::use_popover_sync` +
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
//! debounced open/close instead of the two ever agreeing.

use std::rc::Rc;
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

    use_context_provider(|| NavigationMenuContext {
        open,
        set_open,
        disabled: props.disabled,
        focus,
        hover_open: use_delayed_action(),
        hover_close: use_delayed_action(),
        blur_close: use_delayed_action(),
    });

    rsx! {
        nav {
            "data-disabled": (props.disabled)(),
            ..props.attributes,
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
    /// Filled by the first [`NavigationMenuLink`] to mount inside this
    /// item's open content (first-write-wins; reset to `None` whenever the
    /// content closes). Together with `focus_first_link` below, this is
    /// the whole mechanism behind ArrowDown-from-an-open-trigger jumping
    /// into the panel's first link -- deliberately not the full
    /// `crate::collection` roving-focus machinery, since nothing here
    /// needs to roam *among* content links (Tab already does; see this
    /// module's doc) or know any content link's position, only "the
    /// first one, whichever mounted first".
    first_content_link: Signal<Option<Rc<MountedData>>>,
    /// Set by the trigger's own ArrowDown handler; consumed (and cleared)
    /// once `first_content_link` is available.
    focus_first_link: Signal<bool>,
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

    let mut item_ctx = use_context_provider(|| NavigationMenuItemContext {
        index: props.index,
        is_open,
        disabled: props.disabled,
        content_id,
        trigger_id,
        first_content_link: Signal::new(None),
        focus_first_link: Signal::new(false),
    });

    use_effect(move || {
        if !is_open() {
            item_ctx.first_content_link.set(None);
            item_ctx.focus_first_link.set(false);
        }
    });

    rsx! {
        li {
            "data-state": if is_open() { "open" } else { "closed" },
            "data-disabled": (ctx.disabled)() || (props.disabled)(),
            ..props.attributes,
            {props.children}
        }
    }
}

/// Provided only by [`NavigationMenuContent`], so a [`NavigationMenuLink`]
/// can tell whether it is a top-level item or a link inside an open panel
/// (context-type presence, the same technique `NavbarItem` uses
/// `try_use_context::<NavbarNavContext>()` for -- `navbar.rs`).
#[derive(Clone, Copy)]
struct NavigationMenuContentContext {
    register_first_link: Callback<Rc<MountedData>>,
}

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

    rsx! {
        button {
            onmounted,
            // Ties this trigger to the web-arm content's `position-anchor`
            // once promoted to the top layer -- see
            // `crate::top_layer::anchor_name_style`. Inert off the web arm.
            style: crate::top_layer::anchor_name_style(&item_ctx.content_id.cloned()),
            type: "button",
            aria_expanded: is_open(),
            aria_controls: item_ctx.content_id.cloned(),
            "data-state": if is_open() { "open" } else { "closed" },
            "data-disabled": disabled(),

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
                    // "if focus is on a button and its dropdown is
                    // expanded, moves focus to the first link in the
                    // dropdown" -- disclosure-navigation-hybrid.html's
                    // keyboard table.
                    Key::ArrowDown if is_open.cloned() => {
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
    // ArrowDown) once both it and a first-registered link are available --
    // see `NavigationMenuItemContext::first_content_link`'s doc.
    use_effect(move || {
        if !(item_ctx.focus_first_link)() {
            return;
        }
        let Some(md) = item_ctx.first_content_link.cloned() else {
            return;
        };
        item_ctx.focus_first_link.set(false);
        spawn(async move {
            let _ = md.set_focus(true).await;
        });
    });

    let register_first_link = use_callback(move |data: Rc<MountedData>| {
        if item_ctx.first_content_link.peek().is_none() {
            item_ctx.first_content_link.set(Some(data));
        }
    });
    use_context_provider(|| NavigationMenuContentContext {
        register_first_link,
    });

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

    crate::top_layer::use_popover_sync(
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
    ]);
    let attributes = crate::top_layer::anchored_content_attributes(&id, attributes);

    rsx! {
        div {
            id: id.clone(),
            popover: crate::top_layer::PopoverKind::Manual.as_str(),
            "data-state": if open() { "open" } else { "closed" },

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
    let attributes = merge_attributes(vec![attributes, labelledby]);

    rsx! {
        div {
            id,
            "data-state": if (item_ctx.is_open)() { "open" } else { "closed" },

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
    let item_ctx: NavigationMenuItemContext = use_context();
    let content_ctx: Option<NavigationMenuContentContext> = try_use_context();
    let is_top_level = content_ctx.is_none();
    let disabled = move || (ctx.disabled)() || (item_ctx.disabled)() || (props.disabled)();

    // `use_item` is always called (Rules of Hooks: this component's hook
    // order must never depend on a runtime value) -- but only a top-level
    // link is meant to participate in the top-level Left/Right collection.
    // A content link instead gets a private, per-instance, never-queried
    // collection (`use_hook`, stable across this instance's own
    // re-renders): registering into it is inert bookkeeping nobody ever
    // reads, simpler than branching the hook call itself. `is_top_level`
    // cannot change for a mounted instance (it is fixed by where this
    // component sits in the tree), so which branch runs is itself stable
    // across this instance's re-renders either way.
    let private_collection = use_hook(|| {
        CollectionState::new(
            ReadSignal::new(Signal::new(false)),
            CollectionOptions::default(),
        )
    });
    let collection = if is_top_level {
        ctx.focus
    } else {
        private_collection
    };
    let item = use_item(collection_item(collection, item_ctx.index).disabled(disabled));
    let mut onmounted_item = item.onmounted();

    rsx! {
        a {
            onmounted: move |evt: MountedEvent| {
                onmounted_item(evt.clone());
                if let Some(content_ctx) = content_ctx {
                    content_ctx.register_first_link.call(evt.data());
                }
            },
            aria_current: props.active.then_some("page"),
            // `<a>` has no native disabled semantics (unlike
            // `<button disabled>`), so `data-disabled` alone would look,
            // to assistive tech, like a perfectly normal active link --
            // same fix as `NavbarItem`/`DropdownMenuItem` (docs/backlog.md
            // row 39).
            aria_disabled: disabled(),
            "data-active": props.active,
            "data-disabled": disabled(),

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
            },
            onblur: move |_| {
                ctx.blur_close.schedule(FOCUS_LEAVE_CLOSE_DELAY, move || {
                    ctx.set_open.call(None);
                });
            },
            onkeydown: move |event: Event<KeyboardData>| {
                if !is_top_level {
                    return;
                }
                match event.key() {
                    Key::ArrowLeft => ctx.focus.focus_prev(),
                    Key::ArrowRight => ctx.focus.focus_next(),
                    _ => return,
                }
                event.prevent_default();
            },

            ..props.attributes,
            {props.children}
        }
    }
}
