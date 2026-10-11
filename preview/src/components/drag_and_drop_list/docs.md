Allows users to create vertically sortable lists supporting drag and drop, touch or keyboard input.

## Component Structure

```rust
DragAndDropList {
    // Items to be rendered
    items,
    // Whether the list items should be removable
    is_removable,
    // Opacity of the dragged item's ghost, 0.0 to 1.0 (default 0.9)
    ghost_opacity,
    // Room the other items make at the drop slot: a CSS length, or a bare
    // number of pixels (default 25px)
    drop_gap,
}
```

## Ghost and drop gap

While an item is dragged it stays in the list as a **ghost** (dashed outline, see-through background) and the other items part at the slot where it would land, with a line marking the slot.

- `ghost_opacity` sets how opaque the ghost is. The default, `0.9`, keeps the item's text readable while the dashed outline and see-through background still mark it as lifted: on a white page primary text stays at about 14.6:1 and muted text at about 4:1, where the previous `0.5` left them at 3.5:1 and 2:1. Muted text needs `0.96` or more to reach 4.5:1, so raise it for a stricter theme. Values outside `0.0..=1.0` are clamped.
- `drop_gap` sets how much room the other items make: every item after the slot moves by this much, so the space between the slot's two neighbours grows by exactly this length. The default is `25px`. It takes any non-negative CSS length (`"1.5rem"`, `"var(--dx-space-6)"`); a bare number is read as pixels. The items move with a `transform`, so the list's own box does not grow: leave this much room below the list.

Both are written as CSS custom properties on the list's root element, so they can be themed from a stylesheet instead of a prop:

```css
.my-list {
  --dx-dnd-ghost-opacity: 0.8;
  --dx-dnd-drop-gap: 1.5rem;
}
```

The room opens with the same transform transition as the rest of the list and is instant under `prefers-reduced-motion: reduce`. Keyboard reordering (Enter, arrow keys, Escape), the screen-reader announcements and right-to-left layouts are unaffected. The browser's own drag image, the translucent copy that follows the pointer, is drawn by the browser and is not styled by these options.

## Motion

While an item is dragged the items after the slot part by a `transform` transition, sized by `drop_gap` (`--dx-dnd-drop-gap`), and the dragged item stays as a ghost whose opacity is `ghost_opacity` (`--dx-dnd-ghost-opacity`). The motion only exists during a drag, so there is nothing to pause. It is instant under `prefers-reduced-motion: reduce`. See [Motion](/docs#motion) for the tokens, reduced motion and the per-component table.
