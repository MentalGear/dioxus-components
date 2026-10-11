The Progress component is used to display the progress of a task or operation. It can be used to indicate loading states, file uploads, or any other process that takes time to complete.

## Component Structure

```rust
Progress {
    // The current progress value (0 to max
    value: 0.5,
    // The maximum value of the progress (default is 100.0)
    max: 1.0,
    // Elements that will be displayed inside the progress bar
    {children}
}
```

## Motion

Only the indeterminate bar loops: a `translateX` sweep, 1 s, forever. It is paused whenever the bar is off-screen or the tab is hidden (`dioxus_primitives::activity::use_motion_when`); a determinate bar subscribes to nothing. The determinate fill animates `width` for 250 ms, a literal in `progress/style.css`.

To change them, override `animation` on `.dx-progress-indicator` (indeterminate) or its `transition` (determinate). Neither has a reduced-motion rule yet; add one to your copy if you need it. See [Motion](/docs#motion) for the tokens, reduced motion and the per-component table.
