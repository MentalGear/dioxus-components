The NavigationMenu component displays a collection of links and disclosure panels for site navigation.

Unlike [Navbar](/component/navbar/), which implements the APG Menu and
Menubar pattern (`role="menubar"`/`"menu"`/`"menuitem"`, roving `tabindex`),
NavigationMenu implements the APG Disclosure (Show/Hide) Navigation
pattern: plain links and `button[aria-expanded][aria-controls]` disclosure
triggers, with no menu role anywhere. Every trigger and link keeps its
native tab stop, so Tab/Shift+Tab move through them (and, once a panel is
open, through its links) in DOM order -- there is no roving-focus
collection to manage. Use this component for real site navigation; use
Navbar (or [Menubar](/component/menubar/)) when you need the fuller
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
                NavigationMenuLink { href: "/component/accordion/", "Accordion" }
                NavigationMenuLink { href: "/component/dialog/", "Dialog" }
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

`NavigationMenuLink` renders a plain `<a>`, not a router `Link`. If your app is
served below a base path, prefix each `href` with it yourself.

## Open on hover, or click only

By default a trigger opens its panel when the pointer rests on it for a
moment (150 ms), and the panel closes again when the pointer leaves the
trigger and the panel. Pass `open_on_hover: false` to the `NavigationMenu` for
**click activation** instead:

```rust
NavigationMenu {
    aria_label: "Main",
    // Default is `true`.
    open_on_hover: false,
    NavigationMenuList { /* ... */ }
}
```

With `open_on_hover: false` hovering a trigger does nothing, a panel opens
only on click (or `Enter` / `Space` / `ArrowDown`), and the pointer leaving a
panel does **not** close it. It closes on a second click of its trigger,
`Escape`, choosing a link, focus leaving the navigation menu, or a pointer
press anywhere outside it. Keyboard behaviour is the same in both modes, and
touch never hover-opens (a tap is a click). Use it when panels hold dense
content a pointer should be able to cross freely, or when you simply do not
want menus opening under a resting mouse. See the `click_only` variant.

`open_on_hover` is this library's name for the switch (Base UI calls the same
idea `openOnHover` on `Menu.Trigger`); Radix's and Base UI's own
`NavigationMenu` only tune the hover *delay* and have no switch. The delays
here are fixed.

## Keyboard interaction

- `Enter` / `Space`: activates a focused trigger, toggling its panel.
- `Escape`: closes the open panel and returns focus to its trigger.
- `Tab` / `Shift+Tab`: move through every top-level trigger/link, and (once
  a panel is open) through its links, in page order.
- `Left` / `Right` arrow (optional in APG, supported here): move between
  top-level items.
- `Home` / `End` (optional in APG, supported here): move to the first/last
  top-level item.
- `Down` arrow on a trigger: opens its panel (if not already open) and
  moves focus into its first link.
- `Up` / `Down` arrow on a link inside an open panel (optional in APG,
  supported here): move to the previous/next link in that panel, stopping
  at the first/last link rather than wrapping.
- Moving focus out of the whole navigation menu closes an open panel.

## Motion

Opening content slides in over `--dx-motion-duration-slow` and out over `--dx-motion-duration-base` (`opacity`, `scale` and `translate`, never `transform`); the trigger chevron rotates with `--dx-motion-ease-standard`. Retune with those tokens. Reduced motion swaps in `--dx-motion-duration-reduced`. Nothing loops. See [Motion](/docs#motion) for the tokens, reduced motion and the per-component table.
