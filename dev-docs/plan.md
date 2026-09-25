# The plan

The single authoritative sequence. Detail for each item lives in the documents linked from it; where an ordering here differs from a list in another document, **this one wins**.

**Status (2026-09-02): Phases 0–4 are done, merged to `main`.** All research is complete and merged; Phases 0–3 landed via PR #8, and Phase 4's architecture decision (native `<dialog>`/`popover=`), its two migrations (Migration A: DropdownMenu/ContextMenu/Menubar/Select/Combobox/Toast onto the top layer; Migration B: modal `Dialog`/`AlertDialog` onto native `<dialog>`), and a series of 2026-09-01/02 incident-response and hardening rounds (cfg-axis production incident, hydration-parity oracle, keyboard-open-contract fixes, menu-role-contract fix, anchored-overlay visualViewport tracking, touch text-entry zoom floor, global-stylesheet fix) have all since landed via PRs #10–#35. Phase 5 is partially done (the FLIP sub-problem landed via CSS `position-try-fallbacks`); Phase 6 has not been started. See each phase's table below for per-item status, and [`backlog.md`](./backlog.md) for what remains.

**Status (2026-09-18): Phase 5 is complete** (5.1 FLIP + shift/size clamp, 5.2 point-anchor clamp — `backlog.md` row 10; 5.3 dropped, see its row), **Phase 6 is in progress** (rows 11 and 12 taken this session; RTL, row 13, still waits on a fixture decision), and the shadcn-parity component wave (`component-backlog.md`) continues: Command and Input OTP landed 2026-09-14; Table, Data Table, Drawer, Navigation Menu and Resizable were taken 2026-09-18 (status per row in that file). A queue evaluation on 2026-09-18 concurred with this plan's ordering and corrected three things — see `backlog.md`, "2026-09-18 — queue evaluation and what was taken".

**Status (2026-09-19): batch 2 landed** (`backlog.md` rows 77-82) — a round of user-reported fixes against the components the 2026-09-18 wave had just shipped (Command palette focus, Data Table filter question, Tag Group sizing, Sheet, Resizable, Navigation Menu's dead exit animation plus its deliberately-deferred keyboard surface, and three Drawer fixes sharing one native-`<dialog>` centering construction with Sheet), two new reactivity guard scripts (`check-hooks-in-closures.sh`, `check-self-subscribing-effects.sh`), the CSS-hot-reload root-cause fix, and the remaining Playwright hardening (base-URL migration finished, three full-suite-only test-timing fixes). Integrated at `f7987aa` — full account in `backlog.md` rows 77-82.

**Status (2026-09-19): batch 3 landed** (`backlog.md` rows 13, 20-22, 45-46, 83-87). **Phase 6 row 13 (RTL) is now done** for the 13 components with RTL-aware interaction logic — the new `direction.rs` primitive (`Direction`/`DirectionProvider`/`use_direction`, `Direction::resolve_horizontal` as the single key-resolution site), the per-family key/CSS flips, and a new tier-3 oracle (`playwright/oracle/tier3-radix/rtl.spec.ts`, 25 rules) with its own fixture decision (a `rtl` variant page per RTL-aware component); `navigation_menu`/`drawer`/`command`/`date_picker` still have no RTL support, and whether `top_layer.rs`'s JS anchor engine is itself direction-aware remains an open question — full account in `backlog.md` row 13. The SSG lane (rows 22/46) was **executed for the first time** — enumerable path-segment routes, a production build with 71 routes and 0 not-found shells, hydration-parity 11/11. `date_picker` gained 22 new tests plus two page-hang fixes (rows 20/83, the same reactive-cycle family as row 73). The main-thread responsiveness oracle (row 45) landed, 55/58 green, with three subjects (`combobox`/`command`/`date_picker` `open`) red by design pending a future performance-focused batch. Integrated at `3ce62f5` — full account, every row, in `backlog.md`; the batch-3 integration outcome itself is row 87. **Carousel and Chart remain awaiting the user's own decision** — both fully researched this round (`dev-docs/research/chart-2026-09-19.md`, `dev-docs/research/carousel-2026-09-19.md`; `backlog.md` row 86), neither built.

**Correction to this file's own Phase 6 status line below** (now updated in place too): it previously cited "per `conformance-harness.md`'s queue item 8" for the RTL fixture-decision gap — verified, not assumed, and found to be a mis-citation: `conformance-harness.md` has no RTL-related queue; the only numbered list at a position resembling "item 8" is its tier-2 HTML rule checklist, whose item 8 is the unrelated `form`-attribute rule (`backlog.md` row 5). The RTL fixture decision that line described is resolved regardless, per row 13 above. `playwright/oracle/tier3-radix/rtl.spec.ts`'s own header repeats the same mis-citation, inherited from this file — not fixed there by this docs pass (`dev-docs/**` is this pass's own scope, not `playwright/**`), flagged for whoever next touches that spec file.

**Status (2026-09-20): round 4 landed** (`backlog.md` rows 84, 85, 89, 91-92). Carousel — the last shadcn/ui catalog item this file's own paragraph above still listed as "awaiting the user's own decision" — is now built and landed: the CSS scroll-snap engine (option (a) from `carousel-2026-09-19.md` §8), with `loop`, autoplay/rotation-control, and the tablist variant deferred to fast-follows per the approved decisions, and vertical orientation plus the RTL-swapped root arrow keys shipped in v1 since both fell out of the same construction for free; full account `backlog.md` row 91, and `component-backlog.md`'s own Carousel row. (Chart's own decision was already recorded as landed — see Open decisions item 6 below and `backlog.md` row 88 — so as of this round neither catalog item awaits a decision any longer.) Also this round: date picker's six a11y/UX findings (row 84, filed findings-only during batch 3) and the DropdownMenu/Menubar/ContextMenu external-focus defect (row 85, also filed during batch 3) are now fixed by construction; row 89's toggle/toggle_group/pagination residue is closed with two verified-benign non-fixes, not further code changes. A build-process trap found during this round's integration (row 92) — a brand-new component can leave `preview`'s `OUT_DIR` stale against a pre-existing `target` dir — is recorded there as a backlog item, not a phase/plan one.

**Status (2026-09-25 — round 8): user approved rows 90, 93, 101, 102; rows 4, 6, 14, 15, 22, 23, 28, 29, 30, 33, 40, 41b stayed explicitly user-gated and were not touched.** Work was dispatched as parallel Sonnet lanes with disjoint file ownership, per `CLAUDE.md`'s lane rule. **Chart stage 2 (row 90) is now LANDED** — all seven parked lanes from the 2026-09-20 handoff (`dev-docs/issues/chart-stage2-handoff.md`) were integrated from their pushed `worktree-agent-*` branches: per-chart-type galleries now match shadcn's own inventory exactly (Area 10/10, Bar 10/10, Line 10/10, Tooltip 9/9, Radar 14/14, Pie 11/11, Radial 6/6 — 70 demos), with the five cross-lane constructions that handoff document listed as still-needed (struct-update spread, `ChartKind::has_full_table()`, `StackMode`-aware layout, a family-agnostic tooltip-anchor construction, and two CSS blocks) all landed. Browser + SSG validation of the fully-integrated gallery tree was reported in progress in a separate lane at hand-off time; this docs pass could not itself re-verify that run, so treat it as reported, not independently confirmed here. **Row 93 (the duplicate-attribute-beside-an-unmerged-spread class) is now CLOSED** — the `check-attr-spread-collision.sh` gate itself had already landed; this round burned its debt register down from 788 to 1 across four lanes (overlay/menu primitives, remaining primitives, preview wrappers, carousel, plus chart's own count via stage 2's struct-update construction), leaving one verified false positive (`PopoverTrigger`'s `id`). One side effect worth keeping in mind for anyone editing tests: `merge_attributes` sorts attributes by name, which changed rendered attribute order and broke two string-slicing tests (now re-anchored on the tag start, not a matched substring) — see `backlog.md`'s new row 108 for the proposed by-construction fix (a shared test helper that parses attributes into a map), not yet built. **Row 102 (carousel edge rubber-band) LANDED** as mode B3 (edge transform on `CarouselContent`, never touching the browser's own scroll position during a wheel/trackpad gesture) and **row 101 FIXED** (the reference-fixture axe violation, scoped out via `expectNoAxeViolations`'s `include` option rather than editing the vendored page). Carousel fast-follows (`loop`, autoplay + APG rotation control, a tablist variant) were reported in progress by a separate lane at the time of this docs pass, not yet landed — see `backlog.md` row 91's own addendum. **Correction on record for row 13 and this file's own Phase 6 status line above:** the "`navigation_menu`/`drawer`/`command`/`date_picker` have no RTL support yet" follow-up is not a shadcn-parity gap — row 13 itself already found shadcn's own stylesheets carry essentially no RTL support at all; those four components already have logical-property CSS (`3ce62f5`) and lack only direction-aware keyboard logic plus an `rtl` fixture, and for `Command` (a vertical list) and `Drawer` (a physical `side` prop) there may be nothing meaningful to flip in the first place. Kept as an optional beyond-parity item, not a gap. Full per-row detail, evidence and commit citations: `backlog.md` rows 90, 91, 93, 101, 102, and its new rows 105-108.

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
| 2.4 | `use_animated_open` unmount race | `jcgruenhage@6f0a69f0` **+ a generation counter neither fork has** | **Done** 2026-08-29/30 (both landed). The follow-on `Wervice@a704c517` tooltip fade this item gated is now **landed 2026-09-18** — `backlog.md` row 7 (sequenced after row 19, its own dependency, in the same lane) |

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
| 5.2 | `ContextMenu` viewport clamping | Neither fork covers it; **not solvable by CSS anchors** — `ContextMenu` is positioned at click coordinates, needs the virtual-anchor JS path | **Landed 2026-09-14** — `top_layer::use_point_anchor_clamp`, a small parallel hook (not a branch inside `use_anchor_position_fallback`); see `backlog.md` row 10's 2026-09-14 addendum for the full account |
| 5.3 | Keep the CSS clamp on `fix/preview-a11y-ux` as defence-in-depth | Costs nothing; still helps non-wasm targets | **Dropped 2026-09-18** — the branch exists neither in this clone nor on the fork remote (only `main` does, verified with `git ls-remote`), so there is nothing to keep; the JS fallback now clamps on every engine (`backlog.md` row 10), so a re-derivation would only matter for a native-target report, and none exists |

## Phase 6 — Deferred but real

Typeahead for menus (`dq typeahead.rs`, 78 ln) — **do not touch `select/`**, whose matcher beats both alternatives. RTL (`dq direction.rs`, 83 ln, plus the key-flip *concept*, not the 708-line file). `pub mod portal`, `Toggle`'s `class` prop, and the public `CalendarDayState` API for upstream issue #199.

**Status (2026-09-18): rows 11 and 12 landed.** Typeahead (row 11 — `primitives/src/typeahead.rs`, a concept port of `dq typeahead.rs`'s prefix-matching idea, wired into DropdownMenu/ContextMenu/Menubar) and `pub mod portal`/`Toggle`'s `class` prop/public `CalendarDayState` (row 12, three from-scratch API-surface commits) both landed 2026-09-18 — see `backlog.md`'s "2026-09-18 — queue evaluation and what was taken" for the account. **Row 13 (RTL) landed 2026-09-19 (batch 3)** — `dq direction.rs`'s key-flip *concept* is now ported (`primitives/src/direction.rs`, `Direction::resolve_horizontal`), with a fixture decision made and a tier-3 oracle (`rtl.spec.ts`, 25 rules) proving it against 13 components; full account `backlog.md` row 13. (This line previously cited "`conformance-harness.md`'s queue item 8" for the fixture-decision gap — see the correction near the top of this file; the citation was wrong, but the fixture decision itself is real and landed regardless.)

---

## Open decisions

Owned by a person, not by this document. Status as of 2026-09-02:

1. **Where fixes land** — upstream PRs, this fork, or both. **Still open.** Upstream `main` had not moved since 2026-06-29 when this was written; **it moved on 2026-09-07/08** (three commits — one cosmetic fix adopted, two `use_animated_open` fixes already subsumed here; `backlog.md` row 66), so the rebase cost below is real after all. Nothing here is blocked *on* upstream, but anything carried locally is a permanent rebase cost on files upstream actively changes. Tracked as `backlog.md` row 6.
2. **Phase 4's native question** (4.1). **Resolved by taking 4.2** (native `<dialog>`) without waiting on an upstream answer — caveat 1 was investigated by execution instead and found not to block. Asking upstream *why* `797b343e` dropped `<dialog>` remains open as a courtesy/consolidation question, not a blocker.
3. **Phase 5's dependency question** (5.1). **Reframed, not answered as originally posed** — see Phase 5 above: the FLIP sub-problem is solved CSS-natively, sidestepping the `sr`/`dq` fork-dependency choice entirely; that choice would only resurface if shift/size clamping ends up needing a JS collision library.
4. **Whether to talk to the fork authors at all.** `dignifiedquire` and `sarendipitee` independently built scroll lock, focus coordination and collision detection while upstream sat still. **Still open** — consolidation is worth more than any cherry-pick sequence, and neither is an upstream contributor, so nobody is currently merging this work.
5. **Whether native (Blitz) is a target you care about.** If yes, several documented gaps are worse there than described, because `eval` is a no-op on that renderer. **Still open**, though narrower now than when this was written: `preview`'s `desktop` feature lacks `dioxus-primitives/web` (`backlog.md` row 23), a one-line gap in the same family as the cfg-axis incident, untested because no desktop build has been exercised yet.
6. **Chart engine -> `dioxus-community/dioxus-charts` (new, 2026-09-19).** User decision, recorded in `backlog.md` row 88: keep building the chart engine in-repo now, structured so `primitives/src/chart/engine/` (scales, ticks, curves, stacking, series geometry, the data-table model) stays separable from the shadcn-shaped layer on top (`ChartConfig` -> CSS variables, `ChartContainer`, `ChartTooltip`, `ChartLegend`, keyboard stepping via `crate::direction`) — pure Rust, no dependency on this crate's other modules, so it can be lifted into `dioxus-community/dioxus-charts` later. Landed 2026-09-19 (`backlog.md` row 88): the engine/layer seam is real and verified (`engine/` has no `dioxus` types and no `use crate::` of this crate's other modules, confirmed by reading every file under it). **Still open:** the actual upstreaming — offering the engine to the `dioxus-community` org once this repo's own Chart has proven itself (oracle green, deployed) — needs the user's own GitHub account or a session with that repository attached (this session is scoped to `MentalGear/dioxus-components` only), so, like open decision 1's own upstream question, it is owned by a person, not by this document.

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
