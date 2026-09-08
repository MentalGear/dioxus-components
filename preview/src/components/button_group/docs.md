The Button Group component visually merges a row (or column) of `Button`s into one control -- shared borders, and only the outer corners rounded.

## Component Structure

```rust
ButtonGroup {
    Button { variant: ButtonVariant::Outline, "Archive" }
    Button { variant: ButtonVariant::Outline, "Report" }
    // A visual divider between two clusters of buttons.
    ButtonGroupSeparator {}
    Button { variant: ButtonVariant::Outline, "..." }
}
```
