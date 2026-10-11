The Combobox component is an autocomplete input with a filterable popup list.

Filtering preserves the order defined by the rendered `ComboboxOption` elements and their `index`
props. If you want query-dependent ranking, control `query`, sort your item data in user code,
render the options in that sorted order, and assign indexes from the sorted list.

## Behavior

The input shows the selected option's label. Clicking, focusing or arrowing into it never rewrites
that text: the caret lands where the user clicked, the popup lists **every** option, and the selected
one is marked and scrolled into view. Filtering starts with the first edit, and from then on the
input's own text is the query. Escape, blurring, or picking an option ends the edit and puts the
selected label back.

## Component Structure

```rust
let mut value = use_signal(|| None::<String>);
let mut query = use_signal(String::new);

Combobox::<String> {
    value: Some(value.into()),
    on_value_change: move |next: Option<String>| {
        value.set(next);
    },
    query: Some(query()),
    on_query_change: move |next| query.set(next),
    placeholder: "Select framework...",
    aria_label: "Select framework",
    list_aria_label: "Frameworks",
    ComboboxEmpty { "No framework found." }
    ComboboxOption::<String> {
        index: 0usize,
        value: "next".to_string(),
        text_value: "Next.js",
        "Next.js"
    }
}
```

## Motion

The list fades and scales in over `--dx-motion-duration-base` (`dx-picker-in`) and out over `--dx-motion-duration-fast`. Retune with those tokens. Reduced motion swaps in `--dx-motion-duration-reduced`. Nothing loops. See [Motion](/docs#motion) for the tokens, reduced motion and the per-component table.
