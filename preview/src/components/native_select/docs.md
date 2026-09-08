The Native Select component is a styled native `<select>`, for the common case where the browser's own picker is preferable to a fully custom listbox. It comes with the platform's own keyboard, typeahead, and form-participation behaviour for free.

## Component Structure

```rust
NativeSelect {
    value: "{fruit}",
    onchange: move |e: FormEvent| fruit.set(e.value()),
    option { value: "apple", "Apple" }
    option { value: "banana", "Banana" }
}
```
