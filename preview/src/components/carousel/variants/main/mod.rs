use super::super::component::*;
use dioxus::prelude::*;
use dioxus_icons::lucide::{ChevronLeft, ChevronRight};

/// Slide 1's placeholder is a real `<button>` (the rest stay plain `div`s,
/// matching every other variant) rather than a second demo variant, so
/// this one page can prove both pointer-drag halves at once: dragging it
/// pages the carousel like any other point on the track, but a plain
/// click still activates the button -- see `playwright/carousel.spec.ts`'s
/// own drag describe block, which drives exactly this element.
#[component]
pub fn Demo() -> Element {
    let mut slide_one_clicked = use_signal(|| false);
    rsx! {
        // `width: 100%` is load-bearing next to `max-width`, not redundant.
        // `.dx-component-preview-frame` is a flex column, so its cross axis is
        // horizontal; this wrapper's own `margin: 0 auto` makes both cross-axis
        // margins `auto`, and per flexbox those win over `align-self: stretch`.
        // With cross-size left `auto` the wrapper shrink-wraps its content
        // instead of reaching `max-width` -- measured at 176px against an
        // intended 320px, i.e. shadcn's own card width. An explicit `width`
        // resolves the cross size before auto margins apply, so the wrapper
        // fills the frame and `max-width` clamps it as intended. Setting
        // `align-items: stretch` on the frame does NOT fix this (auto margins
        // still win) and would widen ~80 other demos that have no `max-width`;
        // both measured, see `dev-docs/backlog.md` row 94.
        div { style: "width: 100%; max-width: 20rem; margin: 0 auto;",
            Carousel { aria_label: "Featured photos",
                CarouselPrevious { ChevronLeft {} }
                CarouselNext { ChevronRight {} }
                CarouselContent {
                    for i in 0..5usize {
                        CarouselItem { key: "{i}", index: i,
                            if i == 0 {
                                button {
                                    r#type: "button",
                                    "data-testid": "carousel-slide-button",
                                    "data-clicked": slide_one_clicked(),
                                    // `font: inherit` MUST precede `font-size: 2rem`, not follow
                                    // it -- this is the root cause of the "slide 1's font-size is
                                    // different" bug, not a UA-stylesheet mystery. `font` is a
                                    // shorthand covering font-style/variant/weight/stretch/
                                    // *font-size*/line-height/family; within one `style`
                                    // attribute the later declaration always wins for an
                                    // overlapping property, same specificity or not. The
                                    // previous ordering put `font: inherit` LAST, so it silently
                                    // clobbered the explicit `font-size: 2rem` back to
                                    // `inherit` -- and a `<button>` doesn't already inherit
                                    // font-size from its ancestors the way a `<div>` does (every
                                    // mainstream UA stylesheet gives form controls their own
                                    // non-inheriting font), so "inherit" resolved all the way up
                                    // to the page's own base size (16px) instead of this slide's
                                    // intended 32px. Measured live: slide 0 (button) computed
                                    // 16px, slides 1-4 (plain divs, which never touch `font` at
                                    // all and so keep ordinary CSS inheritance) computed 32px.
                                    // With `font: inherit` first, family/weight/style/variant/
                                    // line-height still resolve to inherit -- matching the divs,
                                    // which get those the same way, by never setting them -- and
                                    // the explicit `font-size: 2rem` that follows is the
                                    // declaration that actually wins, matching the divs'
                                    // explicit value instead of clobbering it.
                                    //
                                    // `padding: 0` guards the box model the same way: browsers
                                    // give `<button>` non-zero default padding (a `<div>` has
                                    // none), which would otherwise make this element render
                                    // taller than its sibling divs despite the identical explicit
                                    // `height`/`border` below.
                                    //
                                    // The remaining gap was in exactly how that identical `height`
                                    // and `width: 100%` resolve, found only by comparing the two
                                    // elements' own `getBoundingClientRect()` side by side after the
                                    // fixes above: even with matching font/padding/border, the
                                    // button still rendered 192x320 against the div's 194x320.
                                    // Cause: Chromium's UA stylesheet defaults `<button>` (a form
                                    // control) to `box-sizing: border-box` (confirmed via
                                    // `getComputedStyle(el).boxSizing`), unlike a plain `<div>`,
                                    // which defaults to `content-box` -- so the *same* declared
                                    // `height: 12rem` means two different things: for the div
                                    // (content-box) it is the content height, with the 1px+1px
                                    // border added on top (192 + 2 = 194px total); for the button
                                    // (border-box) it is already the *total* height, with the
                                    // border carved out of it (192px total, unchanged). Two
                                    // approaches were tried and measured before this one: forcing
                                    // `box-sizing: content-box` on the button matches the height
                                    // (194 = 194) but then reopens the identical gap on `width` --
                                    // `width: 100%` under content-box is *also* the content size,
                                    // so the border adds on top there too (322px against the div's
                                    // 320px); removing `width: 100%` entirely does not help either
                                    // -- a `<button>` does not take on a div's own "auto width fills
                                    // the row" block-layout behavior just because `display: flex` is
                                    // set on it, measured collapsing to its own shrink-to-fit content
                                    // width (~14px) instead. Since `box-sizing` cannot be set
                                    // per-axis, matching both at once means keeping the button's own
                                    // native `border-box` (stated explicitly here, not left as an
                                    // unstated UA default) and asking for the *total* height it
                                    // should have directly: `calc(12rem + 2px)`, the same 194px the
                                    // div's own content-box interpretation of `height: 12rem` plus
                                    // its border already produces -- while `width: 100%` under
                                    // border-box already means "the row's own full width" with no
                                    // adjustment needed, matching the div exactly (both already
                                    // measured 320px, before any box-sizing changes at all).
                                    style: "display: flex; align-items: center; justify-content: center; width: 100%; height: calc(12rem + 2px); border: 1px solid var(--primary-color-6); border-radius: var(--dx-radius-lg); background: none; padding: 0; margin: 0; box-sizing: border-box; cursor: pointer; font: inherit; font-size: 2rem; color: inherit;",
                                    onclick: move |_| slide_one_clicked.set(true),
                                    if slide_one_clicked() { "Clicked!" } else { "{i + 1}" }
                                }
                            } else {
                                div {
                                    style: "display: flex; align-items: center; justify-content: center; height: 12rem; border: 1px solid var(--primary-color-6); border-radius: var(--dx-radius-lg); font-size: 2rem;",
                                    "{i + 1}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
