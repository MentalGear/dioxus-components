The accordion component is used to display collapsible content panels for presenting information in a limited amount of space. It allows users to expand and collapse sections to view more or less content.

## Component Structure

```rust
// The accordion component must wrap all accordion items.
Accordion {
    // Each accordion item contains both a trigger the expanded contents of the item.
    AccordionItem {
        // Each item must have an index starting from 0 to control where the item is placed.
        index: 0,
        // The trigger is used to expand or collapse the item.
        AccordionTrigger {}
        // The content that is shown when the item is expanded.
        AccordionContent {}
    }
}
```


## Searchable closed content

By default closed items are unmounted after their close animation, so find-in-page (Ctrl+F) cannot see their text and a `#fragment` link cannot open them. Pass `hidden_until_found: true` on `Accordion` to keep every item's content mounted and mark it `hidden="until-found"` once it is closed. The browser then searches it, and when a match is inside a closed item it reveals the content and the accordion opens that item: `on_change` is called with `true`, and with `allow_multiple_open: false` (the default) the open item closes, exactly as if the trigger had been clicked. See the `until_found` variant: try Ctrl+F for the word in its hint.

```rust
Accordion {
    hidden_until_found: true,
    AccordionItem {
        index: 0,
        AccordionTrigger { "Returns" }
        AccordionContent { "Text that Ctrl+F can find while closed" }
    }
}
```

Animations are unchanged: opening and closing by trigger animate the height as before, and the content is re-hidden once the close animation finishes. Opening because of a match is not animated, because the browser has already scrolled to the match at full size. `AccordionContent` exposes `data-closing` (only while the close animation runs) and `data-revealed` (open because of a match) for styling this; key a close animation on `data-closing`, not on `data-open="false"`, which is also how a closed panel looks at rest.

Cost: every item's content is mounted (and, on a server-rendered page, hydrated) while closed, and it is in the server-rendered HTML. That is why it is opt-in.

### Browser support

| Engine | `hidden="until-found"` | `beforematch` |
| --- | --- | --- |
| Chrome / Edge 102+ | Yes | Yes |
| Firefox | 148+ (139 to 147: reveals, may scroll to the wrong place) | 139+ |
| Safari | 26.2+, partial: reveals the content but does not scroll to the match | 26.2+ |
| Older engines | Treated as plain `hidden`: the content stays closed and unsearchable, exactly as without the prop | n/a |

Nothing breaks where it is unsupported: the content is simply not found by find-in-page, as it is by default.

### Accessibility

Closed `until-found` content is hidden from the accessibility tree exactly like `hidden`: screen readers do not read it while the item is closed, and the trigger's `aria-expanded` stays `false`. Find-in-page is the one way to reach it without activating the trigger; it is exposed as soon as it opens, and `aria-expanded` follows.
