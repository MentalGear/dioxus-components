The Carousel component is a slideshow of slides the user pages through with Previous/Next buttons, `ArrowLeft`/`ArrowRight` (or `ArrowUp`/`ArrowDown` when vertical), or by dragging/scrolling the track directly. It implements the [WAI-ARIA Carousel pattern](https://www.w3.org/WAI/ARIA/apg/patterns/carousel/)'s "basic" (prev/next, no picker) style.

## Component Structure

```rust
// CarouselPrevious/CarouselNext are positioned with CSS (absolutely, so
// DOM order doesn't affect where they appear), but come BEFORE
// CarouselContent in markup so they precede the slide content in the
// page's Tab order -- matching the APG reference implementation exactly.
Carousel {
    // An accessible name is required (aria-label or aria-labelledby) and
    // must not contain the word "carousel" -- the region's own
    // aria-roledescription already says that.
    aria_label: "Featured photos",

    CarouselPrevious { /* an icon or "Previous" */ }
    CarouselNext { /* an icon or "Next" */ }

    // The scroll-snap track.
    CarouselContent {
        // Each slide gets a 0-based index, contiguous from 0 -- the same
        // convention Tabs' TabTrigger/TabContent use for their own index.
        CarouselItem { index: 0usize, /* slide 1 content */ }
        CarouselItem { index: 1usize, /* slide 2 content */ }
        CarouselItem { index: 2usize, /* slide 3 content */ }
    }
}
```

Each `CarouselItem` defaults its own accessible name to `"{n} of {m}"` (APG's own sanctioned exception to "don't put position/size in an accessible name") unless you supply your own `aria-label`/`aria-labelledby`.

## Sizing slides

How much of the track each slide occupies is a CSS decision, not a prop: `CarouselItem` defaults to `flex: 0 0 100%` (one full slide per view). Override it per item with an inline `style` (or `flex_basis`) to build a "peek"/multi-item-per-view layout -- see the `multiple` variant.

## Orientation

`orientation: CarouselOrientation::Vertical` pages with `ArrowUp`/`ArrowDown` instead of `ArrowLeft`/`ArrowRight`, and scrolls on the block axis. A vertical carousel needs an explicit height on `CarouselContent` (e.g. `style: "height: 20rem;"`) -- there is nothing else to derive one from, the same way `ScrollArea` needs an explicit `height`.

## A custom picker (dot indicators, etc.)

`use_carousel()` returns a read-only `CarouselApi` (`selected`, `count`, `can_scroll_prev`, `can_scroll_next`) plus `scroll_to(index)`, for building any picker UI alongside or instead of `CarouselPrevious`/`CarouselNext`. See the `indicators` variant's `CarouselIndicators`, or the `indicators` demo's own composition:

```rust
let api = use_carousel();
rsx! {
    for i in 0..api.count {
        button {
            "data-active": i == api.selected,
            onclick: move |_| api.scroll_to(i),
        }
    }
}
```

## Direction / RTL

`Carousel` accepts a `dir: Option<Direction>` prop (defaulting to the nearest `DirectionProvider`, or LTR). Under RTL, the root's `ArrowLeft`/`ArrowRight` paging swaps: `ArrowLeft` moves to the *next* slide, `ArrowRight` to the *previous* one (matching Radix's shared `RovingFocusGroup` convention, the same one `Tabs` follows). Scrolling itself needs no such swap at all -- slide order in the DOM never changes, and the browser's own `scrollIntoView` already resolves the correct physical position under `dir="rtl"`. `CarouselPrevious`/`CarouselNext` also reposition correctly on their own (`inset-inline-start`/`-end`), and any chevron-style icon placed inside either one is automatically mirrored (`transform: scaleX(-1)`, horizontal orientation only) so a caller who uses the same icon regardless of direction still gets one pointing the right way. See the `rtl` variant.

## What v1 does not include yet

- **Infinite looping.** `CarouselPrevious`/`CarouselNext` are genuinely `disabled` (native `disabled`, not just `aria-disabled`) at the first/last slide -- matching shadcn's own default carousel, which doesn't loop either.
- **Autoplay / a rotation control.** The APG pattern's "basic" style has no rotation requirement at all, so a manually-paged carousel like this one is fully conformant on its own; a rotation control (plus the `aria-live` region and focus/hover-pause behavior that only matter once one exists) is a planned fast-follow.
- **A tablist picker variant** (the APG pattern's "tabbed" style, slide picker = tabs). Planned as a fast-follow composed on `Tabs`' own roving-tabindex machinery.
