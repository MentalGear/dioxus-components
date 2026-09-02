# The plan

The single authoritative sequence. Detail for each item lives in the documents linked from it; where an ordering here differs from a list in another document, **this one wins**.

**Status (2026-09-02): Phases 0–4 are done, merged to `main`.** All research is complete and merged; Phases 0–3 landed via PR #8, and Phase 4's architecture decision (native `<dialog>`/`popover=`), its two migrations (Migration A: DropdownMenu/ContextMenu/Menubar/Select/Combobox/Toast onto the top layer; Migration B: modal `Dialog`/`AlertDialog` onto native `<dialog>`), and a series of 2026-09-01/02 incident-response and hardening rounds (cfg-axis production incident, hydration-parity oracle, keyboard-open-contract fixes, menu-role-contract fix, anchored-overlay visualViewport tracking, touch text-entry zoom floor, global-stylesheet fix) have all since landed via PRs #10–#35. Phase 5 is partially done (the FLIP sub-problem landed via CSS `position-try-fallbacks`); Phase 6 has not been started. See each phase's table below for per-item status, and [`backlog.md`](./backlog.md) for what remains.

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
| 0.1 | **Green baseline** — build the workspace and run the full Playwright suite, recording what passes today | Every later claim of "this fixes X" needs a before | **Done** — `baseline.md` (2026-08-29): 119/127 passing; see that doc's own note on what's since changed |
| 0.2 | **Oracle structure** — `playwright/oracle/{tier1-apg,tier2-html,tier3-radix,reference,subjects}` | Keeps rule tiers from blurring into "matches Radix" | **Done** — directory tree exists as specified in `conformance-harness.md` |
| 0.3 | **Vendor APG reference pages**, pinned by commit | Upgrades tier 1 calibration from an internal control (Dialog, chosen because it already worked) to W3C's own reference | **Done** — `playwright/oracle/reference/7e4034b/`, pinned commit `7e4034b`, see that directory's README |
| 0.4 | **Form fixture in `preview/`** — a real `<form>` with a submit button and a native control beside each component | Blocks all of Phase 1; **no form demo exists today** | **Done** — `preview/src/components/form/component.rs` (`FormFixture`); exercised by `oracle/tier2-html/form-participation.spec.ts` |

## Phase 1 — Form controls that lie *(highest severity)*

`RadioGroup` and `Select` declare `name`/`required`, document them as being for form submission, and never reference them. A developer following the documented API ships a form that silently omits the field.

| # | Item | Source | Status |
|---|---|---|---|
| 1.1 | `Switch` — forward `required` to the hidden input | `dignifiedquire@switch.rs:128` — one line | **Done** 2026-08-29 (`switch.rs`) |
| 1.2 | `RadioGroup` — per-item hidden `<input type="radio">` | `dq radio_group.rs:267-279` / `sr :338-348`, independently identical | **Done** 2026-08-29 (`radio_group.rs`) |
| 1.3 | `Select` — hidden native `<select>` with mirrored options | `dq select/components/select.rs:158-186`; needed a new `required` prop | **Done** 2026-08-29 (`select/components/select.rs`) |

Not new design: the pattern is already in-tree in `Checkbox`. Render unconditionally, matching `Checkbox` — the reasoning is in `recommended-implementations.md`, and it is a framework limitation this repo already documented in `complaints.md`, not a preference.

## Phase 2 — The verified mined fixes *(cheapest — already cherry-pick tested)*

| # | Item | Source | Status |
|---|---|---|---|
| 2.1 | `RangeSlider` thumb identity at collision | `sarendipitee@42b56dd3` | **Done** 2026-08-29, clean cherry-pick |
| 2.2 | `VirtualList` borrow held across the call that reads it | `sarendipitee@799a4ff3` | **Done** 2026-08-29, primitives hunk only |
| 2.3 | Popover self-dismiss on internal click | `sarendipitee@f63ee07e` | **Done** 2026-08-29; fixed the shared `use_outside_dismiss`, Dialog re-tested |
| 2.4 | `use_animated_open` unmount race | `jcgruenhage@6f0a69f0` **+ a generation counter neither fork has** | **Done** 2026-08-29/30 (both landed). The follow-on `Wervice@a704c517` tooltip fade this item gated is **not** landed — tracked as `backlog.md` row 7, "Unblocked" |

All four shipped with a regression test per the definition of done. Provenance ledger: `lifting-from-forks.md` §8.

## Phase 3 — Accessibility behaviour

| # | Item | Source | Status |
|---|---|---|---|
| 3.1 | **Focus restore, menu family** | `dq lib.rs:241-255`, Radix `onCloseAutoFocus` semantics | **Done** 2026-08-30 — `use_previous` + `use_refocus_on_close_unless`, wired into DropdownMenu/ContextMenu/Select/Menubar; also moves focus *off* the closed item, which our measurement found and no source handled; `oracle-focus-restore.spec.ts` 4 red → 5/5 green |
| 3.2 | **Body scroll lock** | `dq scroll_lock.rs` (58 ln) + `sr`'s unlock-flash guard | **Done** 2026-08-30, hardened through several later generations (scrollbar-gutter baseline PR #10, event-interception rewrite PR #16); native `<dialog>` does not scroll-lock on its own, so this stayed needed after Phase 4 |

## Phase 4 — The native-platform decision

**Open question, and the highest-leverage one.** Native `<dialog>` + `showModal()` supplies focus trap, focus restore, inert background and top layer as browser behaviour — subsuming a 743-line `focus_scope.rs` and a 91-line `aria_hidden.rs` port.

| # | Item | Note | Status |
|---|---|---|---|
| 4.1 | **Ask upstream why `797b343e` dropped `<dialog>`** | No recorded rationale; not a blocker for Phases 0–3 | **Not started** — user decision (`backlog.md` row 6, "upstream engagement"), unaffected by 4.2 going ahead anyway |
| 4.2 | If clear → native `<dialog>`, with `open` bound declaratively as the floor | Blitz styles `dialog:not([open])` as `display:none`, and `open` needs no JS | **Done** 2026-08-31 (Phase 4.2) — modal `Dialog`/`AlertDialog` web arm renders a real `<dialog>` + `showModal()`; native/Blitz arm unchanged; own oracle `oracle/tier2-html/native-dialog.spec.ts` |
| 4.3 | If blocked → port `aria_hidden.rs` **and finish the Dialog/AlertDialog wiring that fork never did** | Strictly more work for less capability | **N/A** — 4.2 was taken; caveat 1 investigated by execution and found not to block (see `recommended-implementations.md`) |
| 4.4 | Non-modal overlays → `popover=` top layer | Fixes clipping inside `overflow:hidden`/transformed ancestors | **Done** 2026-08-31 (Phase 4.4), then extended to every remaining overlay across two migrations: Migration A (DropdownMenu, ContextMenu, Menubar, Select, Combobox, Toast onto the top layer) and Migration B (modal Popover onto the native-dialog engine) — both complete as of PR #23 |

## Phase 5 — Collision detection

No overlay did any as of Phase 4. Placement was static CSS keyed off `data-side`, so anything near a viewport edge rendered off-screen; `ContextMenu` opens at raw click coordinates unclamped.

**Status (2026-09-01): 5.1 reframed and its FLIP sub-problem landed.** A bits-ui/Radix source comparison found both delegate to Floating UI's identical flip/shift/size pipeline; since CSS Anchor Positioning was already in the stack from Phase 4.4, `position-try-fallbacks: flip-block, flip-inline` covers FLIP natively (zero JS, PR #17), with `use_anchor_position_fallback` making the same flip decision from viewport math on engines without native support. Neither fork's dependency (`sr floating.rs` nor `dq`'s vendored port) was taken — the CSS-native approach superseded the 5.1 dependency question below for the FLIP sub-problem. Shift/size clamping and 5.2 remain open.

| # | Item | Note | Status |
|---|---|---|---|
| 5.1 | **Dependency decision** — `sr floating.rs` (269 ln, external crates) vs `dq`'s vendored port (18 files, 3,262 ln) | Superseded for FLIP by the CSS-native `position-try-fallbacks` approach above; the dependency question survives only if shift/size clamping ends up needing a JS collision-detection library | **FLIP done** (PR #17); shift/size clamping **not started** |
| 5.2 | `ContextMenu` viewport clamping | Neither fork covers it; **not solvable by CSS anchors** — `ContextMenu` is positioned at click coordinates, needs the virtual-anchor JS path | **Not started** — `backlog.md` row 10 |
| 5.3 | Keep the CSS clamp on `fix/preview-a11y-ux` as defence-in-depth | Costs nothing; still helps non-wasm targets | **Not started** — that branch is still unmerged (verified: not an ancestor of `main`) |

## Phase 6 — Deferred but real

Typeahead for menus (`dq typeahead.rs`, 78 ln) — **do not touch `select/`**, whose matcher beats both alternatives. RTL (`dq direction.rs`, 83 ln, plus the key-flip *concept*, not the 708-line file). `pub mod portal`, `Toggle`'s `class` prop, and the public `CalendarDayState` API for upstream issue #199.

**Status: not started.** All four items are still open — `backlog.md` rows 11 (typeahead), 13 (RTL), and 12 (portal/class prop/`CalendarDayState`).

---

## Open decisions

Owned by a person, not by this document. Status as of 2026-09-02:

1. **Where fixes land** — upstream PRs, this fork, or both. **Still open.** Upstream `main` has not moved since 2026-06-29, so nothing here is blocked *on* upstream, but anything carried locally is a permanent rebase cost on files upstream actively changes. Tracked as `backlog.md` row 6.
2. **Phase 4's native question** (4.1). **Resolved by taking 4.2** (native `<dialog>`) without waiting on an upstream answer — caveat 1 was investigated by execution instead and found not to block. Asking upstream *why* `797b343e` dropped `<dialog>` remains open as a courtesy/consolidation question, not a blocker.
3. **Phase 5's dependency question** (5.1). **Reframed, not answered as originally posed** — see Phase 5 above: the FLIP sub-problem is solved CSS-natively, sidestepping the `sr`/`dq` fork-dependency choice entirely; that choice would only resurface if shift/size clamping ends up needing a JS collision library.
4. **Whether to talk to the fork authors at all.** `dignifiedquire` and `sarendipitee` independently built scroll lock, focus coordination and collision detection while upstream sat still. **Still open** — consolidation is worth more than any cherry-pick sequence, and neither is an upstream contributor, so nobody is currently merging this work.
5. **Whether native (Blitz) is a target you care about.** If yes, several documented gaps are worse there than described, because `eval` is a no-op on that renderer. **Still open**, though narrower now than when this was written: `preview`'s `desktop` feature lacks `dioxus-primitives/web` (`backlog.md` row 23), a one-line gap in the same family as the cfg-axis incident, untested because no desktop build has been exercised yet.

## What is already done

- Fork network mined: 81 forks, 111 novel refs → 4 adoptable fixes, ~11 conditional, ~45 rejected. All 4 adoptable fixes landed (Phase 2).
- Capability inventory across a11y, overlays and forms.
- Porting playbook, harness design, and best-of-each recommendations.
- Phases 0–4 built and merged: form participation (Phase 1), the four mined fixes (Phase 2), focus restore + scroll lock (Phase 3), and the native-platform decision with both migrations complete (Phase 4) — see each phase's table above for landing PRs.
- Phase 5's FLIP sub-problem landed (CSS `position-try-fallbacks`, PR #17); shift/size clamping and `ContextMenu`'s point-anchor clamp remain open.
- A 2026-09-01 production incident (cfg-axis: markup split on `target_family = "wasm"` instead of the `web` feature, breaking the deployed SSG site) was root-caused, fixed, and given a standing regression guard (`scripts/check-cfg-axis.sh`) plus a new oracle axis (`oracle/hydration-parity.spec.ts`) — see `recommended-implementations.md` Caveat 1.
- A 2026-09-01/02 round of hardening beyond the original plan: the keyboard-matrix oracle's 12 reds → 0 (DropdownMenu/Menubar/Select's shared open-with-focus contract, Slider's missing Home/End/PageUp/PageDown, HoverCard's missing Escape dismiss), the menu pattern-class role contract (`menu_semantics.rs`), anchored-overlay visualViewport tracking for the iOS keyboard case, a text-entry touch-zoom font-size floor, and a global-stylesheet `@import` fix — see `recommended-implementations.md`'s per-item sections and `backlog.md` for the individual rows.
- Oracles written and executed well beyond the original one: focus restore (5/5 green), keyboard matrix, menu roles, top layer (11 rules), native dialog, touch focus-zoom, global stylesheet, form participation, scroll lock, hydration parity — see `conformance-harness.md`'s Status section for the full current inventory.
- Several adversarial review rounds across the research and execution documents, catching factual errors and broken commands.
