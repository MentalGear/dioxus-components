The context menu component can be used to define a context menu that is displayed when the user right-clicks on an element. It can contain various menu items that the user can interact with.

## Component Structure

```rust
// The context menu component must wrap all context menu items.
ContextMenu {
    // The context menu trigger is the element that will display the context menu when right-clicked.
    ContextMenuTrigger {
        // The content of the trigger
        {children}
    }
    // The context menu content contains all the items that will be displayed in the context menu.
    ContextMenuContent {
        // Each context menu item represents an individual action in the context menu. Items are displayed in order based on the order of the index property.
        ContextMenuItem {
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

A `ContextMenuContent` may nest a submenu with `ContextMenuSub`/
`ContextMenuSubTrigger`/`ContextMenuSubContent`/`ContextMenuSubItem`, the
same APG "Menu and Menubar" pattern submenu contract
`dropdown_menu`'s `DropdownMenuSub` implements (ArrowRight/Enter/Space
opens and moves focus to the first item; Escape/ArrowLeft closes and
returns focus to the sub-trigger; the parent menu stays open). A submenu
may hold plain items but not a further nested submenu of its own (one
level of nesting).

```rust
ContextMenuContent {
    ContextMenuSub {
        // The sub-trigger is itself an item of the enclosing menu, so it
        // takes an `index` the same way `ContextMenuItem` does.
        ContextMenuSubTrigger {
            index: 0,
            "More tools"
        }
        // Only rendered while the submenu is open.
        ContextMenuSubContent {
            ContextMenuSubItem {
                index: 0,
                value: "duplicate".to_string(),
                on_select: |value: String| {
                    // Selecting a sub-item closes the entire menu tree,
                    // not just this submenu.
                },
            }
        }
    }
}
```