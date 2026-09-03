# Draft issue: `css_module`'s class-scoping pass silently ignores `@supports` blocks

**Status:** drafted 2026-09-03, not filed. Ready to paste once the "before filing" checklist below is done. Upgrades the existing tracking record at `docs/issues/css-module-supports-scoping.md` (kept, points here).

---

**Target repo:** `DioxusLabs/dioxus` (the `manganis-core` crate, `packages/manganis-core` in that monorepo — confirm the exact path against the release tag before filing; this repo consumes it only as a versioned dependency, not as a submodule, so no local copy of `manganis-core`'s source exists to link a permalink from directly).

**Versions in use** (from this repo's own `Cargo.lock`, not assumed): `dioxus 0.7.9`, `manganis 0.7.9`, `manganis-core 0.7.9` — all pinned to the same workspace version, resolved from crates.io.

## Title

`#[css_module]` scoping pass silently drops classes referenced only inside `@supports` blocks

## Body

### Context

`#[css_module(...)]` (via `manganis-core`'s CSS parser) scopes every class name it finds in a stylesheet by appending a content hash — e.g. `.dx-tooltip-content` becomes `.dx-tooltip-content-b50b2adc` — and rewrites the corresponding Rust-side class name the same way, so the DOM and the stylesheet agree. This scoping pass recurses into `@media`, `@layer`, and `@container` blocks to find and rewrite classes nested inside them. It does **not** recurse into `@supports` blocks. Any selector defined only inside an `@supports { … }` body is treated as an opaque, unparsed blob (consistent with how the parser also skips `@include` and any other at-rule it doesn't specifically special-case) — its class name is left unhashed in the compiled stylesheet, while the DOM side still carries the hashed name generated from the un-nested part of the file. The two names can never match, in any browser, and no error or warning is emitted anywhere in the build.

### Minimal reproduction

```rust
#[component]
fn Demo() -> Element {
    rsx! {
        div {
            class: "dx-demo",
            "hello"
        }
    }
}
```

```css
/* demo.module.css */
.dx-demo {
    color: black;
}

@supports (display: grid) {
    .dx-demo {
        color: red; /* never applied, in any browser that reaches this build's @supports check */
    }
}
```

Build with `dx-demo`'s stylesheet passed through `#[css_module(...)]`. Inspect the compiled CSS output: the top-level `.dx-demo` rule is rewritten to `.dx-demo-<hash>` and matches the DOM's rewritten class. The `@supports` block's `.dx-demo` rule is emitted verbatim, unhashed, and never matches anything in the rendered DOM.

### Expected vs actual

- **Expected:** the scoping pass recurses into `@supports` bodies the same way it already does for `@media`/`@layer`/`@container`, rewriting any class selector found inside to the same hashed name used elsewhere in the file.
- **Actual:** selectors inside `@supports` are left completely unscoped. The rule is not merely lower-specificity or overridden — it targets a class name that literally does not exist anywhere in the rendered document, so it can never match, and nothing in the build surfaces this as an error.

### Evidence — how this was found and confirmed

Found building this repo's CSS Anchor Positioning support (`docs/plan.md` Phase 4.4): the anchor-positioning rules for `Tooltip`/`HoverCard`/`Popover` were gated behind `@supports (anchor-name: --a)` inside each component's `#[css_module]` stylesheet. The entire enhancement was dead code on every engine, in every environment — elements fell back silently to the `[popover]` user-agent stylesheet default (an unexpected border, `margin: auto` centering placing content hundreds of pixels away from its trigger). Root-caused by comparing the compiled CSS's class names against the DOM's `class` attribute under computed-style inspection: the `@supports` selectors were byte-for-byte present in the output CSS, unhashed, while the DOM's classes were all hashed. Full account and the workaround convention adopted in the meantime: `docs/issues/css-module-supports-scoping.md` in this repo (`MentalGear/dioxus-components`), and `primitives/src/top_layer.rs`'s `ensure_anchor_positioning_styles`/the `dx-anchor-*` marker-class convention used across `dropdown_menu.rs`, `combobox/components/list.rs`, and other migrated overlay components.

### Proposed fix

In `manganis-core`'s CSS parser (the function walking at-rule bodies to find nested selectors — recurses today for `@media`/`@layer`/`@container`), add `@supports` to the set of at-rules it recurses into rather than treats as an opaque blob via `unknown_block_contents`. Arguably any at-rule containing nested rule bodies (rather than declarations) should recurse by default, with an explicit opt-out list rather than an opt-in list, so a future CSS at-rule doesn't reproduce this same silent-drop failure mode.

### Workaround in use (for context, not part of the report)

This repo's binding convention until upstream fixes the parser: any selector that must live inside `@supports` targets a plain, hand-written marker class (the `dx-anchor-*` family) that is never referenced anywhere outside that `@supports` block, so the scoping pass has nothing of its own to rewrite and the selector matches by construction. Not proposed as the fix — just evidence this is a known, worked-around gap, not a one-off misreading of the parser's behavior.

## Before filing

- [ ] Re-verify against the latest Dioxus release (this draft is written against `0.7.9`; check whether a later release's `manganis-core` changelog already mentions `@supports` recursion).
- [ ] Confirm the exact file/function inside `manganis-core`'s source at the version being reported against, and link a permalink to it (not done here — this repo does not vendor or submodule `manganis-core`'s source, so the claim above about `unknown_block_contents` and the `@media`/`@layer`/`@container` recursion list is from the original 2026 investigation's reading of the crate, not a live link).
- [ ] Link a permalink to `docs/issues/css-module-supports-scoping.md` and `primitives/src/top_layer.rs` at the commit this issue is filed from (both will have moved since 2026-09-03).
- [ ] Confirm GitHub Issues is enabled on the target repo/organization's policy for external reports (this repo's own Issues are disabled, which is why this exists as a drafts file rather than a filed issue in the first place).
