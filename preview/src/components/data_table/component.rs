//! Themed Data Table composition pieces -- shadcn/ui catalog parity
//! (dev-docs/component-backlog.md row 69).
//!
//! Like `Table` (row 68), there is no `dioxus_primitives::data_table`
//! primitive: shadcn/ui's own Data Table is a composition pattern built
//! from `Table` plus existing `Select`/`Checkbox`/`Button`/`Input`, not a
//! new accessible widget with its own ARIA role. This file's three
//! components (`DataTableColumnHeader`, `DataTablePagination`,
//! `DataTableToolbar`) are that same composition, reusable across any
//! table shaped like the demo's payments table -- every element they
//! render comes from `crate::components::{table, button, select, input,
//! checkbox}`'s own themed wrappers (the preview composition rule,
//! dev-docs/preview-composition.md), never a raw element or primitive of
//! their own. The actual sort/filter/pagination *state* is owned by the
//! caller (see `variants/main/mod.rs`); `state.rs` (this folder) is the
//! pure, unit-tested transform logic those state signals are fed through.

use crate::components::button::{Button, ButtonSize, ButtonVariant};
use crate::components::table::TableHead;
use dioxus::prelude::*;
use dioxus_icons::lucide::{ArrowDown, ArrowUp, ArrowUpDown};
use dioxus_primitives::dioxus_attributes::attributes;
use dioxus_primitives::merge_attributes;

#[path = "state.rs"]
mod state;

pub use state::{filter_rows, includes_string, paginate, sort_rows, SortDirection};

/// Props for [`DataTableColumnHeader`].
#[derive(Props, Clone, PartialEq)]
pub struct DataTableColumnHeaderProps {
    /// This column's current sort direction, or `None` when a different
    /// column (or no column at all) is the one currently sorted.
    ///
    /// The APG Table pattern's sortable-table example
    /// (`playwright/oracle/reference/7e4034b/content/patterns/table/
    /// examples/sortable-table.html`, "Role, Property, State, and Tabindex
    /// Attributes"): "`aria-sort=\"value\"` ... \[s\]et on the currently
    /// sorted column. When the sorted column is changed, the `aria-sort`
    /// attribute is removed and set on the newly sorted column." This
    /// component instead always *sets* `aria-sort`, valued `"none"` for
    /// every sortable column that isn't the active one (rather than
    /// omitting the attribute the way the reference page's static HTML
    /// does) -- `"none"` is `aria-sort`'s own documented default value, so
    /// an explicit `"none"` and an absent attribute are equivalent to
    /// assistive tech, and explicit is easier for this crate's own
    /// oracle/axe scans to assert against uniformly. See
    /// `playwright/oracle/tier1-apg/sortable-table.spec.ts` for the rule
    /// this is calibrated against.
    pub sorted: Option<SortDirection>,

    /// Fired when the header's sort button is activated (pointer or
    /// keyboard -- it's a real `button`, so both come for free from the
    /// browser). The caller owns which column is active and the
    /// ascending/descending toggle (`SortDirection::toggled`); this
    /// component is purely presentational.
    pub onclick: EventHandler<MouseEvent>,

    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The column's label, e.g. `"Email"`.
    pub children: Element,
}

/// A sortable column header: a [`TableHead`] whose content is a ghost
/// [`Button`] that toggles the column's sort and shows the current
/// direction, with `aria-sort` on the `th` itself (not the button) per the
/// APG Table pattern. See [`DataTableColumnHeaderProps::sorted`] for the
/// exact `aria-sort` value convention.
#[component]
pub fn DataTableColumnHeader(props: DataTableColumnHeaderProps) -> Element {
    let aria_sort = props.sorted.map_or("none", SortDirection::aria_sort);
    let base = attributes!(th { class: "dx-data-table-column-header" });
    // `aria-sort` is required APG Table-pattern state this wrapper computes
    // from `props.sorted`, not a caller default -- owned-wins.
    let owned = attributes!(th { aria_sort });
    let merged = merge_attributes(vec![base, props.attributes, owned]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/data_table/style.css") }
        TableHead {
            attributes: merged,
            Button {
                variant: ButtonVariant::Ghost,
                size: ButtonSize::Sm,
                class: "dx-data-table-column-header-button",
                onclick: move |event| props.onclick.call(event),
                {props.children}
                match props.sorted {
                    Some(SortDirection::Ascending) => rsx! {
                        ArrowUp { class: "dx-data-table-sort-icon", size: "16px" }
                    },
                    Some(SortDirection::Descending) => rsx! {
                        ArrowDown { class: "dx-data-table-sort-icon", size: "16px" }
                    },
                    None => rsx! {
                        ArrowUpDown { class: "dx-data-table-sort-icon", size: "16px" }
                    },
                }
            }
        }
    }
}

/// Props for [`DataTableToolbar`].
#[derive(Props, Clone, PartialEq)]
pub struct DataTableToolbarProps {
    /// The filter input's current (controlled) value.
    pub value: ReadSignal<String>,

    /// Fired on every keystroke with the input's new value.
    pub oninput: Callback<String>,

    /// Placeholder text, e.g. `"Filter emails..."`.
    #[props(default)]
    pub placeholder: String,

    /// Accessible name for the filter input -- required, since a bare
    /// placeholder is not an accessible name (it disappears once there is
    /// a value, and isn't exposed to every assistive technology the same
    /// way in the first place).
    pub aria_label: String,

    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
}

/// A toolbar holding a text filter [`Input`](crate::components::input::Input).
#[component]
pub fn DataTableToolbar(props: DataTableToolbarProps) -> Element {
    let base = attributes!(div { class: "dx-data-table-toolbar", "data-slot": "data-table-toolbar" });
    let merged = merge_attributes(vec![base, props.attributes]);
    let value = (props.value)();

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/data_table/style.css") }
        div { ..merged,
            crate::components::input::Input {
                class: "dx-data-table-filter",
                "data-slot": "data-table-filter",
                r#type: "text",
                placeholder: props.placeholder.clone(),
                aria_label: props.aria_label.clone(),
                value: "{value}",
                oninput: move |event: FormEvent| props.oninput.call(event.value()),
            }
        }
    }
}

/// Props for [`DataTablePagination`].
#[derive(Props, Clone, PartialEq)]
pub struct DataTablePaginationProps {
    /// Number of rows currently selected (across every page, not just the
    /// one visible).
    pub selected_count: usize,

    /// Number of rows in the filtered result set (before pagination).
    pub total_count: usize,

    /// 0-based index of the current page.
    pub page: usize,

    /// Total number of pages (always >= 1 -- see `state::paginate`).
    pub page_count: usize,

    /// Rows shown per page.
    pub page_size: usize,

    /// The rows-per-page choices offered in the `Select`.
    pub page_size_options: Vec<usize>,

    /// Fired with the newly chosen page size.
    pub on_page_size_change: Callback<usize>,

    /// Fired when "Previous" is activated. Never fired while already on
    /// the first page -- the button is `disabled` there instead.
    pub on_previous: Callback<()>,

    /// Fired when "Next" is activated. Never fired while already on the
    /// last page -- the button is `disabled` there instead.
    pub on_next: Callback<()>,

    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
}

/// The Data Table's footer bar: a "x of y row(s) selected" summary, a
/// rows-per-page [`Select`](crate::components::select::Select), a "Page n
/// of m" readout, and Previous/Next [`Button`]s, disabled at either end.
#[component]
pub fn DataTablePagination(props: DataTablePaginationProps) -> Element {
    let base = attributes!(div { class: "dx-data-table-pagination", "data-slot": "data-table-pagination" });
    let merged = merge_attributes(vec![base, props.attributes]);
    let at_start = props.page == 0;
    let at_end = props.page + 1 >= props.page_count;
    let page_size = props.page_size;
    let on_page_size_change = props.on_page_size_change;
    let on_previous = props.on_previous;
    let on_next = props.on_next;

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/data_table/style.css") }
        div { ..merged,
            div { class: "dx-data-table-pagination-selected",
                "{props.selected_count} of {props.total_count} row(s) selected."
            }
            div { class: "dx-data-table-pagination-controls",
                div { class: "dx-data-table-pagination-size",
                    span { class: "dx-data-table-pagination-size-label", aria_hidden: "true", "Rows per page" }
                    crate::components::select::Select::<usize> {
                        "data-slot": "data-table-page-size",
                        trigger_aria_label: "Rows per page",
                        default_value: page_size,
                        on_value_change: move |value: Option<usize>| {
                            if let Some(value) = value {
                                on_page_size_change.call(value);
                            }
                        },
                        for (i , option) in props.page_size_options.iter().copied().enumerate() {
                            crate::components::select::SelectOption::<usize> {
                                key: "{option}",
                                index: i,
                                value: option,
                                text_value: "{option}",
                                "{option}"
                            }
                        }
                    }
                }
                div { class: "dx-data-table-pagination-page", "Page {props.page + 1} of {props.page_count}" }
                div { class: "dx-data-table-pagination-nav",
                    Button {
                        variant: ButtonVariant::Outline,
                        size: ButtonSize::Sm,
                        disabled: at_start,
                        onclick: move |_| on_previous.call(()),
                        "Previous"
                    }
                    Button {
                        variant: ButtonVariant::Outline,
                        size: ButtonSize::Sm,
                        disabled: at_end,
                        onclick: move |_| on_next.call(()),
                        "Next"
                    }
                }
            }
        }
    }
}
