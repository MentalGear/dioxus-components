//! Defines the [`DropdownMenu`] component and its subcomponents.

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
    });

    use_effect(move || {
        let focused = focus.any_focused();
        if *ctx.open.peek() != focused {
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

    let base = attributes!(button {
        id: ctx.trigger_id,
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
            if !ctx.focus.any_focused() {
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

    rsx! {
        div {
            id: id.clone(),
            role: crate::menu_semantics::MENU_ROLE,
            popover: crate::top_layer::PopoverKind::Auto.as_str(),
            style: crate::top_layer::position_anchor_style(&id),
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
