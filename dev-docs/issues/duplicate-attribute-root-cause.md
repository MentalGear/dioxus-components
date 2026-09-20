<!-- Committed into the repo 2026-09-20 from a session scratchpad, so it survives the
     container it was produced in. Body is the lane's report as written; only this header
     was added. -->

> **Provenance and status.** Produced by a read-only subagent lane (`deconstruct-attr-dup`) on
> 2026-09-20, against `main`@`6b4bd27`, in response to `dev-docs/backlog.md` row 93. Every
> load-bearing claim about `dioxus-core`, `dioxus-ssr`, `dioxus-rsx` and `dioxus-web` was
> independently re-verified in the main loop against the vendored crate sources before this was
> committed; the two claims the lane itself flagged as unverified (Blitz/`dioxus-native`
> `set_attribute` coalescing, and whether `#[props(extends)]`-vs-typed-field exclusivity is
> compiler-enforced or merely conventional) remain unverified here too.
>
> **Nothing in this document has been acted on.** It is an investigation, not a change: no guard
> script has been promoted into `scripts/`, no site has been fixed, and the recommendation at the
> end is awaiting the repository owner's call. Backlog row 93 is the live status; the upstream
> report this unblocks is `drafts/dioxus-ssr-duplicate-attributes.md`.
>
> `$S` throughout the body refers to that session's scratchpad and no longer resolves. The
> companion artifacts worth keeping were committed alongside this file as
> `duplicate-attribute-guard-prototype.py` and its
> `duplicate-attribute-style-shorthand-idents.txt` data file. Run it with no arguments from the
> repo root and it reproduces the 623 findings this document cites; the raw findings list itself
> is therefore not committed. Two changes were made to the prototype when committing it, both
> mechanical: the idents file was renamed to sit beside it, and the default scan roots were
> pinned to `primitives/src` and `preview/src/components` (its original default of `.` walks
> `.claude/worktrees/` and reports tens of thousands of findings from lane-local copies).

---

# Duplicate-attribute class (`id`/`class`/`role`/`data-*`/...): root-cause deconstruction

Lane `deconstruct-attr-dup`, round 4. Read-only investigation; nothing in the
repo tree was modified. Base: `origin/main` @ `6b4bd27` (merged `f2be1d7`).

All file:line citations against third-party crates point at the exact
versions this repo pins (`Cargo.lock`): `dioxus-core 0.7.9`, `dioxus-rsx
0.7.9`, `dioxus-core-macro 0.7.9`, `dioxus-ssr 0.7.9`, `dioxus-web 0.7.9`,
`dioxus-interpreter-js 0.7.9`, `dioxus-html 0.7.9`, all under
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`.

## The three instances, restated

- `b35d671` — 5 sites (`ToastRegionRendered`'s `aria_label`, `Progress`,
  `ContextMenuRoot`/`ContextMenuTrigger`, `PopoverTrigger`, `SelectTrigger`).
- `5fc1439` — `preview/src/components/label/component.rs`'s `class`
  (a different entry point: Dioxus's `#[props(extends = GlobalAttributes)]`
  ad-hoc-key-plus-explicit-`attributes:`-assignment collision, not a raw
  `..spread`, but the same underlying VNode-layer mechanism once both values
  reach `Vec<Attribute>`).
- `f2be1d7` — `dropdown_menu.rs` + `menubar.rs` root `id`/`role`.

## Layer 1 — the `rsx!` macro: cannot see the collision, by construction

`dioxus-rsx-0.7.9/src/element.rs` **does** have its own attribute-merging
step, confusingly also called `merge_attributes` (`Element::merge_attributes`,
`element.rs:228-301`) — but it only merges **literal-vs-literal** duplicates
authored directly in one element block (e.g. `class: "a", class: if x {
"b" }` folds into one `format!`-built value; proven by the crate's own tests,
`element.rs:526-639`, `merge_trivial_attributes`/`merge_formatted_attributes`/
`merge_conditional_attributes`/`merge_all_attributes`).

Spreads are handled completely differently. `Element::parse` (`element.rs:87-103`)
runs `merge_attributes()` over `raw_attributes` **first**, then appends each
`..spread` as its own opaque `Attribute { name: AttributeName::Spread(..),
value: AttrExpr(spread.expr), .. }` onto `merged_attributes` **afterward**,
with the comment (`element.rs:91-93`): "merge the spreads *after* the
attributes are merged... spreads are still counted as dynamic attributes."
There is no code path that compares a literal attribute's name against a
spread's contents, because a spread's contents (`props.attributes`, a runtime
`Vec<Attribute>`) are an opaque `syn::Expr` at macro-expansion time — the
macro cannot enumerate what keys a runtime `Vec` will hold. `dioxus-rsx-0.7.9/
src/attribute.rs:170-177`'s `rendered_as_dynamic_attr` confirms the codegen
shape: a spread just becomes `{#expr}.into_boxed_slice()`, i.e. "whatever
that expression evaluates to, verbatim."

**Verdict: layer 1 is structurally incapable of fixing this**, not merely
unfixed. A macro-time solution would require knowing a runtime value's
keys at compile time.

## Layer 2 — `dioxus-core`'s VNode/diffing: the actual data-model gap

Each dynamic-attribute "slot" (one templated position, e.g. one `div`'s
attribute list) is stored as `Box<[Attribute]>`, and a `VNode`'s
`dynamic_attrs: Box<[Box<[Attribute]>]>` is one such slot per position
(`dioxus-core-0.7.9/src/nodes.rs:88`). The struct's own doc comment
(`nodes.rs:57-88`) states the contract explicitly:

> "The inner list *must* be in the format `[static named attributes,
> remaining dynamically named attributes]`." Example: `[class, every
> attribute in attrs sorted by name]`.

This is an **ordered concatenation with no uniqueness invariant** — nothing
enforces "at most one entry per name." Confirmed at the point this list is
actually consumed for the first mount (shared by every renderer, since
`create()`/`write_attrs` lives in `dioxus-core` itself, not in a
renderer crate):

```
// dioxus-core-0.7.9/src/diff/node.rs:822-825 (write_attrs)
for attr in &**attribute {
    self.write_attribute(attribute_path, attr, id, mount, dom, to);
}
```

This is an **unconditional loop over every `Attribute` in the slot**,
calling `write_attribute` (→ one `WriteMutations::set_attribute` call) once
per entry, regardless of whether an earlier entry in the *same* slot already
used that name. No map, no last-wins resolution, nothing — dioxus-core's
create/mount path never builds a canonical "one value per name" view; it
just replays the list as a sequence of "set" instructions.

The re-diff path (`diff_attributes`, `diff/node.rs:416-498`) is telling by
contrast: it walks old-slot-list vs new-slot-list with a sorted two-pointer
merge (`old_attribute.name.cmp(new_attribute.name)`, `diff/node.rs:438`),
which **assumes** each list is already sorted-and-unique-by-name. It has no
code for "two adjacent entries in the same list share a name" — that
scenario is simply outside what this algorithm models, because nothing
upstream of it guarantees it can't happen. (In practice this repo's
instances never reach a second diff with a *changed* value, so this path
isn't itself where the bug is observed — it's further proof the data model
has no such invariant anywhere in `dioxus-core`.)

**Verdict: this is the earliest point in the pipeline where a decision — "a
slot resolves to at most one value per attribute name before it is handed to
a renderer" — is simply never made.** Every downstream renderer inherits
this raw, unresolved list.

## Layer 3 — renderers: same list, different targets, asymmetric fallout

### `dioxus-ssr`: a stateless string writer (verified by source, not by
observation)

`dioxus-ssr-0.7.9/src/renderer.rs:146-159` (`Segment::Attr` handling):

```rust
let attrs = &*template.dynamic_attrs[*idx];
for attr in attrs {
    if attr.name == "dangerous_inner_html" { ... }
    else if attr.namespace == Some("style") { accumulated_dynamic_styles.push(attr); }
    else if BOOL_ATTRS.contains(&attr.name) { if truthy(&attr.value) { write_attribute(buf, attr)?; } }
    else { write_attribute(buf, attr)?; }
    ...
}
```

and `write_attribute` itself (`renderer.rs:482-498`):

```rust
pub(crate) fn write_attribute<W: Write + ?Sized>(buf: &mut W, attr: &Attribute) -> std::fmt::Result {
    let name = &attr.name;
    match &attr.value {
        AttributeValue::Text(value) => write!(buf, " {name}=\"{}\"", askama_escape::escape(value, ...)),
        ...
    }
}
```

`write_attribute` is **completely stateless** across calls — no "seen
names" set, no map. Two `Attribute`s named `id` in one slot's list
unconditionally produce two ` id="..."` substrings appended to the output
buffer, i.e. one HTML start tag with a duplicate attribute — exactly what
`f2be1d7`'s commit measured by execution (`id="dxc-1091" ...
id="clip-dropdown-menu-root"`). WHATWG HTML tokenization then keeps the
FIRST occurrence and ignores the rest, so any spec-compliant parser
(a browser, Playwright/Chromium reading the served markup) resolves the
attribute to the component's own internal value, discarding the caller's
override. The one legitimate exception is `namespace == Some("style")`
(CSS-shorthand properties, e.g. `padding: "1rem"`): these get accumulated
into one `accumulated_dynamic_styles` Vec and folded into a single
`style="prop:val;prop:val;..."` string at the `StyleMarker` segment
(`renderer.rs:232-249`) — same-named CSS *properties* collide inside CSS's
own last-wins cascade, not as a second HTML attribute, so this sub-case
never produces a WHATWG duplicate-attribute error (see "what the guard
excludes," below, for why this matters to the guard's precision).

### The web (CSR) renderer: the browser DOM coalesces for a structural
reason, not a Dioxus one

`dioxus-web-0.7.9/src/mutations.rs:174-201`'s `set_attribute` forwards to
`Interpreter::set_attribute`, which lowers to
`dioxus-interpreter-js-0.7.9/src/js/set_attribute.js`'s `setAttributeInner`
→ (default case) `setAttributeDefault` → `node.setAttribute(field, value)` —
a real `Element.setAttribute` DOM call. Per the DOM spec, an element's
attribute list (`NamedNodeMap`) holds at most one `Attr` per qualified name;
`setAttribute` looks up any existing attribute by that name and mutates its
value in place, or creates one if absent. So two `set_attribute` mutations
for the same name, issued back-to-back against a **live** DOM node, collapse
to one attribute whose value is whatever the **second** call set — this is a
property of the Web platform's own object model, not a Dioxus dedup
mechanism. Since `write_attrs` emits the literal-authored entry first and the
spread-derived entries second (layer 2's own `[static named, then
dynamically named]` ordering), a caller's override in the spread naturally
ends up as that "last call," so pure CSR *looks* correct — invisibly, by
accident of DOM semantics, not by any decision Dioxus made.

### The nuance the existing commit messages/draft don't state: hydration
does not correct the SSR'd value either

This matters because it changes "which renderer is right" from a cosmetic
test-methodology footnote into a real, permanently-shipped production
defect. `dioxus-web-0.7.9/src/dom.rs:35-49`'s own comment: hydration runs
the *same* `rebuild()`/`create()`/`write_attrs()` code path (it must, to
assign ids for future reactivity), but with mutations suppressed:
"NOTE: running the virtual dom with the `write_mutations` flag set to true
is different from running it with no mutation writer because it still
assigns ids to nodes, but it doesn't write them to the dom." Concretely,
`set_attribute` (`mutations.rs:181-183`) is gated:

```rust
fn set_attribute(&mut self, ...) {
    if self.skip_mutations() { return; }
    ...
}
```

`skip_mutations` is set `true` for the initial hydration walk
(`dioxus-web-0.7.9/src/lib.rs:100`, cleared at `:148`; also
`hydration/hydrate.rs:162`/`:166`). `hydrate.ts` itself never touches
attributes at all (no `attr`/`setAttribute` occurrence in
`dioxus-interpreter-js-0.7.9/src/ts/hydrate.ts`) — hydration only wires up
event listeners and dynamic-text/placeholder markers, deliberately trusting
the server-rendered markup for everything else (this is what makes
hydration cheap: it avoids re-touching DOM the server already produced
correctly).

**Consequence:** in this repo's actual deployment mode — `dx build --ssg
--features fullstack --platform web`, i.e. SSR followed by client hydration,
which is exactly what `hydration-parity.spec.ts` exercises against a real
build per `dev-docs/conformance-harness.md` — the browser's own HTML parser
resolves the duplicate to the FIRST value at initial parse time, and
hydration never re-issues the `set_attribute` calls that would have flipped
it to the caller's override. The wrong value is not a transient flash later
corrected by JS; it is what a real visitor's browser permanently keeps
(until some *other*, value-changing re-render happens to touch that same
slot, which `diff_attributes`, as shown above, will not do for two
structurally-identical duplicate lists across renders). Only a plain,
non-hydrating `dx serve --web` CSR session — i.e. exactly how every lane in
this repo's own dev loop tests changes locally — exercises the accidental
DOM-coalescing "fix." This is precisely why three independent authors, all
testing locally against `dx serve --web`, never saw the bug: the dev-loop's
own renderer mode structurally cannot show it, and only the SSG-lane
Playwright oracle (parsing real static HTML) can.

**Verdict:** the renderers are not "buggy" in the sense of misimplementing
their own contract — dioxus-ssr's writer does exactly what a stateless
string serializer does with an unresolved list, and the web/DOM path
resolves it for free purely because a live `NamedNodeMap` cannot represent
two attributes of the same name. The layer-2 gap is what makes the
divergence *possible*; this layer is where it becomes *visible*, and,
specifically for the SSG+hydrate deployment this repo ships, *permanent*.

### Native/Blitz

Not traced beyond `dioxus-native`/`dioxus-native-dom` being separate crates
in this lockfile implementing their own `WriteMutations`; I did not read
their `set_attribute` implementation. Given `dioxus-core` hands every
renderer the identical unresolved list, and a real native DOM (Blitz's) is,
like the browser's, a structured node tree rather than a string buffer, the
most likely behavior mirrors the web renderer's (repeated "set" collapses to
last-wins) — but this is inference from the shared core contract, not
something I confirmed by reading Blitz's own attribute-application code. I
am flagging this as **not fully determined** rather than asserting it.

## Layer 4 — this repo's own layer: the actual, actionable discriminator

The `f2be1d7` lane's criterion — "a literal attribute beside a spread where
the props struct has no typed field claiming that name" — is confirmed as
the real discriminator, both mechanically and by direct source inspection:

- `use_id_or` (`primitives/src/lib.rs:144-165`) takes an internally
  generated id `Signal` and a caller-supplied `Option<id>`, and reconciles
  them into a single `Memo` (caller wins if present). A component using this
  renders **one** `id:` literal whose value is already resolved — there is
  only ever one `Attribute` for that name, nothing to duplicate.
- This is only possible because the caller's `id` reached a **typed field**
  (e.g. `PopoverRootProps::id: ReadSignal<Option<String>>`,
  `primitives/src/popover.rs:124`) rather than the catch-all
  `attributes: Vec<Attribute>` field. Verified directly:
  `DropdownMenuProps`/`MenubarMenuProps` have `pub attributes: Vec<Attribute>`
  but **no** typed `id`/`role` field (`primitives/src/dropdown_menu.rs`
  around line 124-157) — so a caller's `id`/`role` has nowhere to go but
  `attributes`, colliding with the component's own literal. `popover.rs`'s
  `PopoverRootProps` **does** have a typed `id` field alongside its own
  `attributes` field — confirmed by reading the struct directly — so its
  identical-looking `id: root_id` literal is not an instance. `carousel.rs`
  has no `id`-literal-beside-spread anywhere (grepped directly: its only
  `id:` occurrences are struct-field declarations, not element attributes).
- The mechanism this rests on — a name claimed by a typed field can never
  also land in the `#[props(extends = ...)]` catch-all `Vec<Attribute>` — is
  grounded in `dioxus-core-macro-0.7.9/src/props/mod.rs`'s `extends`
  machinery (`extends_vec_ident`/`extends_impl`, lines ~282-300, ~939+): a
  props struct's builder generates one setter per declared name, and the
  `extends`-marked field only receives names *not* already claimed by an
  explicit field. I verified this machinery exists and generates
  per-field/per-extends-name plumbing; I did **not** trace token-by-token
  that the derive macro would refuse to compile (or otherwise makes it
  impossible) for a name to be claimed by both a field and the extends set
  simultaneously — I'm treating this as "verified structurally, not
  verified as a compile-time-enforced exclusivity guarantee," to be precise
  about the depth of this specific check.

**This is the layer with all the leverage this repo actually has today.**
Neither layer 1 nor layer 2 will change on this repo's timeline; every fix
so far (`b35d671`, `5fc1439`, `f2be1d7`) has been applied here, reactively,
one incident at a time.

## Layer 5 — upstream status

`dev-docs/issues/drafts/dioxus-ssr-duplicate-attributes.md` is a real draft,
"drafted 2026-09-03, not filed" — its own "Before filing" checklist is
unchecked, including "confirm the exact `dioxus-ssr` function/file... not
done here — this investigation read behavior by observation/execution, not
by tracing `dioxus-ssr`'s own source line-by-line." **This investigation now
closes that gap**: the exact function is `render_template`'s `Segment::Attr`
arm plus `write_attribute`, `dioxus-ssr-0.7.9/src/renderer.rs:146-159` and
`:482-498`, cited above. The draft's own two proposed directions both still
hold up under this deeper trace:

1. "`dioxus-ssr` dedupes at serialization time, last-wins" — feasible as a
   narrow, surgical patch entirely inside `renderer.rs`'s `Segment::Attr`
   loop (track seen names in a small `HashSet`/linear scan since slots are
   typically tiny, skip repeats keeping the last). This does **not** fix
   `dioxus-core`'s data model (native/Blitz renderers would still receive an
   unresolved list, and `diff_attributes`'s implicit unique-sorted
   assumption would still not be a real invariant) — it only fixes the one
   renderer whose target (a string) cannot self-coalesce.
2. "Make the diffing/create path itself dedupe (or error) when building a
   slot" — this is really "fix layer 2": resolve `Box<[Attribute]>` to one
   entry per `(name, namespace)` (last-wins, matching what the DOM already
   does for free) at the point a slot is constructed/consumed in
   `dioxus-core`, once, for every renderer. Larger, more central change;
   correct home for a durable, upstream fix.

**Same bug, not a different one.** The three regressions this repo hit are
this exact mechanism (layer 2's unresolved list + layer 3's asymmetric
renderers), reached through two different repo-level entry points (a raw
`..spread`, and `5fc1439`'s `#[props(extends = ...)]` ad-hoc-key path) that
both funnel into the same `Vec<Attribute>` before `dioxus-core` ever sees it.
Nothing found in this investigation suggests a second, unrelated upstream
defect.

## The answer: where does the root cause actually live?

**This is genuinely multi-layer, and the layers disagree about "root cause"
depending on what the question is actually asking:**

- **Mechanistically** (the earliest point a different decision would have
  prevented the whole class): **`dioxus-core`'s data model** (layer 2) —
  a dynamic-attribute slot is defined as an ordered, concatenated list with
  no per-name uniqueness invariant, and the shared mount/create path
  (`write_attrs`) replays it as an unconditional sequence of "set"
  instructions rather than resolving it to one canonical value per name
  before handing it to any renderer. Every other layer's behavior (the
  macro's inability to help, the SSR-vs-CSR value disagreement, the
  permanence of the wrong value in a real SSG+hydrate deployment) is a
  direct consequence of this one absent step.
- **Practically, for this repo, today**: **this repo's own authoring
  surface** (layer 4) — since layers 1-2 are upstream and out of this
  repo's control on any near-term timeline, the actual fault for *why this
  keeps recurring in this codebase specifically* is that nothing in the
  local authoring surface makes "give an element a sensible default
  attribute, let the caller's spread override it" — a completely natural,
  unremarkable way to write a component — visibly different from "give an
  element a sensible default and ALSO accept a spread that might redefine
  the exact same name, both landing in the output." The two look identical
  at the call site and both compile silently.

**Ranked by leverage (highest first):**

1. **Layer 4 — a local static guard / construction change.** Only layer
   this repo can act on immediately; prevents 100% of *future* instances in
   this codebase regardless of upstream's timeline. See Construction, below.
2. **Layer 2 — an upstream `dioxus-core` fix (resolve-on-construct).**
   Highest leverage in an absolute sense (fixes every Dioxus app, every
   renderer, forever) but zero leverage on this repo's own timeline; it is
   a "file it and wait" lever, not a "fix it this round" one.
3. **Layer 3 — a narrow `dioxus-ssr`-only patch.** Smaller and more
   plausible for upstream to accept quickly than a full layer-2 redesign
   (it only touches one renderer's writer), but explicitly does not fix
   native/Blitz or the underlying data-model gap — a partial, renderer-local
   patch, not a class-level fix.
4. **Layer 1 — no leverage.** Categorically not fixable at the macro layer
   for the general case (runtime spread contents are unknowable at
   compile time).

## The construction: guard-script prototype and its real findings

Built and run (see "Files produced," below) against exactly the criterion
row 93 and this investigation converged on: **a literal attribute (bare
ident or `"kebab-string"`) coexisting in the same brace-block as a
`..spread`, where the block's enclosing component has no typed
field/parameter of that exact name.**

### What it is

A ~250-line Python prototype (not a real Rust/rsx! parser): it leans on this
repo's own `cargo fmt` convention (4-space indents, one attribute per line)
to approximate brace nesting instead of tokenizing, resolves each block's
"enclosing component" via the nearest `fn NAME(...)` signature in the same
file (handling both this repo's two authoring styles: manual `props: XProps`
+ a separate `pub struct XProps`, and `#[component]`-sugar inline
parameters), and excludes (a) event handlers and `key`, the same heuristic
`dioxus-rsx` itself uses, and (b) CSS-shorthand style properties, via an
**authoritative** denylist mechanically extracted from this exact Cargo.lock's
own `dioxus-html-0.7.9/src/attribute_groups.rs` (407 idents, e.g. `padding`,
`border`, `display`, `flex_direction` — these fold safely into one `style="…"`
string on SSR, a different, non-erroring mechanism, confirmed above).

### Iteration and false-positive hunting (shown, not just claimed)

- v1 (no style-shorthand exclusion, `#[component]`-style unresolved): 670
  raw findings, but manual inspection immediately showed most of the
  "unresolved" bucket was `#[component]`-style preview components
  (`Alert`, `Card`, ...) whose typed-field set the script simply hadn't
  learned to read yet — not true unknowns.
- Added the `#[component]`-parameter-list extraction and the CSS-shorthand
  denylist (mechanically pulled from `dioxus-html`, not guessed) → 638.
  Spot check of `collapsible.rs` (previously flagging `border_radius`,
  `flex_direction`, `max_width`, `gap`, `display`, `color`) confirmed these
  are real CSS-shorthand style attributes, correctly excluded once the
  denylist's extraction regex covered the `ident in "style";` (no explicit
  kebab string) macro form, not just `ident: "kebab" in "style";` → 407
  idents recovered instead of 343.
- Manually inspecting `context_menu.rs` — the file `f2be1d7`'s own commit
  message cites as the **already-fixed** reference implementation — found
  it still had 26 findings (all `role`/`data-state`/`tabindex`/`popover`/
  `aria_orientation`/`dir`/`aria_disabled`/`data-disabled`/`data-direction`,
  never `id`). Tracing one by hand found a real scanner bug: multi-line
  function signatures (rustfmt's normal style for >1 parameter) left the
  body's opening `{` on a line with no `fn`/name text, so the enclosing
  function was misattributed as `None` ("unresolved") — which in turn
  caused one genuine false positive (`id` on `ContextMenuContentRendered`,
  which *does* have a typed `id: String` parameter, incorrectly flagged
  because the scanner didn't know which function it was in). Fixed by
  tracking a not-yet-consumed `fn NAME` across lines instead of requiring
  it on the same line as the opening brace; also fixed comment lines
  (`/// fn Demo() -> Element {`-shaped doc examples) being eligible to open
  spurious blocks at all, by skipping `//`-prefixed lines outright.
  Re-run: unresolved dropped from 128 to 2; `id` on
  `ContextMenuContentRendered` correctly stopped appearing; role/
  data-state/tabindex/popover/aria_orientation/dir/aria_disabled/
  data-disabled/data-direction on that same file remained, correctly (none
  of `ContextMenuContentRendered`/`ContextMenuItem`/
  `ContextMenuSubContentRendered`/`ContextMenuSubItem` has a typed field for
  any of those names) → **624**.
- The last 2 "unresolved" cases (`primitives/src/slider.rs`'s
  `SliderImpl`) traced to `struct SliderImplProps` being a
  non-`pub` struct, which the struct-detection regex required `pub` for.
  Fixing that (allow optional `pub`) resolved both: `dir` was actually a
  **real false positive** (`SliderImplProps` does have a typed
  `dir: Option<Direction>` field) that would have been silently wrong had
  it landed in the confident bucket instead of "unresolved"; `role` remained
  a genuine finding. Final: **623 findings, 0 unresolved.**

### Known remaining limitations (disclosed, not fixed, in this prototype)

- **Single-line element bodies are invisible** (a real false-negative mode,
  found while cross-checking row 93's own named list): `primitives/src/
  alert_dialog.rs:353` — `h2 { id: ctx.labelledby.clone(), ..props.attributes,
  {props.children} }` — is written on **one line**; the indentation-based
  scanner only recognizes attribute/spread lines as immediate children when
  they appear on their own line at exactly `block_indent + 4`. Both
  `AlertDialogTitle` and `AlertDialogDescription` (row 93's own named `id`
  instances) are real, confirmed by direct reading, and both are missed by
  the automated scan for this reason. **The true count is higher than 623,
  not lower** — this limitation only ever under-counts.
  Because both known scanner bugs found during iteration (multi-line
  signatures, non-`pub` structs) manifested as *false positives* that I
  found and fixed, and this remaining known gap manifests as a
  *false negative*, I did not find a mechanism, after these fixes, that
  would cause the script to over-report. I did not exhaustively hand-verify
  all 623 lines (infeasible in this investigation's scope) — I spot-checked
  roughly a dozen across both authoring styles, several different files,
  and both "always"/"no typed field" categories, all confirmed as
  structurally real; plus the two systematic bugs above, both found and
  fixed rather than papered over.
- `..spread` is recognized lexically; it cannot distinguish an rsx
  attribute spread from Rust's own struct-update syntax (`Foo { a: 1,
  ..Default::default() }`) if that pattern occurred in scanned code. Not
  observed in any manually-checked sample (every resolved spread target was
  `props.attributes`, `attributes`, `merged`, or `rest_attrs`), but not
  proven absent across all 623.
- A literal attribute value that itself wraps across multiple lines is
  invisible (false negative, same direction as the single-line case above).

### Real numbers (final, post-fix run)

- **623 findings across 66 distinct files** (`primitives/src/**` and
  `preview/src/components/**` combined; raw list in `findings-raw.txt`).
- 400 "no typed field/param of this name" (bare-ident attributes: `role`
  92×, `class` 29×, `tabindex` 36×, `aria_label` 24×, `type` 19×, `style`
  12×, `popover` 12×, `dir` 12×, `aria_disabled` 12×, plus many more
  `aria_*` names, each individually lower count).
- 223 "always" (string-literal `"kebab-name"` attributes, which by
  construction can never be claimed by a Rust struct field): `data-state`
  48×, `data-disabled` 48×, `data-direction` 23×, `data-slot` 18×,
  `data-orientation` 16×, etc.
- Heaviest files: `calendar.rs` (31), `popover.rs` (27 — note: `popover.rs`'s
  own `id` is correctly *not* among them, exactly matching `f2be1d7`'s
  claim; its 27 are `data-state` and others), `context_menu.rs` (26, all
  non-`id`, as detailed above), `color_picker.rs` (26), `command.rs` (24),
  `dropdown_menu.rs` (22, residual non-`id`/`role` sites beyond what
  `f2be1d7` fixed), `date_picker.rs` (22), `slider.rs` (20), `navbar.rs`
  (18), `menubar.rs` (17, residual beyond its fixed root).
- All 5 `id`-specific instances row 93 named were independently
  re-confirmed by this scan: `resizable.rs:525` (`ResizablePanel`),
  `toast.rs:968` (`ToastTitle`; `ToastDescription` has the identical shape
  slightly further down), `drag_and_drop_list.rs:470`
  (`DragAndDropInstructions`) — plus `virtual_list.rs:229` under a
  slightly different function name (`VirtualList`, not a `*Container`
  split) — and `alert_dialog.rs`'s `AlertDialogTitle`/`AlertDialogDescription`,
  which the automated scan **missed** (single-line false negative, above)
  but which direct reading (`alert_dialog.rs:353`, `:410`) confirms are
  real, exactly as row 93 described.
- All five `carousel.rs` components row 93 named
  (`Carousel`/`CarouselContent`/`CarouselItem`/`CarouselPrevious`/
  `CarouselNext`) are present with `role`/`aria_roledescription`/
  `aria_label`/`tabindex`/`data-orientation`/`data-direction`/`data-selected`.

**This is not a "dozens of false positives, unshippable" result.** After
fixing the two real scanner bugs found during this investigation, spot
checks found zero remaining false positives among the confident ("no typed
field" / "always") categories, and one further confirmed false negative
(single-line bodies) that only makes the true count larger. The guard's
concept is sound; the specific prototype needs the single-line-body gap
closed (and ideally a real `syn`-based rewrite, not indentation-based
heuristics) before it should gate commits.

### Candidate constructions, evaluated

**(a) A `scripts/check-*.sh` static guard** (this prototype, hardened).
*Prevents*: every future instance of this exact shape, mechanically, at
the point of authoring — the same category of defense as the seven guards
`CLAUDE.md` already lists. *Does not prevent*: anything not expressible as
"literal name beside spread, no typed field" (e.g. the `5fc1439` entry
point — an ad-hoc `#[props(extends)]` key plus a *separately assigned*
`attributes:` field passed to a **child** primitive, which is a different
call-site shape than a same-element spread; a real Rust `syn`-based rewrite
would need to model both). *Cost*: S-M to harden into a real parser
(replacing the indentation heuristic with `syn::parse_file` +
`dioxus-rsx`'s own `Element`/`Attribute` types, which are on crates.io and
importable, would eliminate both disclosed limitations at once and is the
natural "shippable" version of this prototype); separately, M-XL to fix the
~66 files / ~623 sites it flags (only once decided whether ALL of them need
literal fixing immediately, or whether the gate should start as
warn-then-ratchet).

**(b) An API change making merging the only expressible form** (a wrapper
type around `Vec<Attribute>` whose only constructor is "merge with a base,"
or a small macro every component calls instead of raw `..props.attributes`).
*Prevents*: the whole class **by construction** — if `..props.attributes`
itself cannot compile/type-check without having gone through a merge step,
authors cannot reintroduce the bug even by accident, no guard maintenance
required, no false positives/negatives ever. *Does not prevent*: nothing
within its own scope, but it is a larger, invasive refactor across every
component in `primitives/src/**` and `preview/src/components/**` (all ~66
files this scan found, plus any that hand-roll `attributes: Vec<Attribute>`
fields without yet using a spread), and it doesn't unilaterally fix the
`5fc1439`-shaped ad-hoc-key entry point unless that pattern is banned too.
*Cost*: the highest of the local options, but it is the only one that
removes the failure mode rather than detecting it.

**(c) Typed props fields for every attribute name a component renders
literally** (the `use_id_or` treatment, generalized). *Prevents*: exactly
the instances it's applied to, with the same strength as (b) for those
specific names — a typed field cannot also appear in the extends catch-all,
so there is structurally nothing to collide (verified in Layer 4, with the
stated caveat about how deep that verification went). *Does not prevent*: it
doesn't scale to `data-*`/custom-string attributes at all (Rust field names
can't contain hyphens — this is precisely the "always" bucket, 223 of the
623 findings, none of which this construction can ever reach), and it is
per-name, per-component work (equivalent cost to just fixing each of the
623 sites piecemeal, i.e. large, and does not prevent a *new* instance from
being written tomorrow the same way (b) or (a) would).

**(d) An upstream fix plus a local guard until it lands.** *Prevents*:
eventually, for every Dioxus consumer, if and when accepted — this is the
only option that helps anyone outside this repo. *Does not prevent*:
anything on this repo's own near-term timeline; upstream review/release
cycles are not under this repo's control, and the draft issue has sat
unfiled since 2026-09-03. Filing it (now with the exact file:line trace this
investigation adds) is close to free and should happen regardless of what
else is chosen locally, but it cannot be *the* answer to "how do we stop
finding this a fourth time this quarter."

**(e) Extending `hydration-parity.spec.ts` so the collision is exercised for
every component**, not only where a demo page happens to collide.
*Prevents*: nothing at the source level — it is a detection improvement, not
a construction; but it closes the actual reason all three prior instances
went unnoticed for as long as they did ("no demo page happened to exercise
the collision yet"), by manufacturing the collision synthetically per
component (render every component once with a caller-supplied value for
every attribute name its element literals also use) rather than waiting for
an incidental demo. *Does not prevent*: authoring a new instance in the
first place; it only guarantees it's *caught*, and only for components the
oracle is taught to instantiate with the right props (a plumbing cost per
component, similar in shape to a linter's own maintenance cost, but running
at Playwright/build time rather than at edit time).

### Recommendation

**Staged combination, not a single pick:**　**(a) now, hardened to a real
`syn`-based parser, as the immediate gate** — it is the only option cheap
enough to land this round, it directly matches the shape of the seven
guards this repo already runs before every commit, and this prototype
already demonstrates it finds the real, current hit list (623 sites, 66
files) with a low, now-understood false-positive/negative profile. Pair it
with **(e)** since a static guard only stops *new* instances once it starts
running gating commits — the ~623 existing sites need the oracle (or a
deliberate, tracked cleanup pass) to know which of them are actually
*live* bugs today versus latent-but-unexercised, exactly the distinction
`f2be1d7`'s own report drew for its five named survivors. Treat **(b)**
(the wrapper/API-level fix) as the follow-up worth scoping once the size of
the cleanup is known from (a)'s hardened output — it is strictly stronger
than (a) but too large to land blind. Do **(d)** regardless, in parallel,
at near-zero cost, since the file:line trace is now done. **(c)** is not a
general recommendation — it is already the *correct* fix for the specific
`id` cases that need a single caller-overridable identity (mirroring
`use_id_or`), but it cannot be the answer for the 223 `data-*`-shaped
findings this scan found, so treat it as a per-site tool the (a)/(b) cleanup
reaches for when the shape fits, not a class-level construction on its own.

**What none of this subsumes:** a `5fc1439`-shaped collision (ad-hoc
`#[props(extends)]` key plus a separately-threaded `attributes:` field
passed into a *child* component, rather than a same-element `..spread`) is
outside strict (a)'s literal-scan pattern as prototyped here (it would need
either a wider heuristic — flag any component whose call site sets a
`GlobalAttributes`-extended name AND separately forwards a full
`attributes` field to the same child — or the (b) API-level fix, which
subsumes it structurally). This prototype does not currently detect that
shape; I did not build or test a variant that does, for lack of remaining
budget in this investigation.

## Files produced (all under `$S/round4/deconstruct/`, nothing in the repo)

- `ROOT-CAUSE.md` — this file.
- `check-attr-spread-collision.py` — the guard prototype (final, post-fix
  version; iteration history in its own docstring and above).
- `style-shorthand-idents.txt` — 407 CSS-shorthand style-property idents,
  mechanically extracted from this lockfile's own
  `dioxus-html-0.7.9/src/attribute_groups.rs`.
- `findings-raw.txt` — the final 623-line raw output
  (`primitives/src` + `preview/src/components`).
- `findings-stats.txt` — the script's own summary counters for that run.

## What I could not determine (stated plainly, not guessed)

- Whether `dioxus-native`/Blitz's own `WriteMutations::set_attribute`
  actually coalesces duplicate calls the same way the web renderer's DOM
  does. I read that it's a separate crate/implementation in this lockfile
  but did not read its source; my "most likely mirrors the web renderer"
  statement above is inference from the shared `dioxus-core` contract, not
  a confirmed read.
- Whether `dioxus-core-macro`'s `Props` derive would *refuse to compile* (or
  is merely unlikely-in-practice) if a struct declared both a named field
  and an `extends`-covered attribute of the same name. I confirmed the
  `extends`/`extends_vec_ident` machinery exists and generates per-name
  builder plumbing; I did not trace far enough to say the exclusivity is a
  hard compiler-enforced guarantee versus a strong convention nothing in
  this codebase violates.
- A full, exhaustive hand-verification of all 623 guard findings. I
  spot-checked roughly a dozen (both authoring styles, multiple files,
  both confidence buckets) plus root-caused and fixed the two systematic
  bugs the iteration process surfaced; I did not verify the remaining
  ~610 individually.
- I did not run a full SSG build in this lane (no fix is landing, so there
  is nothing new to prove end-to-end; `f2be1d7`'s own report already carries
  the red→green SSG evidence for the three sites it fixed, and this
  investigation's job was the deconstruction, not another build). This
  matches the task brief's own steer that a full SSG build is "almost
  certainly unnecessary" here.
