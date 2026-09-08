The Spinner component is a continuously rotating loading indicator. It renders `role="status"` with an accessible name so assistive technology announces the busy state.

## Component Structure

```rust
Spinner {
    // The accessible name announced for the loading state. Defaults to "Loading".
    label: "Saving",
}
```
