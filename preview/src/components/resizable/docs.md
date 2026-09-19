The Resizable components create a group of panels divided by draggable, keyboard-operable
handles, implementing the W3C APG Window Splitter pattern.

## Component Structure

```rust
// ResizablePanelGroup lays out its panels along `direction` (Horizontal is the default:
// panels side by side).
ResizablePanelGroup {
    direction: ResizableDirection::Horizontal,
    // Called whenever a drag or keyboard interaction resizes the group, whether or not
    // `sizes` is controlled.
    on_sizes_change: |sizes: Vec<f64>| {},

    // Each panel's `index` matches the boundary index of the handle(s) beside it.
    ResizablePanel {
        index: 0usize,
        // Initial share of the group, as a percentage. Panels with no `default_size` split
        // whatever is left over equally.
        default_size: 50.0,
        min_size: 20.0,
        "Left panel"
    }

    // A handle sits ON a boundary: `index: 0` is the boundary between panel 0 and panel 1.
    ResizableHandle {
        index: 0usize,
        // Required: the APG pattern gives the handle the same accessible name as its
        // "primary" (preceding) pane.
        aria_label: "Left panel",
    }

    ResizablePanel {
        index: 1usize,
        default_size: 50.0,
        "Right panel"
    }
}
```

## Handle grip

A [`ResizableHandle`] renders whatever children you give it -- typically a small grip glyph, as
in shadcn/ui's own Resizable demo, using `.dx-resizable-handle-grip`'s built-in styling and any
icon:

```rust
use dioxus_icons::lucide::GripVertical;

ResizableHandle {
    index: 0usize,
    aria_label: "Left panel",
    span { class: "dx-resizable-handle-grip", GripVertical { size: "0.625rem" } }
}
```

The grip is purely decorative (`pointer-events: none`) -- the handle `div` itself carries the
drag/keyboard behavior -- and rotates automatically with the handle in a vertical-direction
group. Omit it for a plain 1px divider line with no visible grip.

## Collapsible panels

A panel can collapse below its own `min_size` -- down to `collapsed_size` (`0.0` by default) --
when its preceding handle's Home key or Enter key is pressed:

```rust
ResizablePanel {
    index: 0usize,
    default_size: 25.0,
    min_size: 15.0,
    collapsible: true,
    collapsed_size: 0.0,
    "Sidebar"
}
```

## Keyboard interaction

Per the APG Window Splitter pattern's own "Keyboard Interaction" section:

- **Left/Right Arrow** (horizontal-direction groups) / **Up/Down Arrow** (vertical-direction
  groups): move the handle's boundary by 1% of the group's size (10% with Shift). The other
  axis's arrow keys are ignored.
- **Home**: move the boundary to give the primary (preceding) pane its smallest allowed size --
  its `collapsed_size` if `collapsible`, otherwise its `min_size`.
- **End**: move the boundary to give the primary pane its largest allowed size (`max_size`).
- **Enter**: toggle collapse/restore, if the primary pane is `collapsible`.

## Nesting

A `ResizablePanel` can contain its own nested `ResizablePanelGroup` (typically on the other
`direction`) to build a layout with more than one split axis.

## Direction / RTL

`ResizablePanelGroup` accepts a `dir: Option<Direction>` prop, consulted only when `direction: ResizableDirection::Horizontal` (a vertical group never mirrors). Under RTL, the group's own panels render in the browser's native `dir`-relative order (plain `flex-direction: row`, which the CSS Flexbox spec already mirrors under `dir="rtl"` -- deliberately *not* `row-reverse`, which would cancel that automatic mirroring), and each `ResizableHandle`'s `ArrowLeft`/`ArrowRight` swaps so a physical arrow key always moves the divider in that same physical direction. No Radix/shadcn original exists for this component to cite directly (shadcn's own `Resizable` wraps `react-resizable-panels`, not a Radix primitive) -- this is an extrapolation from the same arrow-key convention every other RTL-aware component in this library follows. See the `rtl` variant.
