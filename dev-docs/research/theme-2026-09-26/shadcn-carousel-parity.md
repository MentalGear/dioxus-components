<!-- Research notes written 2026-09-26 by a read-only research lane. Point-in-time; shadcn-ui/ui@98a1fe6. -->

# shadcn `base` carousel parity research

Source: `shadcn-ui/ui` @ `98a1fe67b439324ddc857f47fbdce056600a4329` (2026-09-21), cloned
into `scratchpad/research/shadcn-ui`. Docs page: `apps/v4/content/docs/components/base/carousel.mdx`
(`base: base` frontmatter → style `base-nova`, i.e. `registry/styles/style-nova.css` +
`registry/bases/base/ui/{carousel,card,button}.tsx`). WebFetch of the live page confirms section
order: Sizes → Spacing → Orientation → Options → API → Events → Plugins → RTL, and that the first
(top) demo shows 5 numbered slides, one whole slide visible at a time, styled as a bordered/shadowed
card.

**Important context finding:** our `style.css`'s own comments (lines 71-77, 87-93) say the 28px
button size / `box-shadow: none` were "measured live" against
`ui.shadcn.com/docs/components/carousel` — the **old, un-prefixed URL**. That page no longer exists
in this checkout; the site now splits into `base`/`aria`/`radix` variants
(`apps/v4/content/docs/components/{base,aria,radix}/carousel.mdx`), each with its own visual
skin (`cn-card`, `cn-carousel-*` CSS classes for `base`; Tailwind utility classes for `radix`/
`new-york-v4`). Our component was evidently built/measured against a different, likely
now-superseded page than the one the owner is comparing against today. The button geometry
(28×28, no shadow, `rounded-full`) still happens to roughly match `base`'s `icon-sm` + `rounded-full`
override (`style-nova.css:269-275`), so that part survived by coincidence; the **slide styling and
gap model did not carry over at all** (see below) — plausibly because the old reference page never
modeled slides as `Card`s the same way, or that part of the port was never revisited after the
site restructured.

## shadcn `base` truth (per example)

All examples live in `apps/v4/examples/base/carousel-*.tsx`, rendered under `styles/base-nova/ui/*`
(built from `registry/bases/base/ui/*` + `registry/styles/style-nova.css`).

- **carousel.tsx** (`registry/bases/base/ui/carousel.tsx`):
  - `CarouselContent` (L135-154): outer `overflow-hidden` div wraps inner `flex` div,
    `-ml-4` horizontal / `-mt-4 flex-col` vertical.
  - `CarouselItem` (L156-172): `min-w-0 shrink-0 grow-0 basis-full`, `pl-4` horizontal / `pt-4`
    vertical.
  - `CarouselPrevious`/`Next` (L174-246): `Button variant="outline" size="icon-sm"`, absolutely
    positioned `-left-12`/`-right-12` (horizontal) or `-top-12`/`-bottom-12` rotated 90° (vertical),
    class `cn-carousel-previous`/`-next` (→ `rounded-full`, `style-nova.css:269-275`), `disabled`
    bound to `canScrollPrev`/`canScrollNext`.
  - Root: `role="region" aria-roledescription="carousel"`, `onKeyDownCapture` for `ArrowLeft`/
    `ArrowRight` (ArrowUp/Down not swapped for vertical — only horizontal keys are wired at all).
  - `useCarousel` context exposes `canScrollPrev`/`canScrollNext`, `scrollPrev`/`scrollNext`,
    `opts`, `orientation`. No `setApi`/RTL/plugin logic differs from other bases.

- **Card look** (`registry/bases/base/ui/card.tsx` + `style-nova.css:244-266`): `.cn-card` =
  `ring-foreground/10 ring-1`, `bg-card`, `rounded-xl`, `py-4` (`--card-spacing`), `text-sm`,
  `overflow-hidden`. `.cn-card-content` = `px-(--card-spacing)`. Every demo's slide is
  `<Card><CardContent className="flex aspect-square items-center justify-center p-6">`, i.e.
  a rounded, ring-bordered, `aspect-square` box with `p-6` (24px) padding, centered oversized
  number (`text-4xl font-semibold`, `text-3xl` in the multi-item demos, `text-2xl` in spacing demo),
  wrapped in a `div.p-1` (4px) gutter — that gutter, not the card border, is what visually
  separates cards, since the actual inter-item gap is 0 beyond the `-ml-4`/`pl-4` compensation.

- **carousel-demo.tsx** (top-of-page, "Sizes" is the *next* section but this demo has no
  container/size classes beyond the outer `Carousel`): `className="w-full max-w-[12rem] sm:max-w-xs"`,
  default `basis-full` (unmodified) → **exactly one whole card per view**, 5 slides,
  `CarouselPrevious`/`Next` shown unconditionally.

- **carousel-size.tsx** ("Sizes"): `opts={{align:"start"}}`, container
  `w-full max-w-[12rem] sm:max-w-xs md:max-w-sm`, item `className="basis-1/2 lg:basis-1/3"` →
  2 whole cards at `sm`, 3 whole cards at `lg`. No custom gap (`pl-4`/`-ml-4` default stays).

- **carousel-spacing.tsx** ("Spacing"): same container as size; `CarouselContent
  className="-ml-1"`, item `className="basis-1/2 pl-1 lg:basis-1/3"` → gap shrunk from the 16px
  default (`ml-4`/`pl-4`) to 4px (`ml-1`/`pl-1`) uniformly (not responsive-stepped in this example,
  unlike the doc prose's second code sample which shows `-ml-2 md:-ml-4` / `pl-2 md:pl-4`).

- **carousel-orientation.tsx** ("Orientation"): `orientation="vertical"`, `opts={{align:"start"}}`,
  container `max-w-xs`, `CarouselContent className="-mt-1 h-[270px]"`, item
  `className="basis-1/2 pt-1"` (no `aspect-square` here — plain `p-6` centered content, height
  comes from `CarouselContent`'s own explicit `h-[270px]`, split across 2 whole items).

- **carousel-api.tsx** ("API"): wrapped in `div.mx-auto.max-w-[10rem] sm:max-w-xs`,
  `Carousel setApi={setApi}` unmodified basis (1 whole slide/view), counter text
  `"Slide {current} of {count}"` in a `py-2 text-center text-sm text-muted-foreground` div below
  the carousel — a real DOM element, not aria-only.

- **carousel-plugin.tsx** ("Plugins"): `Autoplay({ delay: 2000, stopOnInteraction: true })`,
  `onMouseEnter={plugin.current.stop}` / `onMouseLeave={plugin.current.reset}` (hover
  stops/resets, distinct from our `stop_on_mouse_enter` which — per our docs.md — pauses and
  auto-resumes on un-hover unless focus also holds it; shadcn's plugin fully **stops** the
  Autoplay instance on hover and **resets** its timer on leave, not pause/resume). No
  `stopOnInteraction` equivalent shown as re-startable in the UI — once stopped by interaction it
  requires no explicit restart control (`base` has no rotation-control button at all, unlike our
  `CarouselRotationControl`/APG auto-rotating pattern — that's an accessibility feature we have
  that shadcn's plugin demo doesn't).

- **carousel-rtl.tsx** ("RTL"): `Carousel dir={dir} opts={{direction: dir}}`, container
  `max-w-[12rem] sm:max-w-xs`, default basis (1 whole slide), Arabic-numeral formatting is
  demo-specific (irrelevant to us), `CarouselPrevious`/`Next` get `rtl:rotate-180`... actually in
  `base`'s own `carousel.tsx` the RTL mirroring is baked into the icon via `cn-rtl-flip` (not a
  className passed at the call site in this specific `base` example) — matches our approach
  (mirroring lives in the component's own CSS, not caller-supplied classes).

- **carousel-multiple.tsx**: exists in `examples/base/` but is **not referenced by the `base`
  docs mdx at all** (only `carousel-size.tsx` is, which is functionally identical: `sm:basis-1/2
  lg:basis-1/3`, `align:start`, container `mx-auto max-w-xs sm:max-w-sm`, Previous/Next
  `hidden sm:inline-flex`). Confirms there is no separate "peek" example in shadcn `base` at all —
  "Sizes" *is* their multi-slide demo, and it always shows whole slides.

## The 2½-slides root cause (precise)

Our `multiple` variant (`preview/src/components/carousel/variants/multiple/mod.rs:16-27`): wrapper
`max-width: 34rem` (544px), `Carousel` reserves `padding-inline: var(--dx-space-12)` (48px each
side, `style.css:205-207`) for Previous/Next, leaving a content box of `544 - 96 = 448px`.
`CarouselItem` basis is overridden inline to `flex-basis: 40%` (not a prop — "a CSS decision, not a
prop" per `docs.md`'s "Sizing slides" section), and `.dx-carousel-content` sets `gap: var(--dx-space-4)`
(16px) as a real flex `gap` (`style.css:24-25`), not shadcn's margin/padding compensation.

Flex `gap` is **additive** to percentage `flex-basis` — it is not subtracted from the container's
distributed main size the way shadcn's negative-margin model implicitly is. With N=3 basis-40%
items and 2 gaps inside a 448px content box: `3 × 0.40 × 448 + 2 × 16 = 537.6 + 32 = 569.6px` of
demanded main size against 448px available — i.e. massive overconstraint, resolved by the flex
items simply overflowing/scrolling; the *visible* portion at any scroll position is: full item 1
(179.2px) + full item 2 (179.2px) + gap (16px) + gap (16px) + partial item 3 = only
`448 - 179.2 - 16 - 179.2 - 16 = 57.6px` of item 3's 179.2px width visible ≈ **32% of a third
slide**, i.e. **~2.3 slides total** — this is the arithmetic behind the owner's "2½ slides"
observation.

shadcn avoids this by construction: `CarouselContent` has `-ml-4`, every `CarouselItem` has `pl-4`
**inside its own border-box** and a plain `basis-1/2`/`basis-1/3` (exact simple fractions). Because
the gap lives inside each item's own box (as `padding-left`) rather than as inter-item flex `gap`,
N items' basis fractions still sum to exactly 100% of the (slightly widened, `-ml-4`-compensated)
content box — so `n = 1/basis` always divides evenly and whole slides always fit, with no
calc()/gap arithmetic required at the call site. Two independent causes compound in ours:
(a) `40%` is not `1/n` for any small whole `n` (an intentional "peek" demo, per the variant's own
doc comment), and (b) even a proper `1/n` basis would still overflow under our real `gap`
property unless the item's basis is computed as `calc(1/n × 100% - gap × (n-1)/n)`.

## Grouped change list

**(a) Slide visual style** — effort S/M each:
1. No Card look at all: ours is a plain `border: 1px solid`/`font-size` div (`variants/main/mod.rs:118-122`);
   shadcn wraps every slide in a `Card`/`CardContent` — `rounded-xl`, `ring-1 ring-foreground/10`,
   `aspect-square`, `p-6` padding, centered oversized number, `p-1` outer gutter
   (`registry/bases/base/ui/card.tsx`, `style-nova.css:244-266`, all `examples/base/carousel-*.tsx`).
   → give our demos (not the primitive) a themed slide look using our own tokens
   (`--dx-radius-xl`/`--dx-shadow`/`var(--dx-space-6)` padding) — a demo-only CSS class, e.g.
   `.dx-demo-carousel-card`, aspect-square + centered content. **M** (new demo CSS + touch every
   variant's slide markup).
2. Typography sizes differ by demo (4xl/3xl/2xl) matching item density — ours is a flat `2rem`/
   `1.5rem` ad hoc per variant, not systematized. **S**, fold into (1)'s new class with a density
   modifier.

**(b) Spacing/gap options** — effort M:
3. shadcn exposes gap as a compositional choice at the call site (`-ml-1/pl-1` … `-ml-4/pl-4`,
   docs.md's own "Spacing" prose, `carousel-spacing.tsx`); we hardcode `gap: var(--dx-space-4)`
   on `.dx-carousel-content` (`style.css:24-25`) with **no exposed variable or prop**. Introduce a
   `--dx-carousel-gap` custom property on `.dx-carousel-content` (default `var(--dx-space-4)`,
   override per-instance via inline style or a themed `gap` prop on the wrapper's `CarouselContent`)
   — keeps the plain-CSS/no-Tailwind convention (row 32) intact. **M**.
4. Any exposed gap must be **basis-aware** per the root-cause above — document (and ideally
   provide a calc() helper/example in docs.md) `flex-basis: calc(100%/n - var(--dx-carousel-gap) *
   (n-1)/n)` for an n-up layout, not a bare percentage. **S** (docs) + **M** (if we add a helper
   class per common n).

**(c) Sizes/multiple — whole slides by default** — effort M:
5. Rename/rebuild `multiple` to mirror shadcn's `carousel-size.tsx` exactly: `align:start`-equivalent
   (our primitive has no `align` option — check `primitives/src/carousel.rs` for `align`; if absent,
   note as a **primitive gap**, effort **M**, since Embla's `align:"start"` vs default `"center"`
   changes scroll-snap resting position, not just visuals) and exact fractions (`basis: 50%`/`33%`
   with the calc() from (4)) so N whole slides show, no overflow slice. **M**.
6. Add the "peek" behavior as an explicit, separate, opt-in feature — CSS variables
   `--dx-carousel-per-view` (integer) and `--dx-carousel-peek` (0–1 fraction of the next slide to
   show), with `flex-basis: calc((100% - var(--dx-carousel-peek) * (100%/var(--dx-carousel-per-view)))
   / var(--dx-carousel-per-view) - var(--dx-carousel-gap) * (var(--dx-carousel-per-view) - 1) /
   var(--dx-carousel-per-view))` (sketch — needs verification against a live layout, not shipped
   as-is) so a peek demo states its intent instead of an unlabeled "40%" magic number. **M**.

**(d) Per-demo parity edits** — effort S each, all in `preview/src/components/carousel/variants/*/mod.rs`:
7. Container widths: shadcn's are all `max-w-[12rem] sm:max-w-xs` (responsive, narrower on mobile)
   vs our fixed `max-width: 26rem`/`34rem`. Adopt the responsive clamp pattern.
8. Orientation demo: shadcn uses `h-[270px]` fixed content height + `basis-1/2` (2 whole items,
   no `aspect-square`); ours (`variants/vertical/mod.rs`, not yet inspected in depth this pass —
   flag for the implementation lane) should be checked against this exact height/basis pair.
9. API demo: shadcn's counter is a real "Slide x of y" div under the carousel; confirm our
   `indicators`/any API-demo equivalent renders the same visible string, not just an ARIA-only one.
10. Plugin demo: shadcn's Autoplay is `delay: 2000, stopOnInteraction: true`, hover **stop/reset**
    (not pause/resume). Our `CarouselAutoplayProps` (docs.md "Autoplay") defaults `delay_ms: 4000`,
    `stop_on_mouse_enter: true` with pause/resume-on-unhover semantics — a deliberate divergence
    (documented, matches "vendored tabbed reference"), not a bug; flag for the owner to confirm
    keep-as-is vs match shadcn's stop/reset + 2000ms.

**(e) Our extra demos — keep/rename/retire** (owner direction per task):
11. `tabs` → rename to **"Indicators"** display name in the preview nav/docs (component itself
    already correctly named `CarouselTabList`/`CarouselTab`, APG "tabbed" pattern) — note we
    *already have a separate* `indicators` variant (custom dot picker built on `use_carousel()`,
    `component.rs`'s `CarouselIndicators`) distinct from `tabs` (APG tablist semantics) — these are
    two different things today; confirm with owner which one the "tabs → Indicators" rename
    targets, since renaming `tabs` to "Indicators" would collide with the existing `indicators`
    variant's display name. **S** (naming decision, not code).
12. Retire `looping` and `looping_rtl` (rewind-style loop demos) in favor of `virtual_loop`
    (seamless-loop path) per owner's stated direction — check nothing else (docs.md's "Looping"
    section prose, other demos' cross-links) still points at the retired variants before deleting.
    **S**.

**(f) `carousel.tsx` behaviour differences**:
13. `canScrollPrev`/`canScrollNext`-driven `disabled` — we appear to already do this (style.css:148-152
    `:disabled` styling exists) — confirm the primitive actually sets the HTML `disabled` attribute
    (not just a data-attribute) to match; not verified this pass, flag for implementation lane.
14. Keyboard handling scope: shadcn's `onKeyDownCapture` is on the **root** `Carousel` div (captures
    ArrowLeft/Right from anywhere inside, horizontal only — no vertical Up/Down swap in `base`'s
    own carousel.tsx despite orientation support existing). Our docs.md says root handles
    ArrowLeft/Right (horizontal) or ArrowUp/Down (vertical) — **we support vertical arrow keys,
    shadcn's `base` example does not** — a place we exceed shadcn, not a gap; keep.
15. `opts`/`setApi` — Embla's generic options bag (`align`, `loop`, `direction`, etc.) vs our
    typed prop-per-option (`r#loop`, `dir`, no `align`). Structural difference (Rust idioms vs a
    JS options object) — not a parity gap per se, except item 5's `align` question above.

## Effort legend
S = small (CSS/demo-only, <1 file focus), M = medium (new variable/prop + doc + multi-variant touch).
None of the above were rated L; nothing here requires new primitive architecture beyond possibly
an `align` option (item 5), which needs the primitive's current Embla-equivalent scroll-snap
implementation checked before committing to effort size.
