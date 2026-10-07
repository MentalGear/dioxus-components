The Navbar component can be used to display a navigation bar with collapsible sections.

## Component Structure

```rust
// The Navbar component wraps the entire menu bar and contains the individual menus in the order of their index.
Navbar {
    // The NavbarNav contains the individual menus that can be opened.
    NavbarNav {
        // The index of the menu, used to determine the order in which menus are displayed.
        index: 0,
        // The menubar trigger is the element that will display the menu when activated.
        NavbarTrigger {
            // The content of the trigger button
            {children}
        }
        // The menubar content contains all the items that will be displayed in the menu when it is opened.
        NavbarContent {
            // Each menubar item represents an individual items in the menu.
            NavbarItem {
                // The value of the item which will be passed to the on_select callback when the item is selected.
                value: "",
                on_select: |value: String| {
                    // This callback is triggered when the item is selected.
                    // The value parameter contains the value of the selected item.
                },
            }
        }
    }
}
```

## Direction / RTL

`Navbar` accepts a `dir: Option<Direction>` prop. Its top-level trigger row is always horizontal, so `ArrowLeft`/`ArrowRight` between nav items always swaps under RTL, matching `Menubar`'s identical shape. See the `rtl` variant.

## Open on hover, or click only

By default hovering a `NavbarNav` opens its dropdown, hovering another one
switches to it, and the pointer leaving the nav closes it. Pass
`open_on_hover: false` to the `Navbar` for **click activation**:

```rust
Navbar {
    aria_label: "Main",
    // Default is `true`.
    open_on_hover: false,
    NavbarNav { /* ... */ }
}
```

Hovering then neither opens nor switches dropdowns, and the pointer leaving
an open dropdown does not close it. A dropdown opens on click (or `Enter` /
`ArrowDown` / `ArrowUp`) and closes on a second click, `Escape`, or a press or
focus outside it. Keyboard behaviour is the same in both modes, and touch never
hover-opens. See the `click_only` variant.
