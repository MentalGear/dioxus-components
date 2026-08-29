# The plan

The single authoritative sequence. Detail for each item lives in the documents linked from it; where an ordering here differs from a list in another document, **this one wins**.

**Status: nothing has been built.** All research is complete and merged; no component code has been changed, and only one claim (focus restore) has been verified by execution.

---

## Definition of done, per item

Every item follows the same loop. An item is not done until all five hold:

1. A conformance test exists, cites its rule source, and **fails first** — per [`conformance-harness.md`](./conformance-harness.md). A port that lands without a red test has proved nothing.
2. The implementation follows [`recommended-implementations.md`](./recommended-implementations.md), which picks the best source per gap rather than copying one fork.
3. Provenance and licence handled per [`lifting-from-forks.md`](./lifting-from-forks.md) §1 — cherry-picks keep their author; lifted files get a header.
4. The repo's own checks pass: `cargo clippy --workspace --examples --tests --all-features --all-targets -- -D warnings`, `cargo fmt --check`, `cargo test --workspace`, stylelint, and Playwright.
5. The provenance ledger in [`lifting-from-forks.md`](./lifting-from-forks.md) §8 is updated.

---

## Phase 0 — Foundations

Nothing below can be validated without these, and none of them touch component behaviour.

| # | Item | Why | Status |
|---|---|---|---|
| 0.1 | **Green baseline** — build the workspace and run the full Playwright suite, recording what passes today | Every later claim of "this fixes X" needs a before | Not started. The preview builds (~4 min); see `lifting-from-forks.md` §6 for the `dx`/proxy/Chromium gotchas |
| 0.2 | **Oracle structure** — `playwright/oracle/{tier1-apg,tier2-html,tier3-radix,reference,subjects}` | Keeps rule tiers from blurring into "matches Radix" | Not started; layout specified in `conformance-harness.md` |
| 0.3 | **Vendor APG reference pages**, pinned by commit | Upgrades tier 1 calibration from an internal control (Dialog, chosen because it already worked) to W3C's own reference | Not started; pages verified reachable |
| 0.4 | **Form fixture in `preview/`** — a real `<form>` with a submit button and a native control beside each component | Blocks all of Phase 1; **no form demo exists today** | Not started |

## Phase 1 — Form controls that lie *(highest severity)*

`RadioGroup` and `Select` declare `name`/`required`, document them as being for form submission, and never reference them. A developer following the documented API ships a form that silently omits the field.

| # | Item | Source | Depends on |
|---|---|---|---|
| 1.1 | `Switch` — forward `required` to the hidden input | `dignifiedquire@switch.rs:128` — one line | 0.4 |
| 1.2 | `RadioGroup` — per-item hidden `<input type="radio">` | `dq radio_group.rs:267-279` / `sr :338-348`, independently identical | 0.4 |
| 1.3 | `Select` — hidden native `<select>` with mirrored options | `dq select/components/select.rs:158-186`; needs a `required` prop that does not exist yet | 0.4, 1.2 |

Not new design: the pattern is already in-tree in `Checkbox`. Render unconditionally, matching `Checkbox` — the reasoning is in `recommended-implementations.md`, and it is a framework limitation this repo already documented in `complaints.md`, not a preference.

## Phase 2 — The verified mined fixes *(cheapest — already cherry-pick tested)*

| # | Item | Source | Note |
|---|---|---|---|
| 2.1 | `RangeSlider` thumb identity at collision | `sarendipitee@42b56dd3` | Clean cherry-pick |
| 2.2 | `VirtualList` borrow held across the call that reads it | `sarendipitee@799a4ff3` | Primitives hunk only |
| 2.3 | Popover self-dismiss on internal click | `sarendipitee@f63ee07e` | Fixes the shared `use_outside_dismiss`; **re-test Dialog too** |
| 2.4 | `use_animated_open` unmount race | `jcgruenhage@6f0a69f0` **+ a generation counter neither fork has** | Then, and only then, `Wervice@a704c517` tooltip fade |

None shipped with a test; each needs a regression test. Sequence and exact commands: `adopt-fork-fixes-results.md` §8.

## Phase 3 — Accessibility behaviour

| # | Item | Source | Note |
|---|---|---|---|
| 3.1 | **Focus restore, menu family** | `dq lib.rs:241-255`, Radix `onCloseAutoFocus` semantics | **The oracle is already written and red** — `oracle-focus-restore.spec.ts`. Must also move focus *off* the closed item, which our measurement found and no source handles |
| 3.2 | **Body scroll lock** | `dq scroll_lock.rs` (58 ln) + `sr`'s unlock-flash guard | Needed regardless of how Phase 4 resolves; native `<dialog>` does not scroll-lock |

## Phase 4 — The native-platform decision

**Open question, and the highest-leverage one.** Native `<dialog>` + `showModal()` supplies focus trap, focus restore, inert background and top layer as browser behaviour — subsuming a 743-line `focus_scope.rs` and a 91-line `aria_hidden.rs` port.

| # | Item | Note |
|---|---|---|
| 4.1 | **Ask upstream why `797b343e` dropped `<dialog>`** | No recorded rationale; not a blocker for Phases 0–3 |
| 4.2 | If clear → native `<dialog>`, with `open` bound declaratively as the floor | Blitz styles `dialog:not([open])` as `display:none`, and `open` needs no JS |
| 4.3 | If blocked → port `aria_hidden.rs` **and finish the Dialog/AlertDialog wiring that fork never did** | Strictly more work for less capability |
| 4.4 | Non-modal overlays → `popover=` top layer | Fixes clipping inside `overflow:hidden`/transformed ancestors |

## Phase 5 — Collision detection

No overlay does any. Placement is static CSS keyed off `data-side`, so anything near a viewport edge renders off-screen; `ContextMenu` opens at raw click coordinates unclamped.

| # | Item | Note |
|---|---|---|
| 5.1 | **Dependency decision** — `sr floating.rs` (269 ln, external crates) vs `dq`'s vendored port (18 files, 3,262 ln) | Recommendation: the wrapper; keep the port as contingency |
| 5.2 | `ContextMenu` viewport clamping | Neither fork covers it |
| 5.3 | Keep the CSS clamp on `fix/preview-a11y-ux` as defence-in-depth | Costs nothing; still helps non-wasm targets |

## Phase 6 — Deferred but real

Typeahead for menus (`dq typeahead.rs`, 78 ln) — **do not touch `select/`**, whose matcher beats both alternatives. RTL (`dq direction.rs`, 83 ln, plus the key-flip *concept*, not the 708-line file). `pub mod portal`, `Toggle`'s `class` prop, and the public `CalendarDayState` API for upstream issue #199.

---

## Open decisions

Owned by a person, not by this document:

1. **Where fixes land** — upstream PRs, this fork, or both. Upstream `main` has not moved since 2026-06-29, so nothing here is blocked *on* upstream, but anything carried locally is a permanent rebase cost on files upstream actively changes.
2. **Phase 4's native question** (4.1).
3. **Phase 5's dependency question** (5.1).
4. **Whether to talk to the fork authors at all.** `dignifiedquire` and `sarendipitee` independently built scroll lock, focus coordination and collision detection while upstream sat still. Consolidation is worth more than any cherry-pick sequence — and neither is an upstream contributor, so nobody is currently merging this work.
5. **Whether native (Blitz) is a target you care about.** If yes, several documented gaps are worse there than described, because `eval` is a no-op on that renderer.

## What is already done

- Fork network mined: 81 forks, 111 novel refs → 4 adoptable fixes, ~11 conditional, ~45 rejected.
- Capability inventory across a11y, overlays and forms.
- Porting playbook, harness design, and best-of-each recommendations.
- One oracle written and executed: 4 red, 1 control green — which also corrected the static analysis.
- Four adversarial review rounds, which caught six factual errors and two broken commands across the set.
