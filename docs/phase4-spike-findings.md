# Phase 4 spike — findings

**Branch:** `spike/native-dialog` (local only; never pushed, never merged).
**Prototype:** `primitives/src/spike_native_dialog.rs` (10 disposable
components), wired onto a throwaway preview route (`preview/src/components/spike_dialog/`,
`/component/?name=spike_dialog`), exercised by
`playwright/spike-native-dialog.spec.ts` (16 tests, all in
`test.describe` blocks numbered to match the experiments below).

**Result of the run referenced throughout this doc:** 16/16 passing,
stable across 3 consecutive full-suite runs (48/48 total). Every test
encodes one empirical claim; several of the "passing" tests assert a
*surprising* result rather than the originally expected one -- those are
flagged explicitly. Nothing here was taken on faith from
`docs/recommended-implementations.md` or the dignifiedquire fork without
being run against a real browser.

Chromium version: whatever ships in this environment's
`/opt/pw-browsers/chromium-1194` (Playwright's bundled Chromium). All
findings below are Chromium-only; no other engine was tested, which
matters most for the two new hazards (focus-trap's `<body>` stop, and the
scroll-lock defeat) -- see the verdict section.

---

## Experiment 1 — root-cause repro (`b3f6de53` shape)

**Built:** `SpikeUnsynced` — `showModal()`/`close()` driven one-way from a
signal via `document::eval`, with no listener on the dialog's `close`
event. Shaped exactly like the upstream commit that introduced `<dialog>`:
send `id` + `is_open` into JS, JS calls `showModal()`/`close()`, and
nothing comes back.

**Asserted:** open via trigger -> visible, `data-state="open"`. Press
Escape -> dialog closes natively (`dialog.open === false`, not visible).
Signal-driven status span still reads `data-state="open"`. Click the
trigger once more (this computes `!open()`, i.e. `!true`, since the signal
never learned about the close) -> dialog stays closed and the status flips
to `"closed"` with no visible change -- the "clicking open again does
nothing" symptom.

**RESULT: RED, as predicted.** All four assertions passed exactly as
described. This reproduces, by execution rather than by reading a bug
report, the exact defect class referenced by `dioxus#2962` / `dioxus#4319`
and the reason PR #47 gave `<dialog>` up in favour of a JS focus trap: a
one-way signal binding desyncs the instant the browser closes the dialog
by any means the signal doesn't know about (Escape here; a `::backdrop`
click or a `method="dialog"` form submit would do the same).

## Experiment 2 — the fix (close-event sync + open guard)

**Built:** `SpikeSynced`, using two helpers shared by every later
experiment:
- `use_dialog_close_sync` — the "eval-channel listener" pattern already in
  this crate (`use_form_reset_listener`, `primitives/src/lib.rs`): one
  `document::eval` that runs once, wires `dialog.addEventListener('close',
  () => dioxus.send(true))`, and a `spawn`ed loop that calls `set_open.set(false)`
  on every message.
- `use_dialog_open_driver` — a second `document::eval`, keyed off `open()`,
  that checks `dialog.open` before calling `showModal()`/`close()`.

**Asserted:** 6 full open -> Escape -> reopen cycles (status attribute
checked after every step), plus 5 open -> close-button -> reopen cycles.

**RESULT: GREEN.** All 11 assertions across both tests passed on every one
of 3 suite runs. Plain `document::eval` -- no `wasm_bindgen`, no new
dependency -- was sufficient; it did not prove racy at this interaction
rate (see "what remains untested" below for the caveat on this).

## Experiment 3 — VDOM clobber

Three variants, because the interesting question splits into three
independent claims.

**3a -- `SpikeClobberUnbound`.** `open` is never bound in the `dialog {
}` rsx block; the browser owns the attribute entirely via
`showModal()`/`close()`. A `setInterval`-driven tick (30 ms) increments a
signal rendered both inside and outside the dialog, forcing frequent
Dioxus re-renders while the dialog is open.

*Asserted:* the tick text actually changes over 1.5s (~50 renders), the
dialog is still open and `.open === true` afterward, and the in-dialog
tick text also advanced.

*RESULT: GREEN.* The dialog survived every re-render. Dioxus's diffing
never touches the `open` attribute because it was never part of the vdom's
model of that element -- there was nothing to clobber. This confirms the
plan's implicit assumption that a native `<dialog>`'s browser-owned state
is safe from vdom reconciliation **as long as you never bind the attribute
Dioxus is not supposed to touch.**

**3b -- `SpikeClobberBoundUnguarded`.** `open: open()` bound declaratively
(the Blitz floor from caveat 2) *and* `showModal()`/`close()` called
unconditionally, no guard.

*Asserted:* toggling on throws, caught and reported to a DOM node.

*RESULT: RED, exactly as the docs predicted.* Captured text:

```
InvalidStateError: Failed to execute 'showModal' on 'HTMLDialogElement':
The dialog is already open as a non-modal dialog, and therefore cannot be
opened as a modal dialog.
```

Sequence: Dioxus renders first (this is effect-after-render ordering, not
a race) and sets the `open` attribute, which Chromium reflects to
`dialog.open === true` immediately; the effect then calls `showModal()` on
an already-open dialog and the browser throws.

**3c -- `SpikeClobberBoundGuarded`.** Same declarative `open` binding, but
through `use_dialog_open_driver`'s guard (`if (!dialog.open) showModal()`).

*Asserted:* no error, dialog visible, but `dialog.matches(':modal')` is
`false`.

*RESULT: RED in a way the docs did not spell out -- this is the spike's
first real "new hazard."* The guard does stop the crash, but because the
render already set `dialog.open = true` via the attribute *before* the
effect runs, the guard's own condition (`!dialog.open`) is false, so
`showModal()` is **never called at all**. The dialog renders as a plain,
non-modal, in-flow open `<dialog>` -- no `::backdrop`, no top layer, no
focus trap, no inertness. The crash is gone but so is every capability
Phase 4 exists to get. **Binding `open` declaratively and driving
`showModal()` from the same render pass are not just something to "not do
blindly" (the doc's phrasing) -- they are mutually exclusive on any target
where `showModal()` actually runs.** The declarative `open` floor is only
safe to bind on targets where the JS path is inert anyway (Blitz, where
`document::eval` no-ops) -- i.e. the binding must be
`#[cfg(not(target_arch = "wasm32"))]`-gated (or equivalent), never
unconditional.

## Experiment 4 — native subsumption

**Built:** `SpikeSubsumption` -- a `SpikeSynced`-style dialog with three
focusable elements (`f1`, `f2`, `f3-close`), nested inside a clipping
ancestor (`overflow: hidden; transform: translateZ(0); height: 60px`) that
also holds the trigger, plus a background button and input **outside**
that ancestor to probe inertness.

**(a) Focus trap.** *Asserted (revised -- see hazard below):* focus never
lands on the three background/trigger ids while tabbing 8 times, and all
three in-dialog stops are visited.

*RESULT: GREEN, but only after a revision -- second new hazard found by
execution.* The original assertion ("focus after each Tab is one of
`f1`/`f2`/`f3-close`") was RED on first run. Debugging
(`document.activeElement` logged after each of 10 Tabs) showed:

```
0 f2       1 f3-close     2 <body> (id="")     3 f1
4 f2       5 f3-close     6 <body> (id="")     7 f1  ...
```

Tabbing off the *last* focusable element in an open modal `<dialog>` in
Chromium lands on `<body>` for exactly one Tab stop before the *next* Tab
wraps to the first element -- it does not wrap directly. This is
reproducible and consistent (checked over 10 Tabs / 3+ cycles). More
surprising: `document.body.inert` reads `false` at every point during this
-- Chromium does **not** reflect the "background is behaviourally inert"
state onto the `inert` IDL property of `<body>`, even though (per part
(c), below) it genuinely blocks clicks and `.focus()` there. **Do not use
`element.inert` as a test oracle or an implementation signal for "is this
inside an open native modal's inert region" -- it will read `false`
regardless.** The revised, meaningful assertion -- focus never reaches
*interactive content outside the dialog* -- holds cleanly.

**(b) Focus restore.** *Asserted:* Escape closes the dialog and the
trigger regains focus. *RESULT: GREEN*, first try, no surprises -- this is
the one part of native `<dialog>` that behaved exactly as documented.

**(c) Inert background.** *Asserted:* a real mouse click (via
`page.mouse.click` at the background button's coordinates, bypassing
Playwright's actionability checks) does not increment its click counter;
`el.focus()` called directly on the background input does not move
`document.activeElement`, which stays inside the dialog.

*RESULT: GREEN.* Both assertions passed. This is the one place where
`(a)`'s `inert === false` reading is actively misleading: the background
*is* inert in every behavioural sense that matters, it just doesn't say so
on the property.

**(d) Top layer.** *Asserted:* with a 60px-tall `overflow: hidden`
ancestor around the dialog, the dialog's rendered bounding box is taller
than that ancestor's clip height (60px) -- a real dialog with a heading,
two inputs, and a button cannot fit in 60px, so if it were clipped by
ordinary CSS the rendered box would be capped there.

*RESULT: GREEN.* Captured boxes: ancestor `{y: 348.75, height: 62}`,
dialog `{y: 297.09, width: 506.2, height: 125.8}` -- the dialog's box is
~2x the ancestor's clip height and its `y` sits *above* the ancestor's
top edge entirely. Top-layer rendering escapes the clip exactly as
documented.

## Experiment 5 — nested dialogs

**Built:** `SpikeNested` -- dialog B's `<dialog>` markup is a DOM child of
dialog A's; B's trigger lives inside A.

**Asserted:** open A, open B from inside it; Escape closes B only (A
stays visible); focus lands on B's trigger (back inside A); Escape again
closes A; focus lands on A's original page trigger.

**RESULT: GREEN,** first try. Chromium's top-layer stack handles nesting
correctly with **zero extra code** on top of the same
`use_dialog_close_sync` + `use_dialog_open_driver` pair used everywhere
else -- each dialog's own guarded driver and close listener was
sufficient; nothing coordinates the two dialogs explicitly. This directly
confirms the plan's claim that nested modals -- which the current
`FocusTrap` cannot handle, having no `pause()`/`unpause()` -- become a
pure browser concern under this design.

## Experiment 6 — scroll lock still needed

**Built:** `SpikeScrollNoLock` (no scroll-lock hook at all) and
`SpikeScrollLocked` (identically shaped, but wired through this repo's
real, already-shipped `crate::scroll_lock::use_scroll_lock`, the Phase 3.2
hook). Page has a 2000px spacer so `<html>` (confirmed
`document.scrollingElement`) has room to scroll.

**RESULT: RED for the locked case -- the most significant new hazard the
spike found, and the one that most needs a person's attention before Phase
4 ships.**

- *No lock:* `page.mouse.wheel(0, 800)` moves `window.scrollY` from 0 to a
  positive value. Expected and confirmed.
- *Locked, as currently shipped:* with `use_scroll_lock` wired (confirmed
  via `getComputedStyle` that `overflow: hidden` really is set on both
  `<html>` and `<body>` at the moment of the wheel event), the identical
  wheel gesture **still scrolls the page** -- observed `scrollY` moved by
  the same amount as the unlocked case in side-by-side runs. This was
  further narrowed with extra instrumentation (see this file's git history
  / session transcript for the full debug trail, not kept as committed
  code):
  - A `window`-level *capturing* `wheel` listener with `preventDefault()`
    **does fire** (`wheelCount === 1`) and **does call**
    `preventDefault()`, yet the scroll still happens. The identical
    listener on a plain page with no dialog open correctly blocks the
    scroll (`scrollY` stays `0`).
  - `document.elementFromPoint(x, y)` returns the `<dialog>` element for
    *every* coordinate in the viewport while it is open modally --
    `showModal()`'s `::backdrop` owns hit-testing everywhere.
  - Keyboard-driven scroll (`PageDown`) shows the same defeat.
  - **Working mitigation, verified in isolation:** freezing `<body>` with
    `position: fixed; top: -{scrollY}px` (the technique
    `react-remove-scroll` and similar libraries use, and which
    `docs/recommended-implementations.md` §5 already flags this repo as
    *not* having relative to Radix) blocks both the wheel and the
    `PageDown` gesture completely (`scrollY` stayed `0` through both). This
    fix was proven against the spike's fixtures but is **not** wired into
    `crate::scroll_lock` -- that would be production work, out of scope
    for a disposable spike.

**Most likely mechanism** (a hypothesis, not confirmed against Chromium's
source): a `showModal()` backdrop is not part of the normal CSS layout box
tree that `overflow: hidden` scroll-chaining walks, so wheel/keyboard
scroll input that hits the backdrop has no scrollable ancestor to chain
through in the usual sense and falls back to scrolling the viewport
directly -- bypassing whatever `overflow` value the actual DOM ancestors
carry. (Separately, and *not* an oracle for the hazard above: a purely
*programmatic* `window.scrollTo()` also succeeds under `overflow: hidden`
regardless of any dialog -- that's ordinary, well-known CSS behaviour,
unrelated to this finding, and was not used as evidence for it.)

**This means `docs/recommended-implementations.md` §5's "native `<dialog>`
does not itself prevent background scrolling -- this is needed regardless
of item 2" undersold the risk.** It reads as "Phase 3.2's scroll lock is
necessary and sufficient once bolted onto Phase 4." It is necessary but,
as currently implemented (`overflow: hidden`), **not sufficient**: it
provides zero protection once the modal is switched to a native
`<dialog>`. The `position: fixed` body-freeze technique is a proven
alternative and should replace (or supplement) the `overflow: hidden`
approach in `crate::scroll_lock` before Phase 4 ships, at least for the
modal path.

## Experiment 7 — `popover="auto"`

**Built:** `SpikePopover` -- fully declarative: `popover: "auto"` on the
content `div`, `popovertarget`/`popovertargetaction` on the trigger
button, **zero** Rust-side state, **zero** `document::eval`. Content is a
DOM child of a `overflow: hidden; height: 50px` ancestor.

**Asserted:** content hidden initially; trigger click shows it; clicking
an outside button (light dismiss) hides it again; content's rendered
height (~80px + padding/border) exceeds the 50px clip ancestor.

**RESULT: GREEN**, first try, on every assertion. This is the cleanest
result in the whole spike: Dioxus renders `popovertarget`/`popover`
faithfully, and the browser handles open state, light dismiss, and
top-layer escape with **no Rust code participating at all** -- not even
the `document::eval` pattern used everywhere else. This is strong evidence
for item 4.4 as a near-zero-cost win for any overlay that doesn't need
Rust-side state synced back (a plain non-modal popover). Overlays that
*do* need `open` mirrored into a signal (most of this crate's menu family)
would still need the same `toggle`-event eval-channel pattern verified in
experiments 2-6, which was not separately re-tested for `popover` here
(see "what remains untested").

---

## Verdict

**4.2 (native `<dialog>` for `Dialog`, with close/cancel sync) is
supported by the evidence, with two hazards that must be designed for
before it ships, not discovered after.** The core defect that got
`<dialog>` removed in `797b343e` (experiment 1) is real, reproducible, and
fully fixed by the eval-channel close-sync + open-guard pattern
(experiment 2) using tooling already in this crate -- no new dependency.
Focus trap, focus restore, inert background, top layer, and nested-modal
stacking (experiments 4-5) all work as the fork and the docs claimed, with
one caveat each on the trap (the `<body>` stop) and inertness (the
misleading `inert` property). Scroll lock (experiment 6) is the one place
the evidence contradicts the plan's assumption outright: **ship the
`position: fixed` body-freeze technique in `crate::scroll_lock`, or Phase
4 regresses Phase 3.2 the moment `Dialog` moves to native `<dialog>`.**

**4.4 (`popover="auto"` for non-modal overlays) is strongly supported** --
experiment 7 was the cleanest result of the entire spike, requiring no
Rust-side synchronization at all for the pure light-dismiss case.

### Implementation checklist (what Phase 4 must include)

1. **Close/cancel sync**, not a one-way binding: a `close`-event listener
   (the eval-channel pattern from `use_form_reset_listener`, reused as
   `use_dialog_close_sync` in the spike) that writes the browser's own
   closing back into the `open` signal. Without this, experiment 1's
   defect returns.
2. **An `.open` guard** before every `showModal()`/`close()` call
   (`use_dialog_open_driver` in the spike), to avoid `InvalidStateError`
   when the signal fires a redundant state change.
3. **The declarative `open` floor is `#[cfg]`-gated, not unconditional.**
   Bind `open` in rsx only where `showModal()` will not also run in the
   same build (i.e., only for non-wasm/Blitz targets, or wherever the JS
   driver is compiled out). Experiment 3c shows that binding it
   unconditionally alongside a guarded `showModal()` silently drops all
   modal behaviour rather than erroring -- a worse failure mode than the
   crash it replaces, because nothing signals it happened.
4. **Scroll lock must move off pure `overflow: hidden`** for the modal
   path -- add the `position: fixed` body-freeze technique (verified
   working in experiment 6) to `crate::scroll_lock`, or gate it in
   alongside the native-dialog work so the regression ships fixed rather
   than discovered later by a user.
5. **Do not use the `inert` IDL/attribute as an oracle** for "is this
   background inert" in either tests or implementation logic -- Chromium
   does not set it on `<body>` even while genuinely enforcing inertness.
   Test the actual behaviour (click, `.focus()`) instead, as experiment 4c
   does.
6. **Expect, and do not fight, the `<body>` intermediate Tab stop** at the
   ends of the native focus trap's cycle. It is invisible to the user (no
   visible focus ring lands anywhere) and does not let focus escape the
   modal, so it needs no code -- but a test suite asserting "focus is
   always one of these N elements" without allowing for it will be
   spuriously red, as experiment 4a's first draft was.

### New hazards discovered that reading did not predict

- Binding `open` declaratively *and* guarding `showModal()` in the same
  build doesn't crash -- it silently disables the modal (3c).
- The native focus trap's cycle includes a `<body>` stop that
  `element.inert` reports as `false` (4a).
- `crate::scroll_lock`'s `overflow: hidden` approach provides **zero**
  protection once the modal is a native `<dialog>` -- confirmed with a
  `preventDefault()`-calling capture listener that still doesn't stop the
  scroll (6). This is the one finding here that should gate the Phase 4
  go/no-go decision, not just inform its implementation.

### What could not be made conclusive, and why

- **Whether `document::eval`'s close-sync "proves racy" under load** was
  not established either way. The task called for falling back to
  `wasm_bindgen` (the dq fork's approach) "if eval proves racy"; the
  spike's interaction rate (manual clicks, keyboard input, one dialog at a
  time except experiment 5's two) never produced a dropped or
  out-of-order event across dozens of cycles, but this is not a
  stress test and a genuinely adversarial one (e.g., opening/closing
  faster than an eval round-trip, or many simultaneous dialogs) was out of
  scope for the time available. Treat eval as viable, not as proven robust
  under load.
- **Blitz/native-target behaviour** was not tested at all -- this
  environment only runs the `web` target. Every finding above is Chromium
  web-only; the plan's Blitz-floor reasoning (bind `open` declaratively
  where `eval` no-ops) rests on the capability table in
  `docs/recommended-implementations.md`, not on anything this spike ran.
- **Other engines** (Firefox, WebKit/Safari) were not exercised --
  Playwright's bundled Chromium was the only browser available in this
  environment. The two new hazards (the `<body>` Tab stop and the
  scroll-lock defeat) are exactly the kind of behaviour that plausibly
  varies by engine; both should be re-verified on at least WebKit before
  Phase 4 implementation locks in `crate::scroll_lock`'s fix or asserts
  the focus-trap shape in a real (non-spike) test suite.
- **`popover`'s `toggle`-event sync path** (mirroring `open` into a signal
  for menus/comboboxes, as opposed to experiment 7's pure-declarative
  case) was not separately built or tested. The dq fork's `top_layer.rs`
  covers it via the same eval-channel shape already proven for `<dialog>`
  in experiments 2-6, so the risk is judged low, but it was not run.

---

# Round 2 — solved by construction

Same branch, same disposable spike module
(`primitives/src/spike_native_dialog.rs`, experiments 8-9), extended
Playwright spec (`playwright/spike-native-dialog.spec.ts`, now 24 tests
across all `test.describe` blocks, 4 new ones added for this round). Round
1's philosophy was "find the hazard and describe a mitigation." Round 2's
was: for each hazard, find the actual mechanism, then make the failure
*unrepresentable*, and prove it by execution -- not "less likely," fixed.

**Headline result: round 1's central scroll-lock claim did not survive
scrutiny. It was a test artifact, not a browser defect.** Plain
`overflow: hidden` on `<html>`/`<body>` already fully blocks wheel,
`PageDown`, and `Space` scrolling behind an open native modal `<dialog>` --
zero defeat, zero jump (it never touches `scrollTop`). This changes the
Phase 4 go/no-go picture from round 1's doc: scroll-lock's *only* real,
confirmed defect is the (separately real, separately fixed) scrollbar-gap
regression, not a scroll-blocking failure. See Construction A below for the
full evidence trail and how this was found.

## Construction A — the scroll-lock artifact, and the real (harder) bug underneath

**Root cause of round 1's claim.** Re-instrumented experiment 6's fixture
with precise per-step logging instead of a single before/after assertion.
The sequence in round 1's test was:

```
scrollTo(0, 0)                                  // scrollY = 0
locator('#...trigger').click()                  // scrollY = 2079 !!
  <-- Playwright's actionability check scrolled the (off-screen, far down
      the page) trigger button into view BEFORE the click fires and BEFORE
      the dialog -- and its scroll lock -- exist at all
wait for dialog visible                         // scrollY = 2079 (unchanged)
assert overflow: hidden on html/body            // true (the lock is real)
mouse.wheel(0, 800)                             // scrollY = 2079 (unchanged)
assert scrollY > 0                              // PASSES -- but for a reason
                                                 // that has nothing to do
                                                 // with the wheel event
```

Confirmed directly: `scrollY` immediately after `.click()` resolves is
already `2079`, before any wheel event fires. The subsequent
`mouse.wheel(0, 800)` changes `scrollY` by exactly `0` in every rerun
(`2079 -> 2079`, `0 -> 0`, `1000 -> 1000`) -- the wheel gesture never moved
anything; `expect(scrollY).toBeGreaterThan(0)` merely happened to already
be true from the click-triggered auto-scroll, and the test's own
methodology (checking an absolute value, not a delta) hid this. The
"`preventDefault()`-calling capturing listener still doesn't stop the
scroll" observation in round 1's original narrative has the identical
explanation: the listener fires and calls `preventDefault()` correctly,
and `scrollY` is unchanged by the wheel -- but was already non-zero before
it, from the same artifact, which round 1's before/after comparison did
not isolate.

**Corrected measurement** (`playwright/spike-native-dialog.spec.ts`,
"Round 2, Construction A -- scroll-lock root cause, revisited", 4 tests):
with the baseline scroll position actually held constant (re-zeroed *after*
the dialog and its lock are engaged, or set to an explicit value like
`1000` never touched by any click), wheel, `PageDown`, and `Space` are all
completely blocked, at scrollY 0 and at a large value, and at every
`elementFromPoint`-sampled coordinate including all four viewport corners
(where hit-testing does resolve to the `<dialog>` element itself -- the one
part of round 1's mechanism hypothesis that does hold, just not the part
that explained the "defeat"). One caveat found and worked around: a
`Space` keypress while a dialog's own Close button holds focus (which
`showModal()` auto-focuses) legitimately *activates that button* per HTML
semantics -- not a scroll-lock defect, but it will masquerade as one
("scrollY changed after Space!") if the test doesn't blur first.

**The real, separate bug underneath, now fixed by construction.**
`overflow: hidden` never compensates for the vertical scrollbar's width,
so removing it shifts layout horizontally -- on any engine with a
non-overlay scrollbar. This repo's default headless Chromium renders
0-width overlay scrollbars (confirmed: a genuinely 2613px-tall scrollable
page still reports `innerWidth - documentElement.clientWidth === 0`), so
this defect is invisible in the committed suite's normal run. Forced a
real classic 15px scrollbar with `xvfb-run` (headed Chromium under a
virtual X server) to make it reproducible: the *shipped* lock shifts a
`position: fixed; right: 0` probe element by exactly 15px the instant it
engages (see Construction C).

Building the fix took two wrong attempts before landing on the
construction that actually holds in both scrollbar regimes:

1. **Wrong (round 1 of this round's own investigation): measure the gap
   (`innerWidth - clientWidth`) and add it as `padding-right` on
   `<body>`** -- the textbook `react-remove-scroll`-style recipe.
   *Falsified by execution* under Xvfb: `padding-right` on `<body>` only
   compensates normal-flow content. A `position: fixed; right: 0` element
   (a realistic right-aligned navbar action) is positioned against the
   *initial containing block*, whose size is set by the true viewport net
   of the actual scrollbar and is not influenced by any element's padding.
   The probe still shifted the full 15px, uncorrected.
2. **Wrong in the opposite direction: `scrollbar-gutter: stable` +
   `overflow: hidden`, toggled together only at lock time.** This does fix
   the classic-scrollbar case -- but on a platform whose *unlocked* state
   uses an overlay scrollbar (this repo's headless default), turning
   `scrollbar-gutter: stable` on introduces a reservation that was never
   there before: Chromium reserves the classic-scrollbar width for
   `stable` regardless of whether overlay scrollbars are otherwise in use.
   Confirmed by execution: in headless Chromium, the same probe element
   went from flush (`right: 1280`) to newly inset (`right: 1265`) the
   moment the transient toggle engaged -- introducing exactly the defect
   being fixed, just on the other kind of platform.
3. **Correct: stop treating the gutter as part of the lock/unlock
   transition at all.** Reserve it *permanently*, once
   (`overflow-y: auto; scrollbar-gutter: stable;` -- in a real
   implementation a static base-stylesheet rule; done here as a one-time
   mount effect since this is a disposable spike), and let locking only
   toggle `overflow-y` between `auto` and `hidden`. Because
   `scrollbar-gutter: stable`'s reservation does not depend on which
   non-`visible` value `overflow-y` currently holds, the available width
   is now bit-for-bit identical before, during, and after every lock
   cycle -- there is no gap value left to compute, patch, or invert.
   Confirmed against both regimes: Xvfb/classic-scrollbar (gap 15 -> 0 ->
   15, probe's right edge constant at every step) and this repo's default
   headless/overlay environment (gap 0 throughout, probe's right edge
   still constant).

The scroll-block half of the fix is the already-proven `position: fixed;
top: -{scrollY}px` body-freeze (round 1's mitigation, now wired up with
the gutter fix and a real save/restore cycle instead of being demonstrated
in isolation). One more non-obvious, execution-only finding surfaced
building this: the moment `<body>` is taken out of flow, `<html>`'s
`scrollTop` (and therefore `window.scrollY`) is clamped to `0` by the
browser immediately -- *before* any wheel or keyboard event -- because
`<html>`'s scrollable extent has genuinely collapsed to zero. The visual
position is held entirely by `top: -{scrollY}px`, not by leaving
`scrollTop` alone. A test asserting "`scrollY` stays equal to its pre-lock
value while locked" is therefore asserting the wrong invariant; the
correct one (used in the committed tests) is "`scrollY` stays at whatever
value it settled to *the instant the lock engaged*, across every
subsequent scroll gesture," with the pre-lock value checked only once,
after unlock, as the restore assertion.

**Proof (`playwright/spike-native-dialog.spec.ts`, "Round 2, Construction A
-- compensated (jump-free) scroll lock", 2 tests, scrollY 0 and 1600):**
per cycle -- open (a) marker's `getBoundingClientRect().top` identical
before and immediately after locking (no jump); (b) wheel, `PageDown`, and
`Space` all held at the locked-in scrollY; (c) a `position: fixed;
right: 0` probe's `.right` identical before and while locked (no
horizontal shift); close -- `scrollY` restored to the exact pre-lock
value, marker `.top` and probe `.right` both back to their original
readings. All 24 suite tests green across 3 consecutive headless runs and
one full run under Xvfb with a forced real 15px scrollbar (both the
6-test Construction A subset and the full 24-test suite).

**Honest limits.** This is a hand-rolled hook proven in one spike fixture,
not backport-ready production code: it does not handle iOS momentum
scroll, does not coordinate with `crate::scroll_lock`'s existing nested-
lock refcounting (Construction C's target, `scroll_lock.rs`, already has
that; this hook doesn't), and `scrollbar-gutter` support should be
double-checked on the actual minimum-supported-browser list before this
technique is ported into `scroll_lock.rs` for real (it is broadly
supported in current Chromium/Firefox; historically absent in older
WebKit/Safari, meaning the fixed-position half of this fix would silently
not apply there -- Safari support was not verified in this environment,
which only runs Chromium).

## Construction B — cfg-split floor/modal separation

**Root cause of the interleaving** (experiment 3c, round 1): Dioxus
applies the declaratively-bound `open` attribute during the render pass;
the effect that calls `showModal()` runs *after* that render commits. By
the time the guarded driver's `if (!dialog.open) showModal()` check runs,
the attribute has already made `dialog.open === true`, so the guard's
condition is false and `showModal()` is skipped -- silently. No exception,
no console warning: the dialog renders as an inert, non-modal, in-flow
open element. The two code paths (declarative `open` binding, and a JS
driver that also writes `open` via `showModal()`/`close()`) are not
merely risky to combine; they race on which one gets to define "is this
dialog open" for a given render, and the declarative one always wins that
race because it runs first.

**Construction.** Added experiment 8, `SpikeCfgSplit`
(`primitives/src/spike_native_dialog.rs`): the component body is split
into two `cfg`-gated free functions, `spike_cfg_split_body`, so only one
of them exists in any given compiled artifact:

```rust
#[cfg(target_family = "wasm")]
fn spike_cfg_split_body(...) -> Element { /* web arm: showModal()-driven,
    `open` never bound in rsx */ }

#[cfg(not(target_family = "wasm"))]
fn spike_cfg_split_body(...) -> Element { /* native arm: `open` bound
    declaratively, zero document::eval calls */ }
```

The web arm cannot bind `open` declaratively -- the code that would do
that does not exist in that build. The native arm cannot call
`showModal()` unconditionally-then-guarded -- there is no `document::eval`
call in that arm at all. Experiment 3c's race requires both paths to be
compiled into the same binary; here, only one ever is. The hazard isn't
mitigated, it's absent from the artifact.

**Proof.**
- `cargo check -p dioxus-primitives --features web --target
  wasm32-unknown-unknown` -- clean (the web arm, matching what `dx run
  --web` actually builds).
- `cargo check -p dioxus-primitives` (host triple,
  `x86_64-unknown-linux-gnu`, no target flag -- this environment's default,
  non-wasm) -- clean (the native arm).
- `playwright/spike-native-dialog.spec.ts`, "Round 2, Construction B":
  20 open/close cycles driven by direct DOM `.click()` calls inside a
  single `page.evaluate`, yielding only one `requestAnimationFrame` tick
  between clicks (no Playwright-level waits, retries, or actionability
  checks), asserting on every cycle that `dialog.open` implies
  `dialog.matches(':modal')`. Zero bad cycles, stable.

**The cfg axis, and why `target_family = "wasm"` is the wrong one for a
real implementation.** This spike's split is gated on `target_family =
"wasm"` because that is the only axis this single-target environment
(`dx run --web`, Playwright/Chromium) can actually exercise both sides
of -- flipping the `not(wasm)` arm on and checking it with `cargo check`
on the host triple. But the workspace's own dependency graph shows this is
not the axis a shipped implementation should use:

- `preview/Cargo.toml` and `test-harness/Cargo.toml` both define a
  `desktop` feature as `dioxus/desktop` -- `dioxus-desktop` (checked
  directly: `dioxus-desktop-0.7.9`'s `Cargo.toml`) has **no**
  `target_arch`/`target_family` cfg gate of its own; it is a normal crate
  that compiles for `windows`/`linux`/`macos`/`ios`/`android` as a native
  (non-wasm32) binary, using a real system webview (wry/tao) as its
  rendering engine.
- Checked `dioxus-desktop`'s `Document::eval` impl
  (`dioxus-desktop-0.7.9/src/document.rs`): it is a real implementation
  that runs JS in that webview -- `showModal()` genuinely executes there,
  exactly as it does on web.
- Checked the *actual* Blitz-based "native" renderer,
  `dioxus-native` (the crate backing the `native` feature both
  `Cargo.toml`s also define): its `Document::eval`
  (`dioxus-native-0.7.9/src/contexts.rs`) is a literal
  `NoOpDocument.eval(js)` -- `eval` compiles and runs, but the JS is
  never executed. This -- not "non-wasm" -- is where the declarative-floor
  arm is actually required, because it is the only renderer where
  `showModal()` truly cannot run.

So `#[cfg(not(target_family = "wasm"))]` would put a **desktop-webview
build on the declarative-floor (native) arm**, even though that build runs
a real browser engine via `dioxus-desktop` and needs the exact same
`showModal()`-driven web arm that a wasm32 build does. That silently
reintroduces experiment 3c's hazard on desktop, the one platform this cfg
split is supposed to have made impossible everywhere. **The right axis is
a Cargo feature that mirrors which renderer backend is linked in --
matching this crate's own existing `web`/`router` feature pattern (e.g.
gate the declarative floor on the `native` feature, alongside `web`, not
on `target_family`) -- not `target_arch`/`target_family`, which conflates
"non-wasm binary" with "no working JS engine" and gets desktop-webview
wrong.** This environment cannot build or run a `dioxus-desktop` target to
verify that split directly (no desktop runtime available here), so this
conclusion rests on reading the three `Cargo.toml`s and the two crates'
`Document::eval` implementations cited above, not on an executed desktop
build -- the one place in this round where the evidence is textual rather
than a passing/failing test.

## Construction C — retrofit check on the shipped `scroll_lock.rs`

**Question:** does today's shipped Phase 3.2 `crate::scroll_lock` (used by
the current non-native `Dialog`/`AlertDialog`/`Popover`) have the jump
problem, the gap problem, or neither, independent of Phase 4?

**Jump: no.** `overflow: hidden` never writes to `scrollTop`; per
Construction A's corrected measurement, there is no scroll-position change
to jump from in the first place.

**Gap: yes, confirmed by execution -- but only visible with a real
scrollbar, which this repo's default test environment does not have.**
This repo's headless Chromium renders 0-width overlay scrollbars even for
a genuinely 2613px-tall scrollable page (`innerWidth -
documentElement.clientWidth === 0`), so the shipped lock's known
limitation (already documented in `scroll_lock.rs`'s own module comment:
"No ... scrollbar-gap compensation") cannot be exercised by any test that
only runs in this environment's default configuration. Forced a real
classic scrollbar with `xvfb-run` (Chromium headed under a virtual X
server, no code changes, same binary) and re-ran the exact same
production `/component/?name=dialog` demo page:

| environment | natural scrollbar gap | shift on opening the real Dialog |
|---|---|---|
| this repo's default headless Chromium | 0px | 0px |
| headed Chromium under `xvfb-run` (real scrollbar) | 15px | 15px |

The committed test (`playwright/spike-native-dialog.spec.ts`, "Round 2,
Construction C") asserts `shift === gapBeforeLock` rather than hardcoding
either number, so it is meaningful evidence in whichever environment runs
it: it stays green in this repo's normal CI-shaped run (0 == 0, honestly
reflecting "no gap here to regress on"), and the Xvfb run above is the
positive confirmation that the same assertion, on the same page, catches a
real 15px regression the moment a classic scrollbar exists. Tested against
the real Dialog demo page rather than reusing a spike fixture, deliberately
-- Construction A's compensated-lock fixture lives on the same spike page
as the rest of this document's experiments and sets a page-global
`scrollbar-gutter: stable` on `<html>` for its own purposes; reusing that
page for Construction C's measurement was tried first and found to
silently zero out the very shift being measured (the gutter reservation
from the unrelated fixture was already absorbing the space), which is
itself a small illustration of why Construction A's fix must be scoped
per-application, not per-component, when it eventually ships.

**Verdict: yes, backport the scrollbar-gap fix independently of Phase
4.** The regression is real, reproducible, and currently silent in this
repo's own test suite only because of a headless-Chromium environment
detail (overlay scrollbars) that will not hold on most users' actual
machines (Windows Chrome/Firefox defaults, Linux without overlay-scrollbar
GTK settings, and other engines all commonly render classic, space-
reserving scrollbars). Construction A's permanent-`scrollbar-gutter`
technique (not the falsified padding-right recipe) is the fix to port; it
requires no Phase 4 native-`<dialog>` work to be useful, since today's
lock already has the defect it fixes.
