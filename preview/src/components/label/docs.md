The Label component is used to provide a label for form elements, enhancing accessibility and usability. It is typically used in conjunction with form controls like inputs, selects, and textareas.

## Component Structure

```rust
Label {
    html_for: "id", // The ID of the form control this label is associated with
}
button {
    id: "id", // The ID of the labeled element
}
```

## Slot variables

A component that composes `Label` (such as `FieldLabel`) retunes it by setting a custom property on the label's element, never by declaring `font-size`, `line-height`, or `color` on its own class: two stylesheets declaring the same property on one element are decided by which sheet loads last, and that order differs from route to route. `.dx-label` reads each of these with its own default, so a variable that is not set changes nothing.

| Variable | Default | Property |
| --- | --- | --- |
| `--dx-label-gap` | `var(--dx-space-2)` | `gap` |
| `--dx-label-color` | `var(--dx-foreground)` | `color` |
| `--dx-label-font-size` | `var(--dx-text-sm)` | `font-size` |
| `--dx-label-font-weight` | `500` | `font-weight` |
| `--dx-label-line-height` | `1` | `line-height` |

```css
/* In the composing component's own stylesheet, on the label's element: */
.dx-field-label { --dx-label-line-height: 1.375; }
```

Set the variable on the label's own element rather than an ancestor: custom properties inherit, so an ancestor's value would reach every nested label too.
