//! Themed `Table` -- shadcn/ui catalog parity
//! (dev-docs/component-backlog.md row 68).
//!
//! Unlike almost every other component in this crate, there is no
//! `dioxus_primitives::table` module to wrap: shadcn/ui's own `table.tsx`
//! is plain, unstyled semantic HTML (`<table>`/`<thead>`/`<tbody>`/
//! `<tfoot>`/`<tr>`/`<th>`/`<td>`/`<caption>`) with no Radix primitive and
//! no ARIA widget role underneath -- confirmed against shadcn's own source
//! and recorded in dev-docs/component-backlog.md row 68 ("none -- plain
//! semantic `<table>` markup, no ARIA engineering needed"). So this file
//! *is* both the "primitive" and the theme layer: every element below is
//! the plain HTML tag plus this crate's `dx-table-*` class, nothing more.
//!
//! Every component here merges its own `dx-table-*` base class with any
//! caller-supplied `class` via `merge_attributes`, the same construction
//! commit 724bfee fixed `Input` with (`git show 724bfee`): a literal
//! `class: "dx-foo"` field followed by a plain `..attributes` spread lets
//! Dioxus's "later duplicate attribute wins" rule silently *replace* the
//! base class instead of combining with it. `merge_attributes` special-
//! cases `class` to concatenate (space-joined) instead, so a caller's own
//! class (e.g. `DataTableColumnHeader`'s `"dx-data-table-column-header"`,
//! composed on top of `TableHead` here) always ends up alongside
//! `"dx-table-head"`, never in place of it.
//!
//! `TableHead`'s `scope: "col"` default (APG's Table pattern:
//! "columnheader ... if the cell contains a title or header information
//! for the column") is written as a literal attribute placed *before* the
//! `..merged` spread in each `th { ... }` body, not folded into the merged
//! `attributes!` base -- the same "later wins" rule then lets a caller's
//! own `scope` (e.g. a row-header `<th>` wanting `scope="row"`) override
//! the literal default, while callers who don't set one keep `"col"`.

use dioxus::prelude::*;
use dioxus_primitives::dioxus_attributes::attributes;
use dioxus_primitives::merge_attributes;

/// Wraps a `<table>` in a horizontally-scrollable container, matching
/// shadcn/ui's own `Table` (`div.overflow-x-auto` + `table`). The
/// container needs no `dx-table` class of its own beyond `dx-table-
/// container` -- only the `<table>` element carries `dx-table`.
#[component]
pub fn Table(
    #[props(extends = GlobalAttributes)]
    #[props(extends = table)]
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(table { class: "dx-table" });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/table/style.css") }
        div { class: "dx-table-container", "data-slot": "table-container",
            table { "data-slot": "table", ..merged, {children} }
        }
    }
}

/// `<thead>`.
#[component]
pub fn TableHeader(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(thead { class: "dx-table-header" });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/table/style.css") }
        thead { "data-slot": "table-header", ..merged, {children} }
    }
}

/// `<tbody>`.
#[component]
pub fn TableBody(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(tbody { class: "dx-table-body" });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/table/style.css") }
        tbody { "data-slot": "table-body", ..merged, {children} }
    }
}

/// `<tfoot>`.
#[component]
pub fn TableFooter(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(tfoot { class: "dx-table-footer" });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/table/style.css") }
        tfoot { "data-slot": "table-footer", ..merged, {children} }
    }
}

/// Props for [`TableRow`].
#[derive(Props, Clone, PartialEq)]
pub struct TableRowProps {
    /// Whether this row is selected (e.g. by a leading checkbox column in
    /// the Data Table composition pattern). Renders `data-state="selected"`
    /// when `true`, matching shadcn/ui's own `table.tsx`
    /// (`data-state={row.getIsSelected() && "selected"}`); the attribute is
    /// omitted entirely rather than set to some "unselected" value when
    /// `false`, mirroring that same `&&` short-circuit.
    #[props(default)]
    pub selected: ReadSignal<bool>,

    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    pub children: Element,
}

/// `<tr>`.
#[component]
pub fn TableRow(props: TableRowProps) -> Element {
    let base = attributes!(tr { class: "dx-table-row" });
    let merged = merge_attributes(vec![base, props.attributes]);
    let data_state: Option<&'static str> = if (props.selected)() { Some("selected") } else { None };

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/table/style.css") }
        tr {
            "data-slot": "table-row",
            "data-state": data_state,
            ..merged,
            {props.children}
        }
    }
}

/// `<th>`. Defaults to `scope="col"` (this crate's `TableHead` is always
/// used as a column header in every demo/consumer so far -- a row-header
/// `<th scope="row">` remains reachable by passing `scope: "row"`
/// explicitly, which overrides the default; see this file's header doc for
/// the override mechanism).
#[component]
pub fn TableHead(
    #[props(extends = GlobalAttributes)]
    #[props(extends = th)]
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(th { class: "dx-table-head" });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/table/style.css") }
        th {
            "data-slot": "table-head",
            scope: "col",
            ..merged,
            {children}
        }
    }
}

/// `<td>`.
#[component]
pub fn TableCell(
    #[props(extends = GlobalAttributes)]
    #[props(extends = td)]
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(td { class: "dx-table-cell" });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/table/style.css") }
        td { "data-slot": "table-cell", ..merged, {children} }
    }
}

/// `<caption>`.
#[component]
pub fn TableCaption(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(caption { class: "dx-table-caption" });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/table/style.css") }
        caption { "data-slot": "table-caption", ..merged, {children} }
    }
}
