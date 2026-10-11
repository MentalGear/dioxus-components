A popover shows rich content in a panel anchored to its trigger. The content is arbitrary: a form, a short explanation, a set of actions. It is not a menu; reach for `DropdownMenu` when the content is a list of commands.

`PopoverRoot` is modal by default (`is_modal`): focus is trapped, the page behind it is inert and a click outside dismisses it. Like shadcn's popover it does not dim the page: `overlay` defaults to `false`. Pass `overlay: true` to paint the same scrim every modal overlay shares behind the panel (the `overlay` variant below). Pass `is_modal: false` for shadcn's behaviour: no focus trap, no overlay, light-dismissed by the browser. `overlay` only affects the modal popover; a non-modal one has no overlay to switch.

## Component Structure

```rust
// The PopoverRoot is the root component that contains the trigger and content.
// `is_modal` (default true) traps focus and makes the page inert; `overlay`
// (default false) additionally dims the page with the shared scrim.
PopoverRoot {
    // The PopoverTrigger is the button that opens the popover (styled like the outline Button).
    PopoverTrigger {
        "Open popover"
    }
    // The PopoverContent is the panel shown when the popover is open.
    PopoverContent {
        side: ContentSide::Bottom,
        align: ContentAlign::Center,
        // Optional: a header with a title and description, then your own content.
        PopoverHeader {
            PopoverTitle { "Dimensions" }
            PopoverDescription { "Set the dimensions for the layer." }
        }
        {children}
    }
}
```

## Motion

The panel fades in and out over `--dx-overlay-duration` (default `--dx-motion-duration-slow`) with `--dx-overlay-ease`. A modal popover uses the shared dialog exit driver, staying open until its exit settles (at most 1500 ms) before it closes. The keyframes animate `opacity` only, so the anchor's own `transform` centring is never replaced. Reduced motion swaps in `--dx-motion-duration-reduced`. Nothing loops. See [Motion](/docs#motion) for the tokens, reduced motion and the per-component table.
