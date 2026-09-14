use dioxus::prelude::*;
use dioxus_icons::lucide::Search;
use dioxus_primitives::combobox::default_combobox_filter;
use dioxus_primitives::command::{self, CommandEmptyProps, CommandGroupLabelProps, CommandItemProps};
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};

// docs/backlog.md row 32: plain `dx-`-prefixed CSS, no `#[css_module]`
// hashing -- delivered the same `asset!()` + `document::Link` way every
// other themed wrapper in this crate delivers its stylesheet (see
// checkbox/component.rs's header comment for the full rationale).

pub use dioxus_primitives::command::CommandGroup;

/// Props for the themed [`CommandDialog`].
#[derive(Props, Clone, PartialEq)]
pub struct CommandDialogProps {
    #[props(default)]
    pub id: ReadSignal<Option<String>>,

    #[props(default)]
    pub open: ReadSignal<Option<bool>>,

    #[props(default)]
    pub default_open: bool,

    #[props(default)]
    pub on_open_change: Callback<bool>,

    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    pub children: Element,
}

/// A modal dialog hosting a [`Command`] palette. Composes
/// `dioxus_primitives::command::CommandDialog` (itself `DialogRoot` +
/// `DialogContent`, unmodified) with this crate's dialog theme.
///
/// `command::CommandDialog` already carries its own literal
/// `dx-command-dialog-backdrop`/`dx-command-dialog` default classes (see
/// that primitive's doc), so this wrapper only needs to forward the
/// caller's own extra `attributes` -- unlike most other themed wrappers in
/// this crate, it does not synthesize a base class of its own.
#[component]
pub fn CommandDialog(props: CommandDialogProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/command/style.css") }
        command::CommandDialog {
            id: props.id,
            open: props.open,
            default_open: props.default_open,
            on_open_change: props.on_open_change,
            attributes: props.attributes,
            {props.children}
        }
    }
}

/// Props for the themed [`Command`].
#[derive(Props, Clone, PartialEq)]
pub struct CommandProps<T: Clone + PartialEq + 'static = String> {
    #[props(default)]
    pub value: Option<ReadSignal<Option<T>>>,

    #[props(default)]
    pub default_value: Option<T>,

    #[props(default)]
    pub on_value_change: Callback<Option<T>>,

    #[props(default)]
    pub disabled: ReadSignal<bool>,

    #[props(default)]
    pub query: ReadSignal<Option<String>>,

    #[props(default)]
    pub default_query: ReadSignal<String>,

    #[props(default)]
    pub on_query_change: Callback<String>,

    #[props(default = ReadSignal::new(Signal::new(true)))]
    pub roving_loop: ReadSignal<bool>,

    #[props(default = Callback::new(|(q, t): (String, String)| default_combobox_filter(&q, &t)))]
    pub filter: Callback<(String, String), bool>,

    #[props(default)]
    pub placeholder: ReadSignal<String>,

    /// Accessible name for the filter input.
    #[props(default)]
    pub input_aria_label: Option<String>,

    /// Accessible name for the item list.
    #[props(default)]
    pub list_aria_label: Option<String>,

    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// [`CommandGroup`]/[`CommandItem`]/[`CommandEmpty`] children, rendered
    /// inside the themed list.
    pub children: Element,
}

/// A filterable command palette. Bundles `command::CommandInput` and
/// `command::CommandList` internally so callers only place group/item/empty
/// children, the same shape the themed `Combobox` wrapper uses for
/// `ComboboxInput`/`ComboboxList`.
#[component]
pub fn Command<T: Clone + PartialEq + 'static>(props: CommandProps<T>) -> Element {
    let base = attributes!(div { class: "dx-command" });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/command/style.css") }
        command::Command::<T> {
            value: props.value,
            default_value: props.default_value,
            on_value_change: props.on_value_change,
            disabled: props.disabled,
            query: props.query,
            default_query: props.default_query,
            on_query_change: props.on_query_change,
            roving_loop: props.roving_loop,
            filter: props.filter,
            attributes: merged,
            div { class: "dx-command-input-wrapper",
                Search { class: "dx-command-search-icon", size: "16px" }
                command::CommandInput {
                    class: "dx-command-input",
                    placeholder: props.placeholder,
                    aria_label: props.input_aria_label.clone(),
                }
            }
            command::CommandList {
                class: "dx-command-list",
                aria_label: props.list_aria_label.clone(),
                {props.children}
            }
        }
    }
}

#[component]
pub fn CommandEmpty(props: CommandEmptyProps) -> Element {
    let base = attributes!(div { class: "dx-command-empty" });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/command/style.css") }
        command::CommandEmpty {
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn CommandGroupLabel(props: CommandGroupLabelProps) -> Element {
    let base = attributes!(div { class: "dx-command-group-label" });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        command::CommandGroupLabel {
            id: props.id,
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn CommandItem<T: Clone + PartialEq + 'static>(props: CommandItemProps<T>) -> Element {
    let base = attributes!(div { class: "dx-command-item" });
    let merged = merge_attributes(vec![base, props.attributes]);

    rsx! {
        command::CommandItem::<T> {
            value: props.value,
            text_value: props.text_value,
            disabled: props.disabled,
            id: props.id,
            index: props.index,
            aria_label: props.aria_label,
            shortcut: props.shortcut,
            attributes: merged,
            {props.children}
        }
    }
}
