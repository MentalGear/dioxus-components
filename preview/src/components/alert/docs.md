The Alert component displays a short, callout-style message -- an inline status, warning, or tip. It renders `role="alert"` so assistive technology announces it as a live region.

## Component Structure

```rust
Alert {
    // Alert::class() controls the visual style. Defaults to `AlertVariant::Default`.
    variant: AlertVariant::Destructive,

    // An optional leading icon; the layout adapts automatically when one is present.
    CircleAlert {}
    AlertTitle { "Unable to process your payment." }
    AlertDescription { "Please verify your billing information and try again." }
}
```
