A popover shows rich content in a panel anchored to its trigger. The content is arbitrary: a form, a short explanation, a set of actions. It is not a menu; reach for `DropdownMenu` when the content is a list of commands.

`PopoverRoot` is modal by default (`is_modal`): focus is trapped, the page behind it is inert, a click outside dismisses it, and the page is dimmed with the same scrim every modal overlay shares. Pass `is_modal: false` for shadcn's behaviour: no focus trap, no overlay, light-dismissed by the browser.

## Component Structure

```rust
// The PopoverRoot is the root component that contains the trigger and content.
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
