The Spinner component is a continuously rotating loading indicator. It renders `role="status"` with an accessible name so assistive technology announces the busy state.

## Component Structure

```rust
Spinner {
    // The accessible name announced for the loading state. Defaults to "Loading".
    label: "Saving",
}
```

## Motion and visibility

The ring spins forever, so it pauses whenever nobody can see it: while it is outside the viewport (plus a 200px pre-roll, so it is already turning when it scrolls in), inside a skipped `content-visibility` subtree, or in a hidden tab. Paused is `animation-play-state: paused`, not removed, so it keeps its angle and resumes seamlessly at zero cost.

This is built in. `Spinner` calls `dioxus_primitives::activity::use_motion()` and spreads `motion.attributes()` on its root; the one `[data-dx-motion="paused"]` rule in `dx-components-theme.css` does the pausing for the element and everything under it. A page of spinners shares one `IntersectionObserver`. If you build your own looping indicator, do the same two lines rather than writing a pause rule. Users who prefer reduced motion are handled by your stylesheet's `prefers-reduced-motion` query, not by this.

The spin is `transform: rotate`, 1 s linear; override `animation-duration` on `.dx-spinner svg` to change it. See [Motion](/docs#motion) for the tokens, reduced motion and the per-component table.
