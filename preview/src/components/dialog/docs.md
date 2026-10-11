The dialog component can be used to display additional information or actions related to an item. It is a simple modal dialog that can be opened and closed by the user.

## Component Structure

```rust
// The dialog component must wrap all dialog elements.
Dialog {
    // The open prop determines if the dialog is currently open or closed.
    open: open(),
    // The dialog title defines the heading of the dialog.
    DialogTitle {
        "Item information"
    }
    // The dialog description provides additional information about the dialog.
    DialogDescription {
        "Here is some additional information about the item."
    }
}
```

## Overlay

A modal dialog dims the page behind it with the shared overlay scrim: the dialog's own `::backdrop`, the same 10% black and 4px blur on every modal overlay. Pass `overlay: false` to keep the modality (focus trap, inert page, click-outside dismissal) without the dim and the blur; the `<dialog>` then carries `data-dx-overlay="off"`.

Closing keeps the dialog a modal -- in the top layer, at the same size and place -- until its exit animation has played, and only then closes it. That holds in every browser; no stylesheet needs an engine-specific keep-alive.

## Motion

A dialog is not closed the instant `open` turns false. It stays open (in the top layer, at the same size and place) until its own exit animation and its backdrop's have settled, at most 1500 ms, and only then calls `close()`, so any exit animation you write on the dialog or its `::backdrop` is played in full. The scrim and the panel share `--dx-overlay-duration` and `--dx-overlay-ease` (defaults `--dx-motion-duration-slow` and `--dx-motion-ease`); retune them in one place, or pass `overlay: false` for no scrim. Keep exit keyframes to `transform`, `translate` and `opacity`. Under `prefers-reduced-motion: reduce` both durations become `--dx-motion-duration-reduced`. Nothing here loops, so there is nothing to pause while unseen. See [Motion](/docs#motion) for the tokens, reduced motion and the per-component table.
