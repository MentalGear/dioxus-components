A slideshow of slides that the user pages through with Previous/Next buttons, the arrow keys, or by dragging or scrolling the track. It follows the [WAI-ARIA carousel pattern](https://www.w3.org/WAI/ARIA/apg/patterns/carousel/) and supports the pattern's three styles: prev/next buttons, an auto-rotating slideshow, and a dot picker.

Slides snap into place with CSS scroll snapping, so touch and trackpad scrolling are native. Everything below is opt-in; a carousel with only `CarouselPrevious`, `CarouselNext` and `CarouselContent` needs no configuration.

## Usage

```rust
Carousel { aria_label: "Featured photos",
    CarouselPrevious { ChevronLeft {} }
    CarouselNext { ChevronRight {} }
    CarouselContent {
        CarouselItem { index: 0usize, "Slide 1" }
        CarouselItem { index: 1usize, "Slide 2" }
        CarouselItem { index: 2usize, "Slide 3" }
    }
}
```

- `Carousel` is the root. Give it an accessible name with `aria_label` (or `aria-labelledby`) that does not contain the word "carousel"; the region already announces itself as one.
- `CarouselPrevious` and `CarouselNext` are buttons. Pass an icon or text as children. Their default accessible names are "Previous slide" and "Next slide". They are positioned with CSS, so place them before `CarouselContent` in the markup to keep them ahead of the slides in the Tab order.
- `CarouselContent` is the scrolling track. Its `draggable` prop (default `true`) turns mouse and pen dragging on or off, and its `gap` prop sets the space between slides (see Slide size and spacing).
- `CarouselItem` is one slide. `index` is 0-based and must be contiguous from 0. Unless you set your own `aria-label`, a slide is named "1 of 5", "2 of 5" and so on.
- `CarouselVirtualContent` replaces `CarouselContent` and its `CarouselItem`s when the slides come from a `Vec<T>`, rendering only a window of them (see Virtual content below).
- `CarouselIndicators` and `CarouselIndicator` add a row of dot pickers.
- `CarouselAutoplay` and `CarouselRotationControl` add auto-rotation and its start/stop button.

## Examples

Each example is shown live under Variants below.

- **Sizes.** Show two slides at once, or three on wider screens, by setting `--dx-carousel-per-view`.
- **Spacing.** Change the gap between slides with the `gap` prop.
- **Peek.** Show a sliver of the next slide with `--dx-carousel-peek`.
- **Align.** Rest each slide against the start, center or end of the track with `align`.
- **Vertical.** Page along the vertical axis with `orientation`; the track needs an explicit height.
- **RTL.** Right-to-left layout: the arrow keys and the buttons mirror.
- **API.** Read the selected slide and the slide count with `use_carousel()` to build a "Slide 2 of 5" label and a custom dot picker.
- **Indicators.** A row of tab-style dots, one per slide.
- **Autoplay.** Auto-rotation with a start/stop button.
- **Rewind.** `loop` with `loop_mode: Rewind` jumps from the last slide back to the first.
- **Virtual loop.** A seamless endless loop over a data set, with virtualisation.
- **Virtual loop (RTL).** The same loop in a right-to-left layout.
- **Virtual many.** 200 slides with only a handful in the DOM at any time.

## Options

### Carousel

- `orientation`: `CarouselOrientation::Horizontal` (default) or `Vertical`.
- `align`: `CarouselAlign::Start` (default), `Center` or `End`. It sets where a slide rests against the track, and only shows when there is leftover space, for example with `--dx-carousel-peek`.
- `r#loop`: whether Previous, Next and the arrow keys wrap around at the ends. Default `false`.
- `loop_mode`: how a loop wraps. `LoopMode::Seamless` (default) or `LoopMode::Rewind`. See Looping below.
- `default_value`: the initially selected slide (0-based) when uncontrolled.
- `value`: the selected slide when you control it yourself.
- `on_value_change`: called with the new index whenever the selected slide changes (see Events).
- `dir`: `Option<Direction>`. Defaults to the nearest `DirectionProvider`, or left-to-right.

### Slide size and spacing

The space between slides is the `gap` prop on `CarouselContent` (and on `CarouselVirtualContent`). The other two are CSS custom properties, not props: set them in a `style` attribute on `Carousel` or `CarouselContent`, or in your own stylesheet.

- `gap`: `Option<String>`, the space between slides as any CSS length or variable, for example `gap: "var(--dx-space-2)"` or `gap: "1.5rem"`. Leave it unset to keep the default (`var(--dx-space-4)`, 1rem). It works the same in both orientations, along the direction the carousel scrolls.

- `--dx-carousel-per-view`: how many whole slides are visible at once. Default `1`. Combine it with a media query to change the count by screen size.
- `--dx-carousel-peek`: a percentage of the track left over for a sliver of the next slide. Default `0%`.
- `--dx-carousel-gap`: the space between slides. Default `var(--dx-space-4)` (1rem). The `gap` prop sets this variable, so you can also set it yourself in a stylesheet; a `style` you pass to `CarouselContent` wins over the prop.

To size a slide yourself, set `flex-basis` on the `CarouselItem`; that wins over the per-view calculation.

shadcn has no `gap` prop. It spaces slides with utility classes: `-ml-N` on `CarouselContent` and `pl-N` on every `CarouselItem`, or `-mt-N` and `pt-N` for a vertical carousel. `gap` is a typed shortcut for the same padding-plus-negative-margin technique, so you do not repeat a class on every slide. It is not a flex `gap`: each slide keeps its exact share of the track, so `--dx-carousel-per-view` still counts whole slides.

### Looping

`r#loop: true` makes Previous, Next and the root arrow keys wrap around. What that looks like depends on the content:

- `CarouselVirtualContent` with virtualisation active loops seamlessly, always: paging past the last slide slides forward to the first without a visible rewind. `loop_mode` does not matter here.
- Everything else (plain `CarouselItem`s, or `CarouselVirtualContent` that is not windowed) has no seamless option. With `LoopMode::Seamless` (the default) `loop` does nothing there and the buttons stay disabled at the ends. Choose `LoopMode::Rewind` to opt in to wrapping: Next on the last slide jumps straight back to the first, and Previous on the first jumps to the last.

Dragging or scrolling past an end never wraps; only the buttons and the arrow keys do. A looping carousel, `Rewind` or seamless, also has no edge stretch (see Dragging and scrolling).

### Autoplay

```rust
Carousel { aria_label: "Featured photos",
    CarouselRotationControl { Play {} }   // first, so it is the first stop in the Tab order
    CarouselAutoplay { delay_ms: 4000u64 }
    CarouselPrevious { ChevronLeft {} }
    CarouselNext { ChevronRight {} }
    CarouselContent { /* CarouselItems */ }
}
```

`CarouselAutoplay` draws nothing; it drives the timer. `CarouselRotationControl` is a button whose accessible name toggles between "Start automatic slide show" and "Stop automatic slide show".

- `delay_ms`: time between slides. Default `4000`.
- `default_playing`: whether rotation starts on mount. Default `true`.
- `stop_on_mouse_enter`: pause while the pointer is over the carousel. Default `true`.
- `stop_on_interaction`: stop rotation for good after the user pages manually (Previous, Next, the arrow keys, a `CarouselIndicator`, or `scroll_to`) until they press the rotation button again. Default `true`. Dragging or scrolling the track does not count.

Rotation also pauses while keyboard focus is inside the carousel. When focus leaves it does not resume by itself; press the rotation button. Without `loop`, rotation stops at the last slide; with `loop`, it wraps and keeps going.

### Indicators

```rust
Carousel { aria_label: "Featured photos",
    CarouselIndicators {
        for i in 0..count {
            CarouselIndicator { key: "{i}", index: i }
        }
    }
    CarouselContent { /* CarouselItems with the same indices */ }
}
```

`CarouselIndicators` is a `tablist`, each `CarouselIndicator` a `tab` linked to its slide. The left and right arrow keys (mirrored in RTL), Home and End move between the dots and show the slide immediately. Focus wraps at the ends.

If you want your own picker design instead, build it with `use_carousel()` (see API).

### Virtual content

```rust
Carousel { aria_label: "Catalog", r#loop: true,
    CarouselPrevious { ChevronLeft {} }
    CarouselNext { ChevronRight {} }
    CarouselVirtualContent::<String> {
        items: my_items,   // ReadSignal<Vec<T>>
        render_item: move |(index, item): (usize, String)| rsx! { div { "{item}" } },
    }
}
```

- `items`: the full data set, in order.
- `render_item`: renders one slide from its index and value.
- `radius`: slides kept mounted on each side of the current one. Default `2`, so at most 5 slides are in the DOM.
- `virtualize`: `None` (default) windows the list only when `items.len() > 2 * radius + 1`; `Some(true)` always windows; `Some(false)` always renders every slide.
- `draggable`: same as on `CarouselContent`.

Everything else (buttons, indicators, autoplay, the arrow keys, `use_carousel()`) works the same as with plain items. The server render and the first client render always contain every slide in order; the window applies after mounting. Once windowed, slides outside it are not in the DOM, so find-in-page and a screen reader's browse mode can only reach the nearby ones. Pass `virtualize: Some(false)` if that matters more than DOM size.

## API

`use_carousel()` reads the nearest `Carousel` and returns a `CarouselApi`. Call it from a component rendered inside `Carousel`; it panics outside one.

- `selected`: the selected slide's index (0-based).
- `count`: the number of slides.
- `can_scroll_prev` and `can_scroll_next`: whether stepping in that direction would move anywhere. Always `true` while a loop wraps.
- `scroll_to(index)`: go to a slide directly. The index is clamped into range.

```rust
#[component]
fn SlideCounter() -> Element {
    let api = use_carousel();
    rsx! {
        p { "Slide {api.selected + 1} of {api.count}" }
        for i in 0..api.count {
            button {
                "aria-label": "Go to slide {i + 1}",
                "data-active": i == api.selected,
                onclick: move |_| api.scroll_to(i),
            }
        }
    }
}
```

## Events

`on_value_change` on `Carousel` receives the new 0-based index each time the selected slide settles on a different one. It fires for every source: Previous/Next, the arrow keys, `scroll_to`, an indicator, autoplay, and the user's own drag, wheel or touch scroll. Pair it with `value` (and optionally `default_value`) to control the selection yourself.

## Accessibility

- The root is a labelled region with `aria-roledescription="carousel"`; each slide has `aria-roledescription="slide"` and is named "n of m" by default. With `CarouselIndicators`, slides become tab panels.
- Previous and Next are real buttons and become genuinely `disabled` at the ends when the carousel does not loop.
- With focus inside the carousel, `ArrowLeft` and `ArrowRight` go to the previous and next slide (swapped in RTL). A vertical carousel uses `ArrowUp` and `ArrowDown` instead.
- With `CarouselIndicators`, the dots follow the tabs pattern: arrow keys, Home and End move focus and select immediately.
- With `CarouselAutoplay`, the track's `aria-live` is `off` while rotating and `polite` once stopped. Put `CarouselRotationControl` first so it is the first focusable element.
- `prefers-reduced-motion: reduce` keeps autoplay off at mount and disables the rubber-band effects described below.

## Dragging and scrolling

Pressing and dragging with a mouse or pen pages the carousel, and it settles smoothly on the nearest slide when released. A drag has to move a few pixels first, so a link or button inside a slide still works as a click. Touch always scrolls natively. In RTL, the direction you drag to reach the next slide is mirrored, like a right-to-left photo gallery. Set `draggable: false` on `CarouselContent` to turn the gesture off.

Dragging past the first or last slide stretches the track with increasing resistance and springs back when you let go. Slides never change from this.

With a wheel or trackpad, the browser scrolls the track natively. At the first or last slide, a push that starts while the carousel is already resting there adds the same brief elastic stretch, only along the carousel's own axis, and springs back when the input stops. A fling that arrives at the end with momentum is left to the browser, so two bounces never stack.

The stretch only happens at the real first or last slide, for mouse and pen dragging as well as for wheel and trackpad scrolling. It never happens on a looping carousel, whether `Rewind` or seamless, because a loop has no ends. A carousel using `CarouselVirtualContent` only stretches while its real first or last slide is actually on screen, not at the edge of the slides currently rendered. The stretch is not drawn under reduced motion.

Where an end of the track is not a real end of your data (any looping carousel, or a `CarouselVirtualContent` partway through a long list), the carousel also turns off the browser's own overscroll bounce along its scroll axis (`overscroll-behavior: none`). A fast flick that reaches the edge of the slides currently rendered then simply stops there instead of bouncing as if the list had ended, and the rendered slides re-centre once you let go. A plain, non-looping carousel, or a virtual list short enough to render in full, keeps the browser's default. While a virtual list sits near one real end, that end gives up the browser's own bounce too (the carousel's own stretch still works there), and a vertical one no longer passes the scroll on to the page at that end.

## Debugging

To record wheel gestures for a bug report, run `localStorage.setItem("dx-carousel-debug", "1")` in the browser console and reload. Each gesture is then logged to `window.__dxCarouselWheel` (the last 50). Nothing is recorded without the flag.

## Known limitations

- Safari, and possibly Firefox on some platforms, may draw their own edge bounce on a gesture that starts at rest, which could stack with the carousel's stretch into a double bounce. This is pending a check on real devices.
- The pattern's "grouped" picker style (a plain button per slide, all in the Tab order) is not provided.

<!-- For maintainers: design rationale, the wheel-band tuning constants and other engineering notes moved to dev-docs/research/carousel-engineering-notes.md -->
