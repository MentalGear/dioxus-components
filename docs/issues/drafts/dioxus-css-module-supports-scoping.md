# Draft issue: `#[css_module]` scoping pass silently skips `@supports` (and every non-allowlisted block at-rule)

**Status:** filing-ready 2026-09-04 (drafted 2026-09-03). Verified against the vendored `manganis-core 0.7.9` source; two independent instances in this repo. Supersedes the tracking record at `docs/issues/css-module-supports-scoping.md` (kept, points here).

**Why still file it although row 32 (plain, unhashed CSS) removes the hazard for this repo:** every `css_module` user hits the same silent no-op, the failure is invisible at build time and in DevTools until you compare hashed and unhashed names, and the fix is a one-line allowlist change with an existing test shape to copy.

**Target repo:** `DioxusLabs/dioxus`, crate `manganis-core`, file `packages/manganis-core/src/css_module_parser.rs` (confirm the path at the tag; the crate ships that file at `src/css_module_parser.rs`).

**Version:** `manganis-core 0.7.9` from crates.io (this repo's `Cargo.lock`; `dioxus 0.7.9`).

---

## Title

`#[css_module]`: class selectors inside `@supports` (and any at-rule other than `media`/`layer`/`container`/`include`) are emitted unhashed and never match the DOM

## Body

### What happens

`#[css_module]` hashes every class selector it finds (`.dx-select-list` → `.dx-select-list-e8d9eb03`) and rewrites the Rust-side names to match. The pass recurses into `@media`, `@layer` and `@container` bodies, but not into `@supports`. A rule written inside `@supports { … }` is copied to the output verbatim, with its original unhashed class name, while the DOM only ever carries the hashed one. The rule can never match in any browser, and nothing warns.

The cause is an allowlist in `at_rule` (`src/css_module_parser.rs`, 0.7.9):

```rust
match identifier {
    "media" | "layer" | "container" | "include" => {
        cut_err(terminated(style_rule_block_contents, '}')).parse_next(input)
    }
    _ => {
        cut_err(terminated(unknown_block_contents, '}')).parse_next(input)?;
        Ok(vec![])
    }
}
```

Everything not in that list, `@supports`, `@scope`, `@starting-style`, `@document`, and any future block at-rule that contains style rules, falls into `unknown_block_contents`, which recognises the block and returns no fragments, so no class inside it is ever rewritten.

### Minimal reproduction

```rust
#[component]
fn Demo() -> Element {
    rsx! { div { class: "demo", "hello" } }
}
```

```css
/* demo.module.css */
.demo { color: black; }

@media (min-width: 1px) {
    .demo { color: blue; }   /* hashed, applies */
}

@supports (display: grid) {
    .demo { color: red; }    /* emitted as `.demo`, unhashed, never applies */
}
```

Compiled output (this repo's build, served asset):

```css
.demo-<hash>{color:black}
@media (min-width:1px){.demo-<hash>{color:blue}}
@supports (display:grid){.demo{color:red}}
```

The `@media` body proves the recursion machinery works; only the allowlist is missing `supports`.

### Expected

Class selectors inside `@supports` bodies are hashed exactly like those inside `@media`.

### Actual

They are emitted verbatim and unscoped. The rule targets a class that does not exist in the rendered document. No build error, no warning.

### Impact (two independent instances in one project, both found only by comparing served CSS to DOM class names)

1. **CSS Anchor Positioning** for tooltips, hover cards and popovers, gated behind `@supports (anchor-name: --a)` per component stylesheet: the whole enhancement was dead in every browser, and content fell back to the `[popover]` UA stylesheet (`margin: auto`, centred hundreds of pixels from its trigger). Workaround: the anchor rules moved into one hand-injected stylesheet that targets deliberately unhashed marker classes (`primitives/src/top_layer.rs`, `ensure_anchor_positioning_styles`).
2. **Select listbox width**, `@supports (anchor-name: --a) { .dx-select-list[popover] { min-width: anchor-size(width) } }`: dead, so the list rendered at full viewport width once it was promoted to the top layer (`min-width: 100%` against the viewport). Reported from a device; root-caused only by reading the served CSS.

This class of bug is likely to grow: `@supports` is exactly where progressive-enhancement CSS for new platform features lives (anchor positioning, `@starting-style` for popover/dialog entry animations, `@scope`).

### Proposed fix

Invert the allowlist: recurse with `style_rule_block_contents` for every block at-rule whose body contains style rules, and keep an explicit deny-list for the at-rules whose bodies are declarations or non-CSS (`@font-face`, `@page`, `@property`, `@counter-style`, `@keyframes` frames, `@font-feature-values`). At minimum, add `"supports" | "scope" | "starting-style"` to the existing match arm. The `test_at_rule_media` / `test_at_rule_layer` tests in the same file give the shape for a `test_at_rule_supports` case.

### References

- This repo, `MentalGear/dioxus-components`: `docs/issues/css-module-supports-scoping.md` (first instance, 2026-09), `docs/backlog.md` rows 32 and 47 (second instance, 2026-09-04), `primitives/src/top_layer.rs` (`ensure_anchor_positioning_styles`, the workaround), `preview/src/components/select/style.css` (the dead rule's replacement comment).

## Before filing

- [x] Verified the parser behaviour against the vendored `manganis-core 0.7.9` source (`at_rule`, `unknown_block_contents`), not from memory.
- [x] Verified `@media` bodies ARE hashed in the served output (so the report's claim "only the allowlist is missing" is exact).
- [ ] Re-check the latest `manganis-core` release notes for an `@supports` change before posting.
- [ ] Replace the file reference with a permalink at the tag being reported against, and the two repo references with permalinks at the filing commit.
