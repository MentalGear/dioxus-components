//! Defines the [`Tooltip`] component and its sub-components, which provide contextual information when hovering or focusing on elements.

use crate::{
    merge_attributes, use_animated_open, use_controlled, use_id_or, use_unique_id, ContentAlign,
    ContentSide,
};
use dioxus::prelude::*;
use dioxus_attributes::attributes;

#[derive(Clone, Copy)]
struct TooltipCtx {
    // State
    open: Memo<bool>,
    set_open: Callback<bool>,
    disabled: ReadSignal<bool>,

    // ARIA attributes
    tooltip_id: Signal<String>,
}

/// The props for the [`Tooltip`] component
#[derive(Props, Clone, PartialEq)]
pub struct TooltipProps {
    /// Whether the tooltip is open
    pub open: ReadSignal<Option<bool>>,

    /// Default open state when uncontrolled
    #[props(default)]
    pub default_open: bool,

    /// Callback when open state changes
    #[props(default)]
    pub on_open_change: Callback<bool>,

    /// Whether the tooltip is disabled
    #[props(default)]
    pub disabled: ReadSignal<bool>,

    /// Additional attributes for the tooltip
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the tooltip component, which should include a [`TooltipTrigger`] and a [`TooltipContent`].
    pub children: Element,
}

/// # Tooltip
///
/// The `Tooltip` component provides contextual information when users hover or focus on an
/// element. It consists of a [`TooltipTrigger`] that activates the tooltip and a [`TooltipContent`]
/// that displays the message.
///
/// ## Example
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::{tooltip::{Tooltip, TooltipContent, TooltipTrigger}, ContentSide};
///
/// #[component]
/// fn Demo() -> Element {
///     rsx! {
///         Tooltip {
///             TooltipTrigger {
///                 "Rich content"
///             }
///             TooltipContent {
///                 side: ContentSide::Left,
///                 style: "width: 200px;",
///                 h4 { style: "margin-top: 0; margin-bottom: 8px;", "Tooltip title" }
///                 p { style: "margin: 0;", "This tooltip contains rich HTML content with styling." }
///             }
///         }
///     }
/// }
/// ```
///
/// ## Styling
///
/// The [`Tooltip`] component defines the following data attributes you can use to control styling:
/// - `data-state`: Indicates the current state of the tooltip. Values are `open` or `closed`.
/// - `data-disabled`: Indicates if the tooltip is disabled. Values are `true` or `false`.
#[component]
pub fn Tooltip(props: TooltipProps) -> Element {
    let (open, set_open) = use_controlled(props.open, props.default_open, props.on_open_change);
    let tooltip_id = use_unique_id();

    let _ctx = use_context_provider(|| TooltipCtx {
        open,
        set_open,
        disabled: props.disabled,
        tooltip_id,
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

/// The props for the [`TooltipTrigger`] component
#[derive(Props, Clone, PartialEq)]
pub struct TooltipTriggerProps {
    /// Optional ID for the trigger element
    #[props(default)]
    pub id: Option<String>,

    /// Render the trigger element as a custom component/element.
    #[props(default)]
    pub r#as: Option<Callback<Vec<Attribute>, Element>>,

    /// Additional attributes for the trigger element
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the trigger element
    pub children: Element,
}

/// # TooltipTrigger
///
/// The trigger element for the [`Tooltip`] component. When users hover over or focus on this element, the tooltip content will be displayed.
///
/// This must be used inside a [`Tooltip`] component.
///
/// ## Example
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::{tooltip::{Tooltip, TooltipContent, TooltipTrigger}, ContentSide};
///
/// #[component]
/// fn Demo() -> Element {
///     rsx! {
///         Tooltip {
///             TooltipTrigger {
///                 "Rich content"
///             }
///             TooltipContent {
///                 side: ContentSide::Left,
///                 style: "width: 200px;",
///                 h4 { style: "margin-top: 0; margin-bottom: 8px;", "Tooltip title" }
///                 p { style: "margin: 0;", "This tooltip contains rich HTML content with styling." }
///             }
///         }
///     }
/// }
/// ```
#[component]
pub fn TooltipTrigger(props: TooltipTriggerProps) -> Element {
    let ctx: TooltipCtx = use_context();

    // Handle mouse events
    let handle_mouse_enter = move |_: Event<MouseData>| {
        if !(ctx.disabled)() {
            ctx.set_open.call(true);
        }
    };

    let handle_mouse_leave = move |_: Event<MouseData>| {
        if !(ctx.disabled)() {
            ctx.set_open.call(false);
        }
    };

    // Handle focus events
    let handle_focus = move |_: Event<FocusData>| {
        if !(ctx.disabled)() {
            ctx.set_open.call(true);
        }
    };

    let handle_blur = move |_: Event<FocusData>| {
        if !(ctx.disabled)() {
            ctx.set_open.call(false);
        }
    };

    // Handle keyboard events
    let handle_keydown = move |event: Event<KeyboardData>| {
        if event.key() == Key::Escape && (ctx.open)() {
            event.prevent_default();
            ctx.set_open.call(false);
        }
    };

    let base = attributes!(div {
        id: props.id.clone(),
        tabindex: "0",
        "aria-describedby": ctx.tooltip_id.cloned(),
        // See `crate::top_layer::anchor_name_style`: ties the trigger to
        // the content's `position-anchor` so the content's `[data-side]`
        // CSS still resolves relative to this trigger once the content is
        // promoted to the top layer. Inert (empty) off the web arm.
        style: crate::top_layer::anchor_name_style(&ctx.tooltip_id.cloned()),
        onmouseenter: handle_mouse_enter,
        onmouseleave: handle_mouse_leave,
        onfocus: handle_focus,
        onblur: handle_blur,
        onkeydown: handle_keydown,
    });
    let merged = merge_attributes(vec![base, props.attributes]);

    if let Some(dynamic) = props.r#as {
        dynamic.call(merged)
    } else {
        rsx! {
            div {
                ..merged,
                {props.children}
            }
        }
    }
}

/// The props for the [`TooltipContent`] component
#[derive(Props, Clone, PartialEq)]
pub struct TooltipContentProps {
    /// Optional ID for the tooltip content
    #[props(default)]
    pub id: ReadSignal<Option<String>>,

    /// Side of the trigger to place the tooltip
    #[props(default = ContentSide::Top)]
    pub side: ContentSide,

    /// Alignment of the tooltip relative to the trigger
    #[props(default = ContentAlign::Center)]
    pub align: ContentAlign,

    /// Additional attributes for the tooltip content element
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the tooltip content
    pub children: Element,
}

/// # TooltipContent
///
/// The content component for the [`Tooltip`] that displays the actual tooltip message. The content will only be
/// rendered when the tooltip is open (as controlled by the [`TooltipTrigger`] component).
///
/// This must be used inside a [`Tooltip`] component.
///
/// ## Example
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::{tooltip::{Tooltip, TooltipContent, TooltipTrigger}, ContentSide};
///
/// #[component]
/// fn Demo() -> Element {
///     rsx! {
///         Tooltip {
///             TooltipTrigger {
///                 "Rich content"
///             }
///             TooltipContent {
///                 side: ContentSide::Left,
///                 style: "width: 200px;",
///                 h4 { style: "margin-top: 0; margin-bottom: 8px;", "Tooltip title" }
///                 p { style: "margin: 0;", "This tooltip contains rich HTML content with styling." }
///             }
///         }
///     }
/// }
/// ```
///
/// ## Styling
///
/// The [`TooltipContent`] component defines the following data attributes you can use to control styling:
/// - `data-state`: Indicates the current state of the tooltip. Values are `open` or `closed`.
/// - `data-side`: Indicates which side of the trigger the tooltip is positioned. Values are `top`, `right`, `bottom`, or `left`.
/// - `data-align`: Indicates the alignment of the tooltip. Values are `start`, `center`, or `end`.
#[component]
pub fn TooltipContent(props: TooltipContentProps) -> Element {
    let mut ctx: TooltipCtx = use_context();

    let unique_id = use_unique_id();
    let id = use_id_or(unique_id, props.id);

    use_effect(move || {
        ctx.tooltip_id.set(id());
    });

    // Only render if the tooltip is open
    let render = use_animated_open(id, ctx.open);

    // Create the tooltip content. `TooltipContentRendered` is a real
    // component (not a plain fn) precisely so it can be mounted/unmounted
    // by `render()` here -- it, and the hooks it calls internally (this
    // slice's `use_popover_sync` on the web arm), get a fresh scope each
    // time, matching this element's actual DOM lifetime. A plain fn called
    // conditionally from inside this `rsx!` would instead attribute those
    // hook calls to *this* component's own scope, where `render()` toggling
    // on every open/close would change the hook count/order across renders
    // of the same mounted `TooltipContent` -- exactly the hazard
    // `docs/phase4-spike-findings.md` Construction B's root-cause analysis
    // describes for a different case (binding `open` vs. calling
    // `showModal()` in the same render pass); see `popover.rs`'s
    // `PopoverContentRendered` for the same fix applied there.
    rsx! {
        if render() {
            TooltipContentRendered {
                id: id.cloned(),
                open: ctx.open,
                set_open: ctx.set_open,
                side: props.side,
                align: props.align,
                attributes: props.attributes,
                children: props.children,
            }
        }
    }
}

/// Web arm (Phase 4.4, docs/plan.md): promote the tooltip to the top layer
/// via `popover="manual"` so it escapes clipping/transformed ancestors
/// (docs/phase4-spike-findings.md experiment 7). `manual`, not `auto`: a
/// Tooltip already owns its entire open/close lifecycle through
/// `TooltipTrigger`'s hover/focus/Escape handlers, and `auto`'s light
/// dismiss (WHATWG HTML §popover-light-dismiss) would fight that — an
/// outside pointerdown that the browser used to (harmlessly) ignore would
/// now race our own signal to close it, and there is no separate
/// interaction this component wants Escape or click-outside to mean beyond
/// what the trigger already does. `crate::top_layer::use_popover_sync`
/// drives `showPopover()`/`hidePopover()` from `open` and mirrors the
/// browser's own `toggle` event back into `set_open` in case anything ever
/// hides it outside that signal.
#[cfg(target_family = "wasm")]
#[component]
fn TooltipContentRendered(
    id: String,
    open: Memo<bool>,
    set_open: Callback<bool>,
    side: ContentSide,
    align: ContentAlign,
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    crate::top_layer::use_popover_sync(id.clone(), open, set_open);

    rsx! {
        div {
            id: id.clone(),
            role: "tooltip",
            popover: crate::top_layer::PopoverKind::Manual.as_str(),
            style: crate::top_layer::position_anchor_style(&id),
            "data-state": if open.cloned() { "open" } else { "closed" },
            "data-side": side.as_str(),
            "data-align": align.as_str(),
            ..attributes,
            {children}
        }
    }
}

/// Native (Blitz) arm: unchanged from before this slice. Blitz implements
/// neither the `popover` attribute nor `document::eval`
/// (`docs/recommended-implementations.md` Caveat 2), so this is the
/// functional floor — a plain, always-in-flow div, visible exactly when
/// `render()` (above) mounts it.
#[cfg(not(target_family = "wasm"))]
#[component]
fn TooltipContentRendered(
    id: String,
    open: Memo<bool>,
    // Unused on this arm -- kept as a same-named field so the shared,
    // cfg-independent call site in `TooltipContent` compiles unchanged on
    // both arms.
    set_open: Callback<bool>,
    side: ContentSide,
    align: ContentAlign,
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let _ = set_open;
    rsx! {
        div {
            id,
            role: "tooltip",
            "data-state": if open.cloned() { "open" } else { "closed" },
            "data-side": side.as_str(),
            "data-align": align.as_str(),
            ..attributes,
            {children}
        }
    }
}
