# css_module silently drops class scoping inside @supports blocks

Status: workaround shipped (PR #16); upstream Dioxus report drafted, not yet filed. GitHub Issues is disabled on this repo — this file is the tracking record. Drafted 2026-09-03, not filed — user decision: [`docs/issues/drafts/dioxus-css-module-supports-scoping.md`](./drafts/dioxus-css-module-supports-scoping.md) (ready-to-paste version of this report, upgraded with reproduction steps and a "before filing" checklist).

## What happens
Any CSS rule wrapped in `@supports { … }` inside a `#[css_module(...)]` stylesheet silently never applies. No error, no warning — dead styles.

## Mechanism
`#[css_module]` scopes every class by appending a hash (`.dx-tooltip-content` → `.dx-tooltip-content-b50b2adc`) in both the DOM and the stylesheet. The scoping pass (`manganis-core`'s `css_module_parser`) only recurses into `@media`, `@layer`, `@container`, and `@include`; every other at-rule — `@supports` included — is consumed by `unknown_block_contents` as an opaque blob and never scanned. The DOM gets the hashed class; selectors inside `@supports` keep the unhashed name and can never match, in any browser.

## How it bit us
Phase 4.4 gated the CSS Anchor Positioning rules for Tooltip/HoverCard/Popover behind `@supports (anchor-name: --a)`. The entire enhancement was dead code everywhere; elements fell back to the `[popover]` UA stylesheet (unexpected 3px border, `margin:auto` centering throwing content hundreds of px off-trigger). Root-caused by computed-style measurement; fixed in #16.

## Workaround convention (binding until upstream fixes the parser)
Selectors used inside `@supports` must reference plain, hand-written marker classes (`dx-anchor-*`) that are never referenced outside `@supports`, so the scoping pass cannot rewrite their DOM side and they match by construction.

## Upstream fix
`manganis-core::css_module_parser`: recurse into `@supports` (arguably any unknown block at-rule) like `@media`.
