//! Defines the [`DialogRoot`] component and its sub-components.
//!
//! ## Native `<dialog>` modality (Phase 4.2, docs/plan.md)
//!
//! [`DialogContent`] dispatches, per render, to one of two real
//! child-component boundaries depending on `is_modal` -- the same pattern
//! `popover.rs`'s `PopoverContentRendered` uses, and for the same two
//! reasons documented there: `is_modal` is a `ReadSignal<bool>` that can in
//! principle change mid-mount (hook-order safety demands separate mounted
//! instances, not an `if`/`else` inside one instance's body), and the modal
//! web arm's dismissal semantics are not merely "the non-modal path with a
//! different callback" -- `use_global_keydown_listener` (this crate's
//! `lib.rs`) calls `event.preventDefault()` on *every* Escape keypress
//! unconditionally while any instance using it is mounted, which would fight
//! the native `<dialog>`'s own `cancel`/`close` handling if a gated-callback
//! version of `use_global_escape_listener` were merely installed and told to
//! do nothing (docs/backlog.md row 3's warning, given when Phase 4.4 landed
//! the same lesson for Popover).
//!
//! `DialogContentNonModal` (`is_modal: false`) is unchanged from before this
//! slice on both targets, per docs/plan.md Phase 4.2's scope.
//! `DialogContentModal` (`is_modal: true`) is cfg-split
//! (`docs/phase4-spike-findings.md` Construction B):
//! - `#[cfg(not(feature = "web"))]`: byte-for-byte the pre-existing
//!   `div` + vendored `FocusTrap` path.
//! - `#[cfg(feature = "web")]`: a real `<dialog>` element, `open`
//!   never bound as an attribute (Construction B's central finding: binding
//!   it declaratively in the same build as a guarded `showModal()` call
//!   doesn't crash, it silently skips the modal state entirely), driven by
//!   `crate::use_dialog_open_driver`/`crate::use_dialog_close_sync` (the
//!   fix for the historical stranded-signal defect,
//!   `docs/recommended-implementations.md` Caveat 1) plus
//!   `crate::use_dialog_backdrop_dismiss` (now shared with `popover.rs`'s
//!   modal web arm too) for the "click far outside the dialog" behaviour
//!   `dialog.spec.ts` already covers -- `use_outside_dismiss`
//!   itself is not reusable here, because a `showModal()` backdrop click's
//!   `event.target` *is* the `<dialog>` element (it contains itself), so
//!   `use_outside_dismiss`'s `!root.contains(e.target)` check can never fire
//!   for it (`docs/phase4-spike-findings.md` experiment 6's
//!   `elementFromPoint` finding). No `use_global_escape_listener` and no
//!   focus-trap eval on this arm: the browser's own `showModal()` supplies
//!   the focus trap, focus restore, inertness, and top layer, and its
//!   `cancel`/`close` events (synced above) already handle Escape.
//!
//! ### Closing keeps the dialog modal until its exit has played
//!
//! `open` going `false` does not `close()` the `<dialog>` at once:
//! `crate::use_dialog_open_driver` waits for the animations and transitions
//! running on the dialog and its `::backdrop` to settle first, and Escape
//! (`cancel`) is routed through state rather than closing natively. The
//! `<dialog>` therefore carries its own `data-state` (`"open"`/`"closed"`) --
//! `[open]` stays set for the whole exit, so a stylesheet keys the exit on
//! `[data-state="closed"]` -- and stays a real modal in the top layer, with the
//! viewport as its containing block, until the end. See that hook's doc for the
//! regression this closes (a closing Sheet/Drawer/Dialog changing size and
//! position, clipped to a `content-visibility` ancestor, the moment it left the
//! top layer).

#[cfg(not(feature = "web"))]
use dioxus::document;
use dioxus::prelude::*;
use dioxus_attributes::attributes;

use crate::{
    merge_attributes, use_animated_open, use_controlled, use_global_escape_listener, use_id_or,
    use_outside_dismiss, use_unique_id,
};

/// Context for the [`DialogRoot`] component
#[derive(Clone, Copy)]
pub struct DialogCtx {
    #[allow(unused)]
    open: Memo<bool>,
    /// Callback to set the open state of the dialog.
    #[allow(unused)]
    set_open: Callback<bool>,

    // Whether the dialog is a modal and should capture focus.
    #[allow(unused)]
    is_modal: ReadSignal<bool>,
    // Whether the dialog dims the page behind it -- see `DialogRootProps::overlay`.
    #[allow(unused)]
    overlay: ReadSignal<bool>,
    dialog_labelledby: Signal<String>,
    dialog_describedby: Signal<String>,
}

impl DialogCtx {
    /// Returns whether the dialog is open.
    pub fn is_open(&self) -> bool {
        self.open.cloned()
    }

    /// Sets the open state of the dialog.
    pub fn set_open(&self, open: bool) {
        self.set_open.call(open);
    }
}

/// The `data-dx-overlay` value for a modal's `overlay` prop: `Some("off")` when the scrim
/// is switched off, `None` (no attribute at all) when it is on.
pub(crate) fn overlay_off(overlay: ReadSignal<bool>) -> Option<&'static str> {
    (!overlay()).then_some("off")
}

/// The props for the [`DialogRoot`] component
#[derive(Props, Clone, PartialEq)]
pub struct DialogRootProps {
    /// The ID of the dialog root element.
    pub id: ReadSignal<Option<String>>,

    /// Whether the dialog is modal. If true, it will trap focus within the dialog when open.
    #[props(default = ReadSignal::new(Signal::new(true)))]
    pub is_modal: ReadSignal<bool>,

    /// Whether a modal dialog dims the page behind it with the shared overlay scrim.
    /// Defaults to `true`. When `false` the content element (the `<dialog>` on the web
    /// arm) and the root element carry `data-dx-overlay="off"`, which a stylesheet turns
    /// into a transparent `::backdrop` (the theme's `dialog[data-dx-overlay="off"]::backdrop`
    /// rule); the modality itself -- focus trap, inert page, click-outside dismissal -- is
    /// untouched. Only the dim and the blur go. A headless user with no stylesheet gets the
    /// attribute and the browser's own `::backdrop`.
    #[props(default = ReadSignal::new(Signal::new(true)))]
    pub overlay: ReadSignal<bool>,

    /// The controlled `open` state of the dialog.
    pub open: ReadSignal<Option<bool>>,

    /// The default `open` state of the dialog if it is not controlled.
    #[props(default)]
    pub default_open: bool,

    /// A callback that is called when the open state changes.
    #[props(default)]
    pub on_open_change: Callback<bool>,

    /// Additional attributes to apply to the dialog root element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the dialog root component.
    pub children: Element,
}

/// # DialogRoot
///
/// The entry point for the dialog. It manages the open state of the dialog and provides context to its children. You
/// can use it to create a backdrop for the dialog if needed. The contents will only be rendered when the dialog is open.
///
/// ## Example
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::dialog::{DialogContent, DialogDescription, DialogRoot, DialogTitle};
///
/// #[component]
/// fn Demo() -> Element {
///     let mut open = use_signal(|| false);
///
///     rsx! {
///         button {
///             onclick: move |_| open.set(true),
///             "Show Dialog"
///         }
///         DialogRoot {
///             open: open(),
///             on_open_change: move |v| open.set(v),
///             DialogContent {
///                 button {
///                     aria_label: "Close",
///                     tabindex: if open() { "0" } else { "-1" },
///                     onclick: move |_| open.set(false),
///                     "×"
///                 }
///                 DialogTitle {
///                     "Item information"
///                 }
///                 DialogDescription {
///                     "Here is some additional information about the item."
///                 }
///             }
///         }
///     }
/// }
/// ```
///
/// ## Styling
///
/// The [`DialogRoot`] component defines the following data attributes you can use to control styling:
/// - `data-state`: Indicates if the dialog is open or closed. It can be either "open" or "closed".
#[component]
pub fn DialogRoot(props: DialogRootProps) -> Element {
    // Installs the permanent scrollbar-gutter baseline as early as possible
    // -- this root mounts whenever a `Dialog` appears on the page, well
    // before `DialogContent` (and its `use_scroll_lock`) first mounts on
    // open. See `scroll_lock.rs`'s module docs and
    // `ensure_scrollbar_gutter_baseline`'s doc comment.
    use_effect(crate::scroll_lock::ensure_scrollbar_gutter_baseline);

    let dialog_labelledby = use_unique_id();
    let dialog_describedby = use_unique_id();

    let (open, set_open) = use_controlled(props.open, props.default_open, props.on_open_change);
    // See this function's doc: captures scroll position before `showModal()`
    // (called from `DialogContent`, mounted as a consequence of the same
    // `open` flip) gets a chance to move it.
    crate::scroll_lock::use_early_scroll_capture(open);

    let unique_id = use_unique_id();
    let id = use_id_or(unique_id, props.id);

    use_context_provider(|| DialogCtx {
        open,
        set_open,
        is_modal: props.is_modal,
        overlay: props.overlay,
        dialog_labelledby,
        dialog_describedby,
    });

    let render = use_animated_open(id, open);

    // Owned by this component -- open state must win over a caller's own
    // attributes (`docs/backlog.md` row 93's duplicate-attribute hazard).
    let attributes = merge_attributes(vec![
        props.attributes,
        attributes!(div {
            aria_hidden: (!open()).then_some("true"),
            "data-state": if open() { "open" } else { "closed" },
            // On the root too, because without a top layer (the non-web arm,
            // `is_modal: false`) this wrapper is what a stylesheet paints the scrim on.
            "data-dx-overlay": overlay_off(props.overlay),
        }),
    ]);

    rsx! {
        {crate::focus_trap_script()}
        if render() {
            div {
                id,
                ..attributes,
                {props.children}
            }
        }
    }
}

/// The props for the [`DialogRoot`] component
#[derive(Props, Clone, PartialEq)]
pub struct DialogContentProps {
    /// The ID of the dialog content element.
    pub id: ReadSignal<Option<String>>,

    /// The class to apply to the dialog content element.
    #[props(default)]
    pub class: Option<String>,

    /// Additional attributes to apply to the dialog content element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
    /// The children of the dialog content.
    pub children: Element,
}

/// # DialogContent
///
/// The content of the dialog. Any interactive content in the dialog should be placed
/// inside this component. It will trap focus within the dialog while it is open
///
/// This must be used inside an [`DialogRoot`] component.
///
/// ## Example
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::dialog::{DialogContent, DialogDescription, DialogRoot, DialogTitle};
///
/// #[component]
/// fn Demo() -> Element {
///     let mut open = use_signal(|| false);
///
///     rsx! {
///         button {
///             onclick: move |_| open.set(true),
///             "Show Dialog"
///         }
///         DialogRoot {
///             open: open(),
///             on_open_change: move |v| open.set(v),
///             DialogContent {
///                 button {
///                     aria_label: "Close",
///                     tabindex: if open() { "0" } else { "-1" },
///                     onclick: move |_| open.set(false),
///                     "×"
///                 }
///                 DialogTitle {
///                     "Item information"
///                 }
///                 DialogDescription {
///                     "Here is some additional information about the item."
///                 }
///             }
///         }
///     }
/// }
/// ```
///
/// ## Styling
///
/// The [`DialogRoot`] component defines the following data attributes you can use to control styling:
/// - `data-state`: Indicates if the dialog is open or closed. It can be either "open" or "closed".
#[component]
pub fn DialogContent(props: DialogContentProps) -> Element {
    let ctx: DialogCtx = use_context();
    let open = ctx.open;
    let is_modal = ctx.is_modal;

    let gen_id = use_unique_id();
    let id = use_id_or(gen_id, props.id);

    // Lock page scroll while the dialog is open and modal -- native
    // `<dialog>`/focus-trap semantics don't cover this. See
    // docs/plan.md Phase 3.2.
    let scroll_lock_active = use_memo(move || is_modal() && open());
    crate::scroll_lock::use_scroll_lock(scroll_lock_active);

    let class = props
        .class
        .clone()
        .unwrap_or_else(|| "dx-dialog".to_string());
    let labelledby = ctx.dialog_labelledby;
    let describedby = ctx.dialog_describedby;

    // Real child-component boundary -- see this module's doc comment for
    // why (hook-order safety, and the modal web arm's dismissal semantics
    // not being reachable via a merely-gated callback).
    if is_modal() {
        rsx! {
            DialogContentModal {
                id,
                class,
                labelledby,
                describedby,
                attributes: props.attributes,
                children: props.children,
            }
        }
    } else {
        rsx! {
            DialogContentNonModal {
                id,
                class,
                labelledby,
                describedby,
                attributes: props.attributes,
                children: props.children,
            }
        }
    }
}

/// `is_modal: false` arm -- unchanged from before Phase 4.2 on both targets
/// (docs/plan.md Phase 4.2 scopes native `<dialog>` to the modal path only).
#[component]
fn DialogContentNonModal(
    id: Memo<String>,
    class: String,
    labelledby: Signal<String>,
    describedby: Signal<String>,
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let ctx: DialogCtx = use_context();
    let set_open = ctx.set_open;

    // Add a escape key listener to the document when the dialog is open. We can't
    // just add this to the dialog itself because it might not be focused if the user
    // is highlighting text or interacting with another element.
    use_global_escape_listener(move || set_open.call(false));
    use_outside_dismiss(id, move || set_open.call(false));

    // Owned by this component -- dialog role/aria wiring must win over a
    // caller's own attributes (`docs/backlog.md` row 93's
    // duplicate-attribute hazard).
    let attributes = merge_attributes(vec![
        attributes,
        attributes!(div {
            role: "dialog",
            aria_modal: "true",
            aria_labelledby: labelledby,
            aria_describedby: describedby,
            // See `DialogRootProps::overlay`: set on every arm so the DOM contract is the same.
            "data-dx-overlay": overlay_off(ctx.overlay),
        }),
    ]);

    rsx! {
        div {
            id,
            class,
            ..attributes,
            {children}
        }
    }
}

/// `is_modal: true` arm, native (Blitz) target -- byte-for-byte the
/// pre-Phase-4.2 path: a plain `div` with the vendored `FocusTrap`.
#[cfg(not(feature = "web"))]
#[component]
fn DialogContentModal(
    id: Memo<String>,
    class: String,
    labelledby: Signal<String>,
    describedby: Signal<String>,
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let ctx: DialogCtx = use_context();
    let open = ctx.open;
    let set_open = ctx.set_open;

    // Add a escape key listener to the document when the dialog is open. We can't
    // just add this to the dialog itself because it might not be focused if the user
    // is highlighting text or interacting with another element.
    use_global_escape_listener(move || set_open.call(false));
    use_outside_dismiss(id, move || set_open.call(false));

    use_effect(move || {
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

    // Owned by this component -- see `DialogContentNonModal`'s identical
    // construction above (`docs/backlog.md` row 93).
    let attributes = merge_attributes(vec![
        attributes,
        attributes!(div {
            role: "dialog",
            aria_modal: "true",
            aria_labelledby: labelledby,
            aria_describedby: describedby,
            // See `DialogRootProps::overlay`: set on every arm so the DOM contract is the same.
            "data-dx-overlay": overlay_off(ctx.overlay),
        }),
    ]);

    rsx! {
        div {
            id,
            class,
            ..attributes,
            {children}
        }
    }
}

/// `is_modal: true` arm, web target (Phase 4.2, docs/plan.md) -- a real
/// `<dialog>` opened with `showModal()`. `open` is never bound as an
/// attribute here (see this module's doc comment); the browser supplies the
/// focus trap, focus restore, background inertness, and top-layer
/// rendering, so this arm installs none of the native arm's JS
/// counterparts.
#[cfg(feature = "web")]
#[component]
fn DialogContentModal(
    id: Memo<String>,
    class: String,
    labelledby: Signal<String>,
    describedby: Signal<String>,
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let ctx: DialogCtx = use_context();
    let open = ctx.open;
    let set_open = ctx.set_open;

    // Browser -> signal (the fix for the historical stranded-signal defect,
    // docs/recommended-implementations.md Caveat 1) and signal -> browser
    // (guarded against `InvalidStateError`), both from `lib.rs`.
    crate::use_dialog_close_sync(id, set_open);
    crate::use_dialog_open_driver(id, open);
    // Native <dialog> has no built-in "click outside to dismiss" the way
    // `popover=` does -- reproduces dialog.spec.ts's "clicking far outside
    // the dialog dismisses it" behaviour. See this module's doc comment for
    // why `use_outside_dismiss` itself can't be reused here.
    crate::use_dialog_backdrop_dismiss(id, move || set_open.call(false));

    // Owned by this component -- see `DialogContentNonModal`'s identical
    // construction above (`docs/backlog.md` row 93).
    let attributes = merge_attributes(vec![
        attributes,
        attributes!(dialog {
            role: "dialog",
            aria_modal: "true",
            aria_labelledby: labelledby,
            aria_describedby: describedby,
            // The dialog's own state, not just the root wrapper's: closing keeps the
            // `<dialog>` open (and modal) while it animates out
            // (`crate::use_dialog_open_driver`), so `[open]` no longer says "closing" and a
            // stylesheet keys the exit on `[data-state="closed"]` instead.
            "data-state": if open() { "open" } else { "closed" },
            "data-dx-overlay": overlay_off(ctx.overlay),
        }),
    ]);

    rsx! {
        dialog {
            id,
            class,
            ..attributes,
            {children}
        }
    }
}

/// The props for the [`DialogTitle`] component
#[derive(Props, Clone, PartialEq)]
pub struct DialogTitleProps {
    /// The ID of the dialog title element.
    pub id: ReadSignal<Option<String>>,
    /// Additional attributes for the dialog title element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
    /// The children of the dialog title.
    pub children: Element,
}

/// # DialogTitle
///
/// The title of the dialog. This will be used to label the dialog for accessibility purposes.
///
/// This must be used inside an [`DialogRoot`] component and should be placed inside an [`DialogContent`] component.
///
/// ## Example
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::dialog::{DialogContent, DialogDescription, DialogRoot, DialogTitle};
///
/// #[component]
/// fn Demo() -> Element {
///     let mut open = use_signal(|| false);
///
///     rsx! {
///         button {
///             onclick: move |_| open.set(true),
///             "Show Dialog"
///         }
///         DialogRoot {
///             open: open(),
///             on_open_change: move |v| open.set(v),
///             DialogContent {
///                 button {
///                     aria_label: "Close",
///                     tabindex: if open() { "0" } else { "-1" },
///                     onclick: move |_| open.set(false),
///                     "×"
///                 }
///                 DialogTitle {
///                     "Item information"
///                 }
///                 DialogDescription {
///                     "Here is some additional information about the item."
///                 }
///             }
///         }
///     }
/// }
/// ```
#[component]
pub fn DialogTitle(props: DialogTitleProps) -> Element {
    let ctx: DialogCtx = use_context();
    let id = use_id_or(ctx.dialog_labelledby, props.id);

    rsx! {
        h2 {
            id: id,
            ..props.attributes,
            {props.children}
        }
    }
}

/// The props for the [`DialogDescription`] component
#[derive(Props, Clone, PartialEq)]
pub struct DialogDescriptionProps {
    /// The ID of the dialog description element.
    pub id: ReadSignal<Option<String>>,
    /// Additional attributes for the dialog description element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
    /// The children of the dialog description.
    pub children: Element,
}

/// # DialogDescription
///
/// The description of the dialog. This will be used to describe the dialog for accessibility purposes.
///
/// This must be used inside an [`DialogRoot`] component and should be placed inside an [`DialogContent`] component.
///
/// ## Example
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::dialog::{DialogContent, DialogDescription, DialogRoot, DialogTitle};
///
/// #[component]
/// fn Demo() -> Element {
///     let mut open = use_signal(|| false);
///
///     rsx! {
///         button {
///             onclick: move |_| open.set(true),
///             "Show Dialog"
///         }
///         DialogRoot {
///             open: open(),
///             on_open_change: move |v| open.set(v),
///             DialogContent {
///                 button {
///                     aria_label: "Close",
///                     tabindex: if open() { "0" } else { "-1" },
///                     onclick: move |_| open.set(false),
///                     "×"
///                 }
///                 DialogTitle {
///                     "Item information"
///                 }
///                 DialogDescription {
///                     "Here is some additional information about the item."
///                 }
///             }
///         }
///     }
/// }
/// ```
#[component]
pub fn DialogDescription(props: DialogDescriptionProps) -> Element {
    let ctx: DialogCtx = use_context();
    let id = use_id_or(ctx.dialog_describedby, props.id);

    rsx! {
        p {
            id: id,
            ..props.attributes,
            {props.children}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Renders `app` and lets the open effect (`use_animated_open`) run, which is what puts the
    /// content in the DOM: it is absent from the first pass.
    fn render_open(app: fn() -> Element) -> String {
        let mut dom = VirtualDom::new(app);
        dom.rebuild_in_place();
        // First pass: the effect flips `show_in_dom`. Second: the content renders.
        dom.process_events();
        dom.render_immediate_to_vec();
        dioxus_ssr::render(&dom)
    }

    #[component]
    fn DialogDefault() -> Element {
        rsx! { DialogRoot { default_open: true, DialogContent { "body" } } }
    }

    #[component]
    fn DialogOverlayOff() -> Element {
        rsx! { DialogRoot { default_open: true, overlay: false, DialogContent { "body" } } }
    }

    #[component]
    fn AlertDialogOverlayOff() -> Element {
        rsx! {
            crate::alert_dialog::AlertDialogRoot {
                default_open: true,
                overlay: false,
                crate::alert_dialog::AlertDialogContent { "body" }
            }
        }
    }

    #[component]
    fn DrawerOverlayOff() -> Element {
        rsx! {
            crate::drawer::Drawer {
                default_open: true,
                overlay: false,
                crate::drawer::DrawerContent { "body" }
            }
        }
    }

    #[component]
    fn CommandDialogOverlayOff() -> Element {
        rsx! {
            crate::command::CommandDialog { default_open: true, overlay: false, "body" }
        }
    }

    #[component]
    fn CommandDialogDefault() -> Element {
        rsx! {
            crate::command::CommandDialog { default_open: true, "body" }
        }
    }

    /// `overlay` defaults to `true`, and `true` renders no attribute at all: the dialog's
    /// `::backdrop` paints the shared scrim and nothing in the DOM says otherwise.
    #[test]
    fn overlay_is_on_by_default_and_renders_no_marker() {
        let html = render_open(DialogDefault);
        assert!(
            html.contains("body"),
            "the content must have rendered: {html}"
        );
        assert!(!html.contains("data-dx-overlay"), "{html}");
        let html = render_open(CommandDialogDefault);
        assert!(html.contains("body"), "{html}");
        assert!(!html.contains("data-dx-overlay"), "{html}");
    }

    /// `overlay: false` puts `data-dx-overlay="off"` on BOTH the root wrapper (the scrim
    /// painter where there is no top layer) and the content element (the `<dialog>` on the
    /// web arm, which the theme's `dialog[data-dx-overlay="off"]::backdrop` rule keys on).
    #[test]
    fn overlay_false_marks_the_root_and_the_content_of_every_composer() {
        type App = fn() -> Element;
        let composers: [(&str, App); 4] = [
            ("DialogRoot", DialogOverlayOff),
            ("AlertDialogRoot", AlertDialogOverlayOff),
            ("Drawer", DrawerOverlayOff),
            ("CommandDialog", CommandDialogOverlayOff),
        ];
        for (name, app) in composers {
            let html = render_open(app);
            assert!(
                html.contains("body"),
                "{name}: the content must have rendered: {html}"
            );
            let marked = html.matches(r#"data-dx-overlay="off""#).count();
            assert_eq!(
                marked, 2,
                "{name}: expected the root and the content to carry data-dx-overlay=\"off\": {html}"
            );
        }
    }
}
