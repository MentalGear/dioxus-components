The Top Layer fixture is the probe surface for the two overlay engines
(`docs/recommended-implementations.md` §2–3, `primitives/src/top_layer.rs`
and the native `<dialog>` path in `primitives/src/dialog.rs`). It is **not an
installable component** — there is no `dx components add top_layer` — which
is why it is listed under Overlays in the sidebar but excluded from the
home-page gallery (`docs/backlog.md` row 43). Its only job is to put every
top-layer surface the library ships next to the browser's own native
implementation of the same feature, so the oracles can tell "the component is
wrong" from "the test is wrong".

Every rule in `playwright/oracle/tier2-html/top-layer.spec.ts` and
`native-dialog.spec.ts` is calibrated against a **native reference** in the
same section: a plain `<div popover="auto">` shown by a
`<button popovertarget>` for the popover engine, and a plain `<dialog>`
opened by `showModal()` for the dialog engine — no Dioxus involved. If a
rule fails on the native reference, the test is wrong, not the component —
the same tier-2 discipline as the [`form`](../form) fixture.

## What it covers

| Surface | Engine | Sections |
|---|---|---|
| `Tooltip`, `HoverCard`, non-modal `Popover` | popover + CSS anchor positioning (JS fallback) | clipping escape, stacking/light-dismiss/Escape, viewport-edge flip |
| `ContextMenu`, `Menubar` | popover engine (Migration A slice 2/3) | clipping escape, point-positioned vs. anchored scroll tracking |
| `DropdownMenu`, `Select`, `Combobox`, `Navbar` | popover engine (Migration A slice 3/3; Navbar 2026-09-03) | clipping escape (Navbar also viewport-edge flip) |
| `Toast` | `popover="manual"` region | top-layer stacking above a high-`z-index` sibling |
| `Dialog` | native `<dialog>` + `showModal()` (Phase 4.2) | clipping escape, background inertness |
| modal `Popover` | native `<dialog>` engine (two-engine completion) | clipping escape, background inertness, anchored placement |

## The eight sections

1. **Clipping escape** (`#clip-box`) — an ancestor with
   `overflow: hidden; height: 60px; transform: translateZ(0)` wraps one
   instance of every popover-engine surface plus the native reference. Each
   content sets `min-height: 100px`, taller than the clip, so "fully visible
   after open" is a real assertion.
2. **Light dismiss, Escape, and stacking** — a high-`z-index` sibling
   (`#stack-sibling`) under a `Popover` and the native reference; the same
   instances serve the outside-click (`#outside-click-target`) and Escape
   assertions.
3. **Toast top-layer stacking** — the toast region above its own
   high-`z-index` sibling (`#toast-stack-sibling`). Composes the raw
   `ToastProvider`; see `docs/backlog.md` row 44 for the device-reported gap
   this composition has.
4. **Point-positioned vs. anchored scroll behavior** — `ContextMenu` (point
   anchor) and `Menubar` (element anchor) under page scroll (Rule 8).
5. **Near-viewport-edge flip** — `Tooltip`, `HoverCard`, `Popover`, `Navbar`
   and the native reference pinned at the right and bottom edges
   (`position-try-fallbacks` on the CSS path, flip + inline clamp on the JS
   fallback, Rules 11–12).
6. **Native `<dialog>` clipping escape** — `Dialog` inside a clip box next to
   a plain `<dialog>`.
7. **Native `<dialog>` background inertness** — a counter button and an input
   behind an open `Dialog` must be inert.
8. **Native `<dialog>` modal Popover** — the modal `Popover` arm on the dialog
   engine: clip escape, inertness, anchored placement, and the
   one-frame-deferred backdrop dismiss (native-dialog Rule 8).

## Composition note

This fixture and the `form` fixture are the two documented exceptions to the
"themed wrappers only" preview rule (`docs/preview-composition.md`): they
compose `dioxus_primitives::` directly so the oracles measure the primitive,
not the theme. That is also why nothing here is styled beyond what a probe
needs — and why a surface that relies on its themed stylesheet for basic
visibility (Toast, row 44) can look broken here while the themed demo is fine.

## Positioning caveat

Positioning is CSS Anchor Positioning (`anchor-name`/`position-anchor`/
`anchor()`/`position-try-fallbacks`) where the engine has it, and
`use_anchor_position_fallback` (`primitives/src/top_layer.rs`) elsewhere:
the fallback measures trigger and content, flips across the block axis and
clamps the inline axis to the visual viewport, and re-measures on scroll,
resize and `visualViewport` changes for the overlay's lifetime. Both paths
are checked by the same edge-flip rules. Remaining gaps are tracked as
`docs/backlog.md` row 10 (shift/size clamping on the CSS path,
`ContextMenu`'s point-anchor clamp).
