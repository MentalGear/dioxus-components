use dioxus::prelude::*;
use dioxus_icons::lucide::{Check, ChevronDown};
use dioxus_primitives::select::{self, SelectGroupLabelProps, SelectOptionProps};
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};
use std::time::Duration;

pub use dioxus_primitives::select::SelectGroup;

// docs/backlog.md row 32: `#[css_module]` is gone -- see checkbox/component.rs's
// header comment for the full delivery-mechanism rationale (asset!() +
// document::Link, embedded in the wrapper so a `dx components add
// select`-copied component needs no extra wiring). This file has TWO
// independent entry components (`Select` and `SelectMulti` -- callers use
// exactly one, never both), so the stylesheet Link is placed in both of
// them rather than only the first, mirroring how `#[css_module]`'s own
// single `OnceLock`-guarded injection used to fire from whichever of the
// two happened to render first. `SelectGroupLabel`/`SelectOption` are
// always composed as children of one of those two (never standalone, per
// this crate's own API), so they don't need their own copy --
// `document::Link`'s `(href, rel)` dedup makes it harmless either way if a
// future edit adds one there too.

/// Props for the themed [`Select`]. Deliberately its own struct rather than
/// a reuse of `dioxus_primitives::select::SelectProps` (as the other
/// wrappers in this file still do for their simpler primitives): callers
/// that need to customize the trigger's accessible name/content or the
/// option list's accessible name (e.g. the dashboard email client's tag
/// filter, or the form fixture's required-field labels) have nowhere to
/// pass that through `SelectTrigger`/`SelectList`'s own props once this
/// wrapper hard-codes them internally, so those become explicit fields here
/// instead.
#[derive(Props, Clone, PartialEq)]
pub struct SelectProps<T: Clone + PartialEq + 'static = String> {
    /// The controlled value of the select. If supplied, the select is
    /// controlled and the signal's `None` value means no option is
    /// selected.
    #[props(default)]
    pub value: Option<ReadSignal<Option<T>>>,

    /// The initial value of the select when uncontrolled.
    #[props(default)]
    pub default_value: Option<T>,

    /// Callback fired when the selected value changes.
    #[props(default)]
    pub on_value_change: Callback<Option<T>>,

    /// Whether the select is disabled.
    #[props(default)]
    pub disabled: ReadSignal<bool>,

    /// The controlled open state of the select popup.
    #[props(default)]
    pub open: ReadSignal<Option<bool>>,

    /// The initial open state when uncontrolled.
    #[props(default)]
    pub default_open: ReadSignal<bool>,

    /// Callback fired when the popup open state changes.
    #[props(default)]
    pub on_open_change: Callback<bool>,

    /// Name of the select for form submission.
    #[props(default)]
    pub name: ReadSignal<String>,

    /// Whether a selection is required for form submission.
    #[props(default)]
    pub required: ReadSignal<bool>,

    /// Whether focus should loop around when reaching the end.
    #[props(default = ReadSignal::new(Signal::new(true)))]
    pub roving_loop: ReadSignal<bool>,

    /// Timeout in milliseconds before clearing the typeahead buffer.
    #[props(default = ReadSignal::new(Signal::new(Duration::from_millis(1000))))]
    pub typeahead_timeout: ReadSignal<Duration>,

    /// ARIA label applied to the trigger button. Ignored when `trigger` is
    /// supplied -- the caller's custom trigger content is then responsible
    /// for its own accessible name.
    #[props(default)]
    pub trigger_aria_label: Option<String>,

    /// ARIA label applied to the option list.
    #[props(default)]
    pub list_aria_label: Option<String>,

    /// Placeholder text shown by the default trigger content
    /// (`SelectValue`) while nothing is selected. Ignored when `trigger` is
    /// supplied.
    #[props(default)]
    pub placeholder: Option<String>,

    /// Extra class appended alongside `"dx-select-trigger"`.
    #[props(default)]
    pub trigger_class: Option<String>,

    /// Extra class appended alongside `"dx-select-list"`.
    #[props(default)]
    pub list_class: Option<String>,

    /// Custom content for the trigger button, replacing the default
    /// `SelectValue`. The themed chevron icon is always appended after it.
    #[props(default)]
    pub trigger: Option<Element>,

    /// Additional attributes for the select's root element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The option/group children rendered inside the themed option list.
    pub children: Element,
}

/// Props for the themed [`SelectMulti`]. See [`SelectProps`] for why this
/// wrapper defines its own props struct instead of reusing the primitive's.
#[derive(Props, Clone, PartialEq)]
pub struct SelectMultiProps<T: Clone + PartialEq + 'static = String> {
    /// The controlled list of selected values.
    #[props(default)]
    pub values: ReadSignal<Option<Vec<T>>>,

    /// The default list of selected values.
    #[props(default)]
    pub default_values: Vec<T>,

    /// Callback when the list of selected values changes.
    #[props(default)]
    pub on_values_change: Callback<Vec<T>>,

    /// Whether the select is disabled.
    #[props(default)]
    pub disabled: ReadSignal<bool>,

    /// The controlled open state of the select popup.
    #[props(default)]
    pub open: ReadSignal<Option<bool>>,

    /// The initial open state when uncontrolled.
    #[props(default)]
    pub default_open: ReadSignal<bool>,

    /// Callback fired when the popup open state changes.
    #[props(default)]
    pub on_open_change: Callback<bool>,

    /// Name of the select for form submission.
    #[props(default)]
    pub name: ReadSignal<String>,

    /// Whether focus should loop around when reaching the end.
    #[props(default = ReadSignal::new(Signal::new(true)))]
    pub roving_loop: ReadSignal<bool>,

    /// Timeout in milliseconds before clearing the typeahead buffer.
    #[props(default = ReadSignal::new(Signal::new(Duration::from_millis(1000))))]
    pub typeahead_timeout: ReadSignal<Duration>,

    /// ARIA label applied to the trigger button. Ignored when `trigger` is
    /// supplied.
    #[props(default)]
    pub trigger_aria_label: Option<String>,

    /// ARIA label applied to the option list.
    #[props(default)]
    pub list_aria_label: Option<String>,

    /// Placeholder text shown by the default trigger content
    /// (`SelectValue`) while nothing is selected. Ignored when `trigger` is
    /// supplied.
    #[props(default)]
    pub placeholder: Option<String>,

    /// Extra class appended alongside `"dx-select-trigger"`.
    #[props(default)]
    pub trigger_class: Option<String>,

    /// Extra class appended alongside `"dx-select-list"`.
    #[props(default)]
    pub list_class: Option<String>,

    /// Custom content for the trigger button, replacing the default
    /// `SelectValue`. The themed chevron icon is always appended after it.
    #[props(default)]
    pub trigger: Option<Element>,

    /// Additional attributes for the select's root element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The option/group children rendered inside the themed option list.
    pub children: Element,
}

/// Merges `extra` alongside `base`, `base`-first, for the handful of themed
/// sub-elements (trigger, list) that take an optional caller-supplied extra
/// class on top of their own fixed theme class.
fn with_extra_class(base: impl std::fmt::Display, extra: &Option<String>) -> String {
    match extra {
        Some(extra) if !extra.is_empty() => format!("{base} {extra}"),
        _ => base.to_string(),
    }
}

#[component]
pub fn Select<T: Clone + PartialEq + 'static>(props: SelectProps<T>) -> Element {
    let base = attributes!(div { class: "dx-select" });
    let merged = merge_attributes(vec![base, props.attributes]);
    let trigger_class = with_extra_class("dx-select-trigger", &props.trigger_class);
    let list_class = with_extra_class("dx-select-list", &props.list_class);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/select/style.css") }
        select::Select {
            value: props.value,
            default_value: props.default_value,
            on_value_change: props.on_value_change,
            disabled: props.disabled,
            open: props.open,
            default_open: props.default_open,
            on_open_change: props.on_open_change,
            name: props.name,
            required: props.required,
            roving_loop: props.roving_loop,
            typeahead_timeout: props.typeahead_timeout,
            attributes: merged,
            select::SelectTrigger {
                class: trigger_class,
                aria_label: props.trigger_aria_label.clone(),
                if let Some(trigger) = &props.trigger {
                    {trigger.clone()}
                } else if let Some(placeholder) = props.placeholder.clone() {
                    select::SelectValue { placeholder }
                } else {
                    // No caller-supplied placeholder: leave `SelectValue`'s
                    // own field unset so its primitive default ("Select an
                    // option") applies, rather than overriding it with an
                    // empty string.
                    select::SelectValue {}
                }
                ChevronDown {
                    class: "dx-select-expand-icon",
                    size: "20px",
                    stroke: "var(--primary-color-7)",
                }
            }
            select::SelectList {
                class: list_class,
                aria_label: props.list_aria_label.clone(),
                {props.children}
            }
        }
    }
}

#[component]
pub fn SelectMulti<T: Clone + PartialEq + 'static>(props: SelectMultiProps<T>) -> Element {
    let base = attributes!(div { class: "dx-select" });
    let merged = merge_attributes(vec![base, props.attributes]);
    let trigger_class = with_extra_class("dx-select-trigger", &props.trigger_class);
    let list_class = with_extra_class("dx-select-list", &props.list_class);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/select/style.css") }
        select::SelectMulti {
            values: props.values,
            default_values: props.default_values,
            on_values_change: props.on_values_change,
            disabled: props.disabled,
            open: props.open,
            default_open: props.default_open,
            on_open_change: props.on_open_change,
            name: props.name,
            roving_loop: props.roving_loop,
            typeahead_timeout: props.typeahead_timeout,
            attributes: merged,
            select::SelectTrigger {
                class: trigger_class,
                aria_label: props.trigger_aria_label.clone(),
                if let Some(trigger) = &props.trigger {
                    {trigger.clone()}
                } else if let Some(placeholder) = props.placeholder.clone() {
                    select::SelectValue { placeholder }
                } else {
                    // No caller-supplied placeholder: leave `SelectValue`'s
                    // own field unset so its primitive default ("Select an
                    // option") applies, rather than overriding it with an
                    // empty string.
                    select::SelectValue {}
                }
                ChevronDown {
                    class: "dx-select-expand-icon",
                    size: "20px",
                    stroke: "var(--primary-color-7)",
                }
            }
            select::SelectList {
                class: list_class,
                aria_label: props.list_aria_label.clone(),
                {props.children}
            }
        }
    }
}

#[component]
pub fn SelectGroupLabel(props: SelectGroupLabelProps) -> Element {
    let base = attributes!(div { class: "dx-select-group-label" });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        select::SelectGroupLabel {
            id: props.id,
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn SelectOption<T: Clone + PartialEq + 'static>(props: SelectOptionProps<T>) -> Element {
    let base = attributes!(div { class: "dx-select-option" });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        select::SelectOption::<T> {
            value: props.value,
            text_value: props.text_value,
            disabled: props.disabled,
            id: props.id,
            index: props.index,
            aria_label: props.aria_label,
            aria_roledescription: props.aria_roledescription,
            attributes: merged,
            {props.children}
            select::SelectItemIndicator {
                Check {
                    size: "1rem",
                    stroke: "var(--secondary-color-5)",
                }
            }
        }
    }
}
