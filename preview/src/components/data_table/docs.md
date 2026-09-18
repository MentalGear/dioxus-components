Data Table is a composition pattern, not a new primitive — the same architecture as shadcn/ui's own Data Table. It builds sorting, filtering, pagination, and row selection out of [`Table`](/component/?name=table&) plus this crate's existing `Select`, `Checkbox`, `Button`, and `Input` components. Three pieces do the composing:

- `DataTableColumnHeader` — a sortable `TableHead`: a ghost `Button` toggles the sort and an icon shows the current direction, with `aria-sort` set on the header cell per the [APG Table pattern](https://www.w3.org/WAI/ARIA/apg/patterns/table/)'s sortable-columns guidance.
- `DataTableToolbar` — a text filter `Input` with an accessible name. The demo below filters like shadcn's `includesString`: a case-insensitive substring match anywhere in the email, not just at the start.
- `DataTablePagination` — a "x of y row(s) selected" summary, a rows-per-page `Select`, a "Page n of m" readout, and Previous/Next `Button`s (disabled at either end).

The actual sort/filter/pagination *state* — which column is sorted and in which direction, the filter text, the current page — is owned by the page that composes these pieces (see the demo below), and derived through a small set of pure, unit-tested helpers (`sort_rows`, `filter_rows`, `paginate`, all generic over `&[T]` + a closure) rather than baked into any of the three components above.

## Component Structure

```rust
DataTableToolbar {
    value: filter,
    oninput: move |v| filter.set(v),
    aria_label: "Filter by email",
}
Table {
    TableHeader {
        TableRow {
            DataTableColumnHeader {
                sorted: if sort_key() == "email" { Some(sort_direction()) } else { None },
                onclick: move |_| toggle_sort("email"),
                "Email"
            }
        }
    }
    TableBody {
        for row in page_rows {
            TableRow { TableCell { "{row.email}" } }
        }
    }
}
DataTablePagination {
    selected_count: selected().len(),
    total_count: filtered_count,
    page: page(),
    page_count,
    page_size: page_size(),
    page_size_options: vec![5, 10, 20],
    on_page_size_change: move |n| page_size.set(n),
    on_previous: move |_| page.set(page().saturating_sub(1)),
    on_next: move |_| page.set(page() + 1),
}
```
