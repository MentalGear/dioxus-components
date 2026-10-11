# Motion

How animation works in shadcn-dioxus: the tokens that set its speed, what happens for users who ask for less motion, why a page full of live demos stays cheap, and how to add your own animated component without re-inventing any of it. The same content is on the site's [`/docs`](https://mentalgear.github.io/shadcn-dioxus/docs/#motion) page, under "Motion"; the per-component table below is the same data in both places (a unit test in `preview/src/main.rs`, `motion_table_matches_motion_md`, fails if they drift).

Short version:

- Speed comes from `--dx-motion-duration-*` and `--dx-motion-ease*` tokens in `dx-components-theme.css`. Change the token, change every component that reads it.
- `prefers-reduced-motion: reduce` is answered by the stylesheets, not by Rust.
- An animation that runs forever may only animate compositor properties (`scripts/check-infinite-animations.sh`).
- Motion nobody can see is not run: loops pause, timers stop, entrances wait for the element to be seen (`primitives/src/activity.rs`, `scripts/check-motion-gating.sh`).
- Overlay `translate`/`scale` keyframes, never `transform` (`scripts/check-anchored-keyframes.sh`).

## Motion tokens

Defined once on `:root` in `preview/assets/dx-components-theme.css` (the "Motion" block). Components read these and carry no fallback literals (`scripts/check-css-vars-defined.sh`), so retuning a token retunes every component that uses it.

| Token | Default | Used for |
|---|---|---|
| `--dx-motion-duration-fast` | `100ms` | hover and highlight colour changes, hover-card fade, menu item backgrounds |
| `--dx-motion-duration-base` | `150ms` | menu and picker fades, chevron rotation, tab colours, drawer drag settle |
| `--dx-motion-duration-slow` | `200ms` | tooltip, toast, navigation slides, carousel arrow fade, the overlay default below |
| `--dx-motion-duration-slower` | `300ms` | accordion open/close, sheet close |
| `--dx-motion-duration-reduced` | `0.01ms` | the `prefers-reduced-motion` replacement duration (not `0ms`: a zero-duration transition can still misfire `transitionend` listeners in some engines) |
| `--dx-motion-ease` | `ease` | the default curve |
| `--dx-motion-ease-in` | `ease-in` | exits |
| `--dx-motion-ease-out` | `ease-out` | entrances |
| `--dx-motion-ease-standard` | `cubic-bezier(0.4, 0, 0.2, 1)` | icon rotation (chevrons) |

Modal surfaces share one more set, so a scrim and its panel cannot disagree: `--dx-overlay-duration` (defaults to `var(--dx-motion-duration-slow)`), `--dx-overlay-ease` (defaults to `var(--dx-motion-ease)`), `--dx-overlay-scrim` (`rgb(0 0 0 / 10%)`) and `--dx-overlay-blur` (`4px`). Dialog, AlertDialog, Sheet, Drawer, CommandDialog and the modal Popover read them.

**Changing them.** Override the token after the theme stylesheet loads, on `:root` for everything or on any ancestor for a subtree:

```css
:root {
  --dx-motion-duration-slow: 300ms;   /* tooltips, toasts, nav slides and (through the default) every overlay */
  --dx-overlay-duration: 120ms;       /* or pin the overlays on their own */
  --dx-motion-ease-out: cubic-bezier(0.16, 1, 0.3, 1);
}
.settings-panel {
  --dx-motion-duration-base: 0.01ms; /* a calmer subtree */
}
```

Not everything is a token yet, said plainly: a few durations are literals in the component sheets (the chart's 500/600/1500 ms load animations, the sheet's 500 ms entrance, the progress bar's 250 ms width transition, the message scroller's 0.4 s hide). They are listed in the table's last column; override them in the sheet you own after `dx components add` copies it.

## Reduced motion

`prefers-reduced-motion` is a different question from "is anybody looking" (below), and the stylesheets answer it:

- **The shared layer.** `dx-components-theme.css` sets `animation-duration` and `transition-duration` to `var(--dx-motion-duration-reduced)` with `!important` under `@media (prefers-reduced-motion: reduce)` for every interactive root (`button`, `a[href]`, `input`, `select`, `textarea`, `summary`, `[tabindex]`, the ARIA widget roles: `menuitem`, `option`, `tab`, `switch`, `checkbox`, `radio`, `slider`, and `[data-dx-interactive]`). It is a flat tag/role list on purpose: shrinking a duration cannot clash with a component's own construction.
- **Loops, by construction.** Every component that loops calls `use_motion` (the visibility gate below), which marks its element with `data-dx-motion-key`. ONE rule in `dx-components-theme.css` sets `animation: none !important` on every `[data-dx-motion-key]` host and everything under it, `::before` and `::after` included, under `prefers-reduced-motion: reduce`. So the spinner, skeleton, indeterminate progress bar and loading avatar stop, and the next looping component is covered by calling `use_motion`. It is `!important` so it wins over a component's `animation:` shorthand whatever the stylesheet order, and it stops rather than shortens: an infinite loop of `--dx-motion-duration-reduced` (0.01 ms) does not stop, it samples a random phase every frame and flickers.
- **"Loading" stays visible (WCAG 2.2: 2.2.2, 2.3.3; essential state).** The stopped element keeps its shape: a still ring for the spinner, a still block for the skeleton and the avatar placeholder, and `role="status"` / `aria-busy` still announce it. The indeterminate progress bar is the one whose still frame would mislead (a half-width bar reads as 50%), so its own sheet draws diagonal stripes across the whole track under reduced motion.
- **Loops outside the convention** (the OTP caret, the `.dx-shimmer` utility) carry their own `@media (prefers-reduced-motion: reduce)` block (`animation: none`).
- **The gate.** `scripts/check-infinite-animations.sh` fails an `infinite` animation that is neither inside a `no-preference` media query, nor stopped by a rule of its own under `reduced` (same selector, `animation: none` or a finite replacement; a duration-only override is rejected), nor in a component sheet whose component calls `use_motion` while the theme carries the `[data-dx-motion-key]` rule. `playwright/oracle/tier2-html/motion-gating.spec.ts` emulates the preference and asserts `animation-name: none` on a spinner, skeleton, progress bar and avatar (backlog row 176).
- **What the shared interactive layer does not reach.** Animated content that is not an interactive root and not a loop: a dialog's entrance, an accordion panel. Those components carry their own `@media (prefers-reduced-motion: reduce)` block (dialog, alert_dialog, sheet, drawer, command, popover, tooltip, hover_card, navbar, navigation_menu, message_scroller, drag_and_drop_list, the carousel arrows).
- **Entrances that do not play at all.** Chart load animations are wrapped in `prefers-reduced-motion: no-preference`; carousel autoplay stays off at mount; `.dx-shimmer` removes its gradient.
- **Your own CSS.** Wrap new animations in `@media (prefers-reduced-motion: no-preference)` or give them a reduced block that uses `var(--dx-motion-duration-reduced)`.

## Compositor-only infinite animations

An animation that runs forever is main-thread work on every vsync for as long as its element exists, unless the browser can hand it to the compositor. Only `transform`, `translate`, `scale`, `rotate` and `opacity` are eligible; anything else (`background-position`, `width`, `color`, `clip-path`, `box-shadow`, a registered custom property) re-runs style and usually paint, 60 times a second per instance. The 2026-10-10 scroll re-profile measured it: the home page's skeletons, spinners and text shimmer kept the main thread awake through every scroll, 630-730 "Animation" style recalcs per scroll for the skeletons alone, and disabling the infinite animations took 15% off the scroll busy time (backlog row 174).

`scripts/check-infinite-animations.sh` makes the class inexpressible: for every `*.css` under `preview/src` and `preview/assets`, a rule whose `animation` or `animation-iteration-count` says `infinite` must name `@keyframes` that only touch those five properties, or the (stylesheet, keyframes) pair must be in the script's `ALLOW` list with a reason. A stale `ALLOW` entry fails the run. Finite animations are bounded, so they are not checked.

**The one exception: the shimmer.** `dx-shimmer-sweep` in `dx-effects.css` animates `background-position`. The highlight is painted inside the glyphs with `background-clip: text`, so the only thing that can move it is the background itself; a translated pseudo-element would move the glyphs with it. It is paint-only (no style cascade or layout), opt-in per element, switched off by `prefers-reduced-motion`, and paused when unseen like every other loop (next section). The gate carries that reasoning in its `ALLOW` entry.

## Visibility gating

A loop or a self-driven timer costs work whether or not anyone is looking at it. Pausing them one by one at each call site is the same mistake as the uncleared `setInterval`: the next animation added forgets to. So "is this visible" is answered once, in `primitives/src/activity.rs`, and components subscribe to the answer. One `IntersectionObserver` per kind is shared by the whole page, plus `visibilitychange` (the tab) and `contentvisibilityautostatechange` (a `content-visibility: auto` subtree the browser is skipping). The reference-counted registry (`window.__dxActivity`) installs its listeners on the first subscriber and removes them with the last. Before the observer reports, and on the server, a motion is active, so there is no hydration mismatch.

The reduced-motion preference is deliberately not part of this signal.

| Hook | Use it for | Behaviour |
|---|---|---|
| `use_motion()` | a CSS loop on an element the component renders | Returns a `Motion`. Spread `motion.attributes()` on the element. While it is outside the viewport plus `ROOT_MARGIN` (`200px` pre-roll, so it is already moving when it scrolls in), inside a skipped `content-visibility` subtree, or in a hidden tab, the element carries `data-dx-motion="paused"`. |
| `use_motion_when(enabled)` | a loop that only sometimes exists (indeterminate progress, a still-loading avatar, a shimmering marker) | Same, but subscribes nothing while `enabled()` is false. |
| `use_motion_active_for(element_id)` | a timer beside an element the component does not own (the carousel's autoplay watches its scroller) | Returns a `ReadSignal<bool>` for the element whose DOM id is given. |
| `use_interval_while(active, period, on_tick)` | a repeating timer | Drops its task while `active` is false (no wakeups at all); when it turns true the loop restarts with a fresh full `period`, no catch-up tick. In `primitives/src/interval.rs`. |
| `use_timeout_while(active, delay, on_fire)` | a one-shot countdown | If `active` goes false before it fires, the countdown is dropped and restarts in full on return. Never fires twice. |
| `use_entered_view_when(enabled)` | one-shot entrances (chart load animations) | Returns an `EnterView`; `entered()` turns true once, when at least `ENTER_RATIO` (40%) of the element is on screen (or 40% of the viewport height for a taller element) with the tab visible. No pre-roll: an entrance that starts before the element is on screen is one nobody saw. The observer lets go after it fires. |
| `use_document_visible()` | timers that carry meaning (a toast's auto-dismiss) | Follows the tab only and ignores scroll position. |

**Pause versus stop.** A CSS loop is paused, not removed: ONE rule in `dx-components-theme.css` turns `data-dx-motion="paused"` into `animation-play-state: paused !important` for the element and everything under it, so a component never names its animation, a future animation is covered without being listed, the spinner resumes at the angle it stopped at, and a paused animation costs nothing. A timer is stopped, because there is no phase to keep: the task is cancelled and the countdown starts over when the element returns, so a slideshow never skips instantly on return. A timer the user is waiting on follows the tab only. A one-shot entrance neither pauses nor stops: it waits to be seen, plays once, and is done.

**What is exempt.** Transitions the user triggered (hover, open, close, a drag) are not gated: they run because the user is looking. The Input OTP caret blinks only on the one focused slot, which is visible by definition, and a hidden tab renders no frames; it is in the gate's `ALLOW`.

**The gate.** `scripts/check-motion-gating.sh` keeps the ungated shapes from coming back. (A) In every `*.rs` under `primitives/src` and `preview/src` (`interval.rs` excepted), a bare `use_interval(` call, and a hand-rolled `loop`/`while` that `sleep(...)`s, must be the gated form or carry `// motion-ok: <reason>` on the same line or the line above (a timer that carries meaning, or one that genuinely lives as long as the page). A hand-rolled loop passes when one of the gate hooks is used within 40 lines before it; a bare `use_interval` never does. (B) Every component `style.css` with an `infinite` animation needs a component or primitive (`preview/src/components/<name>/component.rs`, `primitives/src/<name>.rs` or `primitives/src/<name>/**`) that applies `use_motion`, or an `ALLOW` entry with a reason; so do the non-component sheets in `preview/assets` (the `.dx-shimmer` utility). An `ALLOW` entry that no longer applies fails the run. Not covered, said plainly: a self-re-arming `setTimeout` chain or a `requestAnimationFrame` loop in JS (`scroll-main-thread.spec.ts` and `playwright/oracle/tier2-html/motion-gating.spec.ts` count timer and frame callbacks at runtime) (the reduced-motion preference is checked by `scripts/check-infinite-animations.sh`, see "Reduced motion").

## Anchored keyframes

Anchored overlays (tooltip, hover card, popover, dropdown menu, menubar, navbar, navigation menu, select, combobox, date picker) position themselves with `transform` (`translateX(-50%)` for a centred anchor, `-100%` for an end-aligned one) in the engine stylesheet `primitives/src/top_layer.rs` injects. A `@keyframes` that sets `transform` replaces that value for as long as the animation runs, so the panel plays its whole fade off-centre and snaps into place on the last frame. It was found twice on 2026-10-05: the date picker's panel played its fade 144 px off-centre, the colour picker's 133 px.

The individual `translate`, `scale` and `rotate` properties are separate from `transform` and compose with it, so the centring survives. **Animate `translate` / `scale` / `rotate` in overlay keyframes, never `transform`.** `scripts/check-anchored-keyframes.sh` bans `transform` inside any `@keyframes` of a preview stylesheet; the short `ALLOW` list holds motion that is deliberately `transform` and can never sit on an anchor (the chart's SVG mark grow-in, the edge-docked drawer and sheet slides, the toast stack), each with its reason.

## Modal exits

A native `<dialog>` leaves the top layer in one frame the instant `close()` runs. Outside the top layer a `position: fixed` box is no longer resolved against the viewport but against the nearest ancestor with layout containment, and clipped by its paint containment, so a Sheet "changed size" mid-slide and a Drawer glitched on close (backlog row 141). The shared driver `use_dialog_open_driver` in `primitives/src/lib.rs` therefore keeps the dialog open until its own animations and its `::backdrop`'s have settled (bounded by `DIALOG_EXIT_TIMEOUT_MS`, 1500 ms), and only then calls `close()`. A generation counter guards a reopen during the wait. Dialog, AlertDialog and the modal Popover call the driver directly; Sheet, Drawer and CommandDialog compose `DialogRoot`. The scroll lock releases on the dialog's own `close` event, after the exit animation.

What this means for you: write a normal exit animation on the dialog or its `::backdrop`; the close waits for it (up to 1.5 s). Do not rely on `transition: overlay allow-discrete` (Chromium only, and not used here). Keep exit keyframes to `transform`/`translate`/`opacity` so the layout box is constant while it plays.

## Theme-switch colour transitions are kept on purpose

Switching light/dark (or a preset) makes every component's colour transition run at once, a short wash across the page. next-themes avoids it with `disableTransitionOnChange`; we deliberately do not. The owner wants the transition kept (backlog row 158 part (b), 2026-10-10). If the wash ever causes a problem, the fix lives only inside the single mode writer that row 158 part (a) specifies (`apply_mode`, shared by `preview/src/theme.rs` and the pre-paint script in `preview/index.html`; approved, not built yet): a "switching" flag that disables transitions for one frame, never a second writer.

## Adding your own animated component

Use the hooks; do not write a pause rule or a `setInterval`. Link `dx-components-theme.css` (it carries the one `[data-dx-motion="paused"]` rule; copy that rule if you do not use the theme).

```rust
use dioxus::prelude::*;
use dioxus_primitives::activity::use_motion;
use dioxus_primitives::dioxus_attributes::attributes;
use dioxus_primitives::interval::use_interval_while;
use dioxus_primitives::merge_attributes;

#[component]
fn LiveDot(#[props(extends = GlobalAttributes)] attributes: Vec<Attribute>) -> Element {
    let mut ticks = use_signal(|| 0u32);
    // One answer to "can anyone see this?", shared with every other motion on the page.
    let motion = use_motion();
    // A timer: no task at all while hidden, a fresh full second on return.
    use_interval_while(motion.active(), std::time::Duration::from_secs(1), move || ticks += 1);
    let base = attributes!(span { class: "live-dot", "data-ticks": "{ticks}" });
    // A CSS loop: the spread attribute is all the pause needs.
    let merged = merge_attributes(vec![base, attributes, motion.attributes()]);
    rsx! { span { ..merged } }
}
```

```css
.live-dot { animation: live-dot-pulse 1.2s ease-in-out infinite; }
@keyframes live-dot-pulse { 50% { opacity: 0.4; } }           /* opacity: compositor-only, passes the gate */
@media (prefers-reduced-motion: reduce) { .live-dot { animation: none; } }
```

- An element that only sometimes loops: `use_motion_when(move || loading())`.
- A one-shot entrance: `let enter = use_entered_view_when(move || animate && !done());`, spread `enter.attributes()`, and start the animation when `enter.entered()` is true.
- A timer that must keep counting while scrolled away: `use_timeout_while(use_document_visible(), duration, on_fire)`.
- A timer with no visual (page-lifetime): `// motion-ok: <reason>` on the line, if you run the repository's gate.
- An anchored overlay's keyframes: `translate`/`scale`, not `transform`.
- A forever animation: `transform`, `translate`, `scale`, `rotate`, `opacity` only.

## Per-component table

"While unseen" says what the component does when nobody can see it: **pause** (the CSS loop is paused and keeps its phase), **stop** (the timer is dropped and restarts in full on return), **waits for view** (a one-shot entrance that has not played yet waits to be seen), **exempt** (a short transition the user triggered, or a motion that is visible by definition). Durations written as `--dx-…` are the tokens above.

| Component | What moves | While unseen | Tune or disable |
|---|---|---|---|
| Spinner | the ring rotates, `transform`, 1 s linear, forever | pause (`use_motion`) | override `animation-duration` or set `animation: none` on `.dx-spinner svg`; under reduced motion the ring is a still arc (the theme's `[data-dx-motion-key]` rule) |
| Skeleton | opacity pulse, 2 s, forever | pause (`use_motion`) | override `animation` on `.dx-skeleton`; under reduced motion it is a still block (the theme's `[data-dx-motion-key]` rule) |
| Progress | indeterminate bar sweeps, `translateX` 1 s linear forever; the determinate fill animates `width` for 250 ms | pause while indeterminate (`use_motion_when`); a determinate bar subscribes to nothing | `animation` on `.dx-progress-indicator`; the width transition is a literal in `progress/style.css`; under reduced motion the sweep stops and the bar shows diagonal stripes across the whole track (a still half-width bar would read as 50%) |
| Avatar (loading) | placeholder opacity pulse, 1.5 s, only while the image is loading | pause while loading (`use_motion_when`); nothing is subscribed once loaded | `animation` on `.dx-avatar[data-state="loading"]`; under reduced motion the placeholder is still (the theme's `[data-dx-motion-key]` rule) |
| Carousel autoplay | the timer advancing a slide every `delay_ms` | stop (`use_motion_active_for`; fresh full delay on return); it also stops on hover, focus, interaction and the user's pause | `CarouselAutoplay { delay_ms, default_playing, stop_on_mouse_enter, stop_on_interaction }`; leave `CarouselAutoplay` out for none; reduced motion keeps it off at mount |
| Carousel arrows and dots | the arrow at either end fades; the active dot widens | exempt | `nav_disabled_opacity`, `--dx-carousel-nav-disabled-opacity`, `--dx-carousel-nav-disabled-visibility`; `--dx-motion-duration-slow` |
| Chart entrance | bars grow, lines and areas are revealed, pies and radials sweep, once (500 to 1500 ms) | waits for view (`use_entered_view_when`: 40% visible, no pre-roll, tab visible) | `animate: false`; off under reduced motion; the durations are literals in `chart/style.css`; the hover transitions use `--dx-motion-duration-base` and `--dx-motion-duration-slow` |
| Toast | slide-in and restack transitions; the auto-dismiss countdown | stop the countdown while the tab is hidden, full duration on return (`use_document_visible` + `use_timeout_while`); scrolling never changes it | `duration` and `permanent` on the toast; `--dx-motion-duration-slow` for the slide |
| Dialog, AlertDialog, CommandDialog | scrim fade and panel in/out; the dialog stays open until the exit settles, then `close()` | exempt | `--dx-overlay-duration`, `--dx-overlay-ease`, `--dx-overlay-scrim`, `--dx-overlay-blur`; `overlay: false` drops the scrim |
| Sheet | slides from its edge, 500 ms in and `--dx-motion-duration-slower` out | exempt | the `dx-slide-*` durations in `sheet/style.css`; the scrim through `--dx-overlay-*` |
| Drawer | slides from its edge (`--dx-motion-duration-slow` in, `--dx-motion-duration-base` out); the panel follows the pointer while dragged | exempt | `--dx-motion-duration-*`, `--dx-overlay-*`; the drag offset is the `translate` property |
| Popover | fade in and out, `--dx-overlay-duration` | exempt | `--dx-overlay-duration`, `--dx-overlay-ease` |
| Dropdown menu, context menu, menubar | the menu panel fades and scales in (`--dx-motion-duration-base`), item highlights (`--dx-motion-duration-fast`) | exempt | `--dx-motion-duration-fast`, `--dx-motion-duration-base`, `--dx-motion-ease-in`, `--dx-motion-ease-out` |
| Select, Combobox | the list fades in (`--dx-motion-duration-base`) and out (`--dx-motion-duration-fast`) | exempt | the same tokens; the `dx-picker-in` and `dx-picker-out` keyframes |
| Tooltip, Hover card | fade in and out (tooltip `--dx-motion-duration-slow`, hover card `--dx-motion-duration-fast`) | exempt | the same tokens; reduced motion swaps in `--dx-motion-duration-reduced` |
| Navigation menu, Navbar | the open content slides in (`--dx-motion-duration-slow`) and out (`--dx-motion-duration-base`); the chevron rotates | exempt | `--dx-motion-duration-slow`, `--dx-motion-duration-base`, `--dx-motion-ease-standard` |
| Accordion, Collapsible | accordion panel height opens and closes (`--dx-motion-duration-slower`) and the chevron rotates; Collapsible just shows and hides | exempt | `--dx-motion-duration-slower`; a find-in-page reveal skips the animation by design |
| Tabs | trigger colours and the ghost underline fade (`--dx-motion-duration-base`) | exempt | `--dx-motion-duration-base`, `--dx-motion-ease-standard` |
| Drag and drop list | the dragged item stays as a ghost; the items after the slot part by a `transform` transition | exempt (only runs during a drag) | `ghost_opacity` and `drop_gap`, or `--dx-dnd-ghost-opacity` and `--dx-dnd-drop-gap`; instant under reduced motion |
| Date picker | popover fade; after a selection it waits `close_delay` so the click visibly registered, then plays the normal exit | exempt | `close_delay` (default `300ms`, `Duration::ZERO` closes at once), `close_on_select: false` |
| Message scroller | the jump button slides, scales and fades; the transcript is revealed once its scroll position is applied | exempt | `--dx-motion-duration-slow` for the show; the hide is a literal `0.4s` in `message_scroller/style.css` |
| Input OTP | the caret blinks, `opacity`, 1 s step, forever, on the one focused slot only | exempt (visible by definition; in `check-motion-gating.sh`'s `ALLOW`) | `animation` on `.dx-input-otp-slot[data-active="true"]::after`; under reduced motion the caret is steady (`animation: none`) |
| Shimmer effect | `.dx-shimmer` text highlight sweep (`background-position`, paint-only; the one `ALLOW` entry of the compositor-only gate) | pause when the host applies `use_motion_when` (Marker content, Attachment title) or an ancestor carries `data-dx-motion="paused"`; a bare `.dx-shimmer` elsewhere needs `use_motion()` on its host | `--dx-shimmer-duration`, `--dx-shimmer-color`, `--dx-shimmer-spread`, `--dx-shimmer-angle`; `dx-shimmer-once`, `dx-shimmer-reverse`, `dx-shimmer-none`; off under reduced motion |

## Where to look

- `primitives/src/activity.rs` and `primitives/src/interval.rs`: the hooks, with the reasoning in their module docs.
- `preview/assets/dx-components-theme.css`: the tokens, the shared reduced-motion layer, the one `[data-dx-motion="paused"]` rule and the one `[data-dx-motion-key]` reduced-motion rule.
- `scripts/check-infinite-animations.sh`, `check-motion-gating.sh`, `check-anchored-keyframes.sh`: the three gates, each with its `ALLOW` list and reasons in the header.
- `playwright/oracle/tier2-html/motion-gating.spec.ts`: the runtime proof that gated motion makes no callbacks while unseen.
- `dev-docs/research/round3-2026-10-05.md` section 1 (modal exits) and `dev-docs/backlog.md` rows 141, 158, 174.
