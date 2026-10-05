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

## Checkbox and radio items

`ContextMenuCheckboxItem` and `ContextMenuRadioGroup` + `ContextMenuRadioItem` are shadcn's
`ContextMenuCheckboxItem`/`ContextMenuRadioGroup`/`ContextMenuRadioItem`. They render
`role="menuitemcheckbox"` / `role="menuitemradio"` with an always-present
`aria-checked`; radio items sit in a `role="group"` (name it with
`aria-label` or `aria-labelledby`). They work in a `ContextMenuContent` and in a `ContextMenuSubContent`.

```rust
ContextMenuContent {
    ContextMenuCheckboxItem {
        // Items are ordered by `index`, like plain items.
        index: 0,
        // Controlled: `checked` + `on_checked_change`. Uncontrolled:
        // `default_checked` -- but note the content unmounts when the menu
        // closes, so an uncontrolled item starts over on every open.
        checked: Some(show_status_bar()),
        on_checked_change: move |checked| show_status_bar.set(checked),
        // Selecting closes the menu by default (Radix's `onSelect` default).
        // `false` keeps it open so several items can be toggled in one visit.
        close_on_select: false,
        // A checkbox item has no `value`: set `text_value` to make it a
        // typeahead target.
        text_value: "Status Bar",
        "Status Bar"
    }
    ContextMenuRadioGroup {
        aria_label: "Panel position",
        value: Some(position()),
        on_value_change: move |value| position.set(value),
        // A radio item's `value` is also its typeahead label.
        ContextMenuRadioItem { index: 1, value: "top".to_string(), "Top" }
        ContextMenuRadioItem { index: 2, value: "bottom".to_string(), "Bottom" }
    }
}
```

Click, `Enter` and `Space` all select. A disabled item cannot be selected and
is skipped by arrow keys and typeahead. The check is drawn at the inline end, in a slot the item reserves (the same `--dx-menu-item-indicator-*` construction as the sub-trigger chevron), so a long label never meets it and an unchecked item does not shift its label. See the `checkboxes` and
`radio_group` variants.

## Direction / RTL

`ContextMenu` accepts a `dir: Option<Direction>` prop, inherited by every `ContextMenuSub`. Same open/close-key and side flip as `DropdownMenu`'s -- see that component's note. See the `rtl` variant.
