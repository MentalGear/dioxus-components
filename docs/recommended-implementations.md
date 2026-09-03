# Recommended implementations — best of each source

For every gap in [`capability-gaps.md`](./capability-gaps.md), which solution to actually build, assembled from the strongest parts of upstream, Radix, and the forks rather than copying any one of them wholesale.

Sources compared: this repo (`bf007c1`), `radix-ui/primitives`, `dignifiedquire/dx-components`, `sarendipitee/dioxus-components`, `jcgruenhage/dioxus-components`, and the WHATWG HTML spec.

**Verification:** all source claims below were read from the actual code. **Update (2026-09-02):** most of these recommendations have since been built and verified by execution — form participation (§1), modal dialogs (§2), non-modal overlays (§3), focus restore (§4 and the later §4b/§4c keyboard/role-contract additions), body scroll lock (§5), and the FLIP sub-problem of collision detection (§6) are all landed on `main`; see `plan.md`'s phase tables for what landed and when. Typeahead (§8) and RTL (§9) remain unbuilt, and collision detection's shift/size clamping and `ContextMenu` clamp remain open — see `backlog.md`.

---

## The finding that changes the plan

`dignifiedquire`'s `dialog.rs` documents something worth more than any file we might port:

> Renders a modal dialog using the native `<dialog>` element opened with `showModal()`, which provides a focus trap, ESC-to-close, and inert siblings as native browser behaviours… We do not run our own `FocusScope` wrapper inside the dialog… We do not run our own `aria-hidden` outsider machinery.

One native element closes **four** gaps at once for modal dialogs — focus trap, focus restore, background inertness, and top-layer rendering that escapes every ancestor `overflow`, `transform` and stacking context. `inert` is also strictly stronger than `aria-hidden`: it removes background content from the accessibility tree *and* blocks focus and pointer interaction, where `aria-hidden` only does the first.

So the best plan is **smaller** than "port four modules". Prefer platform features; fall back to JavaScript only where no platform feature exists.

**Two caveats gated this. Both have now been investigated.**

### Caveat 1 — why upstream left `<dialog>`: no recorded reason

The history, in order:

| Date | Commit | |
|---|---|---|
| 2025-04-11 | `b3f6de53` "progress: dialog" (Miles) | **Added** `<dialog>` + `showModal()` — and `is_modal` in the same commit; `dialog.rs` did not exist before it |
| 2025-06-12 | `797b343e` "Fix keyboard navigation for the dialog and toast components (#47)" (Evan Almloff) | **Removed** `dialog.showModal()` and the `dialog {` element; added `window.createFocusTrap` and a document-level Escape listener. **In upstream `main`** |
| 2026-05-13 | `7b25d863` "Port Dialog to native `<dialog>` + show_modal" (dignifiedquire) | Went back the other way — **in that fork only** |

PR #47's description says only: *"Modals need extra logic to handle focus. This PR implements focus traps for dialog and alert dialog, and implements the f6 keyboard shortcut to focus toasts."* No architectural rationale, no mention of `<dialog>` at all; the conversation thread did not load when fetched.

**Conclusion:** the move away from `<dialog>` looks *incidental* to a focus-trap PR rather than a considered rejection of the platform feature. That is not the same as proof it was accidental — ask upstream before proposing a return. But the evidence does not show a deliberate decision to avoid `<dialog>`, which was the risk this caveat was raised against.

> **Update (2026-08-30) — investigated further and spiked.** Deeper archaeology found the concrete defect behind PR #47: `b3f6de53`'s implementation was **one-way** — `showModal()`/`close()` driven from the `open` signal with zero `onclose`/`oncancel` sync, so a native Escape stranded the signal at `true` and the dialog could never reopen; exactly issue #45's "cannot be used with just the keyboard". Dioxus had (and still has) no typed dialog-event binding — `dioxus#2962` (onclose, open since 2024-09) and `dioxus#4319` (dialog methods, filed 12 days *after* #47 merged). So the removal was a rational retreat from a one-way binding forced by a framework gap, compounded by uniformity (AlertDialog and Toast never used `<dialog>`; one FocusTrap fixed all three). **Every load-bearing claim in this section has now been verified by execution** — the stranded-signal repro, the close-sync fix, top-layer/inertness/focus-restore subsumption, free nested dialogs, and `popover="auto"` all confirmed in Chromium, plus two hazards reading did not predict — of which round 2's root-cause pass **falsified one**: the claimed scroll-lock bypass behind a native `<dialog>` was a test artifact (`locator.click()` auto-scrolled the trigger into view before the lock engaged), and plain `overflow:hidden` in fact blocks wheel/keyboard scroll behind a native modal correctly — no `position:fixed` freeze needed, no scroll-position jump to compensate for. What survived scrutiny: (1) the silent no-`showModal` state — root cause: Dioxus commits the declarative `open` attribute during render, *before* the effect calling `showModal()` runs, so a runtime guard reads stale state; solved by construction with a cfg-split component body whose web arm never binds `open` and whose native arm never calls eval — and the correct split axis is a **renderer feature flag, not `target_family="wasm"`**, because dioxus-desktop is a non-wasm binary with a fully working webview `eval`; and (2) a real scrollbar-gap layout shift present in the shipped `scroll_lock.rs` **today** (15px under a classic scrollbar; invisible on overlay-scrollbar platforms and in headless CI), fixed by construction with a permanent `scrollbar-gutter: stable` baseline + `overflow-y` toggle — the textbook `padding-right` recipe was falsified for `position:fixed` elements, and a transient gutter toggle shifts overlay-scrollbar platforms instead. Details, test-by-test, both rounds: [`phase4-spike-findings.md`](./phase4-spike-findings.md).

**Blitz compatibility is unlikely to have been the motive.** Three arguments weigh against it — circumstantial, not dispositive:

1. **The replacement is *more* JavaScript-dependent, not less.** `<dialog>` + `showModal()` is declarative markup plus one call; what replaced it is `document::eval` for the focus trap *and* a document-level Escape listener. On Blitz, `eval` is a no-op, so the replacement is strictly *more* broken on native than what it replaced — a `<dialog open>` at least renders there under Blitz's UA stylesheet. A Blitz-motivated change would have moved toward declarative markup, not away from it.
2. **Native is not in CI.** No workflow in `.github/workflows/` mentions native, desktop or Blitz; CI covers web, SSG, Playwright and stylelint. A target that is never built is unlikely to have driven the design.
3. **The PR frames itself purely as a focus/keyboard fix**, and never mentions `<dialog>`.

A more plausible motive — *hypothesis, not evidence* — is uniformity across `is_modal: true`/`false`. Native `<dialog>` behaves very differently in the two modes (`showModal()` gives top layer, backdrop and inertness; `show()` gives none of them), whereas one custom implementation behaves identically in both. The added Escape listener's own comment points the same way: *"we can't just add this to the dialog itself because it might not be focused if the user is highlighting text or interacting with another element."*

> **Update (2026-09-01) — the axis caveat above was correct, and shipped anyway: a production incident, root-caused and fixed.** The 2026-08-30 update (above) already named the fix in passing — "the correct split axis is a renderer feature flag, not `target_family="wasm"`" — but nothing had yet forced the point. This update is that forcing: the deployed site (https://mentalgear.github.io/dioxus-components/), built by CI as fullstack SSG (`.github/workflows/web.yml`: `ssg: true, features: fullstack`), was broken page-wide — every overlay's events silently stopped working after a hard page load ("Dropdown Menu → Open Menu: nothing happens" was the reported symptom). Root cause, confirmed by execution (reproduced locally byte-for-byte against the live deployed HTML): the SSG prerender runs the **server binary**, a host (non-wasm) build — and every migrated overlay component (`dialog.rs`, `alert_dialog.rs`, `popover.rs`, `tooltip.rs`, `hover_card.rs`, `dropdown_menu.rs`, `context_menu.rs`, `menubar.rs`, `toast.rs`, `top_layer.rs`, `select`/`combobox`'s list components) still split its *rendered markup* on `#[cfg(target_family = "wasm")]`. That predicate is false on the SSG server's host binary, so the server rendered the **native arm** (plain `div`s, no `popover` attribute, no `<dialog>` elements) while the wasm client then hydrated against that structurally different tree — a server/client markup mismatch, not a cosmetic one, and one Dioxus's hydration model has no graceful recovery for: event delegation is wired by walking the hydrated tree, so this one mismatch silently left listeners unattached page-wide, not just on the mismatched element.
>
> **The corrected rule, stated precisely** (this is the rule `scripts/check-cfg-axis.sh` now enforces mechanically): rendered markup, component structure, and attribute choice split on the **`web` Cargo feature** (`#[cfg(feature = "web")]` / `#[cfg(not(feature = "web"))]`) — a *renderer* question, true for both the wasm browser client and a host (non-wasm) build with that feature on (the fullstack SSG server, and `dioxus-desktop`'s webview). `target_family = "wasm"` is an *execution-target* question, and stays legitimate only for genuinely wasm-only execution internals nested *inside* a feature-gated hook's body — never at module or component granularity. In practice, this crate has no such internals today: `document::eval` is a cross-renderer Dioxus API that compiles and runs (inertly, absent a real document) on every target, so every leaf hook that used to gate on `target_family = "wasm"` (`use_popover_sync`, `use_popover_shown_while_mounted`, `use_anchor_position_fallback`, `use_dialog_open_driver`, `use_dialog_close_sync`, `use_dialog_backdrop_dismiss`, in `top_layer.rs`/`lib.rs`) now gates on `feature = "web"` too, with no inner target-family guard needed — verified by `cargo check -p dioxus-primitives --features web` (host) and `--target wasm32-unknown-unknown` (wasm) both succeeding with the change in place, and `cargo test -p dioxus-primitives --features web` staying green. The wiring half of the fix (a corrected `cfg` predicate does nothing on its own): `preview/Cargo.toml`'s `server` feature had to gain `dioxus-primitives/web` — before this fix it enabled `dioxus/server` + `dioxus/fullstack` only, so the SSG server build compiled primitives *without* the `web` feature regardless of the cfg predicate's axis. `playwright/oracle/hydration-parity.spec.ts` is the black-box regression oracle for the deployed symptom; this section (plus `docs/backlog.md`'s "SSG lane in CI" item) is the standing prevention.

> **Second divergence class found 2026-09-01: attribute override order.** The axis fix above closes the *structural* mismatch (which markup arm renders), but a component can still SSR/CSR-diverge on a *single element's attribute values* even once both lanes render the same web arm. Any component that emits an explicit attribute and then spreads caller `attributes` after it (`div { aria_label: "default", ..attributes }`) is affected whenever a caller overrides that same attribute name: the SSR renderer serializes *both* the default and the override into one start tag, and WHATWG HTML's duplicate-attribute parse error keeps only the *first* (the component's own default) — while the CSR/hydrated DOM path applies attributes sequentially and keeps the *last* (the caller's override). Server and client end up agreeing on tree shape but disagreeing on a computed value, invisible to the hydration-parity rules that check structure/interactivity, but able to fail axe's `landmark-unique` wherever two landmarks are meant to be told apart by a caller-supplied name. Found in `ToastRegionRendered`'s `aria_label` (two `ToastProvider`s on one page) and, once regression-tested generally, also live in `Progress`, `ContextMenuRoot`/`ContextMenuTrigger`, `PopoverTrigger`, and `SelectTrigger` — same root cause, five different components. Fixed by construction with `merge_attributes` (dedupe, caller-wins) rather than plain sequential spreads; `playwright/oracle/hydration-parity.spec.ts` Rule 4 is the standing regression oracle. Details: `primitives/src/toast.rs`'s `ToastRegionRendered` doc ("Attribute-override dedup") and `primitives/src/lib.rs`'s `fold_style_attributes` doc (the one sub-case `merge_attributes` alone cannot fix: a literal `style: "..."` string colliding with the caller's CSS-shorthand style properties, which Dioxus's own SSR renderer combines through an entirely separate code path).

### Caveat 2 — non-browser targets: mostly dissolves

First, a correction to how I framed this: Dioxus *desktop* runs a real webview, so `<dialog>` and `popover=` work there. The non-browser renderer is `dioxus-native`, built on **Blitz**.

Checked against `DioxusLabs/blitz@main`:

| Capability | Blitz |
|---|---|
| `showModal` / `show_modal` | **0 occurrences** |
| `popover` attribute support | **0** |
| top-layer API | **0** |
| `inert` attribute | **0** — its 3 `inert` hits are about `<template>` contents |
| `document::eval` | **`dioxus-native` implements it as `NoOpDocument.eval`** (`packages/dioxus-native/src/contexts.rs`) |
| UA stylesheet | **Full Firefox-derived rules**: `dialog`, `dialog:not([open]) { display: none }`, `dialog:modal`, `dialog::backdrop`, `[popover]:not(:popover-open)` |

The decisive line is `eval`. Because it is a no-op on native, **all 14 `document::eval` sites in `primitives/src` already do nothing there today** — no focus trap, no Escape listener, no `Checkbox` indeterminate sync, and `use_animated_open`'s close path awaits a response that never arrives. The library's interactive behaviour is already effectively web-and-webview-only.

So a native-first Dialog is **not worse** on Blitz than the status quo, and is better everywhere else. With one design detail that must be got right:

- Blitz styles `dialog:not([open])` as `display: none`, so a `<dialog>` is invisible unless the `open` attribute is present — and `open` is a plain attribute Dioxus can bind **declaratively, without JS**.
- Therefore: bind `open` as the floor (giving Blitz a visible, in-flow, non-modal dialog), and call `showModal()` where `eval` actually works (giving web/webview the top layer, backdrop, inertness and focus trap).
- Do not do both blindly: `showModal()` on a dialog already opened via the attribute throws `InvalidStateError`. Close first, or gate the attribute on whether the modal path succeeded.

#### Is Blitz going to support this?

It is on the roadmap, endorsed by a Blitz maintainer, with no evidence of imminent delivery:

- **Issue [#196 "Implement the popover API"](https://github.com/DioxusLabs/blitz/issues/196)** — opened by `nicoburns` in **March 2025**, still **open**, and the issue page reports *no branches or pull requests* linked. Roughly seventeen months without linked work.
- No `<dialog>`/`showModal` issue exists at all.
- The UA stylesheet already carries `dialog:modal`, `-moz-top-layer`, `dialog::backdrop` and `[popover]:not(:popover-open)` — but that is inherited Firefox styling with no implementation behind it.

**The reasoning in that issue matters more than its status.** `nicoburns` argues for the popover API precisely because it lets *"stateful popovers such as menus and tooltips be implemented in pure HTML/CSS (without JavaScript)"*, which *"aligns well with Blitz's lack of JavaScript support."*

That is the same argument this document makes, from the renderer's side — and it flips the risk calculation. A native-first `dioxus-primitives` is **aligned with where Blitz is heading**: if and when popover and dialog land, those behaviours arrive on native for free. The current JS-based approach can *never* work there, because `eval` is a no-op by design rather than by omission. So native-first is not merely the better web choice; it is the only one of the two that has a path to working on native at all.

Do not, however, plan a schedule around it. Seventeen months open with no linked PR is the number to weigh, not the intent.

**Net effect on the recommendation:** caveat 2 no longer blocks item 2 or item 3, but it does mandate the declarative-`open` floor rather than a JS-only path. Caveat 1 remains a question for upstream, not a demonstrated objection — and specifically not a Blitz-compatibility decision.

---

## Per-gap recommendations

### 1. Form participation — `RadioGroup`, `Select`, `Switch`

**Build:** this repo's own `Checkbox` pattern, with Radix's attribute completeness.

| Decision | Take from | Why |
|---|---|---|
| A real submittable element | Both agree | HTML admits only `button`/`input`/`select`/`textarea` to the entry list; ARIA cannot change this, and `ElementInternals` is unavailable because Dioxus renders plain DOM |
| Hiding technique | **This repo** (`checkbox.rs:279-296`) — `aria_hidden`, `tabindex="-1"`, `position:absolute; opacity:0; pointer-events:none` | Already in-tree, already reviewed, consistent |
| Render unconditionally | **This repo**, over Radix's `closest('form')` gate | Radix's gate needs a DOM query; in Dioxus that is a `document::eval` round-trip and a hydration hazard. Radix itself defaults the gate to `true` when it cannot measure |
| Attribute set — `required`, `disabled`, `form` | **Radix** | This repo forwards `name`/`value` but drops `required` on `Switch`; `form` allows a control outside the `<form>` |
| `Select`: hidden native `<select>` with mirrored `<option>`s | **Radix's BubbleSelect**, via `dignifiedquire@select/components/select.rs:158-186` | A real `<select required>` gets full native validation UI, which a hidden `<input>` cannot provide |

Not novel work: the pattern is in-tree and simply was never extended.

#### Why upstream diverged from Radix here — and who to follow

It looks like a style preference. It isn't: **it is a documented framework limitation**, and upstream wrote it down.

`complaints.md` in this repository, under *"No way to know a component or element's parent, siblings, or children"*, cites **the exact Radix line** this decision turns on:

> Take [radix-primitives' switch](https://github.com/radix-ui/primitives/blob/6e75e117977c9e6ffa939e6951a707f16ba0f95e/packages/react/switch/src/switch.tsx#L51) as an example. It detects when the switch is in a form and creates an input so that the switch's value bubbles with the form submit event.

Radix's gate is `control.closest('form')` — a synchronous ancestor query available in React the moment a ref is attached. Dioxus offers no equivalent: `MountedData` exposes focus, scroll and rect operations, not ancestor traversal. Reproducing the gate would require a `document::eval` round-trip per control instance — asynchronous, racy against first paint, and a hydration hazard under the SSG/fullstack builds this repo's `web.yml` produces, since the server would render one tree and the client another.

**Follow upstream: render unconditionally.** Four reasons:

1. The conditional gate is not implementable here at acceptable cost, and upstream had already identified precisely this blocker.
2. **Radix's own fallback is to render it.** When `control` is null and it cannot measure — during SSR — `isFormControl` defaults to `true`, with the comment *"so that events bubble to forms without JS"*. In Dioxus, "cannot measure" is the permanent condition, so unconditional rendering *is* Radix's documented behaviour for that case.
3. The cost is negligible. The input carries `aria-hidden` and `tabindex="-1"` and is visually removed, so it is inert to assistive technology, keyboard, and layout alike. The residual risks are cosmetic — browser autofill and password-manager heuristics may notice a stray named input, and it appears in `document.querySelectorAll('input')`.
4. It matches `Checkbox`, which is already in-tree and reviewed. Consistency inside one library beats fidelity to another library's constraint.

Two things to carry over from Radix regardless: forward the **`form` attribute**, which gives users an explicit way to associate a control rendered outside its `<form>` — the deliberate escape hatch that makes the missing gate a non-issue — and forward **`required`**, which `Switch` currently drops.

Revisit only if Dioxus gains ancestor awareness. That `complaints.md` entry is still open: `DioxusLabs/dioxus@main` (0.8.0-alpha.1) has no parent-traversal API, so this is a framework gap worth filing upstream rather than a decision to keep relitigating.

### 2. Modal dialogs — focus trap, focus restore, inertness, top layer

**Build:** native `<dialog>` + `showModal()`, per `dignifiedquire` — *if* caveat 1 clears.

That single change subsumes the ports of `focus_scope.rs` (743 lines) and `aria_hidden.rs` (91), and removes the vendored `focus-trap.ts`/`.js` pair along with its generated-file trap. Nested modals — which the current `FocusTrap` cannot handle, having no `pause()`/`unpause()` — become a browser concern.

If caveat 1 blocks it, fall back to: `dignifiedquire`'s `aria_hidden.rs` (marker-based so nested overlays don't unhide each other), **wired into `Dialog` and `AlertDialog`, which that fork never did**, and prefer setting `inert` over `aria-hidden` where the target browsers allow.

### 3. Non-modal overlays — menus, popovers, tooltips

**Build:** `popover="auto"` / `"manual"` (`dignifiedquire@top_layer.rs`, 189 lines) for top-layer rendering and light dismiss, with the same fallback requirement as caveat 2.

This fixes clipping inside `overflow:hidden` and transformed ancestors — which no CSS workaround can.

**2026-09-02 addendum — an anchored overlay's tracking must be unconditional and visual-viewport-aware.** Found from a user report (iOS Safari: the Combobox options list rendering on top of its own search input after focusing/typing) and fixed in `use_anchor_position_fallback` (`primitives/src/top_layer.rs`): a JS-measured positioning fallback that only re-measures once, or only while it already believes it is the active path, is not actually maintaining a lifetime guarantee — it is maintaining a snapshot from whenever it last happened to run. "This engine's anchor-positioning integration looked correct at open" is a verdict about that one instant, not a verdict that holds for the rest of the overlay's open lifetime; an on-screen keyboard appearing after open (summoned by the very focus that opened the overlay, for a text-input trigger like Combobox) is exactly the kind of layout change such a snapshot cannot see. Two changes together closed this, and either alone would not have: (1) tracking listeners (`scroll`/`resize`) now install for the **entire** open lifetime of every anchored overlay, not gated on whether the first measurement happened to need the fallback — the existing `matches()` re-check is what keeps a still-correctly-anchored engine's extra re-checks a costless no-op, so this is not a behavior change for the common case; (2) every viewport read goes through `window.visualViewport` (falling back to `window.inner{Width,Height}` when absent), since `window.inner*` does not shrink for an iOS on-screen keyboard while `visualViewport` does — a fix that got (1) right but kept reading `window.inner*` would re-measure on schedule and still compute the wrong answer. Regression oracle: `playwright/oracle/tier2-html/top-layer.spec.ts` Rule 11.

**2026-09-03 addendum — the opening-gesture false positive, and `Navbar`'s missing migration (user device review, iOS 18 Safari).** Two separate findings from the same review round, both fixed by construction rather than reproduced on this sandbox's Chromium (see each's own doc for exactly what could and could not be reproduced here):

- **Light dismiss/backdrop-dismiss must not see the click that opened the overlay.** Report: "Show Popover" (the home page's default, modal `Popover`) shows only its `::backdrop`; the popover itself flashes and disappears. The modal arm's only dismissal path is `use_dialog_backdrop_dismiss` (`primitives/src/lib.rs`) — a JS `click` listener on the `<dialog>` element, discriminating backdrop-vs-content clicks by a bounding-rect test. In this repo's Chromium, that listener structurally cannot see the opening tap's own `click` (its `target` is fixed to the trigger button — a DOM sibling of the dialog, not an ancestor — for the event's entire dispatch), so this stays unreproduced here; but iOS Safari's touch-to-click synthesis computes a synthesized click's target lazily, and `showModal()` mutating the DOM between `touchend` and that synthesis can retarget it at the freshly-shown dialog, landing at the *trigger's* on-screen position — outside a trigger-anchored (not viewport-centered) modal `Popover`'s own box, which the rect test then reads as "outside" and dismisses. Fixed regardless of the exact per-engine mechanism: `use_dialog_backdrop_dismiss`'s listener attachment is deferred by one `requestAnimationFrame`, so the opening gesture's own click — and any same-gesture follow-up event an engine might synthesize immediately after it — can never reach the listener at all, by construction, not by guessing at Safari's exact timing; a genuinely later dismiss click is unaffected (one frame is far shorter than a real second tap or a test framework's own action round-trip). The `Auto`-kind popovers (`DropdownMenu`/`Menubar`/`Select`/non-modal `Popover` — see `PopoverKind`'s doc) are exposed to the same *class* of bug through the browser's own native light-dismiss algorithm instead (no listener of this crate's own is in the loop there at all, so this construction doesn't reach them) — audited, confirmed not reproducible here either, and left unfixed this round: every one of those triggers is a toggle button, and `popovertarget`'s default "toggle" action risks a double-toggle race against this crate's own Rust-driven toggle with no way to verify the outcome without a real device. Tracked as a residual risk, `docs/backlog.md`. Regression oracle: `playwright/oracle/tier2-html/native-dialog.spec.ts` Rule 8 (a black-box touch-emulated open, which cannot fail here either way, plus a construction-level check that the listener is genuinely deferred).
- **`Navbar` never migrated onto the top-layer engine at all.** User question: "why does the Navbar menu not auto-flip when it gets cut off at the bottom/top like the other menus?" Grepping `navbar.rs` for `top_layer`/`use_anchor_position_fallback`/`popover` returned nothing — `NavbarNav`'s Migration A (Phase 4.4) slice was simply never done, so its content stayed a plain CSS-positioned `div` (`position: absolute; top: 100%; left: 0`) with none of the clipping-escape, top-layer-stacking, or viewport-edge-flip properties `DropdownMenu`/`Menubar`/`Select` already have. Fixed by the same construction as `Menubar` (the closest sibling: a per-nav trigger/content pair, no configurable `side`/`align`, always below/start-aligned): `NavbarContentRendered`'s web arm renders `popover="auto"`, anchored to its own trigger via `anchor_name_style`/`position_anchor_style`, carrying a new `dx-anchor-navbar` marker class added throughout the shared, engine-injected anchor-positioning stylesheet (`ensure_anchor_positioning_styles`, `primitives/src/top_layer.rs`); the native (Blitz) arm is unchanged. `preview/src/components/navbar/style.css`'s `.dx-navbar-content` needed the same `overflow: visible; border: none;` UA-popover-stylesheet reset `Menubar`/`DropdownMenu` already carry (their own precedent, not a new pattern). Keyboard/role contract untouched (`navbar.spec.ts`, `oracle/tier1-apg/keyboard-matrix.spec.ts` stay green). Regression oracles: `top-layer.spec.ts` Rule 1 (clip escape), Rule 5 (block-axis flip — the reported symptom directly, via a trigger pinned at the viewport's bottom edge rather than a `side` prop, since `NavbarContent` has none), and Rule 11's `ANCHORED_OVERLAYS` array (self-overlap/iOS-keyboard contract) — all RED against the pre-migration plain `div` before this fix, confirmed by execution.

### 4. Focus restore for the menu family

**Build:** Radix's *semantics* on `dignifiedquire`'s *shape*, corrected by our own measurement.

- Behaviour: Radix's `onCloseAutoFocus` — return focus to the trigger **unless** the close came from interacting outside.
- Shape: `dignifiedquire@lib.rs:241-255` (`use_refocus_on_close_unless`), ~15 lines, dropping onto the `trigger_id` fields this repo already has. It needs `use_previous`, which does not exist here and must be written.
- **Correction from execution:** our oracle showed `DropdownMenu` and `Menubar` leave focus *on the item of the closed menu*, not on `<body>`. Restoring to the trigger is therefore necessary but not sufficient — focus must also be moved off the item. Neither reference handles this, because neither has this bug.

### 4b. Keyboard open contract (menu-family triggers)

**Finding from execution (2026-09-02):** `oracle/tier1-apg/keyboard-matrix.spec.ts` found the same shape of bug independently in `DropdownMenu`, `Menubar`, and `Select`: each trigger's open keys (Enter, Space, ArrowDown, ArrowUp) were separate `match` arms, and only some of them also requested initial focus — so e.g. Enter opened `DropdownMenu` but left focus stranded on the trigger, while ArrowDown on the same trigger correctly focused the first item. Patching the under-covered arms one at a time would have left the next component with the same footgun.

**Build:** one "open with focus" path per component, so opening and requesting focus can never drift apart again:

- `DropdownMenu`: `DropdownMenuContext::open_with_focus(CollectionPlacement)` sets `initial_focus` then `open`, in that order, before any render can observe an inconsistent state. `DropdownMenuContent` resolves it via the existing `use_deferred_collection_focus` (already used by `Menubar`), so focus lands correctly regardless of whether the content is already mounted.
- `Menubar`: extended the existing `initial_focus: Signal<Option<CollectionPlacement>>` (already used by the ArrowDown/Up arms) to Enter and Space too, and wired Space explicitly at `MenubarMenu`'s `onkeydown` — `MenubarTrigger` wires only `onpointerup`, never `onclick`, so a Space keydown's synthesized click previously had no listener at all.
- `Select`: `SelectContext::open_with_selected_or_first_focus()` is the shared path for Enter and Space, focusing the selected option if one exists (else the first) — APG select-only combobox's "focus the listbox with the current option active." Alt+ArrowDown is the one open key that must *not* move focus at all (APG Optional); a new `keep_trigger_focus` flag on `SelectContext` suppresses `SelectListRendered`'s own "nothing is focused yet, focus the listbox container" fallback for that one path, so the trigger keeps real DOM focus. Plain pointer-click open is deliberately untouched on all three components — it already has its own APG-permitted behavior (focus stays on/near the trigger) with its own green coverage (`dropdown-menu.spec.ts`, `select.spec.ts`).

Cited rule source throughout: each pattern's own "Keyboard Interaction" section, quoted per-row in the matrix oracle rather than from memory.

**Two sibling findings from the same oracle sweep, different bug shape (not an open-contract drift, so not folded into the pattern above):** `Slider` had `Home`/`End`/`Page Up`/`Page Down` entirely unimplemented (`MoveEvent::from_keyboard` matched only the four arrow keys) — fixed with a `home_end_target` helper and the existing "10x step" convention Shift+Arrow already used, both unit-tested (this module is mutation-tested). `HoverCard` had no `onkeydown` handler at all, unlike `Tooltip`'s `handle_keydown`, so it never dismissed on Escape despite being built to the same show-on-focus/hide-on-blur contract Tooltip's own APG citation requires — fixed by wiring the same handler onto `HoverCardTrigger` and `HoverCardContent`. Together with the pattern above, these closed all 12 reds `oracle/tier1-apg/keyboard-matrix.spec.ts` found.

### 4c. Pattern-class role contract (menu-family roles)

**Finding from execution (2026-09-02, `docs/backlog.md` row 24):** `DropdownMenu` rendered the APG **listbox** pattern's roles (`aria-haspopup="listbox"` / `role="listbox"` / `role="option"`) while `ContextMenu` and `Menubar` — two other implementations of the same APG **menu-button**/**menu-and-menubar** pattern class, built on the same `use_item`/`collection_item` collection plumbing — correctly used the **menu** pattern's roles (`aria-haspopup="menu"` / `role="menu"` / `role="menuitem"`). `DropdownMenu` has no selection model at all (no `value`/`selected` state, no `aria-selected` on any item; activating an item calls `on_select` and closes the menu — action semantics), so the listbox roles were simply wrong, not a stylistic choice. Root cause: each component hand-wrote its own role/token string literals, so three implementations of one pattern class could — and did — drift independently.

**Build:** a single shared module, `primitives/src/menu_semantics.rs`, holding the pattern class's three literals as `pub(crate)` consts (`MENU_ROLE`, `MENU_ITEM_ROLE`, `MENU_TRIGGER_HASPOPUP`), each doc-commented with its APG citation. `dropdown_menu.rs`, `context_menu.rs`, and `menubar.rs` all read their `role`/`aria-haspopup` attributes from this module instead of hand-written strings, so a role can only drift again if the shared module itself is edited — a fourth menu-pattern component added later inherits the correct roles by construction rather than by remembering to copy them. Verified by `oracle/tier1-apg/menu-roles.spec.ts`, calibrated against the vendored APG menu-button-actions.html reference (see that file's header for the exact citations) rather than against another of this library's own components.

The general lesson generalizes past this one pattern: **when the same conformance rule has already been implemented correctly by sibling components, extract what they agree on into one shared definition** rather than re-deriving or hand-copying it into the next component — the oracle then checks that the shared definition is actually used everywhere, not just that each component's own literals happen to currently agree with each other.

### 5. Body scroll lock

**Build:** `dignifiedquire@scroll_lock.rs` (58 lines) as the base — it refcounts for nested modals *and* restores the original `overflow` value, which `sarendipitee`'s does not — plus `sarendipitee`'s guard against the unlock flash when a second modal opens while the first is tearing down.

Known limitation in **both**: no iOS momentum-scroll handling and no scrollbar-gap compensation. Radix delegates this to `react-remove-scroll`; we have no equivalent. Decide whether that matters before claiming the gap closed, and note that native `<dialog>` does **not** itself prevent background scrolling — this is needed regardless of item 2.

### 6. Collision detection

**Build:** `sarendipitee@floating.rs` (269 lines) — a thin `use_position()` hook over the external `floating-ui-dioxus`/`-dom` crates, reusing this repo's existing `ContentSide`/`ContentAlign` names, with a `#[cfg(target_family = "wasm")]` split and a native fallback that reproduces today's CSS behaviour.

Prefer it over `dignifiedquire`'s in-repo port (3,262 lines of Rust + a 1,158-line `popper.rs`) for maintenance reasons, but keep that port as the contingency if the external crates stall — it additionally implements `collision_padding`, sticky behaviour, arrow and size middleware.

Keep the CSS clamp already on this repo's `fix/preview-a11y-ux` branch as defence-in-depth: it costs nothing and still helps non-wasm targets (still not merged as of 2026-09-02 — verified not an ancestor of `main`). `ContextMenu` needs separate viewport clamping either way, since it is positioned at click coordinates rather than anchored.

> **Update (2026-09-01) — the FLIP sub-problem no longer needs either dependency.** A bits-ui/Radix source comparison found both mature libraries delegate to Floating UI's identical flip/shift/size/arrow/hide pipeline (bits-ui is a line-for-line Svelte port of Radix's Popper) — but by this date CSS Anchor Positioning was already in this repo's stack from Phase 4.4, and `position-try-fallbacks: flip-block, flip-inline` covers FLIP natively, zero JS, additive to the existing `@supports` anchor blocks. Landed the same day: one declaration in each of the three `@supports` blocks, plus making `use_anchor_position_fallback` flip-aware so it defers to a legitimate CSS flip and only overrides when neither the primary nor flipped placement fits. Neither `sr floating.rs` nor `dq`'s vendored port was needed for this. The dependency decision above therefore survives only for what's left: shift/size clamping and `ContextMenu`'s point-anchor clamp (the latter is not solvable by CSS anchors at all — it needs a virtual-anchor JS path regardless of which collision library, if any, gets adopted). See `backlog.md` row 10 and `plan.md` Phase 5.

> **Update (2026-09-03) — the JS fallback's first piece of shift, closing a real device report.** User report: the home page widget-masonry `ColorPicker`'s overlay clips against the viewport's edge on a small screen (iOS 18 Safari, no CSS Anchor Positioning support at all there — confirmed by the report — so this engine always runs the JS fallback, never the CSS-native path). Root cause, found by inspecting `use_anchor_position_fallback` rather than assuming the stated hypothesis ("no inline-axis handling"): inline-axis **flip** was already implemented (the `opposite` map already swaps `side="left"`/`"right"` on overflow) — the real gap is that a `side="top"`/`"bottom"` placement's horizontal position comes entirely from `align` and was never checked against the viewport at all, on *either* engine (this crate's `@supports` block declares no shift primitive, so a CSS-conforming engine has exactly the same gap for this one case — just not the one the reported device hits). Fixed with a plain clamp on `target.left` into `[EDGE_MARGIN, vw - EDGE_MARGIN - cw]`, applied after the existing flip decision in the same `reposition()` function so scroll/resize tracking re-clamps it for free; deliberately not mirrored into the CSS `@supports` contract this round (no `position-try-fallbacks` keyword shifts along an axis the way this clamp does — a materially larger change than this device's gap needs), so a genuinely conforming engine is untouched, exactly as the existing flip early-return already keeps it. Confirmed by execution: reproduced (both the reported component, and two of `top-layer.spec.ts`'s own `edge-bottom-*` fixture cases landing over 190px past the viewport edge) under the same no-anchor-engine simulation Rule 11 already uses (`stripAnchorSupportsBlock` + `MOBILE_VIEWPORT`); RED before, GREEN after. Regression oracle: `top-layer.spec.ts` Rule 12.

### 7. `use_animated_open`

**Build:** neither patch as written.

`jcgruenhage@6f0a69f0` is the correct base — it catches at the end of the chain and deliberately *declines to send*, so a stale task cannot overwrite newer state. `ziimakc@573cc1e9` (upstream PR #291) swallows per-animation rejections, which lets a stale close task set `show_in_dom = false` after a reopen: a stuck-open element becomes one that vanishes while open.

Add what neither has: a **per-cycle generation counter**, so the rejection path *can* resolve when it is still the current cycle. That closes `jcgruenhage`'s residual case, where an animation cancelled with no successor leaves the element mounted.

### 8. Typeahead

**Build:** `dignifiedquire@typeahead.rs` (78 lines, prefix matching) for the *menu* family only — and **do not touch `select/`**, whose Levenshtein, keyboard-layout-aware matcher with a configurable timeout is better than both Radix's and the fork's. Its only dependency, `dioxus_sdk_time::sleep`, is already used in-tree.

Best-of here means recognising that upstream already wins one.

### 9. RTL

**Build:** `dignifiedquire@direction.rs` (83 lines) as-is for the context/provider, and port only the *concept* of `direction_aware_key()` into this repo's existing `collection.rs` handlers. Do not take the 708-line `roving_focus.rs`, which would replace a collection system that is already correct.

### 10. Text-entry font-size floor (iOS Safari touch-zoom)

**The rule:** every focusable text-entry element (`input` of a text-like type, `textarea`, `select`, `[contenteditable]`) must compute `font-size >= 16px` at a coarse-pointer (touch) viewport. This is documented WebKit/Apple platform behaviour, not a W3C rule -- Mobile Safari zooms the page in when a focused text field's computed font-size is below 16px, and zooms back out on blur -- so no other engine reproduces it and no spec mandates it (see `playwright/oracle/tier2-html/touch-focus-zoom.spec.ts`'s header for the exact citation).

**Why `maximum-scale=1`/`user-scalable=no` is not the fix:** that suppresses the *symptom* by disabling the user's own pinch-zoom globally, which fails WCAG 2.1 SC 1.4.4 "Resize Text" (no exception for form fields) -- trading a minor UX annoyance for a real accessibility regression for low-vision users. `preview/index.html`'s viewport meta stays zoomable; it was already correct and needed no change.

**Build:** shadcn's own `Input` -- `text-base md:text-sm`, i.e. 16px below the `md` breakpoint and 14px at/above it -- is the construction: float the font-size to exactly 16px under `@media (pointer: coarse)`, change nothing else, so the intentional desktop size is untouched. This repo's `Input`/`Textarea` already did this (`preview/src/components/{input,textarea}/style.css`); the gap was that it had been applied per component by hand rather than by construction.

**Two layers, because a themed component ships without the app's global CSS:**

1. **Component layer** -- every themed component that renders a real (focusable, non-`type=range`/checkbox/radio) text-entry element gets its own `@media (pointer: coarse) { .dx-foo { font-size: 16px; } }`, in that component's own `style.css`, so `dx components add <name>` carries the fix into a consumer's project. Found needing it by execution, not by grepping for `input` (a `select` or a `contenteditable` span is easy to miss that way): `Combobox`'s `.dx-combobox-input` (0.875rem), and `Calendar`'s `.dx-calendar-month-select`/`.dx-calendar-year-select` -- an `opacity: 0` native `<select>` overlaid on a styled value span, the standard construction for a styled select whose native touch picker should still open on tap, and therefore a real, focusable tap target despite being invisible.
2. **App layer** -- one catch-all rule in `preview/assets/main.css`, using the same selector shape (every text-like `input`/`textarea`/`select`/`[contenteditable]`, `tabindex="-1"` excluded -- see below), so a raw HTML element written directly in the preview app (not a themed wrapper) can never regress this by omission. Caught by this layer: the navbar's `<select class="dx-language-select">` present on every route, `top_layer`'s and `form`'s deliberately-raw reference/probe controls, and (this app's own `dioxus_primitives::` import exemption, `docs/preview-composition.md`) `top_layer`'s Combobox-primitive fixture trigger. A component- or app-specific CSS override can still beat both floors on specificity -- found in the dashboard email client, whose `.ec-thread-compose-row [data-slot="textarea"]` (0.9375rem) out-specifies `Textarea`'s own `.dx-textarea` floor and needed its own matching rule in `email_client.css`.

**One exclusion, found by execution:** `Select`'s hidden native `<select data-slot="select-native">` (the "BubbleSelect" form-participation pattern, §1 above) sets `tabindex="-1"` alongside `pointer-events: none` and `opacity: 0` -- unreachable by tab or tap, so the platform behaviour this rule is about can never trigger on it. `touch-focus-zoom.spec.ts`'s selector excludes `[tabindex="-1"]` on that basis, not to paper over a gap.

---

## Testing — best of all three suites

| Take | From | Why |
|---|---|---|
| Playwright e2e in real browsers | **This repo** | 32 specs / 123 tests, strong on keyboard (167 `keyboard.press`, 84 `toBeFocused`) — keep as the backbone |
| Per-component axe assertions | **Radix** (`vitest-axe`) | This repo runs axe in only 3 of 32 specs; Radix does 7 calls in checkbox alone |
| Cargo unit tests for algorithms | **This repo** | Already the right split — 10 of 63 modules, the algorithmic ones |
| Entry-list assertions (`FormData` after submit) | **Nobody** | Radix has zero `FormData` assertions; this is genuinely new and rests on the HTML spec |
| Tiered rules with external calibration | **New** — see [`conformance-harness.md`](./conformance-harness.md) | Every rule traceable to APG, HTML, or a labelled opinion |
| In-process keyboard tests | `hovinen`'s `dioxus-test` branch | Optional; needs Dioxus 0.8, so not near-term |

## Sequence

> Superseded by [`plan.md`](./plan.md), which reconciles this with the batches in `adopt-fork-fixes-results.md` §0 and the queue in `lifting-from-forks.md` §7. Kept here for the per-gap rationale.

1. **Ask upstream about caveat 1** — the history shows no recorded rationale, so this is a question, not a blocker. It does not need to be answered before item 1.
2. **Form participation** (item 1) — highest severity, no caveats, applies an in-tree pattern.
3. **Scroll lock** (5) — needed regardless of how 2 resolves.
4. **Focus restore** (4) — the oracle is already written and red.
5. **`use_animated_open`** (7) and the other three mined fixes.
6. **Collision detection** (6) — largest, and a dependency decision.
7. Typeahead (8), RTL (9).

Write the failing test before each, per the harness document. This ordering is historical (it predates execution); `plan.md`'s phase tables record what has actually landed, in what order it actually happened.

### Global stylesheet rule — never `@import` a remote stylesheet from the app stylesheet (2026-09-02)

A stylesheet is applied only once it is loaded, and it is not loaded until every `@import` it contains has loaded. `preview/assets/main.css` opened with an `@import` of Google Fonts; wherever that request stalled (a slow CDN, a corporate proxy, this repo's own test sandbox) the whole file — layout, navbar, typography, every app-wide backstop — silently never applied, while `link.sheet` still exposed the parsed rules, which is why nothing looked broken from the inside. Construction: web fonts load from `<link rel="preconnect">` + `<link rel="stylesheet">` in one shared head component (`GlobalHead`), the app stylesheet contains no `@import`, and `oracle/tier2-html/global-stylesheet.spec.ts` asserts on every route that `main.css` is in the applied set. Same class as the cfg-axis and attribute-order incidents: a rule that held on the developer's machine and failed silently elsewhere, closed by an oracle that measures the deployed shape rather than the source.

