The DropdownMenu component is used to create a dropdown menu that can be triggered by a button click. It allows users to select an option from a list of items.

## Component Structure

```rust
// The dropdown menu component must wrap all dropdown items.
DropdownMenu {
    // The dropdown menu trigger is the button that will display the dropdown menu when clicked.
    DropdownMenuTrigger {
        // The content of the trigger to display inside the button.
        {children}
    }
    // The dropdown menu content contains all the items that will be displayed in the dropdown menu.
    DropdownMenuContent {
        // Each dropdown menu item represents an individual option in the dropdown menu. Items are displayed in order based on the order of the index property.
        DropdownMenuItem {
            // The index of the item, used to determine the order in which items are displayed.
            index: 0,
            // The value of the item which will be passed to the on_select callback when the item is selected.
            value: "",
            on_select: |value: String| {
                // This callback is triggered when the item is selected.
                // The value parameter contains the value of the selected item.
            },
        }
    }
}
```

## Nested submenus

A `DropdownMenuContent` (or another submenu's own `DropdownMenuSubContent`)
may nest a submenu with `DropdownMenuSub`/`DropdownMenuSubTrigger`/
`DropdownMenuSubContent`/`DropdownMenuSubItem`, implementing the APG "Menu
and Menubar" pattern's submenu contract (ArrowRight/Enter/Space opens and
moves focus to the first item; Escape/ArrowLeft closes and returns focus to
the sub-trigger; the parent menu stays open). A submenu may hold plain
items but not a further nested submenu of its own (one level of nesting).

```rust
DropdownMenuContent {
    DropdownMenuSub {
        // The sub-trigger is itself an item of the enclosing menu, so it
        // takes an `index` the same way `DropdownMenuItem` does.
        DropdownMenuSubTrigger {
            index: 0,
            "More tools"
        }
        // Only rendered while the submenu is open.
        DropdownMenuSubContent {
            DropdownMenuSubItem {
                index: 0,
                value: "duplicate",
                on_select: |value: String| {
                    // Selecting a sub-item closes the entire menu tree,
                    // not just this submenu.
                },
            }
        }
    }
}
```