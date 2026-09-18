The drawer component is a panel that slides in from the edge of the screen, like the sheet component, but adds a pointer drag gesture (a Vaul-style "swipe to dismiss") on top -- drag its handle or content toward the edge it slid in from to close it, in addition to Escape or a close button.

## Component Structure

```rust
Drawer {
    open: open(),
    // Which edge to slide in from. Available sides: Top, Right, Bottom (default), Left.
    side: DrawerSide::Bottom,
    DrawerContent {
        DrawerHandle {}
        DrawerHeader {
            DrawerTitle { "Move Goal" }
            DrawerDescription { "Set your daily activity goal." }
        }
        DrawerFooter {
            DrawerClose { "Submit" }
            DrawerClose {
                as: |attributes| rsx! {
                    Button { variant: ButtonVariant::Outline, attributes, "Cancel" }
                }
            }
        }
    }
}
```

## Drag to dismiss

Dragging [`DrawerHandle`] or anywhere on [`DrawerContent`] itself toward `side` tracks the pointer live. Releasing past 25% of the panel's own size along that axis, or with enough velocity, finishes closing the drawer; releasing short of that snaps it back open. Set `dismissible: false` on `Drawer` to disable the drag gesture entirely while keeping Escape and `DrawerClose` working.

A drag never starts from an interactive descendant (a button, link, or form control), or from content that is scrolled away from the drag edge -- either is read as "interact with (or scroll) this instead," not "dismiss the drawer."

## DrawerClose with `as` prop

The `as` prop allows you to render a custom element while preserving the close behavior, similar to shadcn/ui's `asChild` pattern.

```rust
// Default: renders as <button>
DrawerClose { "Close" }

// Custom element: attributes include the preset onclick handler
DrawerClose {
    as: |attributes| rsx! {
        a { href: "#", ..attributes, "Go back" }
    }
}
```
