# Draft issue: `dioxus-ssr` keeps both copies of a duplicated attribute; the client DOM path keeps only the last

**Status:** drafted 2026-09-03, not filed.

---

**Target repo:** `DioxusLabs/dioxus` (the `dioxus-ssr` crate).

**Versions in use** (from this repo's `Cargo.lock`): `dioxus 0.7.9`, `dioxus-ssr 0.7.9`, `dioxus-fullstack 0.7.9` — all pinned to the same workspace version, resolved from crates.io.

## Title

`dioxus-ssr` serializes an explicit attribute and a later spread override as two separate attributes on one start tag; the client/hydrated DOM path applies them in order and keeps the last

## Body

### Context

A very common Dioxus component-authoring pattern gives an element a default attribute value and lets a caller override it via a spread:

```rust
rsx! {
    div {
        aria_label: "default",
        ..attributes, // caller-supplied Vec<Attribute>, may itself contain aria_label
    }
}
```

If `attributes` contains its own `aria_label`, the two rendering paths this framework supports disagree about which one wins:

- **`dioxus-ssr`** (`render_template`, or the equivalent template-rendering path — exact function name to confirm against the version being filed against) serializes **both** attributes into the start tag as written, in source order: `<div aria-label="default" aria-label="caller-value">`. WHATWG HTML's tokenizer, on encountering a duplicate attribute name within one start tag, is specified to ignore every occurrence after the first — so any HTML parser (a browser, or Playwright/Chromium reading the served markup) resolves this tag's `aria-label` to `"default"`, discarding the caller's override entirely.
- **The client-side (CSR/hydrated) DOM path** applies the same two attributes sequentially via DOM API calls (`setAttribute`, or the WASM diffing/patching path's equivalent) — whichever is applied last simply overwrites the DOM property, so the caller's override (applied after the component's own default, since it comes from the later `..attributes` spread position) wins.

The two lanes agree on tree *shape* but disagree on this one attribute's *value* — server-rendered/prerendered HTML resolves to the component's hard-coded default, while a client-rendered or post-hydration DOM resolves to the caller's override, for the exact same source.

### Minimal reproduction

```rust
#[component]
fn Labeled(attributes: Vec<Attribute>) -> Element {
    rsx! {
        div {
            aria_label: "default",
            ..attributes,
        }
    }
}

// Caller:
rsx! { Labeled { aria_label: "caller-value" } }
```

- Render this through `dioxus-ssr` and inspect the produced HTML string directly: it contains two `aria-label` attributes on the one `<div>`.
- Parse that same string with any spec-compliant HTML parser (a browser's `DOMParser`, or simply loading the page): `element.getAttribute('aria-label')` reads `"default"`.
- Render the same component through the CSR/wasm client (no SSR involved) and inspect the live DOM: `element.getAttribute('aria-label')` reads `"caller-value"`.

### Expected vs actual

- **Expected:** one canonical answer for "which attribute wins when a component's own attribute and a spread `attributes` override collide on the same name" — and `dioxus-ssr`'s serialized output should embody that same answer, since a fullstack/SSG deployment's server-rendered markup and its post-hydration client DOM are required to describe the same tree for hydration to be meaningful at all.
- **Actual:** `dioxus-ssr` emits both values as two attributes on one tag, silently deferring the "which one wins" decision to whatever HTML parser reads the string later — which happens to resolve it the *opposite* way from how the client-side DOM-mutation path resolves the identical two-write sequence.

### Evidence — how this was found and confirmed

Found during this repo's Phase 4 work on `docs/recommended-implementations.md` (see "Second divergence class found 2026-09-01: attribute override order"), independently of the cfg-axis SSG incident that Caveat 1 documents in the same file. First surfaced as a real bug: `ToastRegionRendered`'s `aria_label` attribute diverged between two `ToastProvider` instances on one page — server-rendered markup gave both regions the same default label (indistinguishable to assistive tech, an axe `landmark-unique`-class failure), while the client-hydrated DOM correctly carried each provider's caller-supplied label. Once regression-tested generally across the codebase, the same root cause was found live in four more components: `Progress`, `ContextMenuRoot`/`ContextMenuTrigger`, `PopoverTrigger`, and `SelectTrigger` — five independently-written components hitting the identical framework-level footgun. This repo's fix: a `merge_attributes` helper (`primitives/src/lib.rs`) that dedupes attributes by `(name, namespace)` before ever reaching `rsx!`, always taking the caller's (later) value — construction-level defence rather than something `dioxus-ssr` itself does. `playwright/oracle/hydration-parity.spec.ts` Rule 4 is the standing regression oracle for this repo's own components; it does not (and cannot) fix the underlying framework behavior for anyone who hasn't adopted the same helper.

One sub-case `merge_attributes` cannot fix, documented separately, is worth flagging as a related but distinct issue if useful context for triage: a component's own literal `style: "a: b;"` attribute (namespace `None`) colliding with a caller's CSS-shorthand style properties (`padding: "1rem"`, namespace `Some("style")`) hits a *different* SSR code path — `dioxus-ssr`'s renderer only combines attributes carrying `namespace == Some("style")` into one served `style="..."` string, so a plain-named `style` attribute is written as a wholly separate, second `style="..."` on the same tag, independent of the duplicate-attribute-name issue above (same WHATWG duplicate-attribute resolution applies, but the two attributes here are already distinguishable to Dioxus's own SSR renderer and it still emits both, per that renderer's own namespace-based grouping). See `primitives/src/lib.rs`'s `fold_style_attributes` doc comment for the full mechanism and this repo's Rust-side workaround (folding both forms into one string before render).

### Proposed fix — ask, don't presume

Two directions, and this report should ask upstream which fits their model rather than presuming one:

1. **`dioxus-ssr` dedupes at serialization time**, keeping the last-written value for a given attribute name — matching the CSR/DOM-mutation path's last-wins semantics, so both lanes agree without every component author needing their own `merge_attributes`-style helper.
2. **The client/diffing path is made to error (or warn) on a duplicate attribute at the same call site** — treating this as a caller mistake to surface loudly rather than silently resolve, on the reasoning that emitting the same attribute name twice in one `rsx!` invocation is very likely never intended, and picking a winner (either winner) papers over what should be a compile-time or runtime diagnostic.

This repo's own fix (`merge_attributes`, caller-wins, i.e. direction 1's semantics enforced by hand at each component) implies a preference for option 1, but that is a judgement call worth putting to whoever owns `dioxus-ssr`'s design, not asserted as the only correct answer.

## Before filing

- [ ] Re-verify against the latest Dioxus release (this draft is written against `0.7.9`).
- [ ] Confirm the exact `dioxus-ssr` function/file handling attribute serialization at the version being reported against and link a permalink to it (not done here — this investigation read behavior by observation/execution, not by tracing `dioxus-ssr`'s own source line-by-line).
- [ ] Link permalinks to `primitives/src/lib.rs`'s `merge_attributes` and `fold_style_attributes`, `primitives/src/toast.rs`'s `ToastRegionRendered` doc comment, and `playwright/oracle/hydration-parity.spec.ts` Rule 4, all at the commit this issue is filed from.
- [ ] Search existing `dioxus`/`dioxus-ssr` issues for prior reports of this exact class before filing, to avoid a duplicate.
