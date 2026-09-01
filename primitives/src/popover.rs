//! Defines the [`PopoverRoot`] component and its sub-components.

use dioxus::document;
use dioxus::prelude::*;

use crate::{
    use_animated_open, use_controlled, use_global_escape_listener, use_id_or, use_outside_dismiss,
    use_unique_id, ContentAlign, ContentSide, FOCUS_TRAP_JS,
};

#[derive(Clone, Copy)]
struct PopoverCtx {
    #[allow(unused)]
    open: Memo<bool>,
    #[allow(unused)]
    set_open: Callback<bool>,

    // Whether the dialog is a modal and should capture focus.
    #[allow(unused)]
    is_modal: ReadSignal<bool>,
    labelledby: Signal<String>,
    root_id: Memo<String>,

    // The current `PopoverContent`'s own element id, kept in sync by that
    // component (mirrors `TooltipCtx::tooltip_id`/`HoverCardCtx::content_id`
    // in `tooltip.rs`/`hover_card.rs`) -- *not* the same thing as
    // `labelledby` above, which is the *trigger's* id (content's
    // `aria-labelledby` points at the trigger, describing the popover by
    // what opened it). `PopoverTrigger`'s `anchor-name` must key off
    // *this* signal, not `labelledby`: `crate::top_layer::
    // position_anchor_style` on the content side builds `--dxa-<content's
    // own id>`, so the trigger's `anchor-name` has to name that same id for
    // `position-anchor` to ever resolve. Confirmed by execution: before this
    // field existed, the trigger declared `anchor-name: --dxa-<labelledby>`
    // (i.e. its own id) while the content declared
    // `position-anchor: --dxa-<content id>` -- two different ids -- so
    // every `anchor()` value in `../../preview/src/components/popover/
    // style.css`'s `@supports` block silently failed to resolve (an
    // `anchor()` naming a nonexistent `anchor-name` is invalid at
    // computed-value time) and the non-modal popover rendered at the
    // fallback `top:0; left:0` corner instead of next to its trigger, on
    // *any* browser, anchor-positioning-capable or not.
    #[allow(unused)]
    content_id: Signal<String>,
}

/// The props for the [`PopoverRoot`] component.
#[derive(Props, Clone, PartialEq)]
pub struct PopoverRootProps {
    /// Whether the popover is a modal and should capture focus.
    #[props(default = ReadSignal::new(Signal::new(true)))]
    pub is_modal: ReadSignal<bool>,

    /// The controlled open state of the popover.
    pub open: ReadSignal<Option<bool>>,

    /// The default open state when uncontrolled.
    #[props(default)]
    pub default_open: bool,

    /// Callback fired when the open state changes.
    #[props(default)]
    pub on_open_change: Callback<bool>,

    /// The id of the popover root element.
    pub id: ReadSignal<Option<String>>,

    /// Additional attributes to apply to the popover root element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the popover root component.
    pub children: Element,
}

/// # PopoverRoot
///
/// The `PopoverRoot` component wraps all the popover components and manages the state. You can define a
/// [`PopoverTrigger`] component to toggle the popover's open state, and a [`PopoverContent`] component
/// to define the content that appears when the popover is open under this component.
///
/// ## Example
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::popover::*;
///
/// #[component]
/// fn Demo() -> Element {
///     let mut open = use_signal(|| false);
///
///     rsx! {
///         PopoverRoot {
///             open: open(),
///             on_open_change: move |v| open.set(v),
///             PopoverTrigger {
///                 "Show Popover"
///             }
///             PopoverContent {
///                 gap: "0.25rem",
///                 h3 {
///                     padding_top: "0.25rem",
///                     padding_bottom: "0.25rem",
///                     width: "100%",
///                     text_align: "center",
///                     margin: 0,
///                     "Delete Item?"
///                 }
///                 button {
///                     onclick: move |_| {
///                         open.set(false);;
///                     },
///                     "Yes!"
///                 }
///             }
///         }
///     }
/// }
/// ```
///
/// ## Styling
///
/// The [`PopoverRoot`] component defines the following data attributes you can use to control styling:
/// - `data-state`: Indicates if the popover is open or closed. Values are `open` or `closed`.
#[component]
pub fn PopoverRoot(props: PopoverRootProps) -> Element {
    // See `DialogRoot`'s identical call for why this must be at the root,
    // not only inside `use_scroll_lock` (called by
    // `PopoverContentRendered`, which mounts lazily on open).
    use_effect(crate::scroll_lock::ensure_scrollbar_gutter_baseline);

    // See `Tooltip`'s identical call (`tooltip.rs`) for why this must be at
    // the root and installed well before the first open, rather than only
    // from `use_anchor_position_fallback` (which only runs once
    // `PopoverNonModalContent` first mounts). Covers every consumer built on
    // `PopoverRoot` too (`ColorPicker`, `DatePicker`), not just this
    // workspace's own "Popover" page.
    #[cfg(target_family = "wasm")]
    use_effect(crate::top_layer::ensure_anchor_positioning_styles);

    let labelledby = use_unique_id();
    let gen_root_id = use_unique_id();
    let root_id = use_id_or(gen_root_id, props.id);
    // See `PopoverCtx::content_id`'s doc: placeholder value until
    // `PopoverContent` mounts and syncs its own id in.
    let content_id = use_unique_id();

    let (open, set_open) = use_controlled(props.open, props.default_open, props.on_open_change);

    use_context_provider(|| PopoverCtx {
        open,
        set_open,
        is_modal: props.is_modal,
        labelledby,
        root_id,
        content_id,
    });

    rsx! {
        div {
            id: root_id,
            "data-state": if open() { "open" } else { "closed" },
            ..props.attributes,
            {props.children}
        }
    }
}

/// The props for the [`PopoverContent`] component.
#[derive(Props, Clone, PartialEq)]
pub struct PopoverContentProps {
    /// The id of the popover content element.
    pub id: ReadSignal<Option<String>>,

    /// CSS class for the popover content.
    #[props(default)]
    pub class: Option<String>,

    /// Side of the trigger to place the popover.
    #[props(default = ContentSide::Bottom)]
    pub side: ContentSide,

    /// Alignment of the popover relative to the trigger.
    #[props(default = ContentAlign::Center)]
    pub align: ContentAlign,

    /// Additional attributes to apply to the content element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the popover content component.
    pub children: Element,
}

/// # PopoverContent
///
/// The `PopoverContent` component defines the content of the popover. This component will
/// only be rendered if the popover is open, and it will handle focus trapping if the popover is modal.
///
/// This must be used inside a [`PopoverRoot`] component.
///
/// ## Example
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::popover::*;
///
/// #[component]
/// fn Demo() -> Element {
///     let mut open = use_signal(|| false);
///
///     rsx! {
///         PopoverRoot {
///             open: open(),
///             on_open_change: move |v| open.set(v),
///             PopoverTrigger {
///                 "Show Popover"
///             }
///             PopoverContent {
///                 gap: "0.25rem",
///                 h3 {
///                     padding_top: "0.25rem",
///                     padding_bottom: "0.25rem",
///                     width: "100%",
///                     text_align: "center",
///                     margin: 0,
///                     "Delete Item?"
///                 }
///                 button {
///                     onclick: move |_| {
///                         open.set(false);;
///                     },
///                     "Yes!"
///                 }
///             }
///         }
///     }
/// }
/// ```
///
/// ## Styling
///
/// The [`PopoverContent`] component defines the following data attributes you can use to control styling:
/// - `data-state`: Indicates if the popover is open or closed. Values are `open` or `closed`.
/// - `data-side`: Indicates the side where the popover is positioned relative to the trigger. Possible values are `top`, `right`, `bottom`, and `left`.
/// - `data-align`: Indicates the alignment of the popover relative to the trigger. Possible values are `start`, `center`, and `end`.
#[component]
pub fn PopoverContent(props: PopoverContentProps) -> Element {
    let ctx: PopoverCtx = use_context();
    let open = ctx.open;
    let is_modal = ctx.is_modal;

    let gen_id = use_unique_id();
    let id = use_id_or(gen_id, props.id);

    // Keep `ctx.content_id` in sync with this content's actual id -- see
    // `PopoverCtx::content_id`'s doc. Mirrors `TooltipContent`'s identical
    // `ctx.tooltip_id.set(id())` in `tooltip.rs`.
    {
        let mut content_id = ctx.content_id;
        use_effect(move || content_id.set(id()));
    }

    let render = use_animated_open(id, ctx.open);

    use_effect(move || {
        if !render() {
            return;
        }
        let is_modal = is_modal();
        if !is_modal {
            // If the dialog is not modal, we don't need to trap focus.
            return;
        }

        let eval = document::eval(
            r#"let id = await dioxus.recv();
            let is_open = await dioxus.recv();
            let dialog = document.getElementById(id);

            if (is_open) {
                dialog.trap = window.createFocusTrap(dialog);
            }
            if (!is_open && dialog.trap) {
                dialog.trap.remove();
                dialog.trap = null;
            }"#,
        );
        let _ = eval.send(id.to_string());
        let _ = eval.send(open.cloned());
    });

    rsx! {
        document::Script {
            src: FOCUS_TRAP_JS,
            defer: true
        }
        if render() {
            PopoverContentRendered {
                id,
                class: props.class,
                side: props.side,
                align: props.align,
                attributes: props.attributes,
                children: props.children
            }
        }
    }
}

/// The rendered content of the popover. This is separated out so the global event listener
/// is only added when the popover is actually rendered.
#[component]
pub fn PopoverContentRendered(
    id: String,
    class: Option<String>,
    side: ContentSide,
    align: ContentAlign,
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let ctx: PopoverCtx = use_context();
    let open = ctx.open;
    let is_open = open();
    let is_modal = ctx.is_modal;

    // Lock page scroll while the popover is open and modal. See
    // docs/plan.md Phase 3.2. Safe to keep unconditional (only the
    // *computed value* depends on `is_modal`, not the hook call itself) --
    // see the child components below for the case that does need care.
    let scroll_lock_active = use_memo(move || is_modal() && open());
    crate::scroll_lock::use_scroll_lock(scroll_lock_active);

    // The modal/non-modal split is a real child-component boundary, not an
    // `if`/`else` inside this component's own body, and each child calls
    // its own dismissal hooks internally (below) rather than this
    // component calling them unconditionally up front. Two reasons:
    //
    // 1. Hook-order safety: `is_modal` is a `ReadSignal<bool>`, so it can
    //    in principle change while a popover stays open. Dioxus (like
    //    React) requires one mounted component instance to call the same
    //    hooks in the same order on every render; branching which hooks
    //    run on a value that can change mid-mount would violate that.
    //    Routing the branch through separate child components sidesteps
    //    the problem entirely: when `is_modal()` flips, Dioxus unmounts
    //    one child and mounts the other, each getting its own fresh hook
    //    scope, so there is no single instance whose hook count/order
    //    could vary.
    // 2. Correctness, not just safety: `use_global_escape_listener`'s
    //    underlying JS calls `event.preventDefault()` on *every* Escape
    //    keypress while any instance is mounted, regardless of what its
    //    Rust callback decides to do -- so merely having the non-modal web
    //    arm call it (even with a callback that declines to act) would
    //    suppress the browser's own `popover="auto"` Escape-dismissal
    //    algorithm before it ever runs. Confirmed by execution: an earlier
    //    version of this fix kept the hook called unconditionally and
    //    gated only the Rust-side callback, and Escape silently stopped
    //    closing the non-modal web popover at all. The non-modal web arm
    //    below simply never calls `use_global_escape_listener`/
    //    `use_outside_dismiss`, so it never installs that
    //    `preventDefault()`.
    if is_modal() {
        rsx! {
            PopoverModalContent { id, class, side, align, attributes, children, is_open }
        }
    } else {
        rsx! {
            PopoverNonModalContent { id, class, side, align, attributes, children, is_open }
        }
    }
}

/// Modal arm: unchanged from before this slice. Native
/// `<dialog>`/`showModal()` top-layer wiring for the modal arm is Phase 4.2
/// (docs/plan.md), not this slice's -- this scope is deliberately just the
/// non-modal arm below.
#[component]
#[allow(clippy::too_many_arguments)]
fn PopoverModalContent(
    id: String,
    class: Option<String>,
    side: ContentSide,
    align: ContentAlign,
    attributes: Vec<Attribute>,
    children: Element,
    is_open: bool,
) -> Element {
    let ctx: PopoverCtx = use_context();
    let set_open = ctx.set_open;

    // Add a escape key listener to the document when the popover is open. We can't
    // just add this to the popover itself because it might not be focused if the user
    // is highlighting text or interacting with another element.
    use_global_escape_listener(move || set_open.call(false));
    use_outside_dismiss(ctx.root_id, move || set_open.call(false));

    rsx! {
        div {
            id,
            role: "dialog",
            aria_modal: "true",
            aria_labelledby: ctx.labelledby,
            aria_hidden: (!is_open).then_some("true"),
            class: class.unwrap_or_else(|| "dx-popover-content".to_string()),
            "data-state": if is_open { "open" } else { "closed" },
            "data-side": side.as_str(),
            "data-align": align.as_str(),
            ..attributes,
            {children}
        }
    }
}

/// Web arm (Phase 4.4, docs/plan.md): render the non-modal content as a
/// `<dialog popover="auto">`, per MDN's own guidance that `<dialog>` and the
/// Popover API "overlap" and are meant to compose -- a `<dialog>` supplies
/// dialog semantics (an implicit ARIA role of `"dialog"`, non-modal here
/// since this element is never opened with `showModal()` --
/// <https://www.w3.org/TR/html-aria/#el-dialog>), and `popover="auto"`
/// supplies top-layer rendering plus light dismiss
/// (WHATWG HTML §popover-light-dismiss). `PopoverModalContent` still sets
/// an explicit `role="dialog"` because it is a plain `div`; here that would
/// be redundant with the element's own implicit role, so it is dropped.
///
/// Deliberately does *not* call `use_global_escape_listener`/
/// `use_outside_dismiss` -- see `PopoverContentRendered`'s comment for why
/// that is required for correctness, not just style, on this arm.
/// `crate::top_layer::use_popover_sync` drives `showPopover()`/
/// `hidePopover()` from `open` and mirrors the browser's own `toggle` event
/// (fired on light dismiss, Escape, or any other close) back into
/// `set_open`, so the Rust signal can never strand the way `docs/
/// recommended-implementations.md` Caveat 1 documents for `<dialog>`'s old
/// one-way `showModal()`/`close()` binding.
#[cfg(target_family = "wasm")]
#[component]
fn PopoverNonModalContent(
    id: String,
    class: Option<String>,
    side: ContentSide,
    align: ContentAlign,
    attributes: Vec<Attribute>,
    children: Element,
    is_open: bool,
) -> Element {
    let ctx: PopoverCtx = use_context();
    crate::top_layer::use_popover_sync(id.clone(), ctx.open, ctx.set_open);
    // JS-measured static positioning fallback for Firefox/WebKit (no CSS
    // Anchor Positioning) -- see `top_layer::use_anchor_position_fallback`'s
    // doc. `anchor_id` is this content's own `id`: `PopoverTrigger`'s
    // `anchor-name` is keyed off `ctx.content_id`, which `PopoverContent`
    // keeps synced to this same id (see `PopoverCtx::content_id`'s doc).
    crate::top_layer::use_anchor_position_fallback(
        id.clone(),
        id.clone(),
        ctx.open,
        side,
        align,
        8,
    );

    // See `tooltip.rs`'s `TooltipContentRendered` for why this hand-written,
    // never-`Styles::`-routed marker class exists: it is what the
    // `@supports (anchor-name: --a)` block in `../../preview/src/components/
    // popover/style.css` selects on, sidestepping `manganis-core`'s
    // `css_module_parser` not scoping classes inside `@supports` bodies.
    //
    // Appended directly onto `class` (not merged into `attributes`, unlike
    // the identical fix in `tooltip.rs`/`hover_card.rs`): this component,
    // unlike those two, renders its consumer-supplied class through a
    // dedicated `class: Option<String>` prop rather than folding it into
    // `attributes` -- and that field is written into this element's `class`
    // attribute *before* `..attributes` is spread below. Merging the marker
    // into `attributes` instead would add a second, later `class` attribute
    // that silently overwrites this one rather than combining with it (only
    // `merge_attributes` itself concatenates same-named `class` attributes;
    // plain rsx attribute lists do not) -- confirmed by execution: doing it
    // that way rendered `class="dx-anchor-popover"` alone, with
    // `dx-popover-content`/its hashed preview variant dropped entirely.
    let class = format!(
        "{} dx-anchor-popover",
        class.unwrap_or_else(|| "dx-popover-content".to_string())
    );

    rsx! {
        dialog {
            id: id.clone(),
            popover: crate::top_layer::PopoverKind::Auto.as_str(),
            style: crate::top_layer::position_anchor_style(&id),
            aria_labelledby: ctx.labelledby,
            aria_hidden: (!is_open).then_some("true"),
            class,
            "data-state": if is_open { "open" } else { "closed" },
            "data-side": side.as_str(),
            "data-align": align.as_str(),
            ..attributes,
            {children}
        }
    }
}

/// Native (Blitz) arm: unchanged from before this slice -- Blitz has no
/// popover-API support at all (`docs/recommended-implementations.md`
/// Caveat 2), so light dismiss / Escape still need this crate's own
/// JS-driven listeners.
#[cfg(not(target_family = "wasm"))]
#[component]
fn PopoverNonModalContent(
    id: String,
    class: Option<String>,
    side: ContentSide,
    align: ContentAlign,
    attributes: Vec<Attribute>,
    children: Element,
    is_open: bool,
) -> Element {
    let ctx: PopoverCtx = use_context();
    let set_open = ctx.set_open;
    use_global_escape_listener(move || set_open.call(false));
    use_outside_dismiss(ctx.root_id, move || set_open.call(false));

    rsx! {
        div {
            id,
            role: "dialog",
            aria_labelledby: ctx.labelledby,
            aria_hidden: (!is_open).then_some("true"),
            class: class.unwrap_or_else(|| "dx-popover-content".to_string()),
            "data-state": if is_open { "open" } else { "closed" },
            "data-side": side.as_str(),
            "data-align": align.as_str(),
            ..attributes,
            {children}
        }
    }
}

/// The props for the [`PopoverTrigger`] component.
#[derive(Props, Clone, PartialEq)]
pub struct PopoverTriggerProps {
    /// Additional attributes to apply to the trigger element.
    #[props(extends = GlobalAttributes)]
    #[props(extends = button)]
    pub attributes: Vec<Attribute>,

    /// The children of the trigger component.
    pub children: Element,
}

/// # PopoverTrigger
///
/// The `PopoverTrigger` is a button that toggles the visibility of the [`PopoverContent`].
///
/// This must be used inside a [`PopoverRoot`] component.
///
/// ## Example
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::popover::*;
///
/// #[component]
/// fn Demo() -> Element {
///     let mut open = use_signal(|| false);
///
///     rsx! {
///         PopoverRoot {
///             open: open(),
///             on_open_change: move |v| open.set(v),
///             PopoverTrigger {
///                 "Show Popover"
///             }
///             PopoverContent {
///                 gap: "0.25rem",
///                 h3 {
///                     padding_top: "0.25rem",
///                     padding_bottom: "0.25rem",
///                     width: "100%",
///                     text_align: "center",
///                     margin: 0,
///                     "Delete Item?"
///                 }
///                 button {
///                     onclick: move |_| {
///                         open.set(false);;
///                     },
///                     "Yes!"
///                 }
///             }
///         }
///     }
/// }
/// ```
#[component]
pub fn PopoverTrigger(props: PopoverTriggerProps) -> Element {
    let ctx: PopoverCtx = use_context();
    let mut id = ctx.labelledby;
    let id_attribute = props
        .attributes
        .iter()
        .find(|attr| attr.name == "id")
        .cloned();
    use_effect(use_reactive!(|id_attribute| {
        if let Some(id_attribute) = id_attribute {
            match &id_attribute.value {
                dioxus_core::AttributeValue::Text(val) => id.set(val.to_string()),
                dioxus_core::AttributeValue::Float(val) => id.set(val.to_string()),
                dioxus_core::AttributeValue::Int(val) => id.set(val.to_string()),
                dioxus_core::AttributeValue::Bool(val) => id.set(val.to_string()),
                _ => {}
            }
        }
    }));

    rsx! {
        button {
            id,
            type: "button",
            // See `crate::top_layer::anchor_name_style`: ties this trigger
            // to the non-modal content's `position-anchor` so its
            // `[data-side]` CSS still resolves relative to this trigger
            // once the content is promoted to the top layer (Phase 4.4).
            // Inert (empty) off the web arm, and unused by the modal arm
            // (Phase 4.2's job), which never sets `position-anchor`.
            //
            // Keyed on `ctx.content_id` -- *not* `id`/`ctx.labelledby`
            // above, which is this trigger's own id (used for the `id`
            // attribute and, via `PopoverContent`'s `aria-labelledby`, an
            // unrelated ARIA relationship). See `PopoverCtx::content_id`'s
            // doc for why those must not be conflated: `position_anchor_
            // style` on the content side is always built from the
            // content's own id, so this side has to name that same id, not
            // the trigger's.
            style: crate::top_layer::anchor_name_style(&ctx.content_id.cloned()),
            onclick: move |e| {
                e.stop_propagation();
                ctx.set_open.call(!(ctx.open)());
            },
            ..props.attributes,
            {props.children}
        }
    }
}
