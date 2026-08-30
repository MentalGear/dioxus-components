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
