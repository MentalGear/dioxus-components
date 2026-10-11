The Tooltip component is used to display additional information when a user hovers over an element.

## Component Structure

```rust
// The Tooltip component wraps the trigger element and the content that will be displayed on hover.
Tooltip {
    // The TooltipTrigger contains the elements that will trigger the tooltip to display when hovered over.
    TooltipTrigger {
        // The elements that will trigger the tooltip when hovered over.
        {children}
    }
    // The TooltipContent contains the content that will be displayed when the user hovers over the trigger.
    TooltipContent {
        // The side of the TooltipTrigger where the content will be displayed. Can be one of Top, Right, Bottom, or Left.
        side: ContentSide::Top,
        // The alignment of the TooltipContent relative to the TooltipTrigger. Can be one of Start, Center, or End.
        align: ContentAlign::Center,
        // The content of the tooltip, which can include text, images, or any other elements.
        {children}
    }
}
```

## No click-only mode

Unlike the menus (`NavigationMenu`, `Navbar`, `Menubar` and the dropdown /
context menu submenus), which take an `open_on_hover` prop to switch hover
off, a tooltip has no such option: hovering or focusing the trigger *is* what
shows it, and it is a non-interactive hint, never the only way to reach
something. For a panel opened by a click, use [Popover](/component/popover/).

## Motion

The tooltip fades in and out over `--dx-motion-duration-slow` (`opacity` only, so the anchor's `transform` centring is untouched). Retune with that token. Reduced motion swaps in `--dx-motion-duration-reduced`. Nothing loops. See [Motion](/docs#motion) for the tokens, reduced motion and the per-component table.
