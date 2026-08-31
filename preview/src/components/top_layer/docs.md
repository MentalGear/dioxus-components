The Top Layer fixture exercises the Phase 4.4 (`docs/plan.md`) top-layer
promotion — `Tooltip`, `HoverCard`, and the non-modal `Popover` arm all
render with a `popover` attribute on the web arm, escaping clipping and
transformed ancestors (WHATWG HTML §popover, top layer) the way ordinary CSS
positioning cannot.

Every rule the oracle checks (`playwright/oracle/tier2-html/top-layer.spec.ts`)
is calibrated against a **native reference** placed in this same fixture: a
plain `<div popover="auto">` shown by a `<button popovertarget>` — the
browser's own implementation of the identical WHATWG HTML feature, with no
Dioxus involvement at all. If a rule fails on the native reference, the test
is wrong, not the component — the same tier-2 calibration discipline as the
[`form`](../form) fixture.

## Two sections

- **Clipping escape** (`#clip-box`) — an ancestor with
  `overflow: hidden; height: 60px; transform: translateZ(0)` wraps a
  `Tooltip`, a `HoverCard`, a non-modal `PopoverRoot`, and the native
  reference. Each content panel sets an explicit `min-height: 100px`, taller
  than the 60px clip, so opening it and finding it fully visible is a real
  assertion, not a coincidence of a short panel.
- **Light dismiss, Escape, and stacking** — a high-`z-index` sibling
  (`#stack-sibling`) sits where a `Popover` (`#stack-popover-root`) and the
  native reference (`#stack-native-content`) open, to check that top-layer
  content stacks above it. The same instances are reused for the
  light-dismiss (click `#outside-click-target`) and Escape assertions.

## Known gap this fixture exposes

Positioning is CSS Anchor Positioning (`anchor-name`/`position-anchor`/
`anchor()`), not floating-ui or any JS-computed geometry — still purely CSS,
per `primitives/src/top_layer.rs`'s `anchor_name_style`/
`position_anchor_style`. It has no collision/edge-avoidance logic; that is
`docs/plan.md` Phase 5's job, not this slice's, and CSS Anchor Positioning
is not yet implemented in every engine (Chromium-family only, as of this
writing) — an engine without it falls back to the pre-4.4 `[data-side]`
rules, which still escape clipping (the point of this fixture) but without
trigger-relative placement.
