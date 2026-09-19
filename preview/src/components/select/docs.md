The Select component is used to create a dropdown menu that allows users to select one or more options from the select groups.

## Component Structure

```rust
Select::<String> {
    value: "option1",
    on_value_change: |value: String| {
        // Handle the change in selected value.
    },
    SelectGroup {
        SelectGroupLabel { "Group A" }
        SelectOption::<String> {
            index: 0,
            value: "option1",
            "Option 1"
        }
    }
}
```

## Direction / RTL

`Select`/`SelectMulti` accept a `dir: Option<Direction>` prop, emitted as `dir`/`data-direction` on the trigger and the listbox. The trigger/listbox's own keyboard navigation is vertical-only (`ArrowUp`/`ArrowDown`/`Home`/`End`), so there is no arrow-key behavior to flip. See the `rtl` variant.
