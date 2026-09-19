The slider component allows users to select a value from a range by sliding a handle along a track.

## Component Structure

```rust
Slider {
    value: 0.0,
    horizontal: true,
    on_value_change: |value: f64| {
        // Handle the change in slider value.
    },
}
```

For a two-thumb range selector, use `RangeSlider`:

```rust
RangeSlider {
    default_value: 20.0..80.0,
    on_value_change: |value: std::ops::Range<f64>| {
        // value.start and value.end give the two endpoints
    },
}
```

## Direction / RTL

`Slider`/`RangeSlider` accept a `dir: Option<Direction>` prop, consulted only when `horizontal: true` (a vertical slider never mirrors, matching Radix). Under RTL: a click near the left end of the track resolves to a value near `max` (not `min`); `ArrowLeft` increases the value and `ArrowRight` decreases it (the opposite of LTR); the thumb and the filled range visually anchor to the opposite edge. `ArrowUp`/`ArrowDown` are never affected. See the `rtl` variant.
