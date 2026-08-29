# Recommended implementations — best of each source

For every gap in [`capability-gaps.md`](./capability-gaps.md), which solution to actually build, assembled from the strongest parts of upstream, Radix, and the forks rather than copying any one of them wholesale.

Sources compared: this repo (`bf007c1`), `radix-ui/primitives`, `dignifiedquire/dx-components`, `sarendipitee/dioxus-components`, `jcgruenhage/dioxus-components`, and the WHATWG HTML spec.

**Verification:** all source claims below were read from the actual code. None of the *recommendations* have been built or run.

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

**Blitz compatibility is unlikely to have been the motive.** Three arguments weigh against it — circumstantial, not dispositive:

1. **The replacement is *more* JavaScript-dependent, not less.** `<dialog>` + `showModal()` is declarative markup plus one call; what replaced it is `document::eval` for the focus trap *and* a document-level Escape listener. On Blitz, `eval` is a no-op, so the replacement is strictly *more* broken on native than what it replaced — a `<dialog open>` at least renders there under Blitz's UA stylesheet. A Blitz-motivated change would have moved toward declarative markup, not away from it.
2. **Native is not in CI.** No workflow in `.github/workflows/` mentions native, desktop or Blitz; CI covers web, SSG, Playwright and stylelint. A target that is never built is unlikely to have driven the design.
3. **The PR frames itself purely as a focus/keyboard fix**, and never mentions `<dialog>`.

A more plausible motive — *hypothesis, not evidence* — is uniformity across `is_modal: true`/`false`. Native `<dialog>` behaves very differently in the two modes (`showModal()` gives top layer, backdrop and inertness; `show()` gives none of them), whereas one custom implementation behaves identically in both. The added Escape listener's own comment points the same way: *"we can't just add this to the dialog itself because it might not be focused if the user is highlighting text or interacting with another element."*

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

### 4. Focus restore for the menu family

**Build:** Radix's *semantics* on `dignifiedquire`'s *shape*, corrected by our own measurement.

- Behaviour: Radix's `onCloseAutoFocus` — return focus to the trigger **unless** the close came from interacting outside.
- Shape: `dignifiedquire@lib.rs:241-255` (`use_refocus_on_close_unless`), ~15 lines, dropping onto the `trigger_id` fields this repo already has. It needs `use_previous`, which does not exist here and must be written.
- **Correction from execution:** our oracle showed `DropdownMenu` and `Menubar` leave focus *on the item of the closed menu*, not on `<body>`. Restoring to the trigger is therefore necessary but not sufficient — focus must also be moved off the item. Neither reference handles this, because neither has this bug.

### 5. Body scroll lock

**Build:** `dignifiedquire@scroll_lock.rs` (58 lines) as the base — it refcounts for nested modals *and* restores the original `overflow` value, which `sarendipitee`'s does not — plus `sarendipitee`'s guard against the unlock flash when a second modal opens while the first is tearing down.

Known limitation in **both**: no iOS momentum-scroll handling and no scrollbar-gap compensation. Radix delegates this to `react-remove-scroll`; we have no equivalent. Decide whether that matters before claiming the gap closed, and note that native `<dialog>` does **not** itself prevent background scrolling — this is needed regardless of item 2.

### 6. Collision detection

**Build:** `sarendipitee@floating.rs` (269 lines) — a thin `use_position()` hook over the external `floating-ui-dioxus`/`-dom` crates, reusing this repo's existing `ContentSide`/`ContentAlign` names, with a `#[cfg(target_family = "wasm")]` split and a native fallback that reproduces today's CSS behaviour.

Prefer it over `dignifiedquire`'s in-repo port (3,262 lines of Rust + a 1,158-line `popper.rs`) for maintenance reasons, but keep that port as the contingency if the external crates stall — it additionally implements `collision_padding`, sticky behaviour, arrow and size middleware.

Keep the CSS clamp already on this repo's `fix/preview-a11y-ux` branch as defence-in-depth: it costs nothing and still helps non-wasm targets. `ContextMenu` needs separate viewport clamping either way, since it is positioned at click coordinates rather than anchored.

### 7. `use_animated_open`

**Build:** neither patch as written.

`jcgruenhage@6f0a69f0` is the correct base — it catches at the end of the chain and deliberately *declines to send*, so a stale task cannot overwrite newer state. `ziimakc@573cc1e9` (upstream PR #291) swallows per-animation rejections, which lets a stale close task set `show_in_dom = false` after a reopen: a stuck-open element becomes one that vanishes while open.

Add what neither has: a **per-cycle generation counter**, so the rejection path *can* resolve when it is still the current cycle. That closes `jcgruenhage`'s residual case, where an animation cancelled with no successor leaves the element mounted.

### 8. Typeahead

**Build:** `dignifiedquire@typeahead.rs` (78 lines, prefix matching) for the *menu* family only — and **do not touch `select/`**, whose Levenshtein, keyboard-layout-aware matcher with a configurable timeout is better than both Radix's and the fork's. Its only dependency, `dioxus_sdk_time::sleep`, is already used in-tree.

Best-of here means recognising that upstream already wins one.

### 9. RTL

**Build:** `dignifiedquire@direction.rs` (83 lines) as-is for the context/provider, and port only the *concept* of `direction_aware_key()` into this repo's existing `collection.rs` handlers. Do not take the 708-line `roving_focus.rs`, which would replace a collection system that is already correct.

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

Write the failing test before each, per the harness document. Nothing here has been built.
