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

## Dark mode

The open list of a native `<select>` is drawn by the browser, not by the page, so it only follows the theme if the page declares its `color-scheme`. `dx-components-theme.css` does that on the root (`light dark`, pinned to `dark` or `light` by `<html data-theme>`), and the options take the popover's surface and ink, so the popup, its scrollbar and every other native control match the active mode. If you use this component without the theme file, set `color-scheme` yourself.
