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

Dragging [`DrawerHandle`] or anywhere on [`DrawerContent`] itself toward `side` tracks the pointer live. Only movement toward the drawer's own closing edge dismisses it -- down for the default bottom drawer, up for `side: DrawerSide::Top`, and so on for `Left`/`Right`; dragging the other way rubber-bands and snaps back, however far or fast, and releasing the pointer over the backdrop is not a dismissal either. Releasing past 25% of the panel's own size along the closing axis, or flicking toward the closing edge (at least 0.5px/ms over the last 100ms, after at least 10px), finishes closing the drawer; releasing short of that snaps it back open. Mouse, touch and pen behave the same. Set `dismissible: false` on `Drawer` to disable the drag gesture entirely while keeping Escape and `DrawerClose` working.

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

## Overlay

A modal drawer dims the page behind it with the shared overlay scrim: the dialog's own `::backdrop`, the same 10% black and 4px blur on every modal overlay. Pass `overlay: false` to keep the modality (focus trap, inert page, click-outside dismissal) without the dim and the blur; the `<dialog>` then carries `data-dx-overlay="off"`.

Closing keeps the drawer a modal -- in the top layer, at the same size and place -- until its exit animation has played, and only then closes it. That holds in every browser; no stylesheet needs an engine-specific keep-alive.

## Motion

The drawer slides in over `--dx-motion-duration-slow` and out over `--dx-motion-duration-base`; the scrim follows `--dx-overlay-duration`, `--dx-overlay-ease`, `--dx-overlay-scrim` and `--dx-overlay-blur`. While you drag, the panel follows the pointer through the `translate` property (it composes with the slide's `transform`), and a release settles with a `--dx-motion-duration-base` transition. The drawer stays open until its exit settles (at most 1500 ms) and only then closes. Reduced motion swaps in `--dx-motion-duration-reduced`. Nothing loops. See [Motion](/docs#motion) for the tokens, reduced motion and the per-component table.
