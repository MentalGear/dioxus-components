The NavigationMenu component displays a collection of links and disclosure panels for site navigation.

Unlike [Navbar](/component/?name=navbar), which implements the APG Menu and
Menubar pattern (`role="menubar"`/`"menu"`/`"menuitem"`, roving `tabindex`),
NavigationMenu implements the APG Disclosure (Show/Hide) Navigation
pattern: plain links and `button[aria-expanded][aria-controls]` disclosure
triggers, with no menu role anywhere. Every trigger and link keeps its
native tab stop, so Tab/Shift+Tab move through them (and, once a panel is
open, through its links) in DOM order -- there is no roving-focus
collection to manage. Use this component for real site navigation; use
Navbar (or [Menubar](/component/?name=menubar)) when you need the fuller
menu-button keyboard contract.

## Component Structure

```rust
// The NavigationMenu component wraps the whole nav landmark. It needs an
// accessible name -- pass `aria_label` or `aria_labelledby`.
NavigationMenu {
    aria_label: "Main",
    // The NavigationMenuList is the `ul` that holds the top-level items.
    NavigationMenuList {
        // Each NavigationMenuItem is one top-level entry, in order of its index.
        NavigationMenuItem {
            index: 0usize,
            // A trigger discloses a panel of links on click or hover.
            NavigationMenuTrigger { "Components" }
            NavigationMenuContent {
                // Compose any markup here -- typically a grid of links.
                NavigationMenuLink { href: "/component/?name=accordion", "Accordion" }
                NavigationMenuLink { href: "/component/?name=dialog", "Dialog" }
            }
        }
        NavigationMenuItem {
            index: 1usize,
            // Or an item can be a single plain link with no panel.
            NavigationMenuLink { href: "/docs", "Docs" }
        }
    }
}
```

## Keyboard interaction

- `Enter` / `Space`: activates a focused trigger, toggling its panel.
- `Escape`: closes the open panel and returns focus to its trigger.
- `Tab` / `Shift+Tab`: move through every top-level trigger/link, and (once
  a panel is open) through its links, in page order.
- `Left` / `Right` arrow (optional in APG, supported here): move between
  top-level items.
- `Down` arrow on an open trigger: moves focus into the panel's first link.
- Moving focus out of the whole navigation menu closes an open panel.
