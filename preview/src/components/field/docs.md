The Field component lays out a label, its control, and optional description/error text together, so building an accessible form is mostly composition rather than manual markup. It builds on this repo's native form participation: every themed control already mirrors its state onto a real, hidden native input, so `Field` only supplies layout and label/description/error wiring.

## Component Structure

```rust
FieldSet {
    FieldLegend { "Profile" }
    FieldGroup {
        Field {
            FieldLabel { html_for: "username", "Username" }
            Input { id: "username" }
            FieldDescription { "This is your public display name." }
        }
        Field { invalid: true,
            FieldLabel { html_for: "email", "Email" }
            Input { id: "email", "aria-invalid": "true" }
            FieldError { "Enter a valid email address." }
        }
        FieldSeparator {}
        // Horizontal orientation: the control sits beside its label.
        Field { orientation: FieldOrientation::Horizontal,
            Checkbox { id: "marketing" }
            FieldContent {
                FieldLabel { html_for: "marketing", "Marketing emails" }
                FieldDescription { "Receive emails about new products." }
            }
        }
    }
}
```

## Styling the label

`FieldLabel` renders the themed `Label` (`<label class="dx-label dx-field-label">`) and retunes it through `Label`'s slot variables (see the Label docs): `--dx-label-line-height` always (shadcn's `FieldLabel` is `Label` plus `leading-snug`), and `--dx-label-color` while the field is `invalid`. `Field`'s stylesheet never declares `font-size`, `line-height`, or `color` on the label itself, so the label looks the same on every route regardless of which stylesheet loads first.
