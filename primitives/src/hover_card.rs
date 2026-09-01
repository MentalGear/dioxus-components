//! Defines the [`HoverCard`] component and its subcomponents.

#[cfg(target_family = "wasm")]
use crate::merge_attributes;
use crate::{
    use_animated_open, use_controlled, use_id_or, use_unique_id, ContentAlign, ContentSide,
};
use dioxus::prelude::*;
#[cfg(target_family = "wasm")]
use dioxus_attributes::attributes;

#[derive(Clone, Copy)]
struct HoverCardCtx {
    // State
    open: Memo<bool>,
    set_open: Callback<bool>,
    disabled: ReadSignal<bool>,

    // ARIA attributes
    content_id: Signal<String>,
}

/// The props for the [`HoverCard`] component
#[derive(Props, Clone, PartialEq)]
pub struct HoverCardProps {
    /// Whether the hover card is open
    pub open: ReadSignal<Option<bool>>,

    /// Default open state
    #[props(default)]
    pub default_open: bool,

    /// Callback when open state changes
    #[props(default)]
    pub on_open_change: Callback<bool>,

    /// Whether the hover card is disabled
    #[props(default)]
    pub disabled: ReadSignal<bool>,

    /// Additional attributes for the hover card
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the hover card
    pub children: Element,
}

/// # HoverCard
///
/// The `HoverCard` component wraps a [`HoverCardTrigger`] and a [`HoverCardContent`]. It provides a way to show additional information when hovering over an element.
///
/// ## Example
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::{
///     ContentAlign, ContentSide,
///     hover_card::{
///         HoverCard, HoverCardContent, HoverCardTrigger,
///     }
/// };
///
/// #[component]
/// fn Demo() -> Element {
///     rsx! {
///         HoverCard {
///             HoverCardTrigger {
///                 i { "Dioxus" }
///             }
///             HoverCardContent {
///                 side: ContentSide::Bottom,
///                 div {
///                     padding: "1rem",
///                     "Dioxus is"
///                     i { " the " }
///                     "Rust framework for building fullstack web, desktop, and mobile apps. Iterate with live hotreloading, add server functions, and deploy in record time."
///                 }
///             }
///         }
///     }
/// }
/// ```
///
/// ## Styling
///
/// The [`HoverCard`] component defines the following data attributes you can use to control styling:
/// - `data-state`: Indicates the current state of the hover card. Values are `open` or `closed`.
/// - `data-disabled`: Indicates whether the item is disabled. Values are `true` or `false`.
#[component]
pub fn HoverCard(props: HoverCardProps) -> Element {
    let (open, set_open) = use_controlled(props.open, props.default_open, props.on_open_change);
    // Generate a unique ID for the hover card content
    let content_id = use_unique_id();

    use_context_provider(|| HoverCardCtx {
        open,
        set_open,
        disabled: props.disabled,
        content_id,
    });

    rsx! {
        div {
            "data-state": if open() { "open" } else { "closed" },
            "data-disabled": (props.disabled)(),
            ..props.attributes,

            {props.children}
        }
    }
}

/// The props for the [`HoverCardTrigger`] component
#[derive(Props, Clone, PartialEq)]
pub struct HoverCardTriggerProps {
    /// Optional ID for the trigger element
    #[props(default)]
    pub id: ReadSignal<Option<String>>,

    /// Additional attributes for the hover card trigger
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the hover card trigger
    pub children: Element,
}

/// # HoverCardTrigger
///
/// The [`HoverCardTrigger`] component triggers the [`HoverCardContent`] to appear when hovered or focused.
///
/// This component must be used inside a [`HoverCard`] component.
///
/// ## Example
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::{
///     ContentAlign, ContentSide,
///     hover_card::{
///         HoverCard, HoverCardContent, HoverCardTrigger,
///     }
/// };
/// #[component]
/// fn Demo() -> Element {
///     rsx! {
///         HoverCard {
///             HoverCardTrigger {
///                 i { "Dioxus" }
///             }
///             HoverCardContent {
///                 side: ContentSide::Bottom,
///                 div {
///                     padding: "1rem",
///                     "Dioxus is"
///                     i { " the " }
///                     "Rust framework for building fullstack web, desktop, and mobile apps. Iterate with live hotreloading, add server functions, and deploy in record time."
///                 }
///             }
///         }
///     }
/// }
/// ```
#[component]
pub fn HoverCardTrigger(props: HoverCardTriggerProps) -> Element {
    let ctx: HoverCardCtx = use_context();

    // Generate a unique ID for the trigger
    let trigger_id = use_unique_id();

    // Use use_id_or to handle the ID
    let id = use_id_or(trigger_id, props.id);

    // Handle mouse events
    let open_event = move || {
        if !(ctx.disabled)() {
            ctx.set_open.call(true);
        }
    };

    let close_event = move || {
        if !(ctx.disabled)() {
            ctx.set_open.call(false);
        }
    };

    rsx! {
        div {
            id,
            tabindex: "0", // Make the trigger focusable

            // Mouse events
            onmouseenter: move |_| open_event(),
            onmouseleave: move |_| close_event(),

            // Focus events
            onfocus: move |_| open_event(),
            onblur: move |_| close_event(),

            // ARIA attributes
            role: "button",
            aria_describedby: (ctx.open)().then(|| ctx.content_id.cloned()),

            // See `crate::top_layer::anchor_name_style`: ties this trigger
            // to the content's `position-anchor` so its `[data-side]` CSS
            // still resolves relative to this trigger once the content is
            // promoted to the top layer. Inert (empty) off the web arm.
            style: crate::top_layer::anchor_name_style(&ctx.content_id.cloned()),

            ..props.attributes,
            {props.children}
        }
    }
}

/// The props for the [`HoverCardContent`] component
#[derive(Props, Clone, PartialEq)]
pub struct HoverCardContentProps {
    /// Optional ID for the hover card content
    #[props(default)]
    pub id: ReadSignal<Option<String>>,

    /// Side of the trigger to place the hover card
    #[props(default = ContentSide::Top)]
    pub side: ContentSide,

    /// Alignment of the hover card relative to the trigger
    #[props(default = ContentAlign::Center)]
    pub align: ContentAlign,

    /// Whether to force the hover card to stay open when hovered
    #[props(default = true)]
    pub force_mount: bool,

    /// Additional attributes for the hover card content
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the hover card content
    pub children: Element,
}

/// # HoverCardContent
///
/// The [`HoverCardContent`] component defines the content of the parent [`HoverCard`]. It is only rendered when the hover card is open or if [`HoverCardContentProps::force_mount`] is set to true.
///
/// This component must be used inside a [`HoverCard`] component.
///
/// ## Example
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::{
///     ContentAlign, ContentSide,
///     hover_card::{
///         HoverCard, HoverCardContent, HoverCardTrigger,
///     }
/// };
///
/// #[component]
/// fn Demo() -> Element {
///     rsx! {
///         HoverCard {
///             HoverCardTrigger {
///                 i { "Dioxus" }
///             }
///             HoverCardContent {
///                 side: ContentSide::Bottom,
///                 div {
///                     padding: "1rem",
///                     "Dioxus is"
///                     i { " the " }
///                     "Rust framework for building fullstack web, desktop, and mobile apps. Iterate with live hotreloading, add server functions, and deploy in record time."
///                 }
///             }
///         }
///     }
/// }
/// ```
///
/// ## Styling
///
/// The [`HoverCardContent`] component defines the following data attributes you can use to control styling:
/// - `data-state`: Indicates the current state of the hover card. Values are `open` or `closed`.
/// - `data-side`: Indicates the side of the trigger where the hover card is placed. Values are `top`, `right`, `bottom`, or `left`.
/// - `data-align`: Indicates the alignment of the hover card relative to the trigger. Values are `start`, `center`, or `end`.
#[component]
pub fn HoverCardContent(props: HoverCardContentProps) -> Element {
    let ctx: HoverCardCtx = use_context();

    // Only render if the hover card is open or force_mount is true
    let is_open = (ctx.open)();
    if !is_open && !props.force_mount {
        return rsx!({});
    }

    // Use use_id_or to handle the ID
    let id = use_id_or(ctx.content_id, props.id);

    let render = use_animated_open(id, ctx.open);

    // `HoverCardContentRendered` is a real component (not a plain fn) so it
    // -- and the hooks it calls internally (this slice's `use_popover_sync`
    // on the web arm) -- get a fresh scope each time `render()`
    // mounts/unmounts it, matching this element's actual DOM lifetime. See
    // `tooltip.rs`'s identical `TooltipContentRendered` comment for why a
    // plain fn here would be a conditional-hook-call hazard instead.
    rsx! {
        if render() {
            HoverCardContentRendered {
                id: id.cloned(),
                open: ctx.open,
                set_open: ctx.set_open,
                disabled: ctx.disabled,
                is_open,
                side: props.side,
                align: props.align,
                attributes: props.attributes,
                children: props.children,
            }
        }
    }
}

/// Web arm (Phase 4.4, docs/plan.md): promote to the top layer via
/// `popover="manual"` (see `crate::top_layer::PopoverKind::Manual`'s doc for
/// why `manual` and not `auto` — a `HoverCard`'s own mouseenter/mouseleave
/// pair already owns its lifecycle, and MDN's own naming for this pattern
/// ("hover card") does not imply light dismiss the way a click-triggered
/// popover does).
#[cfg(target_family = "wasm")]
#[component]
fn HoverCardContentRendered(
    id: String,
    open: Memo<bool>,
    set_open: Callback<bool>,
    disabled: ReadSignal<bool>,
    is_open: bool,
    side: ContentSide,
    align: ContentAlign,
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    crate::top_layer::use_popover_sync(id.clone(), open, set_open);
    // JS-measured static positioning fallback for Firefox/WebKit (no CSS
    // Anchor Positioning) -- see `top_layer::use_anchor_position_fallback`'s
    // doc. `anchor_id` is this content's own `id`: `HoverCardTrigger`'s
    // `anchor-name` is keyed off `ctx.content_id`, which this component
    // (below) keeps synced to this same id.
    crate::top_layer::use_anchor_position_fallback(id.clone(), id.clone(), open, side, align, 10);

    let handle_mouse_enter = move |_: Event<MouseData>| {
        if !disabled() {
            set_open.call(true);
        }
    };
    let handle_mouse_leave = move |_: Event<MouseData>| {
        if !disabled() {
            set_open.call(false);
        }
    };

    // See `tooltip.rs`'s `TooltipContentRendered` for why this hand-written,
    // never-`Styles::`-routed marker class exists: it is what the
    // `@supports (anchor-name: --a)` block in `../../preview/src/components/
    // hover_card/style.css` selects on, sidestepping `manganis-core`'s
    // `css_module_parser` not scoping classes inside `@supports` bodies.
    let attributes = merge_attributes(vec![
        attributes,
        attributes!(div {
            class: "dx-anchor-hover-card"
        }),
    ]);

    rsx! {
        div {
            id: id.clone(),
            role: "tooltip",
            popover: crate::top_layer::PopoverKind::Manual.as_str(),
            style: crate::top_layer::position_anchor_style(&id),
            "data-state": if is_open { "open" } else { "closed" },
            "data-side": side.as_str(),
            "data-align": align.as_str(),
            onmouseenter: handle_mouse_enter,
            onmouseleave: handle_mouse_leave,
            ..attributes,
            {children}
        }
    }
}

/// Native (Blitz) arm: unchanged from before this slice — see
/// `docs/recommended-implementations.md` Caveat 2.
#[cfg(not(target_family = "wasm"))]
#[component]
fn HoverCardContentRendered(
    id: String,
    // Unused on this arm -- kept as a same-named field so the shared,
    // cfg-independent call site in `HoverCardContent` compiles unchanged on
    // both arms.
    open: Memo<bool>,
    set_open: Callback<bool>,
    disabled: ReadSignal<bool>,
    is_open: bool,
    side: ContentSide,
    align: ContentAlign,
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let _ = open;
    let handle_mouse_enter = move |_: Event<MouseData>| {
        if !disabled() {
            set_open.call(true);
        }
    };
    let handle_mouse_leave = move |_: Event<MouseData>| {
        if !disabled() {
            set_open.call(false);
        }
    };

    rsx! {
        div {
            id,
            role: "tooltip",
            "data-state": if is_open { "open" } else { "closed" },
            "data-side": side.as_str(),
            "data-align": align.as_str(),
            onmouseenter: handle_mouse_enter,
            onmouseleave: handle_mouse_leave,
            ..attributes,
            {children}
        }
    }
}
