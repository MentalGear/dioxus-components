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

### Open on hover, or click only

By default hovering a `DropdownMenuSubTrigger` opens its submenu after a short
delay (200 ms) and leaving it closes the submenu again after a short grace
delay, so a diagonal move toward the submenu is not misread as leaving. Pass
`open_on_hover: false` to the `DropdownMenuSub` for **click activation**:

```rust
DropdownMenuSub {
    // Default is `true`.
    open_on_hover: false,
    DropdownMenuSubTrigger { index: 0, "Export as" }
    DropdownMenuSubContent { /* ... */ }
}
```

The submenu then opens only on click, `Enter`, `Space` or the open arrow key,
and the pointer never opens *or* closes it: it closes on `Escape` / the close
arrow, an outside click, or focus moving elsewhere. Keyboard behaviour is the
same in both modes, and touch never hover-opens. The prop is named after Base
UI's `Menu.SubmenuTrigger` `openOnHover` (default `true`); it sits on the `Sub`
rather than the trigger because the trigger and the submenu content share the
hover timers. See the `click_only_submenu` variant.

## Checkbox and radio items

`DropdownMenuCheckboxItem` and `DropdownMenuRadioGroup` + `DropdownMenuRadioItem` are shadcn's
`DropdownMenuCheckboxItem`/`DropdownMenuRadioGroup`/`DropdownMenuRadioItem`. They render
`role="menuitemcheckbox"` / `role="menuitemradio"` with an always-present
`aria-checked`; radio items sit in a `role="group"` (name it with
`aria-label` or `aria-labelledby`). They work in a `DropdownMenuContent` and in a `DropdownMenuSubContent`.

```rust
DropdownMenuContent {
    DropdownMenuCheckboxItem {
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
    DropdownMenuRadioGroup {
        aria_label: "Panel position",
        value: Some(position()),
        on_value_change: move |value| position.set(value),
        // A radio item's `value` is also its typeahead label.
        DropdownMenuRadioItem { index: 1, value: "top".to_string(), "Top" }
        DropdownMenuRadioItem { index: 2, value: "bottom".to_string(), "Bottom" }
    }
}
```

Click, `Enter` and `Space` all select. A disabled item cannot be selected and
is skipped by arrow keys and typeahead. The check is drawn at the inline end, in a slot the item reserves (the same `--dx-menu-item-indicator-*` construction as the sub-trigger chevron), so a long label never meets it and an unchecked item does not shift its label. See the `checkboxes` and
`radio_group` variants.

## Direction / RTL

`DropdownMenu` accepts a `dir: Option<Direction>` prop, inherited by every `DropdownMenuSub`. Under RTL: the submenu open key becomes `ArrowLeft` (was `ArrowRight`), the close key becomes `ArrowRight` (was `ArrowLeft`; `Escape` always closes regardless of direction), and each `DropdownMenuSubContent` opens to the left of its trigger instead of the right. See the `rtl` variant.

## Motion

The menu panel fades and scales in over `--dx-motion-duration-base` (`opacity`, `scale` from 0.95 and a 2px `translate`, never `transform`, so the anchor's centring survives) and fades out with `--dx-motion-ease-in`; item highlights use `--dx-motion-duration-fast`. Retune with those tokens. Reduced motion swaps in `--dx-motion-duration-reduced`. Nothing loops. See [Motion](/docs#motion) for the tokens, reduced motion and the per-component table.
