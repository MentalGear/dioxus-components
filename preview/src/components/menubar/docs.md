The Menubar component can be used to display a menu bar with collapsible menus.

## Component Structure

```rust
// The Menubar component wraps the entire menu bar and contains the individual menus in the order of their index.
Menubar {
    // The MenubarMenu contains the individual menus that can be opened.
    MenubarMenu {
        // The index of the menu, used to determine the order in which menus are displayed.
        index: 0,
        // The menubar trigger is the element that will display the menu when activated.
        MenubarTrigger {
            // The content of the trigger button
            {children}
        }
        // The menubar content contains all the items that will be displayed in the menu when it is opened.
        MenubarContent {
            // Each menubar item represents an individual items in the menu.
            MenubarItem {
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

## Checkbox and radio items

`MenubarCheckboxItem` and `MenubarRadioGroup` + `MenubarRadioItem` are shadcn's
`MenubarCheckboxItem`/`MenubarRadioGroup`/`MenubarRadioItem`. They render
`role="menuitemcheckbox"` / `role="menuitemradio"` with an always-present
`aria-checked`; radio items sit in a `role="group"` (name it with
`aria-label` or `aria-labelledby`). They work in a `MenubarContent`.

```rust
MenubarContent {
    MenubarCheckboxItem {
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
    MenubarRadioGroup {
        aria_label: "Panel position",
        value: Some(position()),
        on_value_change: move |value| position.set(value),
        // A radio item's `value` is also its typeahead label.
        MenubarRadioItem { index: 1, value: "top".to_string(), "Top" }
        MenubarRadioItem { index: 2, value: "bottom".to_string(), "Bottom" }
    }
}
```

Click, `Enter` and `Space` all select. A disabled item cannot be selected and
is skipped by arrow keys and typeahead. Unlike the dropdown and context menus, shadcn's menubar draws the check at the inline **start** -- same `--dx-menu-item-indicator-*` slot construction, on the other side. A plain item can reserve the same slot with `"data-inset": "true"` (shadcn's `inset`) so a menu that mixes plain and checkable items keeps its labels on one edge. See the `checkboxes` and
`radio_group` variants.

## Hover switching, or click only

A menubar never opens a menu from a hover alone -- the first menu always takes
a click. Once one is open, though, hovering another trigger switches to it (the
native menubar convention). Pass `open_on_hover: false` to the `Menubar` to turn
that off:

```rust
Menubar {
    // Default is `true`.
    open_on_hover: false,
    MenubarMenu { /* ... */ }
}
```

Moving the pointer across the triggers then does nothing: another menu opens
only on click, `Enter` / `Space` or the arrow keys. Keyboard behaviour is the
same in both modes. See the `click_only` variant.

## Direction / RTL

`Menubar` accepts a `dir: Option<Direction>` prop. Its top-level trigger row is always horizontal, so `ArrowLeft`/`ArrowRight` between menus always swaps under RTL. See the `rtl` variant.
