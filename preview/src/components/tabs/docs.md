The Tabs component is used to create a tabbed interface, allowing users to switch between different views or sections of content.

## Component Structure

```rust
// The Tabs component wraps all tab triggers and contents and orders them based on their index.
Tabs {
    // The TabList component contains all the tab triggers
    TabList {
        // The TabTrigger component is used to create a clickable tab button that switches the active tab.
        TabTrigger {
            // The index of the tab trigger, used to determine the focus order of the tabs.
            index: 0,
            // The value of the tab trigger, which must be unique and is used to identify the active tab.
            value: "tab1",
            // The contents of the tab trigger button
            {children}
        }
    }
    // The TabContent component contains the content that is displayed when the corresponding tab is active.
    TabContent {
        // The index of the tab content, used to determine the focus order of the tabs.
        index: 0,
        // The value of the tab content, which must match the value of the corresponding TabTrigger to be displayed.
        value: "tab1",
        // The content of the tab, which is displayed when the tab is active.
        {children}
    }
}
```

## Default and Ghost styles

`Tabs` takes `variant: TabsVariant`. `Default` is the muted pill with a raised active tab inside a bordered card. `Ghost` is shadcn's `line` variant: a transparent list, no card around the panel, and the active tab carries a 2px underline that fades in (the docs site's DEMO/CODE tabs use it).

## Direction / RTL

`Tabs` accepts a `dir: Option<Direction>` prop (defaulting to the nearest `DirectionProvider`, or LTR). Under RTL, `TabTrigger`'s `ArrowLeft`/`ArrowRight` roving focus swaps: `ArrowLeft` moves to the *next* tab, `ArrowRight` to the *previous* one (matching Radix's shared `RovingFocusGroup` behavior). See the `rtl` variant.

## Searchable inactive tabs

By default inactive panels are unmounted (an empty `hidden` element), so find-in-page (Ctrl+F) cannot see their text and a `#fragment` link cannot open them. Pass `hidden_until_found: true` on `Tabs` to keep every panel's content mounted and mark inactive panels `hidden="until-found"`. The browser then searches them, and when a match is inside an inactive panel it reveals the panel and its tab becomes the active one: `aria-selected` and `data-state` follow, and `on_value_change` is called with that tab's value (a controlled `value` should follow it). See the `until_found` variant: try Ctrl+F for the word in its hint.

```rust
Tabs {
    default_value: "tab1".to_string(),
    hidden_until_found: true,
    // ...TabList and TabTrigger as above...
    TabContent { index: 1usize, value: "tab2".to_string(), "Text that Ctrl+F can find while inactive" }
}
```

Cost: the content of **every** tab is mounted and, on a server-rendered page, hydrated, so effects, timers and charts inside inactive tabs all run, and all of it is in the server-rendered HTML. That is why it is opt-in; leave it off for heavy panels.

### Browser support

| Engine | `hidden="until-found"` | `beforematch` |
| --- | --- | --- |
| Chrome / Edge 102+ | Yes | Yes |
| Firefox | 148+ (139 to 147: reveals, may scroll to the wrong place) | 139+ |
| Safari | 26.2+, partial: reveals the content but does not scroll to the match | 26.2+ |
| Older engines | Treated as plain `hidden`: the content stays closed and unsearchable, exactly as without the prop | n/a |

Nothing breaks where it is unsupported: the content is simply not found by find-in-page, as it is by default.

### Accessibility

Inactive `until-found` panels are hidden from the accessibility tree exactly like `hidden`: screen readers only reach the active panel, as with plain tabs. Find-in-page is the one way to reach an inactive panel without selecting its tab; it is exposed as soon as it activates.

### Styling

`TabContent` carries `hidden` when inactive (`hidden="until-found"` with the opt-in). A stylesheet must not set `display: none` on an `until-found` panel: that overrides the browser rule that keeps its text searchable. The shipped stylesheet skips `[hidden="until-found"]` and takes such a panel out of flow at zero size, so the inactive panels add no empty cards or gaps.
