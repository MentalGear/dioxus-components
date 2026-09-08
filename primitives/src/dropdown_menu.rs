//! Defines the [`DropdownMenu`] component and its subcomponents.
//!
//! Also defines [`DropdownMenuSub`]/[`DropdownMenuSubTrigger`]/
//! [`DropdownMenuSubContent`]/[`DropdownMenuSubItem`], the nested-submenu
//! half of the APG "Menu and Menubar" pattern (`docs/component-backlog.md`
//! row 68). See [`DropdownMenuSub`]'s own doc for the shared, host-
//! independent state machine (`crate::menu_sub`) both this file and
//! `context_menu.rs`'s equivalent build on.

use std::rc::Rc;

use crate::{
    collection::{
        collection_item, use_collection_provider, use_deferred_collection_focus, use_item,
        CollectionPlacement, CollectionState,
    },
    has_own_accessible_name, merge_attributes, use_animated_open, use_controlled, use_id_or,
    use_unique_id,
};
use dioxus::prelude::*;
use dioxus_attributes::attributes;

#[derive(Clone, Copy)]
struct DropdownMenuContext {
    // State
    open: Memo<bool>,
    set_open: Callback<bool>,
    disabled: ReadSignal<bool>,

    // Focus state
    focus: CollectionState,

    // Where focus should land once `DropdownMenuContent` next mounts. Every
    // key that opens the menu (Enter/Space/ArrowDown/ArrowUp) goes through
    // `open_with_focus` below, which sets this *before* flipping `open` --
    // never the reverse -- so opening never depends on the content already
    // being mounted or on the open<->focus sync effect below. See
    // docs/recommended-implementations.md's "keyboard open contract" note.
    initial_focus: Signal<Option<CollectionPlacement>>,

    // Unique ID for the trigger button
    trigger_id: Signal<String>,

    // The current `DropdownMenuContent`'s own element id, kept in sync by
    // that component -- mirrors `PopoverCtx::content_id` (`popover.rs`).
    // `DropdownMenuTrigger`'s `anchor-name` must key off *this* signal, not
    // `trigger_id` (this is the trigger's own id, unrelated), so that
    // `crate::top_layer::position_anchor_style` on the content side (built
    // from the content's own id) and `anchor_name_style` on the trigger
    // side name the same anchor. See `PopoverCtx::content_id`'s doc for the
    // exact bug this guards against if the two ever named different ids.
    content_id: Signal<String>,

    // Whether the open menu should lock page scrolling. See
    // docs/plan.md Phase 3.2.
    modal: ReadSignal<bool>,

    // Set just before a close caused by something *outside* the menu
    // (trigger/item blur), so `use_refocus_on_close_unless` (lib.rs) knows
    // not to yank focus back to the trigger for those closes. Reset to
    // `false` whenever the menu opens. See docs/plan.md Phase 3.1.
    interacted_outside: Signal<bool>,

    // Count of currently-open descendant `DropdownMenuSub` submenus
    // (maintained by each `DropdownMenuSub`'s own effect -- see that
    // component). A submenu's own first item legitimately holds DOM focus
    // once its `DropdownMenuSubContent` opens, and that item is registered
    // in the *submenu's* own roving-focus collection
    // (`crate::menu_sub::SubMenuState::focus`), never in `focus` above --
    // so from this root's point of view `focus.any_focused()` looks
    // unchanged (still whatever it was, often nothing at all for a
    // mouse-driven open that never touched roving focus). Without this
    // counter, the two guards below that read `focus.any_focused()` to
    // decide "did focus really leave the whole menu" (this trigger's own
    // `onblur`, and the open<->focus sync effect right below) misread the
    // trigger's own blur -- fired the instant that submenu item actually
    // receives DOM focus -- as focus having left the entire widget, and
    // close the root out from under its own still-open submenu. Root cause
    // for `oracle/tier1-apg/menu-submenu.spec.ts`'s click/hover-open
    // failures (`docs/component-backlog.md` row 68); see
    // `DropdownMenuSub`'s own comment for how this count is kept accurate
    // across open, close, and an ancestor unmounting a still-open submenu.
    submenu_open_count: Signal<usize>,
}

impl DropdownMenuContext {
    /// The single path every open key (Enter/Space/ArrowDown/ArrowUp) routes
    /// through: request `target` as the focus placement once the content
    /// mounts, then open. Never the other order -- setting `open` first
    /// would let a render happen (or a Playwright poll observe state)
    /// between the two with no focus request recorded yet. Fix-by-
    /// construction for the keyboard matrix's DropdownMenu-trigger rows: see
    /// `oracle/tier1-apg/keyboard-matrix.spec.ts`.
    fn open_with_focus(&mut self, target: CollectionPlacement) {
        self.initial_focus.set(Some(target));
        self.set_open.call(true);
    }
}

/// The props for the [`DropdownMenu`] component
#[derive(Props, Clone, PartialEq)]
pub struct DropdownMenuProps {
    /// Whether the dropdown menu is open. If not provided, the component will be uncontrolled and use `default_open`.
    pub open: ReadSignal<Option<bool>>,

    /// Default open state if the component is not controlled.
    #[props(default)]
    pub default_open: bool,

    /// Callback when the open state changes. This is called when the dropdown menu is opened or closed.
    #[props(default)]
    pub on_open_change: Callback<bool>,

    /// Whether the dropdown menu is disabled. If true, the menu will not open and items will not be selectable.
    #[props(default)]
    pub disabled: ReadSignal<bool>,

    /// Whether the open menu should lock page scrolling, matching Radix's
    /// default. See docs/plan.md Phase 3.2.
    #[props(default = ReadSignal::new(Signal::new(true)))]
    pub modal: ReadSignal<bool>,

    /// Whether focus should loop around when reaching the end.
    #[props(default = ReadSignal::new(Signal::new(true)))]
    pub roving_loop: ReadSignal<bool>,

    /// Additional attributes to apply to the dropdown menu element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the dropdown menu, which should include a [`DropdownMenuTrigger`] and a [`DropdownMenuContent`].
    pub children: Element,
}

/// # DropdownMenu
///
/// The `DropdownMenu` component is a container for a [`DropdownMenuContent`] component activated by a [`DropdownMenuTrigger`] component.
///
/// It implements the APG **menu button** pattern (`aria-haspopup="menu"` on
/// the trigger, `role="menu"` on the content, `role="menuitem"` on each
/// item -- see [`crate::menu_semantics`] for the shared role definitions and
/// their APG citations). **Deliberate ARIA-contract change:** earlier
/// versions of this component instead rendered the APG **listbox** pattern's
/// roles (`aria-haspopup="listbox"` / `role="listbox"` / `role="option"`),
/// inherited from being built on the same collection/roving-focus plumbing
/// as [`crate::select`], a genuine listbox. `DropdownMenu` has no selection
/// model (no `value`/`selected` state, no `aria-selected` on any item --
/// activating an item is an action, via `on_select`, not a selection), so
/// those roles were wrong for it; see `docs/backlog.md` row 24 and
/// `oracle/tier1-apg/menu-roles.spec.ts`. If your code queries
/// `role="option"` (Playwright's `getByRole('option', ...)` or an
/// accessibility-tree assertion) against this component's items, update it
/// to `role="menuitem"`.
///
/// ## Example
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::dropdown_menu::{
///     DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger,
/// };
/// #[component]
/// fn Demo() -> Element {
///     rsx! {
///         DropdownMenu { default_open: false,
///             DropdownMenuTrigger { "Open Menu" }
///             DropdownMenuContent {
///                 DropdownMenuItem::<String> {
///                     value: "edit".to_string(),
///                     index: 0usize,
///                     on_select: move |value| {
///                         tracing::info!("Selected: {}", value);
///                     },
///                     "Edit"
///                 }
///                 DropdownMenuItem::<String> {
///                     value: "undo".to_string(),
///                     index: 1usize,
///                     disabled: true,
///                     on_select: move |value| {
///                         tracing::info!("Selected: {}", value);
///                     },
///                     "Undo"
///                 }
///             }
///         }
///     }
/// }
/// ```
///
/// ## Styling
///
/// The [`DropdownMenu`] component defines the following data attributes you can use to control styling:
/// - `data-state`: Indicates the current state of the dropdown menu. values are `open` or `closed`.
/// - `data-disabled`: Indicates if the dropdown menu is disabled. values are `true` or `false`.
#[component]
pub fn DropdownMenu(props: DropdownMenuProps) -> Element {
    // See `DialogRoot`'s identical call for why this must be at the root,
    // not only inside `use_scroll_lock` (reached via `ScrollLockGuard`,
    // which mounts lazily inside `DropdownMenuContent`'s own open guard).
    use_effect(crate::scroll_lock::ensure_scrollbar_gutter_baseline);

    let (open, set_open) = use_controlled(props.open, props.default_open, props.on_open_change);

    let disabled = props.disabled;
    let trigger_id = use_unique_id();
    // Placeholder value until `DropdownMenuContent` mounts and syncs its
    // own id in -- see `DropdownMenuContext::content_id`'s doc.
    let content_id = use_unique_id();
    let interacted_outside = use_signal(|| false);
    let focus = use_collection_provider(props.roving_loop);
    let initial_focus = use_signal(|| None);
    let submenu_open_count = use_signal(|| 0usize);
    let mut ctx = use_context_provider(|| DropdownMenuContext {
        open,
        set_open,
        disabled,
        focus,
        initial_focus,
        trigger_id,
        content_id,
        modal: props.modal,
        interacted_outside,
        submenu_open_count,
    });

    use_effect(move || {
        let focused = focus.any_focused();
        // See `DropdownMenuContext::submenu_open_count`'s doc: a `.peek()`,
        // not a reactive read -- this effect's one reactive dependency
        // stays `focus.any_focused()` alone, matching `ctx.open.peek()`
        // just below it; the count only needs to be checked at the instant
        // focus-emptiness changes, not on every count change of its own.
        if *ctx.open.peek() != focused && *ctx.submenu_open_count.peek() == 0 {
            (ctx.set_open)(focused);
        }
    });

    // A fresh open shouldn't inherit an `interacted_outside` flag set by a
    // previous close -- otherwise an internal close (Escape, item select)
    // right after an outside-dismiss would wrongly skip refocusing the
    // trigger.
    use_effect(move || {
        if (ctx.open)() {
            ctx.interacted_outside.set(false);
        }
    });

    // APG menu-button: "Escape: Closes the menu and sets focus to the menu
    // button." Radix's `onCloseAutoFocus` skips the refocus when the close
    // was caused by interacting outside the menu (see `interacted_outside`
    // wiring on the trigger/item `onblur` handlers below).
    crate::use_refocus_on_close_unless(
        ctx.open,
        ctx.trigger_id,
        ReadSignal::new(ctx.interacted_outside),
    );

    // Handle escape key to close the menu
    let handle_keydown = move |event: Event<KeyboardData>| {
        if disabled() {
            return;
        }
        match event.key() {
            Key::Enter => {
                if open() {
                    ctx.set_open.call(false);
                } else {
                    ctx.open_with_focus(CollectionPlacement::First);
                }
            }
            Key::Character(c) if c == " " => {
                // APG menu-button (Optional): "Space: Opens the menu and
                // places focus on the first menu item." Mirrors the Enter
                // arm above -- both route through the same open-with-focus
                // path so neither can drift from the other the way this row
                // used to (opening via the native button click instead,
                // which refocused the trigger).
                if !open() {
                    ctx.open_with_focus(CollectionPlacement::First);
                }
            }
            Key::Escape => {
                // Web arm: `DropdownMenuContentRendered`'s `popover="auto"`
                // owns Escape dismissal natively (WHATWG HTML's light-dismiss
                // algorithm) -- returning here before `event.prevent_default()`
                // below leaves the key's default action alone so the browser's
                // own dismissal still runs. Calling both `set_open` *and*
                // `prevent_default` unconditionally here (the pre-migration
                // shape) would race that native algorithm, and the
                // `prevent_default` specifically would suppress the
                // browser's default action for the key outright -- the same
                // "unconditional keydown-suppressor must never run on the
                // web arm" lesson `PopoverContentRendered`'s doc documents
                // for `use_global_escape_listener` (docs/plan.md Phase
                // 4.4/4.2). This is a plain `cfg!()` compile-time branch on
                // ordinary code, not a conditionally-called hook, so it
                // carries none of that lesson's hook-order hazard -- no
                // component-boundary split needed for this one arm. Native
                // (Blitz) arm: unchanged, still closes here directly (Blitz
                // has no popover-API light dismiss to defer to).
                if cfg!(feature = "web") {
                    return;
                }
                ctx.set_open.call(false);
            }
            Key::ArrowDown => {
                // APG (Optional): "Down Arrow: opens the menu and moves
                // focus to the first menu item" from a closed trigger; once
                // open, plain roving-focus navigation.
                if open() {
                    ctx.focus.focus_next();
                } else {
                    ctx.open_with_focus(CollectionPlacement::First);
                }
            }
            Key::ArrowUp => {
                // APG (Optional): "Up Arrow: opens the menu and moves focus
                // to the last menu item" from a closed trigger.
                if open() {
                    ctx.focus.focus_prev();
                } else {
                    ctx.open_with_focus(CollectionPlacement::Last);
                }
            }
            Key::Home => ctx.focus.focus_first(),
            Key::End => ctx.focus.focus_last(),
            _ => return,
        }
        event.prevent_default();
    };

    rsx! {
        div {
            "data-state": if open() { "open" } else { "closed" },
            "data-disabled": (props.disabled)(),
            onkeydown: handle_keydown,
            ..props.attributes,
            {props.children}
        }
    }
}

/// The props for the [`DropdownMenuTrigger`] component
#[derive(Props, Clone, PartialEq)]
pub struct DropdownMenuTriggerProps {
    /// Render the trigger element as a custom component/element.
    #[props(default)]
    pub r#as: Option<Callback<Vec<Attribute>, Element>>,

    /// The ID of the trigger button. If not provided, an internally
    /// generated ID is used -- see `DropdownMenuTrigger`'s use of
    /// `use_id_or` for why this is a typed field rather than left to
    /// `GlobalAttributes` alone (docs/backlog.md row 36).
    pub id: ReadSignal<Option<String>>,

    /// Additional attributes to apply to the trigger element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
    /// The children of the trigger
    pub children: Element,
}

/// # DropdownMenuTrigger
///
/// The trigger button for the parent [`DropdownMenu`]. This button toggles the visibility of the [`DropdownMenuContent`].
///
/// This must be used inside a [`DropdownMenu`] component.
///
/// ## Example
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::dropdown_menu::{
///     DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger,
/// };
/// #[component]
/// fn Demo() -> Element {
///     rsx! {
///         DropdownMenu { default_open: false,
///             DropdownMenuTrigger { "Open Menu" }
///             DropdownMenuContent {
///                 DropdownMenuItem::<String> {
///                     value: "edit".to_string(),
///                     index: 0usize,
///                     on_select: move |value| {
///                         tracing::info!("Selected: {}", value);
///                     },
///                     "Edit"
///                 }
///                 DropdownMenuItem::<String> {
///                     value: "undo".to_string(),
///                     index: 1usize,
///                     disabled: true,
///                     on_select: move |value| {
///                         tracing::info!("Selected: {}", value);
///                     },
///                     "Undo"
///                 }
///             }
///         }
///     }
/// }
/// ```
///
/// ## Styling
///
/// The [`DropdownMenuTrigger`] component defines the following data attributes you can use to control styling:
/// - `data-state`: Indicates the current state of the dropdown menu. values are `open` or `closed`.
/// - `data-disabled`: Indicates if the dropdown menu is disabled. values are `true` or `false`.
#[component]
pub fn DropdownMenuTrigger(props: DropdownMenuTriggerProps) -> Element {
    let mut ctx: DropdownMenuContext = use_context();
    let mut element = use_signal(|| None::<Rc<MountedData>>);

    let open = ctx.open;
    let disabled = ctx.disabled;
    let data_state = if open() { "open" } else { "closed" };

    // docs/backlog.md row 36: `ctx.trigger_id` is the same signal
    // `DropdownMenuContent`'s `aria-labelledby` (below) and
    // `use_refocus_on_close_unless` both read back -- a caller-supplied
    // `id` here must resolve into it, not just win the render. Feeding it
    // straight into `use_id_or` as the generated-id signal does that:
    // `use_id_or`'s own effect writes a caller override back into its
    // `gen_id` argument, which *is* `ctx.trigger_id` here -- mirrors
    // `DialogTitle`'s identical `use_id_or(ctx.dialog_labelledby,
    // props.id)` (`dialog.rs`).
    let id = use_id_or(ctx.trigger_id, props.id);

    let base = attributes!(button {
        id: id.cloned(),
        r#type: "button",
        "data-state": data_state,
        "data-disabled": disabled,
        disabled: disabled,
        aria_expanded: open,
        aria_haspopup: crate::menu_semantics::MENU_TRIGGER_HASPOPUP,
        // See `crate::top_layer::anchor_name_style`: ties this trigger to
        // the web-arm content's `position-anchor` (`DropdownMenuContentRendered`)
        // so its anchor-positioned placement resolves relative to this
        // trigger once promoted to the top layer. Inert (empty) off the web
        // arm, and keyed on `ctx.content_id` -- not `ctx.trigger_id` above,
        // this trigger's own id -- for the same reason `PopoverTrigger`
        // does (see `PopoverCtx::content_id`'s doc in `popover.rs`).
        style: crate::top_layer::anchor_name_style(&ctx.content_id.cloned()),
        onmounted: move |e: MountedEvent| {
            element.set(Some(e.data()));
        },
        onclick: move |_| {
            if disabled() {
                return;
            }

            let new_open = !open();
            ctx.set_open.call(new_open);

            // Focus the element on click. Safari does not do this automatically.
            // https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/button#clicking_and_focus
            if let Some(data) = element() {
                spawn(async move {
                    _ = data.set_focus(true).await;
                });
            }
        },
        onblur: move |_| {
            // See `DropdownMenuContext::submenu_open_count`'s doc: an open
            // submenu's own first item is a legitimate place for focus to
            // land when this button blurs (a mouse-driven or hover-driven
            // submenu open never touches `ctx.focus`, this root's own
            // collection, at all), and must not read as "focus left the
            // whole menu."
            if !ctx.focus.any_focused() && *ctx.submenu_open_count.peek() == 0 {
                ctx.interacted_outside.set(true);
                ctx.focus.clear_focus();
                ctx.set_open.call(false);
            }
        },
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    if let Some(dynamic) = props.r#as {
        dynamic.call(merged)
    } else {
        rsx! {
            button {
                ..merged,
                {props.children}
            }
        }
    }
}

/// The props for the [`DropdownMenuContent`] component
#[derive(Props, Clone, PartialEq)]
pub struct DropdownMenuContentProps {
    /// The ID of the dropdown menu content element. If not provided, a unique ID will be generated.
    pub id: ReadSignal<Option<String>>,
    /// Additional attributes to apply to the dropdown menu content element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
    /// The children of the dropdown menu content, which should include one or more [`DropdownMenuItem`] components.
    pub children: Element,
}

/// # DropdownMenuTrigger
///
/// The contents of a [`DropdownMenu`]. The component will only be rendered when the parent [`DropdownMenu`] is open (as control by the [`DropdownMenuTrigger`]).
///
/// This must be used inside a [`DropdownMenu`] component.
///
/// ## Example
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::dropdown_menu::{
///     DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger,
/// };
/// #[component]
/// fn Demo() -> Element {
///     rsx! {
///         DropdownMenu { default_open: false,
///             DropdownMenuTrigger { "Open Menu" }
///             DropdownMenuContent {
///                 DropdownMenuItem::<String> {
///                     value: "edit".to_string(),
///                     index: 0usize,
///                     on_select: move |value| {
///                         tracing::info!("Selected: {}", value);
///                     },
///                     "Edit"
///                 }
///                 DropdownMenuItem::<String> {
///                     value: "undo".to_string(),
///                     index: 1usize,
///                     disabled: true,
///                     on_select: move |value| {
///                         tracing::info!("Selected: {}", value);
///                     },
///                     "Undo"
///                 }
///             }
///         }
///     }
/// }
/// ```
///
/// ## Styling
///
/// The [`DropdownMenuContent`] component defines the following data attributes you can use to control styling:
/// - `data-state`: Indicates the current state of the dropdown menu. values are `open` or `closed`.
#[component]
pub fn DropdownMenuContent(props: DropdownMenuContentProps) -> Element {
    let mut ctx: DropdownMenuContext = use_context();

    let unique_id = use_unique_id();
    let id = use_id_or(unique_id, props.id);

    // Keep `ctx.content_id` in sync with this content's actual id -- see
    // `DropdownMenuContext::content_id`'s doc. Mirrors `PopoverContent`'s
    // identical `ctx.content_id.set(id())` in `popover.rs`.
    use_effect(move || ctx.content_id.set(id()));

    let render = use_animated_open(id, ctx.open);

    // Apply the focus placement `open_with_focus` (`DropdownMenuContext`)
    // recorded when the menu was opened by keyboard, as soon as the
    // collection has items to focus -- which may be later than `render()`
    // first turning true, since items register via their own effects.
    // Mirrors `MenubarContent`'s identical call (`menubar.rs`).
    use_deferred_collection_focus(ctx.focus, ctx.initial_focus, render);

    // Lock page scroll while the menu is open and modal, matching Radix's
    // default. See docs/plan.md Phase 3.2. `DropdownMenuContent` itself
    // never unmounts (only the rendered content below does, via `render()`),
    // so the lock is held by `ScrollLockGuard` -- a child mounted inside
    // that same conditional -- rather than by this component directly; see
    // that guard's doc comment.
    let modal = ctx.modal;
    let open = ctx.open;
    let scroll_lock_active = use_memo(move || modal() && open());

    rsx! {
        if render() {
            DropdownMenuContentRendered {
                id: id.cloned(),
                attributes: props.attributes,
                scroll_lock_active,
                children: props.children,
            }
        }
    }
}

/// Web arm (docs/backlog.md item 2, non-modal-overlay migration): promote
/// the menu to the top layer via `popover="auto"` so it escapes
/// clipping/transformed ancestors, the same fix already shipped for
/// `Tooltip`/`HoverCard`/non-modal `Popover` (docs/plan.md Phase 4.4). This
/// also gains native light dismiss (WHATWG HTML's light-dismiss algorithm --
/// Escape and outside-pointerdown) for free from the platform.
///
/// This does *not* need a full `use_outside_dismiss`/`use_global_escape_
/// listener` removal the way `PopoverContentRendered` did, because
/// `DropdownMenu` never called either: it already dismisses on "focus left
/// the whole widget" via plain `onblur` handlers on `DropdownMenuTrigger`/
/// `DropdownMenuItem` (no hook, no JS listener, no unconditional
/// `preventDefault()`), which cannot fight the browser's own light-dismiss
/// default action and are left running unchanged on this arm -- a real
/// outside interaction still blurs whatever item/trigger had focus in the
/// common case, so the two mechanisms usually agree rather than race. What
/// *would* fight it is the root `DropdownMenu`'s keydown handler
/// unconditionally consuming Escape; see that handler's own comment for the
/// (non-hook, so no component-boundary split needed for that one arm)
/// carve-out.
///
/// `crate::top_layer::use_popover_sync` drives `showPopover()`/
/// `hidePopover()` from `open` and mirrors the browser's own `toggle` event
/// (fired on light dismiss, Escape, or any other close) back into
/// `set_open`, so the Rust signal can never strand the way `docs/
/// recommended-implementations.md` Caveat 1 documents for `<dialog>`'s old
/// one-way `showModal()`/`close()` binding. It also clears the focus
/// collection on that path -- see its callback's own comment.
#[cfg(feature = "web")]
#[component]
fn DropdownMenuContentRendered(
    id: String,
    attributes: Vec<Attribute>,
    scroll_lock_active: Memo<bool>,
    children: Element,
) -> Element {
    let mut ctx: DropdownMenuContext = use_context();
    let open = ctx.open;

    crate::top_layer::use_popover_sync(
        id.clone(),
        open,
        Callback::new(move |is_open: bool| {
            if !is_open {
                // Native light dismiss (Escape or outside pointerdown)
                // fired -- clear the focus collection too, so a stale
                // "something is still focused" reading can't disagree with
                // `open` on the next interaction (the root's own `open` <->
                // `focus.any_focused()` sync effect only reacts to focus
                // changes, not to this direct `set_open` call, so the two
                // must be kept in step here explicitly). Belt-and-suspenders
                // with the common case: the item that actually held DOM
                // focus is also blurred by the browser's own
                // `[popover]:not(:popover-open) { display: none }` UA rule
                // taking effect, which already clears focus via that item's
                // own `onblur` handler.
                ctx.focus.clear_focus();
                ctx.interacted_outside.set(true);
            }
            ctx.set_open.call(is_open);
        }),
    );
    // JS-measured static positioning fallback for engines without CSS
    // Anchor Positioning -- see `top_layer::use_anchor_position_fallback`'s
    // doc. `side`/`align` match the APG/Radix dropdown-menu default:
    // anchored below the trigger, left-aligned with it (not centered, the
    // `Tooltip`/`HoverCard`/`Popover` default -- a menu's items read
    // left-to-right from the trigger's own left edge).
    crate::top_layer::use_anchor_position_fallback(
        id.clone(),
        id.clone(),
        open,
        crate::ContentSide::Bottom,
        crate::ContentAlign::Start,
        4,
    );

    // See `tooltip.rs`'s `TooltipContentRendered` for why this hand-written,
    // never-`Styles::`-routed marker class exists: it is what the shared,
    // engine-injected anchor-positioning stylesheet (`top_layer::
    // ensure_anchor_positioning_styles`) selects on, sidestepping
    // `manganis-core`'s `css_module_parser` not scoping classes inside
    // `@supports` bodies (`docs/issues/css-module-supports-scoping.md`).
    // docs/backlog.md row 25's own construction, applied here too (this
    // component already carried `aria-labelledby`, but as a bare literal
    // alongside `..attributes` -- the duplicate-attribute hazard
    // `merge_attributes` exists to prevent, `docs/conformance-harness.md`
    // hydration-parity Rule 4, if a caller's own attribute list ever
    // carried a same-named, empty-valued `aria-label`/`aria-labelledby`;
    // see `has_own_accessible_name`'s own doc). Only contributed when the
    // caller hasn't already named this content some other way, and as its
    // own `merge_attributes` input rather than a literal.
    let labelledby: Vec<Attribute> = if has_own_accessible_name(&attributes) {
        Vec::new()
    } else {
        attributes!(div {
            aria_labelledby: "{ctx.trigger_id}"
        })
    };
    let attributes = merge_attributes(vec![
        attributes,
        attributes!(div {
            class: "dx-anchor-dropdown-menu"
        }),
        labelledby,
    ]);
    // Folds the caller's own `style` together with the anchor binding into
    // one `style` attribute -- see `top_layer::anchored_content_attributes`'s
    // doc for why a bare `style: position_anchor_style(&id)` literal
    // alongside `..attributes` is the duplicate-`style` hazard
    // (`docs/conformance-harness.md` hydration-parity Rule 4).
    let attributes = crate::top_layer::anchored_content_attributes(&id, attributes);

    rsx! {
        div {
            id: id.clone(),
            role: crate::menu_semantics::MENU_ROLE,
            popover: crate::top_layer::PopoverKind::Auto.as_str(),
            "data-state": if open() { "open" } else { "closed" },
            onpointerdown: move |event| {
                // The user is starting a click inside the dropdown menu.
                // Prevent the blur event from occurring during pointerdown,
                // to keep the dropdown menu open until pointerup happens,
                // thus enabling onclick/onselect events to fire.
                event.prevent_default();
                event.stop_propagation();
            },
            ..attributes,
            crate::scroll_lock::ScrollLockGuard { active: scroll_lock_active }
            {children}
        }
    }
}

/// Native (Blitz) arm: unchanged from before this slice -- Blitz has no
/// popover-API support at all (`docs/recommended-implementations.md`
/// Caveat 2), so light dismiss / Escape still need this crate's own
/// blur-driven dismissal, and this is the functional floor: a plain,
/// always-in-flow `div`, visible exactly when `render()` (`DropdownMenuContent`)
/// mounts it.
#[cfg(not(feature = "web"))]
#[component]
fn DropdownMenuContentRendered(
    id: String,
    attributes: Vec<Attribute>,
    scroll_lock_active: Memo<bool>,
    children: Element,
) -> Element {
    let ctx: DropdownMenuContext = use_context();

    // See the web arm's identical construction above (docs/backlog.md row
    // 25) for why this is conditional and routed through `merge_attributes`
    // rather than a bare literal alongside `..attributes`.
    let labelledby: Vec<Attribute> = if has_own_accessible_name(&attributes) {
        Vec::new()
    } else {
        attributes!(div {
            aria_labelledby: "{ctx.trigger_id}"
        })
    };
    let attributes = merge_attributes(vec![attributes, labelledby]);

    rsx! {
        div {
            id,
            role: crate::menu_semantics::MENU_ROLE,
            "data-state": if (ctx.open)() { "open" } else { "closed" },
            onpointerdown: move |event| {
                // The user is starting a click inside the dropdown menu.
                // Prevent the blur event from occurring during pointerdown,
                // to keep the dropdown menu open until pointerup happens,
                // thus enabling onclick/onselect events to fire.
                event.prevent_default();
                event.stop_propagation();
            },
            ..attributes,
            crate::scroll_lock::ScrollLockGuard { active: scroll_lock_active }
            {children}
        }
    }
}

/// The props for the [`DropdownMenuItem`] component
#[derive(Props, Clone, PartialEq)]
pub struct DropdownMenuItemProps<T: Clone + PartialEq + 'static> {
    /// The value of the item, which will be passed to the `on_select` callback when clicked.
    pub value: ReadSignal<T>,
    /// The index of the item within the [`DropdownMenuContent`]. This is used to order the items for keyboard navigation.
    pub index: ReadSignal<usize>,

    /// Whether the item is disabled. If true, the item will not be clickable and will not respond to keyboard events.
    /// Defaults to false.
    #[props(default)]
    pub disabled: ReadSignal<bool>,

    /// The callback function that will be called when the item is selected. The value of the item will be passed as an argument.
    #[props(default)]
    pub on_select: Callback<T>,

    /// Additional attributes to apply to the item element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
    /// The children of the item, which will be rendered inside the item element.
    pub children: Element,
}

/// # DropdownMenuTrigger
///
/// An item within a [`DropdownMenuContent`]. This component represents an individual selectable item in the dropdown menu.
///
/// This must be used inside a [`DropdownMenu`] component.
///
/// ## Example
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::dropdown_menu::{
///     DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger,
/// };
/// #[component]
/// fn Demo() -> Element {
///     rsx! {
///         DropdownMenu { default_open: false,
///             DropdownMenuTrigger { "Open Menu" }
///             DropdownMenuContent {
///                 DropdownMenuItem::<String> {
///                     value: "edit".to_string(),
///                     index: 0usize,
///                     on_select: move |value| {
///                         tracing::info!("Selected: {}", value);
///                     },
///                     "Edit"
///                 }
///                 DropdownMenuItem::<String> {
///                     value: "undo".to_string(),
///                     index: 1usize,
///                     disabled: true,
///                     on_select: move |value| {
///                         tracing::info!("Selected: {}", value);
///                     },
///                     "Undo"
///                 }
///             }
///         }
///     }
/// }
/// ```
///
/// ## Styling
///
/// The [`DropdownMenuItem`] component defines the following data attributes you can use to control styling:
/// - `data-disabled`: Indicates whether the item is disabled. Values are `true` or `false`.
#[component]
pub fn DropdownMenuItem<T: Clone + PartialEq + 'static>(
    props: DropdownMenuItemProps<T>,
) -> Element {
    let mut ctx: DropdownMenuContext = use_context();

    let disabled = move || (ctx.disabled)() || (props.disabled)();
    let item = use_item(collection_item(ctx.focus, props.index).disabled(disabled));
    let focused = move || item.focused();
    let onmounted = item.onmounted();

    rsx! {
        div {
            role: crate::menu_semantics::MENU_ITEM_ROLE,
            // Found investigating an axe `color-contrast` finding on this
            // pattern class's disabled state (docs/backlog.md row 39): the
            // `data-disabled` styling hook alone leaves this `role=
            // "menuitem"` exposed to assistive tech as a perfectly normal,
            // active item -- no `aria-disabled`, so nothing here signals
            // "not currently interactive," which is exactly the missing
            // piece a screen reader user needs, and exactly why axe read
            // this item's low-opacity muted text as a real, active-item
            // contrast defect rather than an exempt disabled one.
            // `ContextMenuItem` already sets this (`context_menu.rs`);
            // this was the same gap here and in `MenubarItem`.
            aria_disabled: disabled(),
            "data-disabled": disabled(),
            tabindex: if focused() { "0" } else { "-1" },

            onclick: move |e: Event<MouseData>| {
                e.stop_propagation();
                if !disabled() {
                    props.on_select.call((props.value)());
                    ctx.set_open.call(false);
                }
            },

            onkeydown: move |event: Event<KeyboardData>| {
                if event.key() == Key::Enter || event.key() == Key::Character(" ".to_string()) {
                    if !disabled() {
                        props.on_select.call((props.value)());
                        ctx.set_open.call(false);
                    }
                    event.prevent_default();
                    event.stop_propagation();
                }
            },

            onmounted,

            onblur: move |_| {
                if focused() {
                    ctx.interacted_outside.set(true);
                    ctx.focus.clear_focus();
                }
            },

            ..props.attributes,
            {props.children}
        }
    }
}

/// The props for the [`DropdownMenuSub`] component.
#[derive(Props, Clone, PartialEq)]
pub struct DropdownMenuSubProps {
    /// Whether the submenu is open. If not provided, the component is
    /// uncontrolled and uses `default_open`.
    pub open: ReadSignal<Option<bool>>,

    /// Default open state if the component is not controlled.
    #[props(default)]
    pub default_open: bool,

    /// Callback when the submenu's open state changes.
    #[props(default)]
    pub on_open_change: Callback<bool>,

    /// Whether focus should loop when reaching the end of this submenu's
    /// own items.
    #[props(default = ReadSignal::new(Signal::new(true)))]
    pub roving_loop: ReadSignal<bool>,

    /// The children of the submenu, which should include a
    /// [`DropdownMenuSubTrigger`] and a [`DropdownMenuSubContent`].
    pub children: Element,
}

/// # DropdownMenuSub
///
/// A submenu nested inside a [`DropdownMenu`] (or another
/// [`DropdownMenuContent`]/[`DropdownMenuSubContent`]), implementing the
/// APG "Menu and Menubar" pattern's nested-submenu contract -- what that
/// page calls a "parent menuitem" opening a submenu
/// (`content/patterns/menubar/menu-and-menubar-pattern.html`, "Keyboard
/// Interaction" and "WAI-ARIA Roles, States, and Properties" (both h2),
/// pinned commit `7e4034b262bc0d25332e330d8a582aaf34113829` of
/// `w3c/aria-practices` -- see `playwright/oracle/reference/README.md`;
/// [`DropdownMenuSubTrigger`]'s own doc carries the exact quotes). Renders
/// no element of its
/// own -- purely a context boundary around a [`DropdownMenuSubTrigger`]
/// and a [`DropdownMenuSubContent`], the same shape Radix's
/// `DropdownMenu.Sub` has.
///
/// The state machine (open/close, this submenu's own roving-focus
/// collection, the hover-intent timers its trigger/content use) is shared
/// with [`crate::context_menu`]'s identical `ContextMenuSub` via
/// `crate::menu_sub` -- see that module's doc for exactly what is shared
/// and why the rest (which enclosing collection a trigger registers in,
/// how the content is positioned) has to stay file-local. **Scope: one
/// level of nesting** -- a `DropdownMenuSub`'s own content may hold plain
/// items but not a further nested `DropdownMenuSub`; see `crate::menu_sub`'s
/// module doc for why.
///
/// This must be used inside a [`DropdownMenuContent`] (or another
/// [`DropdownMenuSubContent`]).
///
/// ## Example
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::dropdown_menu::{
///     DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSub,
///     DropdownMenuSubContent, DropdownMenuSubItem, DropdownMenuSubTrigger,
///     DropdownMenuTrigger,
/// };
/// #[component]
/// fn Demo() -> Element {
///     rsx! {
///         DropdownMenu { default_open: false,
///             DropdownMenuTrigger { "Open Menu" }
///             DropdownMenuContent {
///                 DropdownMenuItem::<String> {
///                     value: "edit".to_string(),
///                     index: 0usize,
///                     on_select: move |value| {
///                         tracing::info!("Selected: {}", value);
///                     },
///                     "Edit"
///                 }
///                 DropdownMenuSub {
///                     DropdownMenuSubTrigger { index: 1usize, "More tools" }
///                     DropdownMenuSubContent {
///                         DropdownMenuSubItem::<String> {
///                             value: "duplicate".to_string(),
///                             index: 0usize,
///                             on_select: move |value| {
///                                 tracing::info!("Selected: {}", value);
///                             },
///                             "Duplicate"
///                         }
///                     }
///                 }
///             }
///         }
///     }
/// }
/// ```
#[component]
pub fn DropdownMenuSub(props: DropdownMenuSubProps) -> Element {
    let (open, set_open) = use_controlled(props.open, props.default_open, props.on_open_change);
    let mut sub = crate::menu_sub::use_sub_menu_state(open, set_open, props.roving_loop);

    // Closing (however triggered -- Escape/ArrowLeft, hover-leave, an
    // outside interaction, or the enclosing menu closing entirely and
    // unmounting this component) must not let a later reopen inherit stale
    // focus state from a previous open. Mirrors `MenubarMenu`'s identical
    // cleanup effect (`menubar.rs`).
    use_effect(move || {
        if !(sub.open)() {
            sub.focus.clear_focus();
            sub.initial_focus.set(None);
        }
    });

    // Keep `ctx.submenu_open_count` (`DropdownMenuContext`'s own doc) equal
    // to "is *this* submenu currently open" at all times, so the root's own
    // "did focus really leave the whole menu" guards never misread this
    // submenu's own item legitimately holding focus as an outside blur.
    // `submenu_counted` (not `sub.open` alone) is what makes each
    // transition exactly-once: mount always starts with `sub.open` false
    // (uncontrolled default, or a controlled `open: Some(false)`), so a
    // naive "false branch decrements" would underflow the very first time
    // this effect runs, before this submenu was ever counted as open at
    // all. The cleanup below covers the one path this effect's own body
    // cannot -- the enclosing menu (or an ancestor submenu) unmounting this
    // `DropdownMenuSub` while `sub.open` is still `true`, which runs no
    // further effects, only cleanups.
    let mut ctx: DropdownMenuContext = use_context();
    let mut submenu_counted = use_signal(|| false);
    use_effect(move || {
        let is_open = (sub.open)();
        let was_counted = *submenu_counted.peek();
        if is_open && !was_counted {
            *ctx.submenu_open_count.write() += 1;
            submenu_counted.set(true);
        } else if !is_open && was_counted {
            *ctx.submenu_open_count.write() -= 1;
            submenu_counted.set(false);
        }
    });
    crate::use_effect_cleanup(move || {
        if *submenu_counted.peek() {
            *ctx.submenu_open_count.write() -= 1;
        }
    });

    use_context_provider(|| sub);

    rsx! {
        {props.children}
    }
}

/// The props for the [`DropdownMenuSubTrigger`] component.
#[derive(Props, Clone, PartialEq)]
pub struct DropdownMenuSubTriggerProps {
    /// The index of this sub-trigger within the enclosing
    /// [`DropdownMenuContent`]/[`DropdownMenuSubContent`]. Used to order it
    /// for keyboard navigation, the same as
    /// [`DropdownMenuItemProps::index`].
    pub index: ReadSignal<usize>,

    /// The ID of the sub-trigger element. If not provided, an internally
    /// generated ID is used -- see `DropdownMenuTrigger`'s identical use of
    /// `use_id_or` (`docs/backlog.md` row 36).
    pub id: ReadSignal<Option<String>>,

    /// Whether the sub-trigger (and its submenu) is disabled.
    #[props(default)]
    pub disabled: ReadSignal<bool>,

    /// Additional attributes to apply to the sub-trigger element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the sub-trigger, rendered as its visible label.
    pub children: Element,
}

/// # DropdownMenuSubTrigger
///
/// The trigger for a [`DropdownMenuSub`]'s submenu. Renders as a
/// `menuitem` within the *enclosing* menu -- APG calls this a "parent
/// menuitem": "If activating a menuitem opens a submenu, the menuitem is
/// known as a parent menuitem" ... "A parent menuitem has aria-haspopup
/// set to either menu or true" ... "A parent menuitem has aria-expanded
/// set to false when its child menu is not visible and set to true when
/// the child menu is visible" (`content/patterns/menubar/
/// menu-and-menubar-pattern.html`, "WAI-ARIA Roles, States, and
/// Properties" (h2), pinned commit
/// `7e4034b262bc0d25332e330d8a582aaf34113829` of `w3c/aria-practices` --
/// see `playwright/oracle/reference/README.md`). Also sets
/// `aria-controls` to the submenu's own content id, so the sub-trigger
/// unambiguously owns the popup it opens -- this crate's own choice, not
/// an APG-required property: the pattern page above never mentions
/// `aria-controls` at all (checked against the same pinned commit; no
/// match), so this is additive precision, not a cited requirement.
///
/// ## Keyboard
/// - `ArrowRight`/`Enter`/`Space`: same page, "Keyboard Interaction" (h2).
///   Right Arrow: "When focus is in a menu and on a menuitem that has a
///   submenu, opens the submenu and places focus on its first item."
///   Enter: "When focus is on a menuitem that has a submenu, opens the
///   submenu and places focus on its first item." Space (Optional): the
///   same as Enter.
///
/// ## Hover
/// A pointer hovering this trigger opens its submenu after a short
/// hover-intent delay (not an APG requirement -- see `crate::menu_sub`'s
/// doc for the citation-free rationale); hover-opening does **not** move
/// keyboard focus. Leaving the trigger without entering its submenu closes
/// it after a short grace delay, so a diagonal mouse move from the trigger
/// toward its own submenu content is not misread as "left."
///
/// This must be used inside a [`DropdownMenuSub`].
///
/// ## Styling
///
/// The [`DropdownMenuSubTrigger`] component defines the following data
/// attributes you can use to control styling:
/// - `data-state`: Indicates whether the submenu is open. Values are
///   `open` or `closed`.
/// - `data-disabled`: Indicates if the sub-trigger is disabled. Values are
///   `true` or `false`.
#[component]
pub fn DropdownMenuSubTrigger(props: DropdownMenuSubTriggerProps) -> Element {
    let ctx: DropdownMenuContext = use_context();
    let mut sub: crate::menu_sub::SubMenuState = use_context();

    let disabled = move || (ctx.disabled)() || (props.disabled)();
    // The sub-trigger is the focusable element, so it registers *itself* in
    // the enclosing menu's collection (`ctx.focus`), the same way
    // `MenubarTrigger` registers in `Menubar`'s row rather than its own
    // menu's collection (`menubar.rs`).
    let item = use_item(collection_item(ctx.focus, props.index).disabled(disabled));
    let focused = move || item.focused();
    let onmounted = item.onmounted();

    let mut hover_open = crate::menu_sub::use_delayed_action();
    let mut hover_close = crate::menu_sub::use_delayed_action();

    let id = use_id_or(sub.trigger_id, props.id);

    let base = attributes!(div {
        id: id.cloned(),
        role: crate::menu_semantics::MENU_ITEM_ROLE,
        aria_haspopup: crate::menu_semantics::MENU_TRIGGER_HASPOPUP,
        aria_expanded: sub.open,
        aria_controls: sub.content_id.cloned(),
        aria_disabled: disabled(),
        "data-disabled": disabled(),
        "data-state": if (sub.open)() { "open" } else { "closed" },
        tabindex: if focused() { "0" } else { "-1" },
        // See `crate::top_layer::anchor_name_style`: ties this trigger to
        // the web-arm submenu content's `position-anchor`
        // (`DropdownMenuSubContentRendered`), the same mechanism
        // `DropdownMenuTrigger` uses for the top-level content.
        style: crate::top_layer::anchor_name_style(&sub.content_id.cloned()),

        onmounted,

        onmouseenter: move |_| {
            if disabled() {
                return;
            }
            hover_close.cancel();
            hover_open.schedule(crate::menu_sub::SUBMENU_OPEN_INTENT_DELAY, move || {
                // Hover opens without moving keyboard focus -- see
                // `SubMenuState::initial_focus`'s doc for why this calls
                // `set_open` directly rather than `open_with_focus`.
                sub.set_open.call(true);
            });
        },
        onmouseleave: move |_| {
            hover_open.cancel();
            hover_close.schedule(crate::menu_sub::SUBMENU_CLOSE_GRACE_DELAY, move || {
                sub.set_open.call(false);
            });
        },

        onclick: move |event: Event<MouseData>| {
            event.stop_propagation();
            if disabled() {
                return;
            }
            hover_open.cancel();
            hover_close.cancel();
            // A pointer click (unlike a hover-intent open) does move focus
            // -- touch users have no hover state to rely on, so a tap here
            // must be a complete "open and land in the submenu" action.
            sub.open_with_focus(CollectionPlacement::First);
        },

        onkeydown: move |event: Event<KeyboardData>| {
            if disabled() {
                return;
            }
            match event.key() {
                // APG (see this component's doc): ArrowRight/Enter/Space
                // open the submenu and move focus to its first item.
                Key::ArrowRight | Key::Enter => {
                    sub.open_with_focus(CollectionPlacement::First);
                }
                Key::Character(c) if c == " " => {
                    sub.open_with_focus(CollectionPlacement::First);
                }
                _ => return,
            }
            event.prevent_default();
            event.stop_propagation();
        },

        onblur: move |_| {
            // Focus moving from the trigger into its own just-opened
            // submenu is an expected, internal transition (`sub.focus`
            // becomes non-empty as soon as the first item mounts and
            // `use_deferred_collection_focus` applies the placement
            // `open_with_focus` requested) -- only a blur that leaves
            // *nothing* in the submenu focused is a real "focus left this
            // sub-tree" signal. Never touches `ctx.focus` (the enclosing
            // menu's own collection) -- unlike `DropdownMenuItem`'s
            // identical-looking guard, closing a submenu must not look like
            // the *enclosing* menu itself lost focus.
            if focused() && !sub.focus.any_focused() {
                sub.set_open.call(false);
            }
        },
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        div {
            ..merged,
            {props.children}
        }
    }
}

/// The props for the [`DropdownMenuSubContent`] component.
#[derive(Props, Clone, PartialEq)]
pub struct DropdownMenuSubContentProps {
    /// The ID of the submenu content element. If not provided, a unique ID
    /// will be generated.
    pub id: ReadSignal<Option<String>>,
    /// Additional attributes to apply to the submenu content element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
    /// The children of the submenu content, which should include one or
    /// more [`DropdownMenuSubItem`] components.
    pub children: Element,
}

/// # DropdownMenuSubContent
///
/// The contents of a [`DropdownMenuSub`]'s submenu -- a `role="menu"`,
/// only rendered while its own [`DropdownMenuSubTrigger`] reports it open.
/// This must be used inside a [`DropdownMenuSub`].
///
/// ## Styling
///
/// The [`DropdownMenuSubContent`] component defines the following data
/// attributes you can use to control styling:
/// - `data-state`: Indicates the current state of the submenu. Values are
///   `open` or `closed`.
#[component]
pub fn DropdownMenuSubContent(props: DropdownMenuSubContentProps) -> Element {
    let mut sub: crate::menu_sub::SubMenuState = use_context();

    let unique_id = use_unique_id();
    let id = use_id_or(unique_id, props.id);

    // Keep `sub.content_id` in sync with this content's actual id -- see
    // `SubMenuState::content_id`'s doc. Mirrors `DropdownMenuContent`'s
    // identical `ctx.content_id.set(id())`.
    use_effect(move || sub.content_id.set(id()));

    let render = use_animated_open(id, sub.open);
    use_deferred_collection_focus(sub.focus, sub.initial_focus, render);

    rsx! {
        if render() {
            DropdownMenuSubContentRendered {
                id: id.cloned(),
                attributes: props.attributes,
                children: props.children,
            }
        }
    }
}

/// Web arm: promote this submenu's content to the top layer via
/// `popover="auto"`, the same mechanism `DropdownMenuContentRendered` uses
/// for the top-level content (see that component's doc for the general
/// `popover`/CSS-anchor wiring this mirrors) -- Phase 4.4's top-layer
/// engine already supports stacking one `popover="auto"` element on top of
/// another (`docs/plan.md` Phase 4.4; `docs/component-backlog.md` row 68
/// cites this landing as "removes the hardest part: nested stacking +
/// light dismiss"), so no new stacking mechanism is needed here.
///
/// `auto`, not `manual`: an outside pointerdown/Escape should light-dismiss
/// just this submenu the same way it dismisses a top-level
/// `DropdownMenuContent` -- and because a click landing on a sibling
/// element of the enclosing menu (outside this submenu specifically) is
/// exactly "outside" from this `auto` popover's own point of view, clicking
/// elsewhere in the parent menu closes an open submenu for free, with no
/// extra wiring, the same native mechanism this component's own
/// `onpointerdown` guard (below) protects from prematurely closing *this*
/// submenu when the click actually starts inside it.
#[cfg(feature = "web")]
#[component]
fn DropdownMenuSubContentRendered(
    id: String,
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let mut sub: crate::menu_sub::SubMenuState = use_context();
    let open = sub.open;

    crate::top_layer::use_popover_sync(
        id.clone(),
        open,
        Callback::new(move |is_open: bool| {
            if !is_open {
                sub.focus.clear_focus();
            }
            sub.set_open.call(is_open);
        }),
    );

    // Anchors to this submenu's own trigger, on the inline axis (`Right`
    // in LTR; `top_layer.rs`'s shared `position-try-fallbacks: flip-block,
    // flip-inline` -- and `use_anchor_position_fallback`'s own JS
    // fallback -- flip it to the opposite inline side when it would
    // overflow the viewport, the same mechanism every other anchored
    // overlay in this crate already relies on, not new positioning math).
    // APG does not mandate a submenu's side (implementation detail), but
    // opening to the side rather than below is how every menu-with-
    // submenus reference this crate has looked at (Radix, native OS menus)
    // renders one, so a submenu's items read as a continuation of the row
    // its trigger sits in. Zero gap (not the 4px `DropdownMenuContentRendered`
    // uses for its own trigger-relative gap): trigger and submenu sit
    // inside the *same* logical menu, so there is no affordance need for a
    // visible gap the way a top-level menu wants one below its button --
    // and a zero gap directly abuts the two elements, shrinking the
    // physical dead zone `SUBMENU_CLOSE_GRACE_DELAY` (`crate::menu_sub`)
    // exists to cover for a diagonal mouse move from trigger into content.
    crate::top_layer::use_anchor_position_fallback(
        id.clone(),
        id.clone(),
        open,
        crate::ContentSide::Right,
        crate::ContentAlign::Start,
        0,
    );

    // Reuses the `dx-anchor-dropdown-menu` marker class rather than a new
    // `dx-anchor-dropdown-menu-sub` one: `top_layer.rs`'s shared
    // anchor-positioning stylesheet (`ensure_anchor_positioning_styles`)
    // keys its `[data-side]` rules off a fixed, hand-enumerated list of
    // `dx-anchor-*` marker classes, and this lane cannot add an entry to
    // that list (`top_layer.rs` is out of scope -- another lane owns it,
    // per this task's own constraints). A submenu is positioned by exactly
    // the same `anchor()`/`position-try-fallbacks` rules as a top-level
    // `DropdownMenuContent` -- both are plain `popover="auto"` menus
    // anchored to a trigger, side/align/gap are the only per-call-site
    // knobs `use_anchor_position_fallback` exposes -- so reusing the
    // existing class is a correct fit, not a workaround standing in for a
    // missing one. `context_menu.rs`'s `ContextMenuSubContentRendered`
    // makes the identical choice for the same reason (its own host has no
    // `dx-anchor-context-menu` entry in that list at all, since its
    // top-level content is never anchored).
    let labelledby: Vec<Attribute> = if has_own_accessible_name(&attributes) {
        Vec::new()
    } else {
        attributes!(div {
            aria_labelledby: "{sub.trigger_id}"
        })
    };
    let attributes = merge_attributes(vec![
        attributes,
        attributes!(div {
            class: "dx-anchor-dropdown-menu"
        }),
        labelledby,
    ]);
    let attributes = crate::top_layer::anchored_content_attributes(&id, attributes);

    let trigger_id = sub.trigger_id;
    // APG Menu and Menubar pattern, "Keyboard Interaction" (h2, same pinned
    // section as `DropdownMenuSubTrigger`'s doc):
    //   Left Arrow: "When focus is in a submenu of an item in a menu,
    //   closes the submenu and returns focus to the parent menuitem."
    //   Escape (the pattern's one general rule, not submenu-specific --
    //   there is no separate submenu-only Escape row): "Close the menu
    //   that contains focus and return focus to the element or context,
    //   e.g., menu button or parent menuitem, from which the menu was
    //   opened." -- "the menu that contains focus" is this submenu, and
    //   "the ... parent menuitem ... from which [it] was opened" is this
    //   submenu's own trigger, so the general rule already covers the
    //   submenu case with no separate citation needed.
    // Handled here, directly on the submenu content, with
    // `stop_propagation()` -- not left to bubble to `DropdownMenu`'s own
    // root `onkeydown` (`dropdown_menu.rs`), which only knows how to close
    // the *root* menu, or (native/Blitz arm) would do exactly that if this
    // key reached it. ArrowDown/ArrowUp/Home/End are this submenu's own
    // roving-focus navigation, scoped to `sub.focus` (this submenu's own
    // collection, never the enclosing menu's) -- mirrors
    // `ContextMenuContentRendered`'s identical content-level `onkeydown`
    // shape (`context_menu.rs`), the closer model here than `DropdownMenu`'s
    // root-owns-navigation one, since `DropdownMenuSub` deliberately
    // renders no wrapper element of its own to hang a root handler on.
    let onkeydown = move |event: Event<KeyboardData>| {
        match event.key() {
            Key::Escape | Key::ArrowLeft => {
                sub.set_open.call(false);
                sub.focus.clear_focus();
                let trigger_id = trigger_id.cloned();
                dioxus::document::eval(&format!(
                    "var e=document.getElementById('{trigger_id}');if(e)e.focus()"
                ));
            }
            Key::ArrowDown => sub.focus.focus_next(),
            Key::ArrowUp => sub.focus.focus_prev(),
            Key::Home => sub.focus.focus_first(),
            Key::End => sub.focus.focus_last(),
            _ => return,
        }
        event.prevent_default();
        event.stop_propagation();
    };

    rsx! {
        div {
            id: id.clone(),
            role: crate::menu_semantics::MENU_ROLE,
            popover: crate::top_layer::PopoverKind::Auto.as_str(),
            "data-state": if open() { "open" } else { "closed" },
            onkeydown,
            onpointerdown: move |event| {
                // Same rationale as `DropdownMenuContentRendered`'s
                // identical guard: keep this submenu open across the
                // pointerdown-to-pointerup gap of a click inside it, so
                // `onclick`/`on_select` still fire.
                event.prevent_default();
                event.stop_propagation();
            },
            ..attributes,
            {children}
        }
    }
}

/// Native (Blitz) arm: Blitz has no popover-API support at all
/// (`docs/recommended-implementations.md` Caveat 2), so this stays the
/// functional floor -- a plain, always-in-flow `div`, visible exactly when
/// `render()` (`DropdownMenuSubContent`) mounts it. Mirrors
/// `DropdownMenuContentRendered`'s identical native arm.
#[cfg(not(feature = "web"))]
#[component]
fn DropdownMenuSubContentRendered(
    id: String,
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let mut sub: crate::menu_sub::SubMenuState = use_context();
    let open = sub.open;

    let labelledby: Vec<Attribute> = if has_own_accessible_name(&attributes) {
        Vec::new()
    } else {
        attributes!(div {
            aria_labelledby: "{sub.trigger_id}"
        })
    };
    let attributes = merge_attributes(vec![attributes, labelledby]);

    let trigger_id = sub.trigger_id;
    let onkeydown = move |event: Event<KeyboardData>| {
        match event.key() {
            Key::Escape | Key::ArrowLeft => {
                sub.set_open.call(false);
                sub.focus.clear_focus();
                let trigger_id = trigger_id.cloned();
                dioxus::document::eval(&format!(
                    "var e=document.getElementById('{trigger_id}');if(e)e.focus()"
                ));
            }
            Key::ArrowDown => sub.focus.focus_next(),
            Key::ArrowUp => sub.focus.focus_prev(),
            Key::Home => sub.focus.focus_first(),
            Key::End => sub.focus.focus_last(),
            _ => return,
        }
        event.prevent_default();
        event.stop_propagation();
    };

    rsx! {
        div {
            id,
            role: crate::menu_semantics::MENU_ROLE,
            "data-state": if open() { "open" } else { "closed" },
            onkeydown,
            onpointerdown: move |event| {
                event.prevent_default();
                event.stop_propagation();
            },
            ..attributes,
            {children}
        }
    }
}

/// The props for the [`DropdownMenuSubItem`] component.
#[derive(Props, Clone, PartialEq)]
pub struct DropdownMenuSubItemProps<T: Clone + PartialEq + 'static> {
    /// The value of the item, which will be passed to the `on_select`
    /// callback when clicked.
    pub value: ReadSignal<T>,
    /// The index of the item within the enclosing [`DropdownMenuSubContent`].
    /// This is used to order the items for keyboard navigation.
    pub index: ReadSignal<usize>,

    /// Whether the item is disabled. Defaults to false.
    #[props(default)]
    pub disabled: ReadSignal<bool>,

    /// The callback function that will be called when the item is
    /// selected. The value of the item will be passed as an argument.
    #[props(default)]
    pub on_select: Callback<T>,

    /// Additional attributes to apply to the item element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
    /// The children of the item, which will be rendered inside the item
    /// element.
    pub children: Element,
}

/// # DropdownMenuSubItem
///
/// An item within a [`DropdownMenuSubContent`]. Behaves like
/// [`DropdownMenuItem`], except selecting it closes the **entire** menu
/// tree (this submenu and every ancestor, including the root
/// [`DropdownMenu`]), not just this submenu -- matching Radix's default
/// `DropdownMenu.Sub` behavior, and reached by calling the *root* menu's
/// own `set_open` (found via `DropdownMenuContext`, not this submenu's own
/// `crate::menu_sub::SubMenuState`).
///
/// This must be used inside a [`DropdownMenuSubContent`].
///
/// ## Styling
///
/// The [`DropdownMenuSubItem`] component defines the following data
/// attributes you can use to control styling:
/// - `data-disabled`: Indicates whether the item is disabled. Values are
///   `true` or `false`.
#[component]
pub fn DropdownMenuSubItem<T: Clone + PartialEq + 'static>(
    props: DropdownMenuSubItemProps<T>,
) -> Element {
    let ctx: DropdownMenuContext = use_context();
    let mut sub: crate::menu_sub::SubMenuState = use_context();

    let disabled = move || (ctx.disabled)() || (props.disabled)();
    let item = use_item(collection_item(sub.focus, props.index).disabled(disabled));
    let focused = move || item.focused();
    let onmounted = item.onmounted();

    rsx! {
        div {
            role: crate::menu_semantics::MENU_ITEM_ROLE,
            aria_disabled: disabled(),
            "data-disabled": disabled(),
            tabindex: if focused() { "0" } else { "-1" },

            onclick: move |e: Event<MouseData>| {
                e.stop_propagation();
                if !disabled() {
                    props.on_select.call((props.value)());
                    ctx.set_open.call(false);
                }
            },

            onkeydown: move |event: Event<KeyboardData>| {
                if event.key() == Key::Enter || event.key() == Key::Character(" ".to_string()) {
                    if !disabled() {
                        props.on_select.call((props.value)());
                        ctx.set_open.call(false);
                    }
                    event.prevent_default();
                    event.stop_propagation();
                }
            },

            onmounted,

            onblur: move |_| {
                // Unlike `DropdownMenuItem`'s identical-looking guard,
                // which clears `ctx.focus` (the *root* collection), this
                // must clear only `sub.focus` (this submenu's own) --
                // clearing the root's would wrongly read as "the entire
                // menu lost focus" (see `DropdownMenu`'s own
                // `open <-> focus.any_focused()` sync effect) every time
                // focus moves off the last item in an open submenu, even
                // though the parent menu is still legitimately open.
                if focused() {
                    sub.focus.clear_focus();
                }
            },

            ..props.attributes,
            {props.children}
        }
    }
}
