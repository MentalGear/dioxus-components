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

## Spacing (the gap between slides)

The gap between slides is a `--dx-carousel-gap` custom property, read by `CarouselItem` (a leading-edge `padding-inline-start`/`padding-block-start`) and `CarouselContent` (the exactly-compensating negative `margin-inline-start`/`margin-block-start`) -- the standard shadcn/Tailwind `-ml-4`/`pl-4` idiom, not a real flex `gap`. This matters, not just for parity: a real `gap` is *additive* to a percentage `flex-basis`, so `N` items at `flex-basis: calc(100%/N)` plus `(N-1)` real gaps overflow the track by exactly `(N-1) * gap` -- with the gap living inside each item's own border-box instead, `N` basis fractions always sum to exactly 100% regardless of `N`, so a multi-per-view layout (see "Sizes" below) never needs to subtract a gap term at all.

`.dx-carousel-content` sets `--dx-carousel-gap: var(--dx-space-4)` (16px) by default -- override it per instance with an inline `style="--dx-carousel-gap: var(--dx-space-2);"` on `CarouselContent`. See the `spacing` variant for shadcn's own four presets (`--dx-space-1` through `--dx-space-4`, i.e. `-ml-1/pl-1` ... `-ml-4/pl-4`).

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

## Pointer drag

Click-and-drag anywhere on the track pages the carousel with the mouse or a pen, the same way shadcn/embla-style carousels do -- touch is never affected either way, since it already scrolls the track natively. A drag has to move a few pixels before it takes over, so a plain click on a link or button placed inside a `CarouselItem` still works as a click; a real drag suppresses the synthetic click that would otherwise follow it. Releasing scrolls smoothly to the nearest slide through the same `scrollIntoView` paging path `CarouselPrevious`/`CarouselNext` already use -- not the browser's own scroll-snap re-settling on its own, which (measured) never animated -- and it still lands instantly if the OS/browser reports `prefers-reduced-motion: reduce`, matching every other transition in this component. That settle is picked up by the same bridge (`use_carousel_scroll_tracking`) that already keeps `selected` correct after a native trackpad/touch scroll or a `CarouselPrevious`/`CarouselNext` click, so the Previous/Next buttons' disabled state, the "N of M" slide labels, and a custom picker built on `use_carousel()` all stay correct after a drag too. See `dev-docs/research/carousel-2026-09-19.md` §6 for the full engineering rationale.

Set `draggable: false` on `CarouselContent` to opt a particular carousel out of the gesture entirely (it defaults to `true`).

Dragging past the first or last slide does not bounce back elastically -- a deliberate choice, not a gap: the disabled Previous/Next buttons already signal the boundary, elastic overscroll doesn't exist for a programmatic drag like this one, and it's a macOS/iOS compositor feature to begin with -- a native scroll-snap track doesn't rubber-band on Linux or Windows either, dragged or not.

## Direction / RTL

`Carousel` accepts a `dir: Option<Direction>` prop (defaulting to the nearest `DirectionProvider`, or LTR). Under RTL, the root's `ArrowLeft`/`ArrowRight` paging swaps: `ArrowLeft` moves to the *next* slide, `ArrowRight` to the *previous* one (matching Radix's shared `RovingFocusGroup` convention, the same one `Tabs` follows). Scrolling itself needs no such swap at all -- slide order in the DOM never changes, and the browser's own `scrollIntoView` already resolves the correct physical position under `dir="rtl"`. `CarouselPrevious`/`CarouselNext` also reposition correctly on their own (`inset-inline-start`/`-end`), and any chevron-style icon placed inside either one is automatically mirrored (`transform: scaleX(-1)`, horizontal orientation only) so a caller who uses the same icon regardless of direction still gets one pointing the right way. See the `rtl` variant.

Pointer drag mirrors the same way the keyboard does: dragging is direct manipulation (the track tracks the pointer), so the physical direction that reveals the next slide flips under RTL -- swipe-left-for-next in LTR, swipe-right-for-next in RTL, the same split a right-to-left photo gallery or story viewer already has.

## Looping

`Carousel { r#loop: true }` makes Previous/Next (and the root's own `ArrowLeft`/`ArrowRight`) wrap around at the ends -- **rewind-style**, not an embla-style seamless illusion: from the last slide, Next goes to the first (and vice versa for Previous), via the same `scrollIntoView` paging path every other transition already uses, so it visibly scrolls back across the intervening slides rather than teleporting. No cloned edge slides are ever added (they would show up in the "N of M" count and `:nth-child` styling). Dragging or wheeling past a physical edge still rubber-bands regardless of `loop` -- there is no wrap on a drag/wheel gesture, only on Previous/Next/the root keyboard. `CarouselPrevious`/`CarouselNext` are never `disabled` while `loop` is on. Defaults to `false`. See the `looping` variant.

## Autoplay / rotation control

The APG "auto-rotating" carousel style: add a `CarouselAutoplay` (no visible output -- it just drives the timer) alongside a `CarouselRotationControl` (a real, labeled `<button>`):

```rust
Carousel { aria_label: "Featured photos",
    CarouselRotationControl { /* an icon */ }   // FIRST -- must precede everything else focusable
    CarouselAutoplay { delay_ms: 4000u64 }
    CarouselPrevious { /* ... */ }
    CarouselNext { /* ... */ }
    CarouselContent { /* CarouselItems */ }
}
```

`CarouselRotationControl`'s accessible name toggles between `"Start automatic slide show"`/`"Stop automatic slide show"` -- deliberately no `aria-pressed`, matching APG's own contract that the changing label *is* the state. `CarouselContent` grows an `aria-live` attribute once a `CarouselAutoplay` is present: `"off"` while rotating, `"polite"` once stopped (absent entirely without autoplay, exactly like before this feature existed).

Rotation pauses while keyboard focus is anywhere inside the carousel, or while hovering it. Un-hovering resumes it (unless focus is *also* currently holding it paused); losing focus does **not** auto-resume -- only clicking `CarouselRotationControl` again does (matching the vendored tabbed reference's own accessibility-features prose). `prefers-reduced-motion: reduce` always forces rotation off at mount, checked once client-side, regardless of any `default_playing` you pass.

`CarouselAutoplayProps` mirrors `embla-carousel-autoplay`'s own options: `delay_ms` (default 4000), `stop_on_interaction` (default `true` -- Previous/Next/keyboard/a `CarouselTab`/a picker's `scroll_to` all stop rotation for good until the button is clicked again; a native pointer-drag or wheel scroll does **not** count as "interaction" in this v1), `stop_on_mouse_enter` (default `true`), `default_playing` (default `true`).

Ticking pages through the same path `CarouselNext` uses, so `loop` applies: without `loop`, rotation simply stops once it reaches the last slide (rather than ticking forever against a no-op the way `embla-carousel-autoplay`'s own documented behavior does); with `loop`, it rewinds and keeps going. See the `autoplay` variant.

## Tablist (dot-picker) variant

The APG "tabbed" carousel style: a `CarouselTabList` of `CarouselTab` pickers in place of (or alongside) `CarouselPrevious`/`CarouselNext`:

```rust
Carousel { aria_label: "Featured photos",
    CarouselTabList {
        for i in 0..count {
            CarouselTab { key: "{i}", index: i }
        }
    }
    CarouselContent { /* CarouselItems, same indices */ }
}
```

`CarouselTabList` is `role="tablist"`; each `CarouselTab` is `role="tab"` with a roving `tabindex`, `aria-selected`, and `aria-controls` pointing at its matching `CarouselItem` (which switches its own role from `group` to `tabpanel` once a `CarouselTabList` is present -- `aria-roledescription="slide"` stays either way). `ArrowLeft`/`ArrowRight` (RTL-aware)/`Home`/`End` move focus among tabs and **immediately** activate the newly-focused slide (no `Enter`/click needed -- APG's automatic-activation contract), and always wrap at the ends (independent of `Carousel`'s own `loop`, which governs Previous/Next/the root keyboard instead). See the `tabs` variant.

## Virtualised content

`CarouselVirtualContent` is a data-driven, virtualised drop-in for `CarouselContent` + `CarouselItem`s -- use it in place of them (never alongside them) when your slides come from a `Vec<T>` rather than a fixed set of children:

```rust
Carousel { aria_label: "Featured photos", r#loop: true,
    CarouselPrevious { /* ... */ }
    CarouselNext { /* ... */ }
    CarouselVirtualContent::<String> {
        items: my_items,           // ReadSignal<Vec<T>>
        radius: 2usize,            // default: slides kept mounted on each side
        virtualize: None,          // default: auto -- virtualise only once items.len() > 2*radius+1
        render_item: move |(index, item): (usize, String)| rsx! {
            div { "{item}" }
        },
    }
}
```

Everything else about `Carousel` -- `CarouselPrevious`/`CarouselNext`, a `CarouselTabList`/`CarouselTab` picker, `CarouselAutoplay`, the root's own arrow keys, `use_carousel()` -- works completely unchanged; none of them know whether they're driving a fixed set of `CarouselItem`s or `CarouselVirtualContent`.

**When to virtualise.** With `virtualize` left at its default (`None`), the DOM only ever holds `2 * radius + 1` slides once your data set is bigger than that window -- a smaller data set renders every slide (there's nothing to save). Pass `virtualize: Some(true)` to always window, even for a small data set, or `virtualize: Some(false)` to always render every slide regardless of size.

**Seamless looping.** With `r#loop: true` and virtualisation active, paging past the last item slides physically forward one slide at a time instead of rewinding visibly across every intervening one the way the `CarouselItem`-based `looping` variant does -- see the `virtual_loop` variant. With `r#loop: false` (or virtualisation inactive), looping is the same rewind style as the plain children API. See the `virtual_many` variant for the large-data-set case: 200 items, `loop: false`, DOM never holding more than 5.

**Trade-offs.** A virtualised window is a client-side enhancement layered on a fully compliant plain sequence, not a replacement for one: the server render (and a client's own pre-hydration first paint) always renders every item, in true order, with correct `"{n} of {m}"` labels -- so there is no SSR/no-JS gap. Once JS has mounted and virtualisation activates, though, slides outside the current window are not in the DOM at all, so a screen reader's browse-mode "read from here" and the browser's own find-in-page can only reach the currently-windowed slides, not the full data set -- the same cost every virtualised list (this crate's own `VirtualList` included) carries. If your data set is small enough that this doesn't matter, `virtualize: Some(false)` keeps every slide reachable at all times, at the cost of the DOM holding all of them.

## What v1 does not include yet

- **The APG "grouped" picker style** (plain buttons, one per slide, all in page Tab order, current one `aria-disabled`) -- the pattern page's own least keyboard-friendly of its three styles, and shadcn doesn't have it either.
