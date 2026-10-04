<!-- Research notes written 2026-10-04 by the virtualization-survey lane (read-only research). Point-in-time: source at HEAD 44a103e plus the dirty working tree (message_scroller and the home-page card CSS are uncommitted). Probe numbers are single runs on a shared, loaded box; trust ratios, not milliseconds. -->

# Virtualization / windowing / lazy-render survey, and where `content-visibility` fits

Owner questions:

1. "Where could content-visibility be a better native replacement for our virtualization elements? Make a table."
2. "I think we also support x/y virtualization as a grid, is that correct?"

## 0. Answers

**Q2: No. We have no grid virtualization, and no horizontal virtualization of a list.** `VirtualList` is a vertical-only, single-column list. The only horizontal windowing is the carousel's slide strip (one axis, picked by `orientation`, never both). Evidence:

| Claim | Evidence |
|---|---|
| Props are one-dimensional: an item count, a row-count overscan, a *height* estimate, an index-to-element renderer. No columns, lanes, orientation, or width estimate. | `primitives/src/virtual_list.rs:19-28` (`count`, `buffer`, `estimate_size: Callback<usize, u32>`, `render_item: Callback<usize, Element>`) |
| Scroll reads are `scrollTop`/`clientHeight` only; nothing reads `scrollLeft`/`clientWidth`. | `virtual_list.rs:129-130`, `:285` (writes `scrollTop` only) |
| Items are measured by `rect.height` only. | `virtual_list.rs:202-203` |
| Positioning is one `translateY` on a full-width sizer. | `virtual_list.rs:245-247` |
| Engine state is two scalars, `scroll_offset: u32` and `viewport_size: u32`. | `primitives/src/virtual/virtualizer.rs:19-21` |
| `VirtualItem` is `{key, index, start, size}`, a 1-D prefix sum. | `primitives/src/virtual/types.rs:8-13`, `virtualizer.rs:52-82` |
| The shared window math is 1-D (`window(count, start, end, overscan, wrap)`). | `primitives/src/virtual/window.rs:35` |
| No grid, lanes, or column API anywhere else. rg for `VirtualGrid`, `virtual_grid`, `scroll_left`, `estimate_width`, `column_width` over `primitives/src`, `preview/src`, `labs` has no hits ("lanes" only appears as the word for work lanes in unrelated comments); `scrollLeft` is read only by `carousel.rs`, `drawer.rs` and `scroll_lock.rs`, none of them a virtualizer. | grep, this session |
| The carousel does window horizontally (or vertically), but it is a slide strip: one axis chosen from `ctx.orientation`, window of `2*radius+1` positions around an anchor, no second axis. All three shipped virtual demos are horizontal. | `primitives/src/carousel.rs:4696-4704` (axis style), `:4152-4210` (props), `preview/src/components/carousel/variants/{virtual_many,virtual_loop,virtual_loop_rtl}` |
| Things that look 2-D but are not windowed: `ScrollArea` (`ScrollDirection::Both` is the default, but it is a native `overflow` wrapper that renders all children), `Calendar` (`role="grid"` month cells, paged by view date, `month_count` months), `TagGroup` (`role="grid"` rows, all rendered). | `primitives/src/scroll_area.rs:43-51`, `primitives/src/calendar.rs:423`, `primitives/src/tag_group.rs:742` |

So "partially" is the most generous reading: horizontal windowing exists, but only inside the carousel and only as a 1-D strip. A real grid would be new work, and (see §3) `content-visibility` is a much better first answer for grids than building a 2-D windowing engine.

**Q1: see the table in §2.** Short version: `content-visibility: auto` is already the right tool for the docs-site cards and the message scroller; it is a strong replacement for `VirtualList` up to roughly 5-10k simple rows and a bad one beyond ~50k rows (it does not remove DOM nodes); it cannot replace the looping carousel window; and the single place a native "content-visibility family" feature adds a *user-visible capability* rather than perf is `hidden="until-found"` for accordion / collapsible / tabs panels.

## 1. Inventory

Every virtualization / windowing / lazy-render mechanism found. "Rust re-render" means a Dioxus component re-render driven by a signal; "JS" means a `document::eval` bridge or a script string.

### 1.1 `VirtualList` (`primitives/src/virtual_list.rs`, engine in `primitives/src/virtual/`)

- **Windows:** rows, y axis only, one column.
- **How:** a JS bridge (`virtual_list.rs:126-170`) listens to the container's `scroll` and calls `dioxus.send({offset, viewport, isScrolling})` on every scroll event ("no RAF batching", comment at `:155-157`), with a 600 ms scroll-end debounce (`:165`). Rust writes `scroll_offset`/`is_scrolling` into a `Store` (`virtualizer.rs:122-123`); the render body reads them (`virtualizer.rs:257-258`), so **every scroll event re-renders the whole `VirtualList`**: `get_virtual_items` then `render_item(idx)` for every row in viewport + `buffer` (`virtual_list.rs:222`, `:259`). A binary search (`find_nearest_binary_search`) plus a forward walk picks the range; `window()` pads it with `buffer`.
- **Size model:** measured, with an estimate fallback. A per-item `onresize` (ResizeObserver) writes `item_size_cache: HashMap<usize, u32>` keyed by absolute index (`virtualizer.rs:27`, `:186`); `estimate_size` is used for unmeasured rows, else the adaptive average of measured rows, else 100 px (`virtualizer.rs:57-73`). Total size is frozen while scrolling to stop scrollbar drift (`stable_total_size`, `virtualizer.rs:100-107`).
- **Cost profile at scale (code reading, not benchmarked):**
  - O(window) vdom walk plus one wasm-to-JS hop per scroll event.
  - `compute_measurements` is an **O(N) `Vec<VirtualItem>` rebuild** (24 B per item) whenever `count` or `item_size_cache` changes (`virtual_list.rs:105-117`, `virtualizer.rs:52-82`), i.e. on *every newly measured row*.
  - `onresize` does `measurements.peek().clone()`, an **O(N) clone on every row resize event** (`virtual_list.rs:208`).
  - `item_size_cache` never evicts (`virtualizer.rs:186`): grows to N entries over a full scroll.
  - At N = 1M that is ~24 MB allocated per new measurement and per resize event. At the shipped 2000-row demos it is invisible.
- **Depends on DOM removal:** yes, by design (that is the whole point, for 10k-1M rows).
- **a11y / keyboard:** container `role="list" tabindex="0"`, rows `role="listitem"` with `aria-setsize` / `aria-posinset` (`virtual_list.rs:235-236`, `:256-257`). Roving focus is not implemented: rows are plain content. A focused element inside a row that scrolls out of the window is unmounted with it, so focus would fall to `<body>` (inferred from the keyed unmount; not tested).
- **SSR / find-in-page:** the first render has `viewport_size == 0`, so `calculate_range` returns `None` and **zero rows render in SSR/SSG HTML** (`virtualizer.rs:260`). After hydration only viewport + 2*buffer rows exist, so native find-in-page sees about 30 of 2000 rows.
- **Demos / specs:** `preview/src/components/virtual_list/variants/{main,random_heights}` (2000 rows, `buffer: 12`); real consumer: the email-client dashboard list (`preview/src/dashboard/views/email_client/list_pane.rs:113-121`, mixed 34 px headers / 130 px rows, `estimate_size` provided). `playwright/virtual_list.spec.ts`: scroll-height stability (3 tests), "virtualizes rows and updates on scroll", resize-churn panic regression, axe. No find-in-page, focus, or 10k+ row spec.

### 1.2 `CarouselVirtualContent` (`primitives/src/carousel.rs:4152-4780`)

- **Windows:** slides, along the single scroll axis chosen by `orientation` (horizontal default; vertical supported by the primitive, no virtual vertical demo). Window = `2*radius+1` positions (default `radius: 10`, `:4190`) around an `anchor`, via the shared `window(.., wrap)` math. Auto-virtualizes only when `items.len() > 2*radius+1` (`:4200`); otherwise, and in SSR plus the hydrating first render, renders *all* items.
- **How:** Rust re-render of the window, but **only at idle**: the anchor re-centres after a `scrollend` / idle gate, never mid-gesture (nothing writes scroll position during momentum). Slides are keyed by position so surviving slides keep their DOM node; a looping list keeps sliding physically forward through already-mounted slides (a position can exceed `[0, N)`).
- **Size model:** fixed by CSS, not measured: `flex: 0 0 calc((100% - peek) / per-view)` (`carousel.rs:320-322`). No ResizeObserver per slide, no estimates.
- **Depends on DOM removal:** for the loop, yes in a structural way (the window rotates positions; one `data-index` repeats across positions for small N). For memory, 21 slides vs N slides is the saving, but N is small in practice (demos: 12 and 200).
- **a11y / keyboard:** slides are `role="group"` (`tabpanel` with a tablist) with `aria-roledescription="slide"` and a `"{i} of {N}"` label because `group` supports neither `aria-setsize` nor `aria-posinset` (`carousel.rs:551`). **Non-visible slides are `inert`** (`:4733`, `:4066`), so they are already out of the Tab order and the accessibility tree *and out of find-in-page* (probe, §4: `window.find` cannot find text inside `inert`). Focus is on the scroller (`tabindex="0"`); arrow keys page.
- **Demos / specs:** `virtual_many` (200 items, no loop, 21-slide window), `virtual_loop` (12 items, seamless loop, autoplay, dots), `virtual_loop_rtl`; `playwright/carousel.spec.ts:3357-3660` (DOM-never-holds-more-than-2r+1, loop, a11y), `playwright/carousel-virtual-wheel.spec.ts`, `labs/carousel-lab`.

### 1.3 `MessageScroller` (`primitives/src/message_scroller.rs`, `.js`; **uncommitted**)

- **Windows:** nothing is mounted/unmounted. `Virtualization::ContentVisibility` (the default) keeps every row in the DOM and lets CSS skip off-screen rows: `content-visibility: auto; contain-intrinsic-size: auto var(--dx-message-scroller-row-estimate, 10rem)` on `.dx-message-scroller-item:not([data-keep-rendered])` (`preview/src/components/message_scroller/style.css:72-83`).
- **How:** CSS, plus a small controller that marks the last 8 rows (live edge), the row holding focus, and the rows holding the selection ends with `data-keep-rendered` so they are never skipped (`message_scroller.js:104-150`, `syncKeepRendered`). No per-scroll Rust work; no IntersectionObserver yet (stage 2).
- **Size model:** estimate once (`10rem`), then the browser remembers each row's last rendered height (`auto`).
- **Depends on DOM removal:** no, by choice. The module doc calls mounting only a window "a possible future option, deliberately not part of this API" (`message_scroller.rs:74-76`) and `docs.md:69` adds "not planned for now". Ceiling: DOM and RSS grow linearly (see §4).
- **a11y / find:** `role="log"`, `aria-relevant="additions"`; rows are real DOM so the a11y tree, selection and find-in-page reach all of them. **Caveat the docs do not state:** in Safari 18-25 `content-visibility: auto` does not support find-in-page for skipped content (WebKit bug 283846; fixed in Safari 26, §5).
- **Demos / specs:** `variants/long` (2000 rows); `playwright/message_scroller.spec.ts:336-430` asserts all 2000 rows attached, fewer than 60 rendered, exactly 8 `data-keep-rendered`, `window.find("Message 777.")` succeeds, follow/jump/focus cases.

### 1.4 Docs site: home-page cards and component-page variants

- `preview/assets/main.css:1415-1442` (uncommitted, +185 lines in the working tree): `.dx-component-card { content-visibility: auto; contain-intrinsic-block-size: auto 380px }` and `.dx-widget-card:not(.dx-widget-card-popout) { ... auto 330px }`, 450 px for `<= 640px` viewports. Popout cards are excluded because `content-visibility: auto` implies paint containment, which would clip their dropdown.
- Motivation and measurements: `dev-docs/research/scroll-jank-2026-10-04.md` (4.4k nodes, 29k px page; `PrePaint` 1,851 -> 135 ms; main busy -63%; ~119 layouts per full scroll, one per card entering view).
- **Not yet covered, same shape:** the component page renders every variant inline with a live demo plus a highlighted source block (`preview/src/main.rs:1590-1608`, `.dx-component-variant`; carousel has 13 variants). Same rule would apply, same popout caveat.

### 1.5 Table / DataTable

- `preview/src/components/table/component.rs`: plain semantic `<table>` in a `div.dx-table-container` (horizontal overflow). **No virtualization.**
- `preview/src/components/data_table`: **client-side pagination**, not virtualization: `paginate()` slices the sorted/filtered Vec (`state.rs:127`), page sizes `[5, 10, 20]` (`variants/main/mod.rs:186`), demo data is 12 rows (`variants/main/mod.rs:28`). No `aria-rowcount` / `aria-rowindex` anywhere in the repo's table code (`rg` finds `aria-rowindex`/`aria-colcount` only in `tag_group.rs`).

### 1.6 Calendar / DatePicker

`Calendar` renders `month_count` month grids (`role="grid"`, 6x7 cells) selected by `view_date` (`calendar.rs:423`, `:78`); paging is by state, not scrolling. ~50-150 nodes. Not windowed.

### 1.7 Combobox / Command / Select / Listbox

All options are rendered eagerly as children of the list. **Combobox and Command filter by unmounting non-matches** (`combobox/components/option.rs:96-97`, `if render() && visible()`), which is a form of lazy rendering driven by the query, not by scroll. Lists scroll inside `max-height: 300px` (`combobox/style.css:100-102`) or `18rem` (`command/style.css:145-146`); demos have 5-12 options. Keyboard model: `Select` drives real DOM focus per option through the collection (`collection.rs:348-356`, `set_focus(true)`); Combobox/Command keep focus on the input and use `aria-activedescendant` (`combobox/components/input.rs:141`, `command.rs:455`). I found no `scrollIntoView` / `scroll_to` in either primitive (rg over `primitives/src/combobox`, `command.rs`, `collection.rs`, `selectable.rs`); worth a spec if long lists are a goal. No windowing, no `aria-setsize`.

### 1.8 Charts, ScrollArea, DragAndDropList, Toast

- Charts: SVG, no decimation or windowing (`rg decimat|downsampl|lttb` empty); `onresize` only. A page full of charts is served by card-level `content-visibility` (§1.4), not chart-level.
- `ScrollArea`: native overflow wrapper (`ScrollDirection::{Vertical, Horizontal, Both}`), renders all children.
- `DragAndDropList`: needs every item live as a drop target; not a candidate.
- Toast: a handful of items.

### 1.9 Collapsed content (accordion / collapsible / tabs): not windowing, but the same "keep it cheap and searchable" question

| Component | Closed content today | Evidence |
|---|---|---|
| `Collapsible` | **Unmounted** unless `keep_mounted`. With `keep_mounted` the primitive "does not apply any special ARIA or other attributes" and the styled layer has no closed-state rule (`.dx-collapsible-content { display: contents }` only), so a kept-mounted closed collapsible would simply render visible. No spec covers `keep_mounted`. | `primitives/src/collapsible.rs:21-25`, `:189`; `preview/src/components/collapsible/style.css:31` |
| `Accordion` | **Unmounted** after the close animation (`use_animated_open` holds it ~250 ms past the animation, then removes it). | `primitives/src/accordion.rs:381-387`, `primitives/src/lib.rs:755` |
| `Tabs` | Inactive `TabContent` renders an **empty** `div[hidden]` (children only when selected); the styled layer also has `display: none` for inactive. | `primitives/src/tabs.rs:467-478`, `preview/src/components/tabs/style.css:118` |

All three are therefore **not searchable**: Ctrl+F cannot find text in a closed accordion item, a closed collapsible or an inactive tab, and a `#fragment` link into that content cannot reveal it. Closed-state SSR/SSG HTML also omits the content (no SEO, no no-JS fallback).

## 2. The table: could `content-visibility: auto` (+ `contain-intrinsic-size`) replace it?

Legend: **Yes** = replace; **Hybrid** = keep the mechanism, add `content-visibility`; **No** = keep as is.

| Mechanism | What it windows | Current cost profile | cv replace? | Why | Recommendation |
|---|---|---|---|---|---|
| `VirtualList` (vertical list; 2000-row demos, email client) | Rows, y axis only | O(window) Rust re-render **per scroll event** + wasm-JS hop; O(N) memo rebuild per new measurement; O(N) clone per row resize; SSR renders 0 rows; find-in-page sees ~30 of N rows; focus lost when a focused row unmounts | **Hybrid** (Yes for N up to ~5-10k, No beyond ~50k) | cv removes all of the scroll/measurement machinery and gives SSR HTML, native find, focus, selection, a11y tree for free. But it keeps every DOM node: our probe at 9 nodes/row shows +72 MB / 0.2 s at 10k rows, +149 MB / 1 s at 30k, +445 MB / 1.5 s at 100k (extrapolated ~4 GB at 1M); pure windowing stayed +12 MB at every N. The initial Dioxus mount of N rows is also O(N) in vdom/mutations, which cv does not touch. | Add a `ContentVisibility` mode to `VirtualList` mirroring `Virtualization::ContentVisibility` in `message_scroller` (default for N <= ~5k); keep windowing for large N. Separately: quantize the scroll signal to the visible index range (re-render only when the range changes) and drop the O(N) `peek().clone()`. |
| `VirtualList` rendered overscan rows ("hybrid" inside the window) | Rows in `buffer` above/below the viewport | Each overscan row is fully laid out and painted; `buffer` is 8-12, kept small for that reason | **Hybrid** (limited) | `content-visibility: auto` on rows lets overscan grow cheaply in *layout/paint*. It does **not** reduce the Rust cost (the vdom walk and `render_item` calls grow with `buffer` on every scroll event, §1.1) and it poisons measurement: ResizeObserver reports the `contain-intrinsic-size` of a skipped row, which `resize_item` would cache as a real measurement (`virtual_list.rs:200-216`). Needs `contentvisibilityautostatechange` / `checkVisibility` gating. | Only worth doing after the scroll signal is quantized; otherwise skip. |
| `CarouselVirtualContent`, looping (`virtual_loop`, 12 items) | Slides, one axis (x or y) | 11-21 slides mounted; re-render only at idle; fixed flex-basis, no measurement | **No** | The window *rotates positions* to fake an infinite strip; paint-skipping cannot loop. Non-visible slides are already `inert` by the APG contract, so cv's find-in-page benefit does not exist here. | Keep. |
| `CarouselVirtualContent`, non-loop large N (`virtual_many`, 200) and plain `CarouselItem` strips with big N | Slides, one axis | 21 of 200 slides mounted; "false end" handling needed at the window edge (`carousel.rs` content_overscroll_style) | **Hybrid** | cv works on the inline axis (probe: 1000 flex slides, 9 rendered at start, 15 after a far scroll) and the slide width is already fixed by `flex-basis`, so only the block size needs `contain-intrinsic-size`. Rendering all N with cv would delete the false-end special case, but loses the DOM/memory bound at large N and, since `inert` hides non-visible slides, gains no find-in-page. | Low priority. If wanted: `virtualize: Some(false)` plus cv on slides for N up to a few hundred. Check that the paint containment does not clip slide shadows/focus rings at the gap padding. |
| `MessageScroller` (rows in a transcript) | Nothing mounted/unmounted; CSS skip | Already cv by default; 2000 rows <60 rendered; live-edge/focus/selection rows kept | **Already Yes** | Real DOM keeps find, a11y tree and cross-row selection; windowing is "not planned". | Keep. Document the ceiling (a chat row is ~20-50 nodes, so 10k rows is ~200-500k nodes) and the Safari 18-25 find-in-page gap (§5). Tune `--dx-message-scroller-row-estimate`. |
| Docs home-page cards (`.dx-component-card`, `.dx-widget-card`) | Whole live demos | 70 demos + 14 blocks, 4.4k nodes: any frame re-walked the whole page; cv: busy -63%, `PrePaint` 1,851 -> 135 ms | **Yes** (already in the working tree) | Windowing would unmount live demos and lose their state and timers; cv skips style/layout/paint and animations of off-screen cards while keeping them focusable and `#fragment`-reachable. Popout cards excluded (paint containment clips their dropdown). | Land it. Also apply to `.dx-component-variant` on component pages (carousel has 13). Watch: the masonry is CSS multicol, so a bad estimate re-balances columns until a card has been seen once. |
| `Table` / `DataTable` | Rows, by pagination (5/10/20 per page, 12 demo rows) | Trivial | **No** | Pagination is the right tool. Probe (Chromium 141): `content-visibility: auto` on `<tr>` skipped **0 of 200** rows (internal table boxes do not take size containment); on `<td>` it skipped most cells but table layout stays O(rows). `display: block`/`grid` rows work but drop native table semantics. | Keep pagination. For 10k+ rows build a new div-grid `role="table"` component (cv rows with `aria-rowcount`), not a `<table>` hack. |
| 2-D grid / data grid (does not exist) | Rows and columns | n/a | **Yes**, as the first answer | cv skips on **both axes** (probe: 100x100 grid of 10,000 cells, 495 rendered at the origin and 1,430 after a far diagonal scroll, ~5-14%). `contain-intrinsic-size` needs **both** dimensions (`auto 120px 40px`) when tracks are `auto`: with a single value, skipped cells size to a square and the scroll extent collapsed from 12,000 to 4,000 px in the probe. Fixed (`px`/`fr`) tracks are immune. | If grid virtualization is wanted, start with CSS grid + cv; a 2-D windowing engine is only needed past ~100k cells. |
| `Calendar` / `DatePicker` | Months (`month_count`) | ~50-150 nodes | **No** | Tiny; cv on day cells would interact with date-keyboard navigation and gains nothing. | None. |
| `Combobox` / `Command` / `Select` options | Options (filtered by unmounting non-matches) | Up to a few dozen options in demos | **Hybrid** (opt-in) | Worth it only for lists of ~500-5,000 options (country/timezone pickers): `[role=option] { content-visibility: auto; contain-intrinsic-size: auto 2rem }`. Probe: `focus()` on a skipped element works, un-skips it and scrolls to it; it is re-skipped after focus leaves. Not enough for 10k+ (then windowing, and `aria-activedescendant` on unrendered options becomes the hard part). | Document the CSS recipe; no primitive change. Add a scroll-into-view spec for Combobox/Command (§1.7). |
| `Collapsible` / `Accordion` / `Tabs` closed content | Closed panels | Closed content unmounted (so cheap) but **unsearchable and not deep-linkable** | **Hybrid, a different feature: `hidden="until-found"`** | `hidden="until-found"` is `content-visibility: hidden` that find-in-page and `#fragment` navigation can reveal, firing `beforematch`. Probe: fragment navigation into it fires `beforematch` once; `window.find` finds `until-found` text but not `display: none` text. | Add an opt-in "searchable" mode (§6). Default stays unmounted. |
| Charts (SVG) | n/a | One SVG per chart | **No** | cv is per DOM subtree, not per SVG point; decimation would be the chart-level tool (none exists). Card-level cv already covers the page cost. | None. |
| `ScrollArea`, `DragAndDropList`, `Toast` | n/a | Small | **No** | No windowing to replace. | None. |

## 3. Why cv is a good (and a bad) replacement: the mechanics that matter

1. **What it saves.** Style, layout and paint of off-screen subtrees; their animations stop costing frames. It does **not** remove DOM nodes, vdom nodes, listeners or the Rust render cost of mounting them.
2. **Find-in-page, a11y, selection, focus, `#fragment`:** all keep working on `auto` content in Chromium (probes: `window.find` found and revealed a skipped row; `focus()` on a skipped button worked and scrolled). `content-visibility: hidden` and `inert` content is not findable.
3. **Sizing.** A skipped element lays out as if size-contained, using `contain-intrinsic-size`. `auto <len>` remembers the last rendered size once the element has been rendered, so revisits are exact; the length is only the first guess. An element that has never been rendered (e.g. rows above a far jump) keeps the guess, so `scrollHeight` and the thumb move as rows render. Probe: crawling upward through never-rendered rows with a 56 px guess for ~35-130 px rows produced 5-10 visible corrections (up to ~38 px) per 60 steps, with `overflow-anchor` on or off; a 120 px guess produced larger ones (up to ~114 px). Crude metric, one run each, so read it as "estimates matter", not as a measurement of anchoring. So estimates matter for scroll-up; `message_scroller` and the cards use block-size estimates measured from real pages for that reason.
4. **Axes.** Works on both. Block axis only needs `contain-intrinsic-block-size`; a grid with auto tracks, or a horizontal strip whose widths are not fixed by CSS, needs the inline size too (two-value form). Single-value `contain-intrinsic-size: auto 40px` means 40 x 40.
5. **Containment side effects.** `auto` applies layout + style + paint containment to the element: ink overflow (shadows, focus rings, popovers that are not top-layer) is clipped at the element's box; it forms a containing block for fixed/absolute descendants; and **in a flex column with a bounded height, cv items lose their min-content floor and `flex-shrink` squashes them** (probe: 200 items in a 300 px flex column rendered at 1.5 px each until `flex: none` was added; `message_scroller` already sets `flex-shrink: 0` on its items).
6. **Table internals.** `tr`/`tbody` are not skippable (probe: 0 of 200); `td` is.
7. **ResizeObserver.** A skipped element reports its intrinsic size. Any code that caches ResizeObserver sizes (`VirtualList`) must ignore entries from skipped rows (`checkVisibility({contentVisibilityAuto: true})` or `contentvisibilityautostatechange`).

## 4. Probes (read-only, scratch HTML; numbers are single runs, Chromium 141 headless, software compositing, 4 cores at load average ~18, so compare ratios)

Method: a scroll container of N identical rows (each 1 `b` + 3 `i` children = 9 nodes with its text), built from JS, three modes: **none** (all rows laid out), **cv** (`content-visibility: auto; contain-intrinsic-size: auto 56px`), **windowed** (60 rows plus a spacer, the VirtualList shape). RSS is the browser process tree's resident size minus its baseline.

| Rows (nodes) | none: build + first layout / RSS | cv: build + first layout / RSS | windowed |
|---|---|---|---|
| 2,000 (18k) | 115 ms / +49 MB | 48 ms / +50 MB | 8 ms / +19 MB |
| 10,000 (90k) | 816 ms / +194 MB | 213 ms / +72 MB | 7 ms / +19 MB |
| 30,000 (270k) | 3,152 ms / +388 MB | 1,011 ms / +149 MB | 8 ms / +12 MB |
| 100,000 (900k) | 5,283 ms / +1,545 MB | 1,503 ms / +445 MB | 6 ms / +12 MB |

Reading: cv is ~3-5x faster to first layout and ~2.5-3.5x lighter than rendering everything, but is still linear in N (~4.5 KB per 9-node row; ~4 GB extrapolated at 1M rows, not measured). Windowing is flat. Layout counts and re-layout-on-resize behave the same way (100k rows: re-layout 2,957 ms none vs 242 ms cv). The build here is raw JS DOM; a Dioxus mount adds vdom + mutation cost per node that cv does not reduce.

Other probes: skippable elements (`tr` 0/200, `td` 342/400, block/grid/flex-item divs 200/200 once `flex: none` is set); 2-axis grid (above); 1-D horizontal strip (1000 slides: 9 rendered at start, 15 after scrolling to the far end; `scrollWidth` unchanged); `focus()` and `window.find` on skipped rows (both work and un-skip); `window.find` on `inert` (false), `content-visibility: hidden` (false), `hidden="until-found"` (true), `display: none` (false); fragment navigation into `until-found` fires `beforematch`. Headless `window.find` did **not** fire `beforematch` for `until-found` (only fragment navigation did), so verify the Ctrl+F path by hand before relying on it.

## 5. Browser support (MDN browser-compat-data `main`, fetched 2026-10-04)

| Feature | Chrome | Firefox | Safari |
|---|---|---|---|
| `content-visibility` (`visible`, `hidden`) | 85 | 125 | 18 |
| `content-visibility: auto` | 85 | 125 | **26**. In 18-25 it is partial: *skipped content is not findable via find-in-page* (WebKit bug 283846) |
| `contain-intrinsic-size` (incl. `auto <len>`) | 83 | 107 | 17 |
| `contain-intrinsic-size: auto none` | 117 | 117 | 17 |
| `checkVisibility({contentVisibilityAuto})` | 121 | 122 | 17.4 |
| `contentvisibilityautostatechange` event | 108 | 125 (handler property 130) | 18 |
| `hidden="until-found"` | 102 | 148 (139-147 partial: wrong scroll target) | 26.2, **partial** (does not scroll to the match, WebKit bug 304174) |
| `beforematch` event | 102 | 139 | 26.2 |
| `overflow-anchor` (scroll anchoring) | 56 | 66 | **27** (so Safari <= 26 has no scroll anchoring: estimate errors above the viewport shift content visibly) |

Consequences: (a) the owner-facing "find-in-page keeps working" claim in `message_scroller` docs and the home-page card comment is true on Chromium, Firefox 125+ and Safari 26+, and **false on Safari 18-25**; (b) `until-found` is a progressive enhancement, never a requirement; (c) `contain-intrinsic-size: auto` memory works everywhere `auto` does.

## 6. `hidden="until-found"` for accordion / collapsible / tabs

What it buys is a capability, not performance: Ctrl+F finds text in closed panels and the browser opens the panel; `#fragment` deep links into a closed panel open it. Today none of the three can do either (§1.9).

Sketch (opt-in, default unchanged):

- **Collapsible:** when closed and the opt-in is on, keep the children mounted and render the content wrapper with `hidden="until-found"` instead of unmounting; remove it when open. Today's `keep_mounted` already keeps the children but sets nothing, and the styled layer would show them. A `beforematch` listener calls `set_open(true)`.
- **Accordion:** replace "unmount after close animation" (`use_animated_open`) with "set `hidden="until-found"` after the close animation ends". The grid-rows height animation (`style.css:36-58`) is unaffected while the item is open or animating; only the final closed state changes.
- **Tabs:** render inactive panel children (opt-in), `hidden="until-found"`, and **delete the author `display: none` rule** (`tabs/style.css:118`): an author `display: none` defeats the UA rule that makes `until-found` content findable. `beforematch` selects that tab. Cost: every tab's content is mounted and hydrated (effects, timers, chart demos), which is why this must be opt-in.
- **Plumbing:** Dioxus 0.7 has no typed `onbeforematch`; the repo idiom is a small `document::eval` bridge or an `onmounted` listener.
- **Trade-offs:** all panel content is in the SSR/SSG HTML (bigger HTML, but no-JS readers and crawlers see it); closed content is `content-visibility: hidden`, so it costs ~nothing to lay out; a11y tree excludes it while hidden; Safari 26.2 reveals but may not scroll (bug 304174); older Safari/Firefox ignore the value and treat it as plain `hidden` (content stays unfindable, same as today), so there is no regression path.
- Native alternative for the accordion shape: `<details name="...">` (exclusive group, findable). Not verified here; it would be an API change, not a drop-in.

## 7. Recommendations

1. **`VirtualList`: add a content-visibility mode, keep windowing for large N, and fix the two O(N) paths.** cv mode (default for N up to ~5k) deletes the bridge, the measurement cache and the SSR-empty problem; windowing stays for >10-50k rows and anything in the 100k-1M range, where cv's linear DOM/RSS is disqualifying. Independently of cv: quantize the scroll signal to the visible range (stops a full component re-render per scroll event) and remove the per-resize `measurements.peek().clone()` (`virtual_list.rs:208`).
2. **Land and extend the docs-site card cv.** Add `.dx-component-variant` to the same rule; note the Safari 18-25 find-in-page gap in the comment; keep popout exclusions. `message_scroller` is already right; add the same Safari caveat to its docs.
3. **Add opt-in `hidden="until-found"` to Collapsible/Accordion/Tabs** (the only place a native feature adds a user-visible capability), and do not build 2-D windowing: if a grid is wanted, start from CSS grid + cv with two-value `contain-intrinsic-size`. Leave the looping carousel, `Table`/`DataTable` (paginated), Calendar, charts as they are.

## 8. Side findings (not asked, worth a ticket each)

- `VirtualList` O(N) per-resize clone and O(N) memo rebuild (`virtual_list.rs:105-117`, `:208`); `item_size_cache` never evicts (`virtualizer.rs:186`).
- `VirtualList` SSR/SSG output is an empty list (`virtualizer.rs:260`).
- `Collapsible { keep_mounted: true }` shows closed content in the styled layer (no closed-state rule), and has no spec.
- No `scrollIntoView` for the highlighted option in Combobox/Command (long lists may not follow keyboard highlight).
- No `aria-rowcount`/`aria-rowindex` in `Table`/`DataTable`; fine while paginated, needed before any large-table work.
- `message_scroller` / card comments overstate find-in-page support (Safari 18-25).
