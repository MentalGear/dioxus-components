//! Defines the [`RadioGroup`] component and its sub-components.

use std::collections::HashMap;

use crate::{
    collection::{
        collection_item, use_collection_provider_with, use_item, CollectionOptions, CollectionState,
    },
    direction::{use_direction, Direction, HorizontalNav},
    merge_attributes, use_controlled, use_effect_with_cleanup, use_form_reset_listener,
    use_unique_id,
};
use dioxus::prelude::*;
use dioxus_attributes::attributes;

#[derive(Clone, Copy)]
struct RadioGroupCtx {
    // State
    disabled: ReadSignal<bool>,
    required: ReadSignal<bool>,
    name: ReadSignal<String>,
    /// The value the group resets to on `<form reset>`. Mirrors
    /// `Checkbox`'s `default_checked` (it is the *prop*, not a live snapshot
    /// of the value at mount, so it stays correct even if `default_value`
    /// were ever changed dynamically).
    default_value: ReadSignal<String>,
    value: Memo<String>,
    set_value: Callback<String>,

    // Keyboard nav data
    // A map of tabindex -> value in the enabled radio items
    values: Signal<HashMap<usize, String>>,
    focus: CollectionState,

    horizontal: ReadSignal<bool>,

    // Text direction, for `ArrowLeft`/`ArrowRight`'s roving-focus role --
    // see `direction::Direction::resolve_horizontal`'s doc.
    direction: Direction,
}

impl RadioGroupCtx {
    /// Set the currently focused radio item.
    ///
    /// This should be used by `focus`/`focusout` event only to start tracking focus.
    fn set_focus(&mut self, id: Option<usize>) {
        self.focus.set_focus(id);
    }

    /// Set the value of the radio group.
    fn set_value(&mut self, value: String) {
        let current_value = self.value.peek();
        if *current_value == value {
            return; // No change, do nothing
        }
        self.set_value.call(value);
    }

    fn focus_next(&mut self) {
        self.focus.focus_next();
        self.select_focused_value();
    }

    fn focus_prev(&mut self) {
        self.focus.focus_prev();
        self.select_focused_value();
    }

    fn select_focused_value(&mut self) {
        if let Some(current_focus) = self.focus.focused_index() {
            let value = { self.values.read().get(&current_focus).cloned() };
            if let Some(value) = value {
                self.set_value(value.clone());
            }
        }
    }

    fn focus_start(&mut self) {
        self.focus.focus_first();
    }

    fn focus_end(&mut self) {
        self.focus.focus_last();
    }
}

/// The props for the [`RadioGroup`] component.
#[derive(Props, Clone, PartialEq)]
pub struct RadioGroupProps {
    /// The controlled value of the selected radio item.
    pub value: ReadSignal<Option<String>>,

    /// The default selected value when uncontrolled.
    #[props(default)]
    pub default_value: String,

    /// Callback fired when the selected value changes.
    #[props(default)]
    pub on_value_change: Callback<String>,

    /// Whether the radio group is disabled.
    #[props(default)]
    pub disabled: ReadSignal<bool>,

    /// Whether the radio group is required in a form.
    #[props(default)]
    pub required: ReadSignal<bool>,

    /// The name attribute for form submission.
    #[props(default)]
    pub name: ReadSignal<String>,

    /// Whether the radio group is horizontal.
    #[props(default)]
    pub horizontal: ReadSignal<bool>,

    /// The text direction for `ArrowLeft`/`ArrowRight` roving focus.
    /// Defaults to the nearest [`crate::direction::DirectionProvider`], or
    /// LTR if there is none. See [`crate::direction::use_direction`].
    #[props(default)]
    pub dir: Option<Direction>,

    /// Whether focus should loop around when reaching the end.
    #[props(default = ReadSignal::new(Signal::new(true)))]
    pub roving_loop: ReadSignal<bool>,

    /// Additional attributes to apply to the radio group element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the radio group component.
    pub children: Element,
}

/// # RadioGroup
///
/// The `RadioGroup` component is a container for a group of [`RadioItem`] components that allows users to select a single option from a list of choices.
///
/// ## Example
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::radio_group::{RadioGroup, RadioItem};
///
/// #[component]
/// fn Demo() -> Element {
///     rsx! {
///         RadioGroup {
///             RadioItem {
///                 value: "option1".to_string(),
///                 index: 0usize,
///                 "Blue"
///             }
///             RadioItem {
///                 value: "option2".to_string(),
///                 index: 1usize,
///                 "Red"
///             }
///             RadioItem {
///                 value: "option3".to_string(),
///                 index: 2usize,
///                 disabled: true,
///                 "Green"
///             }
///         }
///     }
/// }
/// ```
///
/// ## Styling
///
/// The [`RadioGroup`] component defines the following data attributes you can use to control styling:
/// - `data-orientation`: Indicates the orientation of the radio group. Values are `horizontal` or `vertical`.
/// - `data-disabled`: Indicates if the radio group is disabled. Values are `true` or `false`.
/// - `data-direction`: The resolved text direction. Values are `ltr` or `rtl`.
#[component]
pub fn RadioGroup(props: RadioGroupProps) -> Element {
    let direction = use_direction(props.dir);
    // Snapshot the default before `use_controlled` consumes it -- each
    // RadioItem's hidden radio needs it (for `initial_checked`) and the
    // group's own form-reset listener needs it (to restore the Rust-side
    // selection). `RadioGroupCtx` derives `Copy`, so this has to be a
    // `Signal`, not the plain `String` the prop is.
    let default_value = use_signal(|| props.default_value.clone());
    let (value, set_value) =
        use_controlled(props.value, props.default_value, props.on_value_change);

    // Native radio-group semantics: when nothing is selected, every radio stays
    // in the tab order until one is focused or checked.
    let focus = use_collection_provider_with(
        props.roving_loop,
        CollectionOptions {
            tabbable_when_empty: true,
        },
    );
    let mut ctx = use_context_provider(|| RadioGroupCtx {
        value,
        set_value,
        disabled: props.disabled,
        required: props.required,
        name: props.name,
        default_value: default_value.into(),

        values: Signal::new(Default::default()),
        focus,
        horizontal: props.horizontal,
        direction,
    });

    let owned = attributes!(div {
        role: "radiogroup",
        "data-orientation": if (props.horizontal)() { "horizontal" } else { "vertical" },
        "data-disabled": (props.disabled)(),
        "data-direction": direction.as_str(),
        aria_required: props.required,
    });
    let merged = merge_attributes(vec![props.attributes.clone(), owned]);

    rsx! {
        div {
            dir: direction.as_str(),

            onfocusout: move |_| ctx.set_focus(None),
            ..merged,

            {props.children}
        }
    }
}

/// The props for the [`RadioItem`] component
#[derive(Props, Clone, PartialEq)]
pub struct RadioItemProps {
    /// The value of the radio item. This will be passed to [`RadioGroupProps::on_value_change`] when selected.
    pub value: ReadSignal<String>,
    /// The index of the radio item within the [`RadioGroup`]. This is used to order the items for keyboard navigation.
    pub index: ReadSignal<usize>,

    /// Whether the radio item is disabled.
    #[props(default)]
    pub disabled: ReadSignal<bool>,

    /// Optional ID for the radio item element.
    pub id: Option<String>,
    /// Optional class for the radio item element.
    pub class: Option<String>,

    /// Additional attributes to apply to the radio item element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the radio item component.
    pub children: Element,
}

/// # RadioItem
///
/// The `RadioItem` component represents a single radio button within a [`RadioGroup`]. Only one radio item can be selected at a time within a group.
///
/// This must be used inside a [`RadioGroup`] component.
///
/// ## Example
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::radio_group::{RadioGroup, RadioItem};
///
/// #[component]
/// fn Demo() -> Element {
///     rsx! {
///         RadioGroup {
///             RadioItem {
///                 value: "option1".to_string(),
///                 index: 0usize,
///                 "Blue"
///             }
///             RadioItem {
///                 value: "option2".to_string(),
///                 index: 1usize,
///                 "Red"
///             }
///             RadioItem {
///                 value: "option3".to_string(),
///                 index: 2usize,
///                 disabled: true,
///                 "Green"
///             }
///         }
///     }
/// }
/// ```
///
/// ## Styling
///
/// The [`RadioItem`] component defines the following data attributes you can use to control styling:
/// - `data-state`: Indicates the state of the radio item. Values are `checked` or `unchecked`.
/// - `data-disabled`: Indicates if the radio item is disabled. Values are `true` or `false`.
#[component]
pub fn RadioItem(props: RadioItemProps) -> Element {
    let mut ctx: RadioGroupCtx = use_context();
    let disabled = move || (ctx.disabled)() || (props.disabled)();

    use_effect_with_cleanup(move || {
        let index = (props.index)();
        if disabled() {
            ctx.values.write().remove(&index);
        } else {
            ctx.values.write().insert(index, (props.value)());
        }
        move || {
            ctx.values.write().remove(&index);
        }
    });

    let value = (props.value)().clone();
    let checked = use_memo(move || (ctx.value)() == value);

    let item = use_item(
        collection_item(ctx.focus, props.index)
            .disabled(disabled)
            .selected(move || checked.cloned()),
    );
    let tab_index = item.tabindex;
    let onmounted = item.onmounted();

    // Hidden native radio mirroring this item, so the group participates in
    // HTML form submission -- see docs/plan.md Phase 1.2. Rendered
    // unconditionally, matching Checkbox's BubbleInput: this repo's decision
    // table (docs/recommended-implementations.md §1) follows upstream over
    // Radix's `closest('form')` gate, which Dioxus has no cheap equivalent
    // for. Shape ported from dignifiedquire/dx-components (MIT OR
    // Apache-2.0), primitives/src/radio_group.rs (RadioGroupItem's hidden
    // `input`, ~L260-273 as read from that fork's tree) -- adapted to this
    // repo's per-item `checked` memo, the `initial_checked`/`checked` split
    // needed for correct `<form reset>` behavior (that fork's own hidden
    // input has the same defect as this repo's `Checkbox::BubbleInput` had;
    // see checkbox.rs), and a `reset` listener that resyncs the Dioxus-side
    // selection, which neither fork implements.
    let hidden_input_id = use_unique_id();
    use_form_reset_listener(hidden_input_id, move || {
        ctx.set_value(ctx.default_value.cloned());
    });
    let is_default = use_memo(move || (ctx.default_value)() == (props.value)());

    // `type` is an overridable default; the rest is this item's own
    // functional state (role/tabindex define the widget and its
    // roving-focus wiring, aria-checked/data-state/data-disabled mirror
    // its real state).
    let defaults = attributes!(button { r#type: "button" });
    let owned = attributes!(button {
        role: "radio",
        tabindex: tab_index,
        aria_checked: checked,
        "data-state": if checked() { "checked" } else { "unchecked" },
        "data-disabled": disabled(),
    });
    let merged = merge_attributes(vec![defaults, props.attributes.clone(), owned]);

    rsx! {
        button {
            id: props.id,
            class: props.class,
            disabled: disabled(),

            onclick: move |_| {
                let value = (props.value)().clone();
                ctx.set_value(value);
            },

            onmounted,
            onfocus: move |_| ctx.set_focus(Some((props.index)())),

            onkeydown: move |event: Event<KeyboardData>| {
                let key = event.key();
                let horizontal = (ctx.horizontal)();
                let mut prevent_default = true;
                match key {
                    Key::ArrowUp if !horizontal => ctx.focus_prev(),
                    Key::ArrowDown if !horizontal => ctx.focus_next(),
                    Key::ArrowLeft | Key::ArrowRight if horizontal => {
                        match ctx.direction.resolve_horizontal(&key) {
                            Some(HorizontalNav::Prev) => ctx.focus_prev(),
                            Some(HorizontalNav::Next) => ctx.focus_next(),
                            None => {}
                        }
                    }
                    Key::Home => ctx.focus_start(),
                    Key::End => ctx.focus_end(),
                    _ => prevent_default = false,
                };
                if prevent_default {
                    event.prevent_default();
                }
            },
            ..merged,

            {props.children}
        }

        input {
            id: hidden_input_id,
            type: "radio",
            aria_hidden: "true",
            tabindex: "-1",
            position: "absolute",
            pointer_events: "none",
            opacity: "0",
            margin: "0",
            transform: "translateX(-100%)",

            name: ctx.name,
            value: (props.value)(),
            required: ctx.required,
            disabled: disabled(),
            // Live sync + default, for the same reason as Checkbox's
            // BubbleInput and Switch's hidden input (see their comments):
            // `checked` keeps this mirror in step with the visible radio,
            // `initial_checked` is what `<form reset>` actually restores from.
            checked: checked(),
            initial_checked: is_default(),
        }
    }
}
