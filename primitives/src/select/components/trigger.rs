//! SelectTrigger component implementation.

use crate::{merge_attributes, use_id_or};
use dioxus::prelude::*;
use dioxus_attributes::attributes;

use super::super::context::SelectContext;

/// The props for the [`SelectTrigger`] component
#[derive(Props, Clone, PartialEq)]
pub struct SelectTriggerProps {
    /// The ID of the trigger button. If not provided, an internally
    /// generated ID is used -- see `SelectTrigger`'s use of `use_id_or`
    /// below for why this is a typed field rather than left to
    /// `GlobalAttributes` alone (docs/backlog.md row 36).
    pub id: ReadSignal<Option<String>>,

    /// Additional attributes for the trigger button
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children to render inside the trigger
    pub children: Element,
}

/// # SelectTrigger
///
/// The trigger button for the [`Select`](super::select::Select) component which controls if the [`SelectList`](super::list::SelectList) is rendered.
///
/// This must be used inside a [`Select`](super::select::Select) component.
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::select::{
///     Select, SelectGroup, SelectGroupLabel, SelectItemIndicator, SelectList, SelectOption,
///     SelectTrigger, SelectValue,
/// };
/// #[component]
/// fn Demo() -> Element {
///     rsx! {
///         Select::<String> {
///             SelectTrigger {
///                 aria_label: "Select Trigger",
///                 width: "12rem",
///                 SelectValue { placeholder: "Select a fruit..." }
///             }
///             SelectList {
///                 aria_label: "Select Demo",
///                 SelectGroup {
///                     SelectGroupLabel { "Fruits" }
///                     SelectOption::<String> {
///                         index: 0usize,
///                         value: "apple",
///                         "Apple"
///                         SelectItemIndicator { "✔️" }
///                     }
///                     SelectOption::<String> {
///                         index: 1usize,
///                         value: "banana",
///                         "Banana"
///                         SelectItemIndicator { "✔️" }
///                     }
///                 }
///             }
///         }
///     }
/// }
/// ```
///
///
/// ## Styling
///
/// The [`SelectTrigger`] component defines a span with a `data-placeholder` attribute if a placeholder is set.
#[component]
pub fn SelectTrigger(props: SelectTriggerProps) -> Element {
    let mut ctx = use_context::<SelectContext>();
    let open = ctx.selectable.open;

    // docs/backlog.md row 36: `ctx.selectable.trigger_id` is the same
    // signal `SelectList`'s `aria-labelledby` (`list.rs`) and the anchor-
    // position fallback (`ctx.selectable.trigger_id.cloned()` below and in
    // `list.rs`) both read back -- so a caller-supplied `id` here must
    // resolve into it, not just win the render. Feeding it straight into
    // `use_id_or` as the generated-id signal does that: `use_id_or`'s own
    // effect writes a caller override back into its `gen_id` argument,
    // which *is* `ctx.selectable.trigger_id` here (no separate local
    // placeholder to keep in sync, unlike `DropdownMenuContent`'s
    // `content_id`) -- mirrors `DialogTitle`'s identical
    // `use_id_or(ctx.dialog_labelledby, props.id)` (`dialog.rs`).
    let id = use_id_or(ctx.selectable.trigger_id, props.id);

    // Merged (caller-wins, deduped) rather than set here and then spread
    // over by `..props.attributes` below: a caller `id`/`aria_label`/etc.
    // override (e.g. the top-layer oracle fixture's
    // `id: "clip-select-trigger"`) used to duplicate the attribute in the
    // SSR'd HTML instead of replacing it -- WHATWG HTML's duplicate-
    // attribute parse error keeps the *first* (this component's own
    // default) while the CSR/hydrated DOM path keeps the *last* (the
    // caller's), so server and client disagreed
    // (`docs/conformance-harness.md` hydration-parity Rule 4; see
    // `toast.rs`'s `ToastRegionRendered` doc for the first component this
    // was found and fixed in).
    let attributes = merge_attributes(vec![
        attributes!(button {
            id: id.cloned(),
            // See `crate::top_layer::anchor_name_style`: ties this trigger
            // to the web-arm listbox's `position-anchor`
            // (`SelectListRendered`, `list.rs`) so its anchor-positioned
            // placement resolves relative to this trigger once promoted to
            // the top layer. Keyed on `ctx.selectable.trigger_id` -- kept in
            // sync with this button's own (possibly caller-overridden) `id`
            // above, so the two sides still agree once promoted to the top
            // layer. Inert (empty) off the web arm.
            style: crate::top_layer::anchor_name_style(&ctx.selectable.trigger_id.cloned()),
            // Standard HTML attributes
            disabled: (ctx.selectable.disabled)(),
            type: "button",
            // ARIA attributes
            aria_haspopup: "listbox",
            aria_expanded: open(),
            aria_controls: ctx.selectable.list_id,
            // NOTE: aria-required is deliberately NOT set here — it is not a
            // supported property on an (implicit) button role, and axe flags
            // it. Requiredness is enforced by the hidden native
            // <select required> mirror; exposing it to AT properly means
            // adopting the APG select-only-combobox trigger role
            // (role="combobox"), which is a larger semantic change tracked as
            // follow-up work.
        }),
        props.attributes,
    ]);

    rsx! {
        button {
            onclick: move |_| {
                ctx.set_open(!open());
            },
            onkeydown: move |event| {
                match event.key() {
                    Key::ArrowUp => {
                        ctx.set_open(true);
                        ctx.selectable
                            .initial_focus
                            .set(ctx.selectable.collection.last_available_index());
                        event.prevent_default();
                        event.stop_propagation();
                    }
                    Key::ArrowDown => {
                        ctx.set_open(true);
                        // APG select-only combobox (Optional): "Alt + Down
                        // Arrow: ... displays the popup without moving
                        // focus." Plain ArrowDown (no Alt) still moves focus
                        // to the first option, same as Up Arrow above.
                        if event.modifiers().alt() {
                            ctx.keep_trigger_focus.set(true);
                        } else {
                            ctx.selectable
                                .initial_focus
                                .set(ctx.selectable.collection.first_available_index());
                        }
                        event.prevent_default();
                        event.stop_propagation();
                    }
                    Key::Enter => {
                        ctx.open_with_selected_or_first_focus();
                        event.prevent_default();
                        event.stop_propagation();
                    }
                    Key::Character(c) if c == " " => {
                        ctx.open_with_selected_or_first_focus();
                        event.prevent_default();
                        event.stop_propagation();
                    }
                    _ => {}
                }
            },

            // Pass through the merged, deduped attributes
            ..attributes,

            // Render children (options)
            {props.children}
        }
    }
}
