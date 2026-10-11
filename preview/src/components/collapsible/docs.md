The `Collapsible` component allows users to create expandable sections that can be toggled open or closed. This is useful for displaying content in a compact manner, such as FAQs, menus, or any content that benefits from being hidden until needed.

## Component Structure

```rust
// The collapsible component must wrap all collapsible items.
Collapsible {
    // The trigger is used to expand or collapse the item.
    CollapsibleTrigger {}
    // The content that is shown when the item is expanded.
    CollapsibleContent {}
}
```

## Searchable closed content

By default closed content is unmounted, so find-in-page (Ctrl+F) cannot see it and a `#fragment` link cannot open it. Pass `hidden_until_found: true` on `Collapsible` to keep the content mounted while closed and mark it `hidden="until-found"`. The browser then searches it, and when a match is inside, it reveals the content and the collapsible opens (`on_open_change` is called with `true`, and a controlled `open` should follow it). See the `until_found` variant: try Ctrl+F for the word in its hint.

```rust
Collapsible {
    hidden_until_found: true,
    CollapsibleTrigger { "Recent Activity" }
    CollapsibleContent { "Text that Ctrl+F can find while closed" }
}
```

`hidden_until_found` overrides `keep_mounted`. Closed content that stays mounted with plain `keep_mounted` now carries the `hidden` attribute, so it is not displayed, not searchable and not in the accessibility tree.

Cost: the content is mounted (and, on a server-rendered page, hydrated) while closed, and it is in the server-rendered HTML. That is why it is opt-in.

### Browser support

| Engine | `hidden="until-found"` | `beforematch` |
| --- | --- | --- |
| Chrome / Edge 102+ | Yes | Yes |
| Firefox | 148+ (139 to 147: reveals, may scroll to the wrong place) | 139+ |
| Safari | 26.2+, partial: reveals the content but does not scroll to the match | 26.2+ |
| Older engines | Treated as plain `hidden`: the content stays closed and unsearchable, exactly as without the prop | n/a |

Nothing breaks where it is unsupported: the content is simply not found by find-in-page, as it is by default.

### Accessibility

Closed `until-found` content is hidden from the accessibility tree exactly like `hidden`: screen readers do not read it while the collapsible is closed. A browser's find-in-page is the one way to reach it without activating the trigger; the content becomes visible and exposed as soon as it opens.

### Styling

Closed content that stays mounted carries `hidden` (`hidden="until-found"` with the opt-in). A stylesheet that sets `display` on `CollapsibleContent` overrides the browser's own `hidden` rule, so it must handle `[hidden]` itself, and `until-found` needs a real box: `display: none` there would make the text unsearchable. The shipped stylesheet does both.

## Motion

`Collapsible` does not animate: its content shows and hides. For an animated height use [Accordion](/component/accordion/). See [Motion](/docs#motion) for the tokens, reduced motion and the per-component table.
