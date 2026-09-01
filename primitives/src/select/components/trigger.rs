//! SelectTrigger component implementation.

use crate::merge_attributes;
use dioxus::prelude::*;
use dioxus_attributes::attributes;

use super::super::context::SelectContext;

/// The props for the [`SelectTrigger`] component
#[derive(Props, Clone, PartialEq)]
pub struct SelectTriggerProps {
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
            id: ctx.selectable.trigger_id,
            // See `crate::top_layer::anchor_name_style`: ties this trigger
            // to the web-arm listbox's `position-anchor`
            // (`SelectListRendered`, `list.rs`) so its anchor-positioned
            // placement resolves relative to this trigger once promoted to
            // the top layer. Keyed on `ctx.selectable.trigger_id` -- this
            // button's own, already-stable id -- rather than a separately
            // synced content id like `DropdownMenu`/`Menubar` use: unlike
            // those two, this id is already fixed and known here, so no
            // extra id-sync plumbing is needed for the two sides to agree.
            // Inert (empty) off the web arm.
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
                        ctx.selectable
                            .initial_focus
                            .set(ctx.selectable.collection.first_available_index());
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
