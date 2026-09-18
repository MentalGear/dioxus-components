The Table component displays tabular data using plain semantic HTML (`<table>`, `<thead>`, `<tbody>`, `<tfoot>`, `<tr>`, `<th>`, `<td>`, `<caption>`) — the same architecture as shadcn/ui's `Table`: no ARIA widget role, no primitive underneath.

## Component Structure

```rust
Table {
    TableCaption { "A list of your recent invoices." }
    TableHeader {
        TableRow {
            TableHead { "Invoice" }
            TableHead { "Status" }
            TableHead { "Amount" }
        }
    }
    TableBody {
        TableRow {
            TableCell { "INV001" }
            TableCell { "Paid" }
            TableCell { "$250.00" }
        }
    }
    TableFooter {
        TableRow {
            TableCell { "Total" }
            TableCell {}
            TableCell { "$250.00" }
        }
    }
}
```

`TableHead` defaults to `scope="col"`; pass `scope: "row"` to use it as a row header instead. `TableRow` accepts an optional `selected: ReadSignal<bool>` prop that sets `data-state="selected"` for row-selection styling — used by the [Data Table](/component/?name=data_table&) composition pattern, which builds sorting, filtering, pagination, and row selection on top of this component.
