The VirtualList component keeps a long list cheap. It has two modes, picked by the `virtualize` prop, and by default it picks for you from the row count.

## Component Structure

```rust
VirtualList {
    // Total number of items.
    count: 2000usize,
    // Optional: the height guess for a row that has not been measured or rendered yet.
    estimate_size: |idx: usize| 80,
    // Optional: how off-screen rows are kept cheap. `Auto` (the default) picks by `count`.
    virtualize: VirtualListMode::Auto,
    // Row renderer callback receives the absolute index.
    render_item: move |idx: usize| rsx! {
        article { "{rows[idx].title}" }
    },
}
```

Give the list a height (or `max-height`) and `overflow-y: auto`: it is the scroll container. The list owns its `id`, `role="list"` and `tabindex="0"`; every row is a `role="listitem"` with `aria-setsize` and `aria-posinset`, in both modes.

## Modes

| Behaviour | `ContentVisibility` | `Windowed` |
|---|---|---|
| What is in the DOM | every row | the rows in view plus `buffer` rows each side |
| Off-screen rows | skipped by the browser, 20 rows at a time (`content-visibility: auto`) | not mounted |
| Scrolling costs Rust | nothing: no listener, no re-render | one re-render every few rows (`buffer / 2`) |
| Server-rendered HTML | every row | none |
| Find-in-page (Ctrl+F), selection, accessibility tree | reach every row (find-in-page: not Safari 18-25) | only the mounted window |
| Focus in a row that scrolls away | kept (the row stays rendered) | lost (the row is unmounted) |
| Row heights | the browser measures; `estimate_size` is its first guess | measured by the list; `estimate_size` is its first guess |
| Cost grows with the row count | yes: DOM and memory are linear (about 4.5 KB per 9-node row: roughly 72 MB at 10,000 rows, 445 MB at 100,000) | no: flat from 10,000 to millions of rows |
| Best for | up to a few thousand rows | long lists, 10,000+ |

### Auto (the default)

`Auto` uses `ContentVisibility` while `count <= auto_threshold` and `Windowed` above it. `auto_threshold` defaults to **1,000** rows, because the cost of keeping every row grows with the count: mounting them is one main-thread task proportional to the row count (in the demo, release build, headless Chromium: about 90 ms for 1,000 rows, 205 ms for 5,000, as one 147 ms task), while a window mounts about 40 rows whatever the count. The mode follows the live `count`: a list that grows past the threshold re-mounts its rows in the other mode (its scroll position resets), so choose an explicit mode if the count hovers around the threshold.

```rust
// Every row in the DOM, however many (a long list of one-line rows can afford it):
VirtualList { count: 20_000usize, virtualize: VirtualListMode::ContentVisibility, .. }
// Windowed from the first row (heavy rows, or find-in-page is not needed):
VirtualList { count: 800usize, virtualize: VirtualListMode::Windowed, .. }
// Move the automatic switch point:
VirtualList { count: 12_000usize, auto_threshold: 10_000usize, .. }
```

### Content-visibility mode

Every row is rendered into the page (and into the server's HTML), and the browser skips layout and paint for the rows that are off-screen. Rows are grouped in chunks of 20 consecutive rows and each chunk carries `content-visibility: auto; contain-intrinsic-block-size: auto <sum of its rows' estimate_size>px`: the estimate is the height the browser assumes for a chunk it has not rendered yet, and `auto` makes it remember the real height afterwards. Close estimates keep the scrollbar steady while you scroll up through rows the browser has never rendered.

Why chunks: the browser does per-frame work for every element it may skip, so skipping each row separately costs the main thread time proportional to the row count on every scroll frame. Measured in the demo (release build, headless Chromium, main-thread time per scroll frame): one skippable element per row cost 7.7 ms at 1,000 rows and 31 ms at 5,000; one per 20 rows costs 1.1 ms and 1.5 ms, the same as a windowed list (1.4 ms at 5,001 rows). The price is up to 19 extra rows rendered around the viewport.

- The chunk holding focus, or an end of a selection, is never skipped: that is the browser's own rule for `content-visibility: auto`, so there is nothing to configure.
- Find-in-page finds text in skipped rows and un-skips the match in Chromium, Firefox 125+ and Safari 26+. **Safari 18-25 does not search skipped content** (WebKit bug 283846); there, use `Windowed` and provide your own search, or accept it.
- `content-visibility: auto` applies paint containment to each chunk, so a shadow, focus ring or non-top-layer popover that overflows a chunk's box is clipped (rows inside a chunk do not clip each other).
- The first mount renders every row, and that is the cost that grows with the count (see the default threshold above: about 90 ms at 1,000 rows, 205 ms at 5,000). Any signal a row reads re-renders all of them. For heavy rows or much larger lists, use `Windowed`.

### Windowed mode

Only the rows in view are mounted, over a spacer that gives the scrollbar the full height. Rows may have any height: each is measured once mounted, and `estimate_size` is the first guess (without it, the list estimates from the average of the rows it has measured). While you scroll, the scrollbar's total is held fixed, and a row that resizes above the viewport adjusts the scroll position so content does not jump.

- `buffer` (default 8) is how many rows to mount beyond each edge of the viewport. The window is re-centred once fewer than half of them remain, so a scroll re-renders about once per `buffer / 2` rows rather than once per scroll event, and between `buffer / 2` and `buffer` rows are always ready beyond each edge.
- Row positions are kept in a Fenwick tree: measuring a row and finding the row at a pixel are `O(log N)`, and nothing is copied per measurement.
- Rows that are not mounted do not exist for native find-in-page, the accessibility tree (each mounted row announces its position, `3 of 100000`) or the server: the server renders no rows, and the first client render is empty until the container has been measured.
- Browsers cap an element's height (about 17 million px in Firefox, 33 million px in Chromium), which limits the spacer: with 100px rows that is roughly 300,000 rows.
- The windowed bridge adds a scroll listener on the container and a resize listener on the window, and removes both when the list unmounts or the mode switches.

## Props

- `count` (required): the number of rows.
- `render_item` (required): renders row `idx`.
- `estimate_size`: row height guess in px, by index.
- `virtualize`: `VirtualListMode::{Auto, ContentVisibility, Windowed}`, default `Auto`.
- `auto_threshold`: the largest `count` `Auto` keeps in `ContentVisibility`; default `1000`.
- `buffer`: windowed mode only; default `8`.
