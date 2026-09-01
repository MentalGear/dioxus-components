//! Defines the [`Menubar`] component and its sub-components.

use dioxus::prelude::*;
#[cfg(target_family = "wasm")]
use dioxus_attributes::attributes;

#[cfg(target_family = "wasm")]
use crate::merge_attributes;
use crate::{
    collection::{
        collection_item, use_collection_provider, use_deferred_collection_focus, use_item,
        CollectionPlacement, CollectionState,
    },
    use_animated_open, use_id_or, use_unique_id,
};

#[derive(Clone, Copy)]
struct MenubarContext {
    // Currently open menu index
    open_menu: Signal<Option<usize>>,
    set_open_menu: Callback<Option<usize>>,
    disabled: ReadSignal<bool>,

    // Focus state
    focus: CollectionState,
}

/// The props for the [`Menubar`] component.
#[derive(Props, Clone, PartialEq)]
pub struct MenubarProps {
    /// Whether the menubar is disabled.
    #[props(default)]
    pub disabled: ReadSignal<bool>,

    /// Whether focus should loop around when reaching the end.
    #[props(default = ReadSignal::new(Signal::new(true)))]
    pub roving_loop: ReadSignal<bool>,

    /// Additional attributes to apply to the menubar element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
    /// The children of the menubar component.
    pub children: Element,
}

/// # Menubar
///
/// The `Menubar` component creates a menu bar that allows users to define multiple grouped dropdowns.
/// Each dropdown menu is represented by a [`MenubarMenu`] component with an associated trigger and content.
///
/// ## Example
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::menubar::{
///     Menubar, MenubarContent, MenubarItem, MenubarMenu, MenubarTrigger,
/// };
/// #[component]
/// fn Demo() -> Element {
///     rsx! {
///         Menubar {
///             MenubarMenu { index: 0usize,
///                 MenubarTrigger { "File" }
///                 MenubarContent {
///                     MenubarItem {
///                         index: 0usize,
///                         value: "new".to_string(),
///                         on_select: move |value| {
///                             tracing::info!("Selected value: {}", value);
///                         },
///                         "New"
///                     }
///                     MenubarItem {
///                         index: 1usize,
///                         value: "open".to_string(),
///                         on_select: move |value| {
///                             tracing::info!("Selected value: {}", value);
///                         },
///                         "Open"
///                     }
///                 }
///             }
///             MenubarMenu { index: 1usize,
///                 MenubarTrigger { "Edit" }
///                 MenubarContent {
///                     MenubarItem {
///                         index: 0usize,
///                         value: "cut".to_string(),
///                         on_select: move |value| {
///                             tracing::info!("Selected value: {}", value);
///                         },
///                         "Cut"
///                     }
///                     MenubarItem {
///                         index: 1usize,
///                         value: "copy".to_string(),
///                         on_select: move |value| {
///                             tracing::info!("Selected value: {}", value);
///                         },
///                         "Copy"
///                     }
///                 }
///             }
///         }
///     }
/// }
/// ```
///
/// ## Styling
///
/// The [`Menubar`] component defines the following data attributes you can use to control styling:
/// - `data-disabled`: Indicates if the menubar is disabled. Values are `true` or `false`.
#[component]
pub fn Menubar(props: MenubarProps) -> Element {
    let mut open_menu = use_signal(|| None);
    let set_open_menu = use_callback(move |idx| open_menu.set(idx));

    let focus = use_collection_provider(props.roving_loop);
    let mut ctx = use_context_provider(|| MenubarContext {
        open_menu,
        set_open_menu,
        disabled: props.disabled,
        focus,
    });
    use_effect(move || {
        let index = ctx.focus.focused_index();
        if ctx.open_menu.peek().is_some() {
            ctx.set_open_menu.call(index);
        }
    });

    rsx! {
        div {
            role: "menubar",
            "data-disabled": (props.disabled)(),
            tabindex: (!ctx.focus.any_focused()).then_some("0"),
            // If the menu receives focus, focus the most recently focused menu item.
            onfocus: move |_| {
                ctx.focus.set_focus(Some(ctx.focus.recent_focus_or_default()));
            },

            ..props.attributes,

            {props.children}
        }
    }
}

#[derive(Clone, Copy)]
struct MenubarMenuContext {
    index: ReadSignal<usize>,
    focus: CollectionState,
    is_open: Memo<bool>,
    disabled: ReadSignal<bool>,
    initial_focus: Signal<Option<CollectionPlacement>>,

    // The current `MenubarContent`'s own element id for *this* menu, kept
    // in sync by that component -- mirrors `DropdownMenuContext::content_id`
    // (`dropdown_menu.rs`). `MenubarTrigger`'s `anchor-name` must key off
    // this signal (not some menubar-wide id) so each menu's content anchors
    // to *its own* trigger, not another menu's -- see
    // `DropdownMenuContext::content_id`'s doc for the exact bug this guards
    // against if trigger and content ever named different ids.
    content_id: Signal<String>,
}

impl MenubarMenuContext {
    fn focus_next(&mut self) {
        self.focus.focus_next();
    }

    fn focus_prev(&mut self) {
        self.focus.focus_prev();
    }
}

/// The props for the [`MenubarMenu`] component.
#[derive(Props, Clone, PartialEq)]
pub struct MenubarMenuProps {
    /// The index of this menu in the menubar. This is used to define the focus order for keyboard navigation.
    pub index: ReadSignal<usize>,

    /// Whether this menu is disabled.
    #[props(default)]
    pub disabled: ReadSignal<bool>,

    /// Additional attributes to apply to the menu element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
    /// The children of the menu component.
    pub children: Element,
}

/// # MenubarMenu
///
/// The `MenubarMenu` component represents a single menu within a menubar. It contains a [`MenubarTrigger`]
/// to open the menu and a [`MenubarContent`] that holds the menu items. Each menu must define an index
/// to establish its position within the menubar.
///
/// This must be used inside a [`Menubar`] component.
///
/// ## Example
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::menubar::{
///     Menubar, MenubarContent, MenubarItem, MenubarMenu, MenubarTrigger,
/// };
/// #[component]
/// fn Demo() -> Element {
///     rsx! {
///         Menubar {
///             MenubarMenu { index: 0usize,
///                 MenubarTrigger { "File" }
///                 MenubarContent {
///                     MenubarItem {
///                         index: 0usize,
///                         value: "new".to_string(),
///                         on_select: move |value| {
///                             tracing::info!("Selected value: {}", value);
///                         },
///                         "New"
///                     }
///                     MenubarItem {
///                         index: 1usize,
///                         value: "open".to_string(),
///                         on_select: move |value| {
///                             tracing::info!("Selected value: {}", value);
///                         },
///                         "Open"
///                     }
///                 }
///             }
///             MenubarMenu { index: 1usize,
///                 MenubarTrigger { "Edit" }
///                 MenubarContent {
///                     MenubarItem {
///                         index: 0usize,
///                         value: "cut".to_string(),
///                         on_select: move |value| {
///                             tracing::info!("Selected value: {}", value);
///                         },
///                         "Cut"
///                     }
///                     MenubarItem {
///                         index: 1usize,
///                         value: "copy".to_string(),
///                         on_select: move |value| {
///                             tracing::info!("Selected value: {}", value);
///                         },
///                         "Copy"
///                     }
///                 }
///             }
///         }
///     }
/// }
/// ```
///
/// ## Styling
///
/// The [`MenubarMenu`] component defines the following data attributes you can use to control styling:
/// - `data-state`: Indicates if the menu is open or closed. Values are `open` or `closed`.
/// - `data-disabled`: Indicates if the menu is disabled. Values are `true` or `false`.
#[component]
pub fn MenubarMenu(props: MenubarMenuProps) -> Element {
    let mut ctx: MenubarContext = use_context();
    let is_open = use_memo(move || (ctx.open_menu)() == Some(props.index.cloned()));
    let focus = use_collection_provider(ctx.focus.loop_signal());
    let initial_focus = use_signal(|| None);
    let disabled = move || (ctx.disabled)() || (props.disabled)();
    // Placeholder value until `MenubarContent` mounts and syncs its own id
    // in -- see `MenubarMenuContext::content_id`'s doc.
    let content_id = use_unique_id();

    let mut menu_ctx = use_context_provider(|| MenubarMenuContext {
        index: props.index,
        focus,
        is_open,
        disabled: props.disabled,
        initial_focus,
        content_id,
    });

    use_effect(move || {
        if !is_open() {
            menu_ctx.focus.clear_focus();
            menu_ctx.initial_focus.set(None);
        }
    });

    rsx! {
        div {
            role: "menu",
            "data-state": if is_open() { "open" } else { "closed" },
            "data-disabled": (ctx.disabled)() || (props.disabled)(),

            onkeydown: move |event: Event<KeyboardData>| {
                match event.key() {
                    Key::Enter if !disabled() => {
                        ctx.set_open_menu.call((!is_open()).then(&*props.index));
                    }
                    Key::Escape => {
                        ctx.set_open_menu.call(None);
                        // APG menubar: "Escape: ... sets focus to the
                        // menubar." Use the existing collection-focus
                        // mechanism (not the shared refocus-on-close hook --
                        // see docs/plan.md Phase 3.1) to move focus back to
                        // this menu's own trigger item. `ctx.focus` never
                        // stopped being this trigger's own index while
                        // keyboard focus roamed *within* the open submenu
                        // (`menu_ctx.focus` tracks that separately), so
                        // `set_focus` alone would be a same-value no-op that
                        // never re-runs `control_mount_focus` -- clear first
                        // to force a real transition.
                        ctx.focus.clear_focus();
                        ctx.focus.set_focus(Some(props.index.cloned()));
                    }
                    Key::ArrowLeft => ctx.focus.focus_prev(),
                    Key::ArrowRight => ctx.focus.focus_next(),
                    Key::ArrowDown if !disabled() => {
                        if !is_open() {
                            menu_ctx.initial_focus.set(Some(CollectionPlacement::First));
                            ctx.set_open_menu.call(Some(props.index.cloned()));
                        } else {
                            menu_ctx.focus_next();
                        }
                    },
                    Key::ArrowUp if !disabled() => {
                        if is_open() {
                            menu_ctx.focus_prev();
                        } else {
                            menu_ctx.initial_focus.set(Some(CollectionPlacement::Last));
                            ctx.set_open_menu.call(Some(props.index.cloned()));
                        }
                    },
                    Key::Home => ctx.focus.focus_first(),
                    Key::End => ctx.focus.focus_last(),
                    _ => return,
                }
                event.prevent_default();
            },

            ..props.attributes,
            {props.children}
        }
    }
}

/// The props for the [`MenubarTrigger`] component.
#[derive(Props, Clone, PartialEq)]
pub struct MenubarTriggerProps {
    /// Additional attributes to apply to the trigger element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
    /// The children of the trigger component.
    pub children: Element,
}

/// # MenubarTrigger
///
/// The `MenubarTrigger` component is a button that opens and closes a [`MenubarMenu`] when clicked.
///
/// This must be used inside a [`MenubarMenu`] component.
///
/// ## Example
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::menubar::{
///     Menubar, MenubarContent, MenubarItem, MenubarMenu, MenubarTrigger,
/// };
/// #[component]
/// fn Demo() -> Element {
///     rsx! {
///         Menubar {
///             MenubarMenu { index: 0usize,
///                 MenubarTrigger { "File" }
///                 MenubarContent {
///                     MenubarItem {
///                         index: 0usize,
///                         value: "new".to_string(),
///                         on_select: move |value| {
///                             tracing::info!("Selected value: {}", value);
///                         },
///                         "New"
///                     }
///                     MenubarItem {
///                         index: 1usize,
///                         value: "open".to_string(),
///                         on_select: move |value| {
///                             tracing::info!("Selected value: {}", value);
///                         },
///                         "Open"
///                     }
///                 }
///             }
///             MenubarMenu { index: 1usize,
///                 MenubarTrigger { "Edit" }
///                 MenubarContent {
///                     MenubarItem {
///                         index: 0usize,
///                         value: "cut".to_string(),
///                         on_select: move |value| {
///                             tracing::info!("Selected value: {}", value);
///                         },
///                         "Cut"
///                     }
///                     MenubarItem {
///                         index: 1usize,
///                         value: "copy".to_string(),
///                         on_select: move |value| {
///                             tracing::info!("Selected value: {}", value);
///                         },
///                         "Copy"
///                     }
///                 }
///             }
///         }
///     }
/// }
/// ```
#[component]
pub fn MenubarTrigger(props: MenubarTriggerProps) -> Element {
    let mut ctx: MenubarContext = use_context();
    let menu_ctx: MenubarMenuContext = use_context();
    let disabled = move || (ctx.disabled)() || (menu_ctx.disabled)();
    // The menu's trigger is the focusable element, so it registers the menu in
    // the parent menubar collection.
    let item = use_item(collection_item(ctx.focus, menu_ctx.index).disabled(disabled));
    let onmounted = item.onmounted();
    let is_open = menu_ctx.is_open;
    let index = menu_ctx.index;
    let is_focused = move || item.focused() && !menu_ctx.focus.any_focused();

    rsx! {
        button {
            onmounted,
            // See `crate::top_layer::anchor_name_style`: ties this trigger
            // to the web-arm content's `position-anchor`
            // (`MenubarContentRendered`) so its anchor-positioned placement
            // resolves relative to *this* trigger once promoted to the top
            // layer. Inert (empty) off the web arm, and keyed on
            // `menu_ctx.content_id` -- not this menu's own index -- for the
            // same reason `DropdownMenuTrigger` keys off
            // `ctx.content_id` (see that doc).
            style: crate::top_layer::anchor_name_style(&menu_ctx.content_id.cloned()),
            onpointerup: move |_| {
                if !disabled() {
                    let new_open = if is_open() { None } else { Some(index.cloned()) };
                    ctx.set_open_menu.call(new_open);
                    ctx.focus.set_focus(Some(index.cloned()));
                }
            },
            onmouseenter: move |_| {
                if !disabled() && (ctx.open_menu)().is_some() {
                    ctx.focus.set_focus(Some(index.cloned()));
                }
            },
            onblur: move |_| {
                if is_focused() {
                    ctx.focus.clear_focus();
                    ctx.set_open_menu.call(None);
                }
            },
            role: "menuitem",
            type: "button",
            tabindex: if is_focused() { "0" } else { "-1" },
            ..props.attributes,
            {props.children}
        }
    }
}

/// The props for the [`MenubarContent`] component.
#[derive(Props, Clone, PartialEq)]
pub struct MenubarContentProps {
    /// The id of the content element.
    pub id: ReadSignal<Option<String>>,
    /// Additional attributes to apply to the content element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
    /// The children of the content component.
    pub children: Element,
}

/// # MenubarContent
///
/// The `MenubarContent` component defines the content of a [`MenubarMenu`]. It will only be rendered if the menu is open.
///
/// This must be used inside a [`MenubarMenu`] component.
///
/// ## Example
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::menubar::{
///     Menubar, MenubarContent, MenubarItem, MenubarMenu, MenubarTrigger,
/// };
/// #[component]
/// fn Demo() -> Element {
///     rsx! {
///         Menubar {
///             MenubarMenu { index: 0usize,
///                 MenubarTrigger { "File" }
///                 MenubarContent {
///                     MenubarItem {
///                         index: 0usize,
///                         value: "new".to_string(),
///                         on_select: move |value| {
///                             tracing::info!("Selected value: {}", value);
///                         },
///                         "New"
///                     }
///                     MenubarItem {
///                         index: 1usize,
///                         value: "open".to_string(),
///                         on_select: move |value| {
///                             tracing::info!("Selected value: {}", value);
///                         },
///                         "Open"
///                     }
///                 }
///             }
///             MenubarMenu { index: 1usize,
///                 MenubarTrigger { "Edit" }
///                 MenubarContent {
///                     MenubarItem {
///                         index: 0usize,
///                         value: "cut".to_string(),
///                         on_select: move |value| {
///                             tracing::info!("Selected value: {}", value);
///                         },
///                         "Cut"
///                     }
///                     MenubarItem {
///                         index: 1usize,
///                         value: "copy".to_string(),
///                         on_select: move |value| {
///                             tracing::info!("Selected value: {}", value);
///                         },
///                         "Copy"
///                     }
///                 }
///             }
///         }
///     }
/// }
/// ```
///
/// ## Styling
///
/// The [`MenubarContent`] component defines the following data attributes you can use to control styling:
/// - `data-state`: Indicates if the menu is open or closed. Values are `open` or `closed`.
#[component]
pub fn MenubarContent(props: MenubarContentProps) -> Element {
    let mut menu_ctx: MenubarMenuContext = use_context();

    let unique_id = use_unique_id();
    let id = use_id_or(unique_id, props.id);

    // Keep `menu_ctx.content_id` in sync with this content's actual id --
    // see `MenubarMenuContext::content_id`'s doc. Mirrors
    // `DropdownMenuContent`'s identical `ctx.content_id.set(id())`
    // (`dropdown_menu.rs`).
    use_effect(move || menu_ctx.content_id.set(id()));

    let render = use_animated_open(id, menu_ctx.is_open);
    use_deferred_collection_focus(menu_ctx.focus, menu_ctx.initial_focus, render);

    rsx! {
        if render() {
            MenubarContentRendered {
                id: id.cloned(),
                attributes: props.attributes,
                children: props.children,
            }
        }
    }
}

/// Web arm (Migration A, slice 2/3): promote each menu's content to the top
/// layer via `popover="auto"`, anchored to its own trigger -- the same
/// mechanism `DropdownMenuContentRendered` uses (`dropdown_menu.rs`), which
/// see for the general `popover`/CSS-anchor wiring this mirrors.
///
/// `auto`, not `manual`: unlike `ContextMenu` (point-opened, no persistent
/// trigger to key dismissal off of), each `MenubarMenu` already has exactly
/// the shape `DropdownMenu` does -- a trigger button plus content anchored
/// to it -- and `auto`'s native light dismiss (Escape, outside pointerdown)
/// is free, spec-correct insurance alongside this crate's own blur-driven
/// close (`MenubarTrigger`/`MenubarItem`'s `onblur` below), not a
/// competing mechanism: closing this content -- by any means, native or
/// Rust-driven -- makes the previously-focused item within it not
/// focusable (`[popover]:not(:popover-open) { display: none }`), which the
/// UA blurs synchronously as part of hiding it, so the *existing*
/// `onblur`-driven close-and-focus-clear logic still fires and still owns
/// what happens next; native light dismiss only ever supplies the same
/// "something outside happened" signal blur already does, for the rare
/// outside interaction that would not otherwise cause a blur (verified
/// during this migration's design pass; see the `use_popover_sync`
/// callback below).
///
/// Escape is still handled entirely by `MenubarMenu`'s own `onkeydown`
/// (unchanged, still calls `prevent_default()` unconditionally): APG
/// requires Escape to move focus to *this specific menu's own trigger*
/// (`ctx.focus.set_focus(Some(props.index))`), which native light dismiss
/// has no way to know how to do (it has no notion of "which trigger opened
/// this"), so this component keeps driving that close+refocus itself
/// rather than deferring to the platform the way `DropdownMenuContent`
/// does -- there is exactly one trigger for `DropdownMenu`, so "return
/// focus to the trigger" needs no such per-index bookkeeping and can be
/// deferred; here it cannot, so `prevent_default()` staying unconditional
/// (no compile-time wasm skip, unlike `DropdownMenu`'s Escape arm) is the
/// correct choice, not an oversight.
#[cfg(target_family = "wasm")]
#[component]
fn MenubarContentRendered(id: String, attributes: Vec<Attribute>, children: Element) -> Element {
    let ctx: MenubarContext = use_context();
    let menu_ctx: MenubarMenuContext = use_context();
    let open = menu_ctx.is_open;
    let index = menu_ctx.index;

    // Drive `showPopover()`/`hidePopover()` from `open`, and sync a native
    // close back into `ctx.open_menu` -- but *only* when this menu is still
    // the one recorded as open. Confirmed necessary by execution's design
    // pass, not defensive boilerplate: `Menubar`'s own effect
    // (`Menubar`'s `use_effect` reacting to `ctx.focus.focused_index()`)
    // can move `ctx.open_menu` straight from `Some(this index)` to
    // `Some(another index)` in one step (arrow-key navigation between
    // triggers while a menu is open) -- and per WHATWG HTML, showing the
    // *new* menu's `auto` popover natively closes this (now-unrelated)
    // sibling `auto` popover for us, firing this same `toggle` callback
    // with `is_open: false` *after* `ctx.open_menu` already points at the
    // other menu. An unconditional `ctx.set_open_menu.call(None)` here
    // would then wrongly close the menu the user just switched to. Guarding
    // on "is `ctx.open_menu` still `Some(this index)`" makes the callback a
    // no-op in that case, while still correctly closing on every close that
    // *is* this menu's own (Escape, item-select, an actual outside
    // interaction) -- all of which either already set `ctx.open_menu` to
    // `None` themselves (making this call idempotent) or never touched it
    // (the rare native-only outside-close case this sync exists for).
    crate::top_layer::use_popover_sync(
        id.clone(),
        open,
        Callback::new(move |is_open: bool| {
            if !is_open && (ctx.open_menu)() == Some(index.cloned()) {
                ctx.set_open_menu.call(None);
            }
        }),
    );
    // JS-measured static positioning fallback for engines without CSS
    // Anchor Positioning -- see `top_layer::use_anchor_position_fallback`'s
    // doc. `side`/`align` and the 8px gap match this menu's pre-migration
    // CSS (`../menubar/style.css`'s `top: 100%; left: 0; margin-top:
    // 0.5rem` -- 0.5rem == 8px at the default root font size): anchored
    // below the trigger, left-aligned with it, matching
    // `DropdownMenuContentRendered`'s identical bottom/start choice for the
    // same visual shape.
    crate::top_layer::use_anchor_position_fallback(
        id.clone(),
        id.clone(),
        open,
        crate::ContentSide::Bottom,
        crate::ContentAlign::Start,
        8,
    );

    // See `dropdown_menu.rs`'s `DropdownMenuContentRendered` for why this
    // hand-written, never-`Styles::`-routed marker class exists: it is what
    // the shared, engine-injected anchor-positioning stylesheet
    // (`top_layer::ensure_anchor_positioning_styles`) selects on,
    // sidestepping `manganis-core`'s `css_module_parser` not scoping
    // classes inside `@supports` bodies.
    let attributes = merge_attributes(vec![
        attributes,
        attributes!(div {
            class: "dx-anchor-menubar"
        }),
    ]);

    rsx! {
        div {
            id: id.clone(),
            role: "menu",
            popover: crate::top_layer::PopoverKind::Auto.as_str(),
            style: crate::top_layer::position_anchor_style(&id),
            "data-state": if open() { "open" } else { "closed" },
            ..attributes,
            {children}
        }
    }
}

/// Native (Blitz) arm: unchanged from before this slice -- Blitz has no
/// popover-API support at all, so this stays the functional floor, a plain,
/// always-in-flow `div`.
#[cfg(not(target_family = "wasm"))]
#[component]
fn MenubarContentRendered(id: String, attributes: Vec<Attribute>, children: Element) -> Element {
    let menu_ctx: MenubarMenuContext = use_context();

    rsx! {
        div {
            id,
            role: "menu",
            "data-state": if (menu_ctx.is_open)() { "open" } else { "closed" },
            ..attributes,
            {children}
        }
    }
}

/// The props for the [`MenubarItem`] component.
#[derive(Props, Clone, PartialEq)]
pub struct MenubarItemProps {
    /// The index of this item within the [`MenubarContent`]. This is used to define the focus order for keyboard navigation.
    pub index: ReadSignal<usize>,

    /// The value associated with this menu item. This value will be passed to the [`Self::on_select`] callback when the item is selected.
    pub value: String,

    /// Whether this menu item is disabled.
    #[props(default)]
    pub disabled: ReadSignal<bool>,

    /// Callback fired when the item is selected. The [`Self::value`] will be passed as an argument.
    #[props(default)]
    pub on_select: Callback<String>,

    /// Additional attributes to apply to the item element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the item component.
    pub children: Element,
}

/// # MenubarItem
///
/// The `MenubarItem` component represents a selectable item within a menu. In addition to calling the
/// [`MenubarItemProps::on_select`] callback, the menu will close when the item is selected.
///
/// This must be used inside a [`MenubarContent`] component.
///
/// ## Example
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::menubar::{
///     Menubar, MenubarContent, MenubarItem, MenubarMenu, MenubarTrigger,
/// };
/// #[component]
/// fn Demo() -> Element {
///     rsx! {
///         Menubar {
///             MenubarMenu { index: 0usize,
///                 MenubarTrigger { "File" }
///                 MenubarContent {
///                     MenubarItem {
///                         index: 0usize,
///                         value: "new".to_string(),
///                         on_select: move |value| {
///                             tracing::info!("Selected value: {}", value);
///                         },
///                         "New"
///                     }
///                     MenubarItem {
///                         index: 1usize,
///                         value: "open".to_string(),
///                         on_select: move |value| {
///                             tracing::info!("Selected value: {}", value);
///                         },
///                         "Open"
///                     }
///                 }
///             }
///             MenubarMenu { index: 1usize,
///                 MenubarTrigger { "Edit" }
///                 MenubarContent {
///                     MenubarItem {
///                         index: 0usize,
///                         value: "cut".to_string(),
///                         on_select: move |value| {
///                             tracing::info!("Selected value: {}", value);
///                         },
///                         "Cut"
///                     }
///                     MenubarItem {
///                         index: 1usize,
///                         value: "copy".to_string(),
///                         on_select: move |value| {
///                             tracing::info!("Selected value: {}", value);
///                         },
///                         "Copy"
///                     }
///                 }
///             }
///         }
///     }
/// }
/// ```
///
/// ## Styling
///
/// The [`MenubarItem`] component defines the following data attributes you can use to control styling:
/// - `data-disabled`: Indicates if the item is disabled. Values are `true` or `false`.
#[component]
pub fn MenubarItem(props: MenubarItemProps) -> Element {
    let mut ctx: MenubarContext = use_context();
    let mut menu_ctx: MenubarMenuContext = use_context();

    let disabled = move || (ctx.disabled)() || (props.disabled)();
    let item = use_item(collection_item(menu_ctx.focus, props.index).disabled(disabled));
    let focused = move || item.focused() && (menu_ctx.is_open)();

    let onmounted = item.onmounted();

    rsx! {
        div {
            role: "menuitem",
            "data-disabled": disabled(),
            tabindex: if focused() { "0" } else { "-1" },

            onpointerdown: {
                let value = props.value.clone();
                move |_| {
                    if !disabled() {
                        props.on_select.call(value.clone());
                        ctx.set_open_menu.call(None);
                        // APG menubar close-focus rule, via the collection's
                        // own focus mechanism -- see docs/plan.md Phase 3.1
                        // and the matching Escape case in `MenubarMenu`.
                        // `ctx.focus` never actually left this trigger's
                        // index while item-level keyboard focus roamed the
                        // (separate) submenu collection, so `set_focus`
                        // alone would be a same-value no-op -- clear first.
                        ctx.focus.clear_focus();
                        ctx.focus.set_focus(Some(menu_ctx.index.cloned()));
                    }
                }
            },

            onkeydown: {
                let value = props.value.clone();
                move |event: Event<KeyboardData>| {
                    if event.key() == Key::Enter || event.key() == Key::Character(" ".to_string()) {
                        if !disabled() {
                            props.on_select.call(value.clone());
                            ctx.set_open_menu.call(None);
                            // See the matching comment on the `onpointerdown`
                            // handler above.
                            ctx.focus.clear_focus();
                            ctx.focus.set_focus(Some(menu_ctx.index.cloned()));
                        }
                        event.prevent_default();
                        event.stop_propagation();
                    }
                }
            },

            onmounted,

            onblur: move |_| {
                if focused() {
                    menu_ctx.focus.clear_focus();
                    ctx.focus.clear_focus();
                    ctx.set_open_menu.call(None);
                }
            },

            ..props.attributes,
            {props.children}
        }
    }
}
