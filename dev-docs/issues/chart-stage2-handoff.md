# Chart stage 2 — per-chart-type gallery handoff (parked)

**RESOLVED 2026-09-25 (round 8):** the user gave the go-ahead (approved `dev-docs/backlog.md`
row 90) and all seven parked lanes below were integrated from their pushed `worktree-agent-*`
branches. Every gallery this document tracks as part-done now matches shadcn's own inventory
exactly — Area 10/10, Bar 10/10, Line 10/10, Tooltip 9/9, Radar 14/14, Pie 11/11, Radial 6/6 (70
demos) — and all five cross-lane constructions in §4 below landed. `RadialBar`'s primitive was
built out from the stub §2/§3 describe as unstarted. The body of this document below is kept
**as the historical record** of the parked state, the per-lane ledger discrepancies, and the
construction designs that were then executed largely as proposed — it is not being rewritten to
read as if stage 2 shipped in one pass. Full landing account, evidence, gate results and commit
range: `dev-docs/backlog.md` row 90's own dated addendum; `component-backlog.md`'s Chart row.
**Not re-verified by this docs pass:** browser + SSG validation of the fully integrated gallery
tree (with row 93's attribute-merge fixes also applied) was reported in progress in a separate
lane at hand-off time — that status is carried forward from that lane's own report, not
independently confirmed here.

---

Status (as of 2026-09-20, superseded above): **parked**, pending the user's go-ahead to resume. Chart stage 2 was
dispatched as seven parallel lanes on 2026-09-19, hit the weekly usage limit mid-flight, was
resumed, then moved to a one-lane-at-a-time process (`s2-area` → `s2-bar` → `s2-line` →
`s2-tooltip` → `s2-radar` → `s2-polar`) of which only `s2-area` got its dedicated turn before the
whole effort was parked. **Six of the seven lanes hold real, substantially-finished work that
exists ONLY in this container**: local git worktrees under
`/home/user/dioxus-components/.claude/worktrees/agent-<id>` on local branches
`worktree-agent-<id>`, uncommitted working-tree edits inside several of those worktrees, and — for
one lane specifically — a complete drafted gallery sitting only in the session scratchpad, which is
more fragile than the worktrees. **Superseded 2026-09-20: all seven `worktree-agent-*` branches
have since been pushed to `origin`** (with WIP checkpoint commits for the four lanes that still had
uncommitted edits, and the scratchpad-only radar gallery copied into its worktree first), so the
committed work is now durable. None of it is on `main` or on `claude/roadmap-evaluation-6eiahc`.
§7 has the per-branch SHAs and says what each branch preserves.

`$S`, used throughout this document, is this container's own session scratchpad —
`/tmp/claude-0/-home-user-dioxus-components/0c0a1bde-8dc9-51c3-91e6-e93c0c00fb9e/scratchpad`, not
part of the git repository and not backed by any durable storage. Anything cited only under `$S`
(as opposed to a worktree path) exists solely on this container's local disk.

This document is written so a fresh session with none of this container's history can either (a)
resume the work directly from the worktrees/scratchpad named below, or (b) rebuild it from scratch
without re-discovering any of the findings, root causes, or constructions already worked out.

## 1. What stage 2 is, and what stage 1 built

shadcn/ui ships one gallery **page per chart type** — `charts/chart-area-*.tsx`,
`chart-bar-*.tsx`, `chart-line-*.tsx`, `chart-pie-*.tsx`, `chart-radar-*.tsx`, `chart-radial-*.tsx`,
`chart-tooltip-*.tsx` — each a set of full demo variants (10 area, 10 bar, 10 line, 11 pie, 14
radar, 6 radial, 9 tooltip = 70 demos total, per `dev-docs/research/chart-2026-09-19.md` §1.2's
inventory). Stage 1 (landed 2026-09-19, `dev-docs/backlog.md` row 88) shipped exactly one
`/component/chart/` page — the shared engine plus four demo variants (`main` interactive area,
`bar`, `line`, `stacked`) — good engine coverage, nowhere near shadcn's per-type breadth. The user
noticed this directly: *"does it have same chart coverage then original shadcn? maybe the docs
should be sub components per type chart"* — approved 2026-09-19 or the exact brief text
(`$S/stage2-common.md`'s own header): *"shadcn gallery parity per chart type; start now, do not
wait for the stage-1 merge."* Stage 2 is that gallery-parity work: one installable-package-adjacent
gallery per chart type (`area_chart`, `bar_chart`, `line_chart`, `pie_chart` + `radial_chart` for
the two polar kinds, `radar_chart`, `chart_tooltip`), porting shadcn's own demos faithfully (data,
series, config, copy — each variant's doc comment cites the exact shadcn source file), plus
whatever new primitive surface (per-family `*Options` structs) those demos need that stage 1 didn't
build.

**What stage 1 built, that stage 2 extends (full contract: `chart-api.md`, copied verbatim into
`$S` from the stage-1 round):** `primitives/src/chart/` — pure math with no `dioxus` types under
`engine/{scale,curve,stack,geometry,table,data}.rs` (an upstream candidate for
`dioxus-community/dioxus-charts`), a Dioxus-facing bridge in `context.rs`
(`ChartContext`/`use_chart`), and the components themselves in
`components/{container,chart,tooltip,legend}.rs` (`ChartContainer`, `Chart`, `ChartTooltip`,
`ChartLegend`). The themed package `preview/src/components/chart/` wraps all four with `dx-chart*`
classes and a stylesheet; `--dx-chart-1..8` categorical tokens live in
`preview/assets/dx-components-theme.css`. Specs: `playwright/chart.spec.ts` +
`playwright/oracle/tier3-radix/chart.spec.ts`, 13/13 at the time stage 1 landed. Stage 1 shipped as
eight commits (`f881ceb`..`9a6cb0c`) merged via PR #62 onto `main` (`1642100`), followed by a README
fix (`9390435`) and then a small, unrelated CSS fix — `chart-legend-overlap`, cherry-picked as
`03927e3`, the current branch HEAD — that moved `ChartTooltip` and `ChartLegend` into separate CSS
grid areas so an overlapping legend/tooltip bug (`ChartTooltip` landing on top of a top-aligned
legend) can't recur. **`03927e3` is not part of stage 2** and is already pushed; it is HEAD only
because it landed on this branch after stage 1 and before stage 2 was dispatched.

Stage 2's own base commit, `9a6cb0c`, is `main`'s `0ae53fc` (an ancestor of the eventual `1642100`
merge) plus the stage-1 chart round. Every stage-2 worktree branches from `9a6cb0c`, **not** from
the later `1642100`/`03927e3` — a fact that matters for §6's integration checklist, since none of
the seven lanes has ever seen the `chart-legend-overlap` fix.

## 2. Per-lane status

Seven lanes: `s2-refactor` (structural, lands first), `s2-area`, `s2-bar`, `s2-line`, `s2-polar`
(covers both `Pie` and `RadialBar`), `s2-radar`, `s2-tooltip`. Agent ids, worktree paths and branch
names below are exact — every worktree still exists on disk and every fact in this section was
re-verified against it directly (`git log`, `git status --porcelain=v1 --untracked-files=all`,
`git diff --stat`), not copied from the ledger without checking. **Where the ledger
(`$S/stage2-lanes.md`) or the progress report (`$S/stage2-progress-report.md`) disagreed with what
the worktree actually contains, the worktree wins and the disagreement is called out explicitly
below** — there were several, all detailed in their lane's own subsection.

### Status table

| Lane | Agent id | Own commits (in order) | Refactor integration | Uncommitted work on disk | Ever run against a live `dx serve`? | Own gates last run |
|---|---|---|---|---|---|---|
| s2-refactor | `af6c70a282d507c23` | `b1cc636` | — (is the refactor) | none, worktree clean | scoped `cargo check -p preview` + SSR tests only; **no dev-server check** (explicitly allowed by the brief for a pure rename with identical rendered output) | 11/11 scripts + `fmt`/`clippy --workspace`/`test --workspace` (dioxus-primitives 269/0, +9) + rustdoc ×2, all green |
| s2-area | `aef4de610bdb8e2cf` | `0502c68`, `da962e2`, `5688459`, `85375d3` | cherry-picked **with conflicts**, resolved inside `da962e2` | none — worktree clean (only an untracked, never-committed local `playwright/session.local.config.ts`) | **yes** — `dx serve --port 8130`, light+dark screenshots saved under `$S/stage2/s2-area/screenshots/`, `area_chart.spec.ts` 25/25, `chart.spec.ts`+oracle 13/13, `preview.spec.ts` 3/3 | 11/11 + `fmt`/`clippy --workspace`/`test --workspace` (dioxus-primitives 279/0, +10) + rustdoc ×2, all green |
| s2-bar | `a1519b1536ba3cc38` | `484270a`, `bc5872f` | clean cherry-pick (`bc5872f`'s diffstat is byte-identical to `b1cc636`'s: 27 files, 1488(+)/627(-)) | **yes** — `components/layout.rs` (+39/-ish), `components/series/bar.rs` (+782 lines!), `engine/geometry.rs` (+77 lines), none committed | **no** — `$S/dx-serve-s2-bar.log` is 621 bytes: only the startup banner, no `Build completed` line anywhere | not run since the two committed commits (which predate all the uncommitted work) |
| s2-line | `ab986dd399489fca7` | `5dceba8`, `9344927` | clean cherry-pick (identical diffstat to `b1cc636`) | **yes** — `components/series/line.rs` (+623 lines), `components/series/mod.rs` (re-export widening), `line_chart/component.rs` (+11) + 6 variant files (+1-2 each), none committed | **partially** — `$S/dx-serve-s2-line.log` shows the cold build actually *completing*: `714.11s INFO Build completed successfully in 713.08s, launching app!` — one step further than the lane's own self-report ("did NOT run this lane's own dx serve") admits, but the log has nothing after that line: no recorded interaction, screenshot, or spec run. Treat as **not interactively verified**, even though the server did come up once | not run since the two committed commits |
| s2-polar | `a73bafbe9f257480b` | `942794d` | **fast-forwarded**, not re-committed — `942794d`'s own parent is `b1cc636` itself (same SHA as `s2-refactor`'s own commit, confirmed via `git cat-file -t`/`git log`), unlike every other lane which produced a new commit object | **yes** — `components/chart.rs` (+104, out-of-ownership, uncommitted), `components/series/pie.rs` (+363: real `PieOptions`/`PieLabels`/`render`), `engine/table.rs` (+104: `table_rows_pie`); plus untracked doc-only skeletons, see its own subsection | **no** — no `$S/dx-serve-s2-polar.log` exists at all | not run since the one committed commit |
| s2-radar | `a54aab42e9a00e001` | `0f99984`, `ae7374a` | clean cherry-pick (`15a8b41`, identical diffstat, single parent = its own base `9a6cb0c`) | **none in the worktree** (`git status`/`git diff` both empty) — **but see below: a complete 14-variant gallery draft exists only in `$S`, the session scratchpad, not copied into the worktree at all** | **no** — no `$S/dx-serve-s2-radar.log` exists at all | 11/11 + rustdoc claimed by the lane's own report for the two committed engine/series commits; not independently re-run by this handoff |
| s2-tooltip | `ae1ca1ec628383540` | `4a10b37`, `cff011b` | clean cherry-pick (identical diffstat) | **yes** — `components/tooltip.rs` (+461 lines!), `components/legend.rs` (+133), `chart_tooltip/component.rs` (+73) + `docs.md` + 4 existing variant tweaks + `preview/src/components/mod.rs` registration widening, none committed; 5 more variant directories untracked | **no** — no `$S/dx-serve-s2-tooltip.log` exists at all | `cargo check -p dioxus-primitives --features web` clean, self-reported "in flight" for `cargo check -p preview`; no workspace-wide gate run recorded since |

Two lanes are gate-green and fully committed (`s2-refactor`, `s2-area`); the other five are
part-done, in the sense the task that dispatched this work uses the word: real progress exists,
none of it is safely landed, and — except for `s2-line`'s one-time server boot — none of it has
been checked against a running page.

### s2-refactor — DONE, merged into every other lane

Worktree: `/home/user/dioxus-components/.claude/worktrees/agent-af6c70a282d507c23`, branch
`worktree-agent-af6c70a282d507c23`, base `9a6cb0c`. One commit:

- `b1cc636` — `refactor(chart): split series rendering per family and reserve the stage-2 extension points` (27 files, +1488/-627)

This is the structural commit every other lane depends on — see §3 for the ownership map it
established. Three deviations from `$S/stage2-common.md`'s literal brief, all stated in the
commit's own message and reproduced here because §6 needs them:

1. `chart/mod.rs`'s `engine` re-export stays an explicit, hand-written list, **not**
   `pub use engine::*;`. A true glob re-exports `engine`'s own submodule *names* too, and the new
   stub `engine::radar` collides with `components::series::radar` (this commit's real Radar family
   file) the instant both are blanket-globbed into `chart::`'s flat namespace —
   `error: ambiguous glob re-exports`, caught by `cargo check` before it ever shipped.
   `components::*` **is** a true glob, per the brief; only `engine`'s differs.
2. `preview/src/components/chart/component.rs`'s **`Chart`** wrapper now forwards via
   `chart::Chart { ..props }` (struct-update spread), not a hand-listed field-by-field call — see
   §4(a); this is the same construction §4(a) asks to also apply to `ChartTooltip`/`ChartLegend`/
   `ChartContainer`, but as of this commit it has only actually been applied to `Chart`.
3. `stacked`'s effective-kind check changed from a deny-list (`!matches!(kind, Line)`) to an
   allow-list (`matches!(kind, Area | Bar)`) — behaviorally identical today, but a deny-list would
   have silently "stacked" every new `ChartKind` variant this commit adds (`Pie`, `Radar`,
   `RadialBar`) unless every call site remembered to re-exclude it by hand.

Gates: all 11 + rustdoc green (`cargo test --workspace`: dioxus-primitives 269/0, +9 over the
pre-round 260 baseline; preview 31/0; dioxus-primitives doctests 143/0). `playwright/chart.spec.ts`
+ the tier-3 oracle were **not** run live — the brief's own gate note allows SSR tests + a clean
`cargo check -p preview` in place of a dev-server check for a pure rename with identical rendered
output, and both were clean.

### s2-area — DONE, gate-green, verified live

Worktree: `/home/user/dioxus-components/.claude/worktrees/agent-aef4de610bdb8e2cf`, branch
`worktree-agent-aef4de610bdb8e2cf`, base `9a6cb0c`. Commits, in order:

1. `0502c68` — `feat(area-chart): add the Area Chart gallery (default/linear/step/stacked/legend/axes/interactive)` — 7 of 10 variants, built against the pre-refactor `Chart` API.
2. `da962e2` — cherry-pick of `b1cc636` (s2-refactor's own commit) — **conflicts resolved**: `preview/src/components/chart/component.rs` and `primitives/src/chart/mod.rs`, kept the refactor's structure and re-added this lane's own `StackMode`/`stack_with_mode`/`Curve` re-exports on top. Diffstat: 27 files, **+1498/-632** — 10 more insertions and 5 more deletions than `b1cc636`'s own 1488/627, which is the exact size of the conflict-resolution work folded into this one commit (confirmed by direct `git show --stat` comparison, not just the lane's own say-so).
3. `5688459` — `feat(chart): add AreaOptions gradient/fill_opacity/connect_nulls/stack_mode` — the real `AreaOptions` implementation (was an empty post-refactor stub), plus an **out-of-ownership** addition to `preview/src/components/chart/component.rs` (see below).
4. `85375d3` — `feat(area-chart): add stacked_expand, gradient, and icons variants` — the 3 remaining variants (10/10 total), `docs.md`, the `examples!` registration line, the `playwright/area_chart.spec.ts` extension.

Registered as `area_chart[linear, step, stacked, stacked_expand, gradient, legend, axes, icons, interactive]` (9 named + `main` = 10/10, confirmed directly in the committed `preview/src/components/mod.rs`).

**Out-of-ownership edit, flagged by the lane itself and confirmed still unreviewed:**
`preview/src/components/chart/component.rs` gained re-exports for `Curve`, `ChartIcon`,
`ChartSeries`, `StackMode`, `AreaOptions` — additive names only, no logic change, but this file is
the one seam every other gallery lane also needs to extend for its own `*Options` type, so it is a
predictable future collision point (§6).

Not wired up (by design, stated in the code and `docs.md`): the `icons` variant sets
`ChartSeries.icon` correctly but renders identically to `legend` — reading that field is
`s2-tooltip`'s own task (§4a), not landed when this lane finished. Also filed, not applied by this
lane (it doesn't own the file): the `StackMode`-aware `components::layout::build` construction,
§4(c).

Gates: all 11 + rustdoc ×2 green; `cargo test --workspace` 279/0 (+10 over 269); one real clippy
finding fixed en route (`clippy::type_complexity`); two real rustdoc findings fixed en route (a
dead intra-doc link, an ambiguous `stack`-as-function-vs-module link). Playwright, against a real
`dx serve --port 8130` server: `area_chart.spec.ts` 25/25, `chart.spec.ts` + oracle 13/13 (confirms
the shared `component.rs` re-export additions didn't regress the generic `chart` package),
`preview.spec.ts` 3/3. Screenshots (light + dark) saved under `$S/stage2/s2-area/screenshots/`.

### s2-bar — part-done, real primitive progress, entirely uncommitted, never run live

Worktree: `/home/user/dioxus-components/.claude/worktrees/agent-a1519b1536ba3cc38`, branch
`worktree-agent-a1519b1536ba3cc38`, base `9a6cb0c`. Commits, in order:

1. `484270a` — `feat(bar-chart): add gallery scaffold + default/multiple/stacked/interactive variants`
2. `bc5872f` — cherry-pick of `b1cc636`, clean (diffstat byte-identical to the original: 27 files, +1488/-627 — no conflict-resolution content folded in)

**Correction to the ledger's own progress-report table:** it describes `484270a` as
"default/multiple/stacked/interactive (4/10)" and separately lists `stacked_legend` among the
*remaining* work. The worktree disagrees: `484270a` already added a fifth variant,
`variants/stacked_legend/mod.rs` (64 lines, `chart-bar-stacked.tsx`'s "Stacked + Legend"), confirmed
both by `git show --stat 484270a` and by the committed registration line itself —
`bar_chart[multiple, stacked, stacked_legend, interactive]` (4 named + `main` = **5**, not 4,
already committed). The commit's subject line simply undercounts its own diff. Treat the ledger's
"remaining: horizontal, negative, mixed, label, label_custom, active, stacked_legend" list as
directional, not exact — drop `stacked_legend` from it.

**Uncommitted, on disk right now** (none of this is in either commit above):
`primitives/src/chart/components/layout.rs` (axis-swap work for horizontal bars),
`primitives/src/chart/components/series/bar.rs` (+782 lines — by far the largest uncommitted diff
of any lane), `primitives/src/chart/engine/geometry.rs` (+77 lines). The progress report's own
"last action before the cut" quote — *"add the zero-line to render_grid"* — is consistent with
`layout.rs` being mid-edit. This is real, substantial, unverified work: no commit, no gate run, no
dev server.

**Never run live**: `$S/dx-serve-s2-bar.log` is 621 bytes and contains only `dx`'s startup banner
(the `Serving your app: preview!` banner and the keybinding hints) — there is no `Build completed`
line anywhere in it. The server was started and never finished its first build, or the log capture
ended before it did. Whoever resumes this lane should assume **zero** live verification of
anything currently on disk.

`playwright/bar_chart.spec.ts` (167 lines, 7 tests) is committed in `484270a`. One of its own test
titles is a live instance of the `mobile`-substring trap in §5 — see there for the exact line.

### s2-line — part-done, close to feature-complete, uncommitted, boot-only server evidence

Worktree: `/home/user/dioxus-components/.claude/worktrees/agent-ab986dd399489fca7`, branch
`worktree-agent-ab986dd399489fca7`, base `9a6cb0c`. Commits, in order:

1. `5dceba8` — `feat(line-chart): add the Line Chart gallery (default/linear/step/multiple/dots/interactive)` — 6 of 10 variants
2. `9344927` — cherry-pick of `b1cc636`, clean (identical diffstat)

**Uncommitted, on disk right now**: `primitives/src/chart/components/series/line.rs` (+623 lines —
`DotContext`, `DotRenderer`, `LineLabels`, the extension points for the four remaining variants),
`primitives/src/chart/components/series/mod.rs` (re-export widening, see below),
`preview/src/components/line_chart/component.rs` (+11) and all six existing variant files
(+1-2 each, mechanical — consistent with the ledger's own description of "a spec-fix pass").

**Applied directly, not left as a ledger request, and it is a repeat of the exact bug class
`s2-tooltip` found independently the same day (`$S/stage2-lanes.md`'s own account, confirmed
present in the uncommitted diff):** `components/series/mod.rs` — refactor-owned-forever — had
`pub use line::LineOptions;`, naming only the one type that existed when the refactor commit
landed. This lane's new `DotContext`/`DotRenderer`/`LineLabels` (all `pub` inside `line.rs`) had
zero external reachability regardless, since `components`/`series` are both non-`pub` modules and
this one re-export line is the only path out. The lane widened it in place to
`pub use line::{DotContext, DotRenderer, LineLabels, LineOptions};`, touching only its own family's
line. **This edit is still uncommitted** — the ledger narrates it in the past tense ("applied
directly"), which reads as done, but nothing has actually been committed to lock it in; a container
loss before it's committed loses it exactly like every other uncommitted diff in this document.

**Live-verification finding, sharper than the ledger's own self-report:** the lane's own DONE-style
note says "Did NOT run this lane's own `dx serve --port 8132` ... against a live server before this
commit," citing disk pressure. `$S/dx-serve-s2-line.log` (705 bytes) shows otherwise in part: after
the startup banner, its one substantive line is `714.11s INFO Build completed successfully in
713.08s, launching app!` — a ~12-minute cold wasm build that *did* finish and *did* reach
"launching app." The log then stops. So the server did come up at least once; there is simply no
evidence afterward of any interaction, screenshot, or spec run against it. Net effect is the same
as the lane's own conclusion — treat this as **not interactively verified** — but the precise
reason is "booted once, never exercised," not "never booted."

Not gate-run since the two committed commits (which predate every uncommitted change above).

### s2-polar — least far along; two chart kinds, one barely started

Worktree: `/home/user/dioxus-components/.claude/worktrees/agent-a73bafbe9f257480b`, branch
`worktree-agent-a73bafbe9f257480b`, base `9a6cb0c`. One commit:

1. `942794d` — `feat(chart): add the polar engine (angle scale, arc_path, pie_layout, centroid)`

**Integration method differs from every other lane, and is worth recording precisely:** this
worktree's history shows `b1cc636` directly — the *exact same commit object* `s2-refactor` produced
(`git log`/`git cat-file -t` both confirm the identical SHA, single parent `9a6cb0c`) — not a
re-committed cherry-pick with its own new SHA the way `s2-area`/`s2-bar`/`s2-line`/`s2-tooltip`/
`s2-radar` each did. This lane had zero commits of its own before incorporating the refactor, so a
plain fast-forward merge moved its branch pointer onto `b1cc636` without creating a new commit —
functionally equivalent to a cherry-pick here (no conflicts either way), but worth knowing if
anyone ever diffs branch histories expecting seven distinct refactor-commit SHAs.

**Uncommitted, on disk right now:** `primitives/src/chart/components/chart.rs` (+104 lines,
**out-of-ownership** — this file is refactor-owned-forever per §3) wires `ChartKind::Pie` to its
own reduced-table shape: a slice's share of the whole is exactly what its wedge angle already
encodes, so `Chart`'s hidden table gets a `Percent` column for `Pie` specifically, via the new
`engine::table::table_rows_pie` (also uncommitted, +104 lines in `engine/table.rs`) — `Radar` and
`RadialBar` are explicitly left on the generic single-series reduction, since "percent of the
total" isn't a meaningful reading of a radial bar's own value the way it is for a pie slice.
`primitives/src/chart/components/series/pie.rs` (+363 lines) has a real, non-stub `PieOptions`,
a `PieLabels` enum, and a real `render` — `Chart` now emits `data-slot="chart-arc"` slices for
`ChartKind::Pie` instead of the generic placeholder group.

**`primitives/src/chart/components/series/radial.rs` is untouched — still exactly `s2-refactor`'s
empty stub** (`RadialOptions {}` with no fields, `render` ignores `ctx`/`opts`), confirmed by
`git diff --stat` showing no changes to this file at all. A `RadialOptions` field sketch (67 lines:
`inner_radius`, `outer_radius`, `start_angle`, more) exists, but **only** in the session scratchpad
at `$S/stage2/s2-polar/radial_options_draft.rs` — it has never been applied to the worktree. No
`render()` draft for RadialBar exists anywhere, scratchpad included.

**The gallery packaging is a skeleton, not a gallery**, for both chart kinds it owns. On disk:
`preview/src/components/pie_chart/{component.json, docs.md}` and
`preview/src/components/radial_chart/{component.json, docs.md}`, both untracked — **and nothing
else**: no `component.rs`, no `style.css`, and each `variants/` directory exists but is completely
empty. Neither gallery is registered in `preview/src/components/mod.rs`'s `examples!` list or in
the root `component.json`'s members. What *does* exist, fully written and untracked, are the two
Playwright specs — `playwright/pie_chart.spec.ts` (224 lines) and `playwright/radial_chart.spec.ts`
(116 lines) — and they are valuable independent of any code: their own `VARIANTS` arrays spell out
the exact target variant lists, matching `chart-api.md`'s inventory precisely:

- Pie (11): `main, separator_none, label, label_list, label_custom, legend, donut, donut_active, donut_text, stacked, interactive`
- Radial (6): `main, label, grid, text, shape, stacked`

These specs also pin down data-slot/attribute names `series::pie::render` must (and a future
`series::radial::render` will need to) satisfy: `data-slot="chart-arc"` with `data-index`,
`data-start-angle`/`data-end-angle` (radians), `data-active`, `data-series` (multi-ring/stacked
cases), `data-slot="chart-arc-label"`, `data-slot="chart-pie-center-text"` (donut/radial center
text), `data-slot="chart-polar-grid"` (radial's background track). **None of this has been checked
against pie.rs's actual current diff, and none of it has ever run** — no dev server for this lane
has ever existed (no `$S/dx-serve-s2-polar.log` at all) — so whoever resumes should verify which of
these the current uncommitted `pie.rs` already satisfies rather than assume alignment.

Not gate-run since the one committed commit (predates all uncommitted work).

### s2-radar — primitive fully done and committed; gallery fully drafted but stuck in the scratchpad

Worktree: `/home/user/dioxus-components/.claude/worktrees/agent-a54aab42e9a00e001`, branch
`worktree-agent-a54aab42e9a00e001`, base `9a6cb0c`. Commits, in order:

1. `0f99984` — `feat(chart): add the Radar family's pure polygon/sector math` — `engine/radar.rs`, 508 lines, 7 public functions (`category_angles`, `point_radial`, `radial_scale`, `radar_polygon_path`, `radar_series_path`, `grid_polygon_path`, `sector_path`), verified in an isolated standalone crate before landing (21 unit tests + 7 doctests, per the lane's own report — not independently re-run here).
2. `ae7374a` — `feat(chart): render the Radar family's own grid, rim labels and hit-sectors` — `components/series/radar.rs`, 632 lines, a real (non-stub) `RadarOptions`, `RadarGrid` enum (`Polygon`/`Circle`/`CircleFill`/`CircleNoLines`/`Fill`/`None`/`Custom`), and a real `render`.
3. `15a8b41` — cherry-pick of `b1cc636`, clean (identical diffstat, single parent = this lane's own base `9a6cb0c`, so it is a genuine re-committed cherry-pick, not a fast-forward like `s2-polar`'s).

**The worktree is clean — no uncommitted changes at all.** Both the pure math and the family
render are entirely committed. This makes Radar's *primitive* work the most complete of any
part-done lane.

**But the gallery is not in the worktree — it exists only in the ephemeral session scratchpad,
which is a materially bigger preservation risk than every other lane's uncommitted-but-in-worktree
files.** At `$S/stage2/s2-radar/` sits a complete, ready-to-drop-in draft:
`radar_chart/component.rs` (thin wrapper, already written with the correct
`crate::components::chart::component::*` one-level-deeper import — see §5), `radar_chart/
component.json`, `radar_chart/docs.md`, and all **14** variant files under `radar_chart/variants/`
(`main, dots, lines_only, multiple, grid_circle, grid_circle_fill, grid_circle_no_lines,
grid_custom, grid_fill, grid_none, icons, label_custom, legend, radius` — matching shadcn's 14
exactly), plus `playwright/radar_chart.spec.ts` (a 202-line draft, 12 `test(` calls — not
necessarily one test per variant), a `variant-plan.md` that documents the exact data/config/grid
per variant with its shadcn source citation and the two exact registration lines to add
(`preview/src/components/mod.rs`: `radar_chart[dots, lines_only, multiple, grid_circle,
grid_circle_fill, grid_circle_no_lines, grid_custom, grid_fill, grid_none, icons, label_custom,
legend, radius],` inserted alphabetically between `progress,` and `radio_group[rtl],`; root
`component.json`: append `"preview/src/components/radar_chart"`), plus an isolated `compile-check`
Cargo crate used to verify the engine math before it was committed (its job is already done — low
ongoing value except as a reference).

A diff against the committed `engine/radar.rs` shows the scratchpad's own `radar.rs` math sketch
has since diverged (48 lines different) from what actually landed — the sketch was superseded by
further work folded into `ae7374a`, so that one specific scratchpad file carries no unique risk.
**The gallery draft is a different matter: nothing in it has a committed or worktree-resident
counterpart anywhere. If the scratchpad is lost before the worktree, this entire drafted gallery is
lost with it, even though the worktree itself would survive.** See §7's preservation recommendation
— copying `$S/stage2/s2-radar/radar_chart/` into the worktree and committing it should be the
single first action of any resumption, ahead of writing any new code for this lane.

**Update 2026-09-20 (main loop, after this document was written):** done — the scratchpad draft was
copied into this worktree and committed as `63de8f8`
(`chore(radar-chart): preserve the s2-radar gallery draft on this lane's branch`): all 17
`preview/src/components/radar_chart/**` files plus `playwright/radar_chart.spec.ts`, with
`rustfmt --edition 2021` applied so a later `cargo fmt --all --check` cannot trip over them, and
the two registration lines recorded in the commit body (the `variant-plan.md` itself stays
scratchpad-only). The draft is still UNVERIFIED — never compiled, never registered, never run — and
inert on the branch, since an unregistered component folder is not in the module tree. The seven
mechanical gate scripts pass with it present. This branch is still local-only: the draft now
survives anything that loses the scratchpad, but not the loss of the container itself (see §7).

Also carried by this lane, still open (§4): the CSS request for `chart/style.css`'s three radar
data-slots (§4e), the `has_full_table()` construction (§4b, filed by this lane against the
refactor-owned `chart.rs`), and the `ChartTooltip` position-formula limitation (§4d) — none of
these three are radar-gallery work, they are requests against files this lane doesn't own.

Not gate-run beyond what the two committed commits' own report claims (11/11 + rustdoc); not
independently re-verified by this handoff, and no dev server for this lane has ever existed.

### s2-tooltip — primitive and full 9-variant gallery drafted, all uncommitted; two wrapper fixes still open

Worktree: `/home/user/dioxus-components/.claude/worktrees/agent-ae1ca1ec628383540`, branch
`worktree-agent-ae1ca1ec628383540`, base `9a6cb0c`. Commits, in order:

1. `4a10b37` — `feat(chart-tooltip): add the tooltip gallery (4 of 9 variants)` — `main`, `indicator_none`, `label_none`, `label_formatter`, all buildable against the pre-refactor primitive (no new prop needed).
2. `cff011b` — cherry-pick of `b1cc636`, clean (identical diffstat).

**Uncommitted, on disk right now, and substantial:** `primitives/src/chart/components/tooltip.rs`
(+461 lines — `TooltipIndicator{Dot, Line, Dashed, None}`, `TooltipRow`, new `indicator`/
`label_key`/`name_key`/`formatter` props, icon rendering reading `ChartSeries.icon` via the
refactor's `ChartIcon`), `primitives/src/chart/components/legend.rs` (+133 lines — `hide_icon`,
the same icon rendering), `preview/src/components/chart_tooltip/component.rs` (+73 lines — adds a
local `ChartTooltipFull` pass-through that forwards every `ChartTooltipProps` field via `..props`,
the same construction as §4(a), so the gallery's own advanced demos can exercise the new fields
without waiting on the shared wrapper fix), `docs.md`, and the four pre-existing variant files
(+1 line each, mechanical). `preview/src/components/mod.rs`'s registration line is uncommitted too,
widened from the committed `chart_tooltip[indicator_none, label_none, label_formatter]` (3+main=4)
to `chart_tooltip[indicator_line, indicator_none, label_custom, label_formatter, formatter, icons,
advanced]` (8 named + `main` = **9/9**, the full shadcn target). Five more variant directories exist
untracked on disk: `advanced`, `formatter`, `icons`, `indicator_line`, `label_custom`. (The root
`component.json`'s `chart_tooltip` member is already committed, from `4a10b37`.)

Both primitive files (`tooltip.rs` clean `cargo check -p dioxus-primitives --features web`,
self-reported) and all 9 gallery variants are functionally written; nothing beyond compiling has
been re-verified by this handoff, and no dev server for this lane has ever existed
(no `$S/dx-serve-s2-tooltip.log`).

**This is the lane that originally found and filed §4(a)** — see there for the full construction —
and it is also the lane whose own worktree file most clearly demonstrates the bug still being open:
`chart_tooltip`'s own advanced/formatter/label_custom demos deliberately route through the local
`ChartTooltipFull` workaround rather than the shared themed `ChartTooltip`, specifically **because**
the shared wrapper still drops those fields. Per this lane's own report, it deliberately did *not*
touch the shared `preview/src/components/chart/component.rs` this round (reverted an in-flight edit
there before the pause), leaving §4(a)'s fix to `ChartTooltip`/`ChartLegend` for whoever integrates
— confirmed still open by direct inspection of that file (see §4a's "current state" code, read
directly from the `s2-refactor` worktree, the shared base every lane sits on).

Also open, filed by this lane (§4e): the CSS request for `chart/style.css`'s indicator/icon/
total-row rules, currently verified only against a scoped, throwaway copy in
`preview/src/components/chart_tooltip/style.css` under `.dx-chart-tooltip-gallery`.

## 3. The refactor's ownership map

This is why seven lanes could all edit `primitives/src/chart/**` at once without colliding, and any
resumption needs it to know which files are safe to touch. Verified directly against each
worktree, not just quoted from the ledger — the "Status" column reflects this handoff's own
`git diff`/`git log` checks, current as of 2026-09-20:

| File | Owner | Status, verified |
|---|---|---|
| `components/series/area.rs` (`AreaOptions`) | s2-area | **done, committed** (`5688459`) |
| `components/series/bar.rs` (`BarOptions`) | s2-bar (+ `components/layout.rs`) | **in progress, uncommitted** (+782 lines on disk) |
| `components/series/line.rs` (`LineOptions`) | s2-line (+ `engine/curve.rs`/`engine/data.rs`) | **in progress, uncommitted** (+623 lines: `DotContext`/`DotRenderer`/`LineLabels`) |
| `components/series/pie.rs` (`PieOptions`) | s2-polar (+ `engine/polar.rs`) | **in progress, uncommitted** (+363 lines: real `PieOptions`/`PieLabels`/`render`) |
| `components/series/radial.rs` (`RadialOptions`) | s2-polar (+ `engine/polar.rs`) | **not started** — still the refactor's own empty stub; a field sketch exists only in `$S/stage2/s2-polar/radial_options_draft.rs`, unapplied |
| `components/series/radar.rs` (`RadarOptions`, `RadarGrid`) | s2-radar (+ `engine/radar.rs`) | **done, committed** (`0f99984`, `ae7374a`) — 632 + 508 lines |
| `components/{tooltip.rs, legend.rs}` | s2-tooltip | **primitive done, uncommitted** (+461/+133 lines) |
| `components/layout.rs` (margins, scales, `SeriesRenderContext`, grid/axis rendering) | s2-bar | **in progress, uncommitted** — also the target of §4(c)'s `StackMode`-awareness request |
| `components/{chart.rs, container.rs, mod.rs, series/mod.rs}`, `engine/mod.rs`, `chart/mod.rs` | **s2-refactor only, forever** | done (`b1cc636`) — **but currently holds two pending out-of-ownership edits**, both uncommitted: `components/chart.rs` (s2-polar's `table_rows_pie` wiring, +104 lines) and `components/series/mod.rs` (s2-line's re-export widening for `DotContext`/`DotRenderer`/`LineLabels`) |

Every family's `render(ctx: &SeriesRenderContext, opts: &<Family>Options) -> Element` is
`pub(crate)`; `Chart` (`components/chart.rs`) dispatches to it with one 6-arm exhaustive match —
extending your own family's file needs no edit to the match arm itself.

`ChartKind::is_cartesian()` gates the grid/axes/cursor/hit-bands and picks the full
(`table_rows`) vs. reduced (`table_rows_single_series`) hidden-table shape for every kind except
`Pie` (which gets its own `table_rows_pie`, §2's s2-polar subsection) — **and Radar, pending §4(b)'s
`has_full_table()` fix**, which currently gets the reduced (wrong) table shape too, since Radar is
non-Cartesian but genuinely holds one value per series per category, unlike Pie/RadialBar.

The shared themed seam, `preview/src/components/chart/component.rs`, is not itself in the ownership
table above (it's the preview/themed layer, not the primitive), but functions as one in practice:
every gallery lane that needs to construct `Chart { <family>: <Family>Options { .. } }` by name
needs its `*Options` type reachable from there or from its own gallery's `component.rs`. Only
`s2-area`'s additions (`Curve`, `ChartIcon`, `ChartSeries`, `StackMode`, `AreaOptions`) are landed
there so far (folded into `da962e2`). `bar_chart`'s own `component.rs` already establishes a second,
equally valid pattern — named (not glob) imports straight from `crate::components::chart::{...}` —
and does not yet import `BarOptions`, which its own uncommitted primitive work will need once
committed. See §6 for the predictable collision this creates.

## 4. Cross-lane constructions still needed

Five constructions, matching the task's own count exactly, each with its root cause, the proposed
fix, and who filed it — plus the two verbatim CSS blocks required as (e).

### (a) `ChartTooltip`/`ChartLegend` (and `ChartContainer`) silently drop new props — struct-update spread

**Root cause, not a one-off** (filed by s2-tooltip, `$S/stage2-lanes.md`): every wrapper function
in `preview/src/components/chart/component.rs` takes the *primitive's own* Props type verbatim
(e.g. `pub fn ChartTooltip(props: ChartTooltipProps) -> Element`), then manually re-lists every
field by name into the inner `chart::ChartTooltip { ... }` call. A caller can set a brand-new field
at the call site — it type-checks, same struct — but unless the wrapper body is *also* edited to
forward it, the value is silently dropped before it reaches the real primitive: no compile error,
no runtime warning, the feature just does nothing. This is the same class of bug that already once
bit `ChartTooltip`'s own `children` field (the trailing-brace-sugar-always-populates-`Some` bug,
fixed in stage 1) — today's version is the identical forwarding gap for any newly-added named
field.

**Current state, read directly from `s2-refactor`'s own worktree** (the shared base every lane sits
on — `preview/src/components/chart/component.rs`, right after `b1cc636` landed): `Chart` **is
already fixed**:

```rust
pub fn Chart(props: ChartProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/chart/style.css") }
        chart::Chart { ..props }
    }
}
```

`ChartTooltip`, `ChartLegend` and `ChartContainer` are **not** — all three still hand-list every
field, confirmed by direct read (not by taking the ledger's word for it):

```rust
// ChartTooltip, as it stands today — every field named by hand, `children`
// handled specially (a named field, not `{props.children}` brace sugar):
chart::ChartTooltip {
    label_format: props.label_format,
    value_format: props.value_format,
    hide_label: props.hide_label,
    hide_indicator: props.hide_indicator,
    attributes: merged,
    children: props.children,
}

// ChartLegend, as it stands today — this is why s2-tooltip's new `hide_icon`
// field cannot reach the DOM through this wrapper at all right now:
chart::ChartLegend {
    vertical_align: props.vertical_align,
    attributes: merged,
}

// ChartContainer, as it stands today:
chart::ChartContainer {
    id: props.id,
    config: props.config,
    data: props.data,
    kind: props.kind,
    attributes: merged,
    { props.children }
}
```

**Proposed construction (s2-tooltip, evidenced and ready to apply — the same spread technique
already proven in this exact codebase at `primitives/src/toast.rs:84`, `Toast { ..props }`, and
already applied to `Chart` above):**

```rust
// ChartTooltip's body, replacing the manual field list:
chart::ChartTooltip { attributes: merged, ..props }
// ChartLegend's body, same shape:
chart::ChartLegend { attributes: merged, ..props }
```
(`attributes` stays listed explicitly since it must carry `merged`, not raw `props.attributes` —
struct-update syntax lets named fields before `..props` override it.) The same shape applies to
`ChartContainer`, though — stated plainly, matching s2-tooltip's own hedge — this has not been
verified to compile cleanly under spread post-refactor; flagging it as the same class, not
asserting the fix is trivial there.

**Subsumes:** `ChartTooltip`, `ChartLegend` (blocks `s2-tooltip`'s `indicator_line`/`label_custom`/
`formatter`/`advanced` demos from being verifiable through the real themed wrapper — that lane
currently works around it with a local `ChartTooltipFull` pass-through, §2), and by the same shape,
`ChartContainer`. **Does not subsume:** `Chart`, already fixed by `s2-refactor`'s own `b1cc636`
(deviation 2, §2).

### (b) The hidden data table's full-vs-reduced choice is wrongly coupled to `is_cartesian()` — `has_full_table()`

**Root cause** (filed by s2-radar): `components/chart.rs`'s
`let rows = if is_cartesian { table_rows(&data) } else { table_rows_single_series(&data) };` ties
the table shape to `ChartKind::is_cartesian()`. That's correct for Pie/RadialBar (a pie slice
genuinely has one value per category) but wrong as a *permanent* rule for Radar: a radar chart
overlays N series per category exactly like Area/Bar/Line (this lane's own gallery plan has
multi-series demos — `multiple`, `legend`, `icons`, `label_custom`, `radius`), and the task brief
requires "hidden table = categories × series" for it. `is_cartesian()` conflates two independent
questions — "does this kind draw a Cartesian grid/axis" and "does this kind hold one value or N per
category" — that happen to coincide for Area/Bar/Line but diverge for Radar vs. Pie/RadialBar.

**Proposed construction:** a second `ChartKind` method,
`fn has_full_table(self) -> bool { !matches!(self, ChartKind::Pie | ChartKind::RadialBar) }` (or
equivalently, an inline `!matches!(kind, Pie | RadialBar)` check in place of `is_cartesian()` for
this one decision). `chart.rs`'s table-shape branch switches to it; `is_cartesian()` stays exactly
as-is for the grid/axis/cursor/hit-bands gate, since Radar draws its own polar grid/hit-sectors, not
Cartesian ones. Additive/non-breaking for every kind that exists today.

**Cannot be applied by s2-radar itself** — `chart.rs` is refactor-owned-forever (§3). Until this
lands, `series::radar::render` is correct, but the hidden table under `ChartKind::Radar` shows only
the first configured series.

### (c) `components::layout::build` needs to be `StackMode`-aware (filed by s2-area, for the layout owner s2-bar)

**Root cause:** `primitives/src/chart/engine/stack.rs` (s2-area-owned) added
`StackMode::{Normal, Expand}` + `stack_with_mode` (additive; d3 `offset/expand.js` semantics —
normalize each row to fractions of its own total, then stack, so every row's top lands at exactly
1.0). `AreaOptions.stack_mode` uses it, but `components::layout::build` (s2-bar-owned) computes
`SeriesRenderContext::y_scale`/`zero_y`/`stacked_spans` once, chart-wide, always from the plain
`stack()` (Normal) — it has no per-family stack-mode input. `area.rs`'s own `render` works around
this by recomputing its own local percent spans + a local `(0.0, 1.0)`-domain `LinearScale` for the
Expand case (correct for the area/line marks themselves), but `Chart`'s shared grid lines, y-axis
tick values, and the tooltip's vertical anchor still reflect the raw, non-percent domain and
visually disagree with a percent-stacked chart. `s2-area`'s own `stacked_expand` demo works around
it today by turning the grid off entirely rather than ship a misleading reference line.

**Proposed construction (subsumes both Area's and Bar's eventual `stack_mode`, not a per-family
special case):** `LayoutParams` gains a `stack_mode: StackMode` field, default `Normal` (every
existing call site keeps today's exact behavior); `build`/`y_extent` call
`stack_with_mode(&rows, p.stack_mode)` instead of the bare `stack(&rows)` they call today, in both
the `stacked_spans` computation and the y-extent scan. `Chart`'s own dispatch (`chart.rs`, refactor-
owned) needs one more line passing `props.area.stack_mode` (or `props.bar.stack_mode`, once
`BarOptions` gets the same field) through to `LayoutParams` based on `kind`. Once this lands,
`area.rs`'s own Expand branch can delete its local recompute and read `ctx.y_scale`/
`ctx.stacked_spans` like the Normal branch already does. Also requested for `BarOptions`, per the
common brief's own note, for a future percent-stacked-bar demo.

**Filed against `s2-bar`'s own owned file** (`components/layout.rs`) — not blocking, since `s2-bar`
currently has substantial uncommitted work in exactly that file (§2); whoever picks this construction
up should coordinate with whatever `layout.rs` state is current at resume time, not this snapshot.

### (d) `ChartTooltip`'s position formula is Cartesian-only — radar's tooltip falls back to dead-centre

**Root cause, stated plainly (filed by s2-radar):**
`primitives/src/chart/components/tooltip.rs`'s percent-position formula
(`layout.x_scale.center(i)`, `layout.y_scale.scale(layout.top_value[i])`) requires
`ChartLayout.x_scale: BandScale` — an affine function of the datum index `i` (evenly-spaced points
along a line). A radar vertex's pixel position is
`center + point_radial(category_angles(n)[i], radial_scale(...).scale(value))`, a **sinusoidal**
function of `i` for n > 2 categories — no `BandScale` can reproduce it (`BandScale::center(i)` is
linear in `i` by construction; a radar's x/y are not). This is not a bug to patch inside radar's own
file: `ChartLayout`/`ChartTooltip` (`context.rs`/`tooltip.rs`, owned by neither the refactor-only
list nor `s2-radar`) would need a family-agnostic anchor representation to fix it by construction —
e.g. replacing (or adding alongside, for non-breaking migration) the three Cartesian-specific
fields with one `anchor_percent: Vec<(f64, f64)>` (per-datum `(left%, top%)`, computed by whichever
family rendered), so `ChartTooltip` indexes it uniformly with no per-`ChartKind` branch of its own.
Whoever owns this also needs `Chart`'s own (Cartesian) computation updated to populate it the same
way — trivial, since it already computes `x_scale.center(i)`/`y_scale.scale(top_value[i])` per
datum; it just needs to divide by width/height and push into the new field instead of reading a
stored `BandScale`/`LinearScale` pair live.

**Current, shipped-as-is behavior until this lands:** `s2-radar`'s `render()` does not write to
`ctx.layout` at all (leaves it at its default, `None`) — every radar `ChartTooltip` demo opens and
closes correctly and shows the correct label/values (content has no dependency on layout), but
positions at `ChartTooltip`'s own existing `(50.0, 50.0)` fallback — dead-centre of the chart —
rather than at the active category's vertex. Not caught by any current test (no test asserts
radar's tooltip pixel position; the drafted `radar_chart.spec.ts` only asserts open/closed state and
content) — flagged here so nobody ships this unremarked.

### (e) Two CSS request blocks for `preview/src/components/chart/style.css` — copied verbatim

Both are append-only additions to the shared themed stylesheet, both currently verified only
against scoped, throwaway copies in each lane's own gallery package (per each lane's own brief),
and both still need to land in the real file, unscoped, before their gallery's demos can be deleted
of their throwaway copy.

**s2-tooltip's block** (indicator shapes, per-series icons, a formatter-built total row):

```css
/* Indicator shapes (ChartTooltipProps::indicator, TooltipIndicator). Dot
 * (the default) already matches the existing base swatch rule below
 * unchanged; None/hide_indicator render no swatch element at all, so
 * neither needs a rule here — only Line and Dashed do. */
.dx-chart-tooltip[data-indicator="line"] [data-slot="chart-tooltip-item"],
.dx-chart-tooltip[data-indicator="dashed"] [data-slot="chart-tooltip-item"] {
  align-items: stretch;
}

.dx-chart-tooltip [data-slot="chart-swatch"][data-indicator="line"] {
  width: 4px;
  height: auto;
  align-self: stretch;
  border-radius: var(--dx-radius-xs);
  background-color: var(--series-color);
}

.dx-chart-tooltip [data-slot="chart-swatch"][data-indicator="dashed"] {
  width: 0;
  height: auto;
  align-self: stretch;
  background-color: transparent;
  border-radius: 0;
  border-inline-start: 1.5px dashed var(--series-color); /* rtl-physical: a vertical rule at the row's own start edge, not a reading-direction concept */
}

/* Per-series icons (ChartSeries.icon), in place of the swatch, in both the
 * tooltip and the legend. */
.dx-chart-tooltip [data-slot="chart-icon"],
.dx-chart-legend [data-slot="chart-icon"] {
  display: inline-flex;
  flex-shrink: 0;
  align-items: center;
  justify-content: center;
  width: 10px;
  height: 10px;
  color: var(--secondary-color-5);
}

.dx-chart-tooltip [data-slot="chart-icon"] svg,
.dx-chart-legend [data-slot="chart-icon"] svg {
  width: 100%;
  height: 100%;
}

/* Legend icons read slightly larger than tooltip ones -- shadcn's own
 * `[&>svg]:h-3 [&>svg]:w-3` (legend) vs `[&>svg]:h-2.5 [&>svg]:w-2.5`
 * (tooltip), chart.tsx. */
.dx-chart-legend [data-slot="chart-icon"] {
  width: 12px;
  height: 12px;
}

/* A ChartTooltipProps::formatter-built total row (chart_tooltip gallery's
 * `advanced` demo). */
.dx-chart-tooltip [data-slot="chart-tooltip-total"] {
  display: flex;
  flex-basis: 100%;
  align-items: center;
  margin-block-start: var(--dx-space-1);
  padding-block-start: var(--dx-space-1);
  border-block-start: 1px solid var(--dx-chart-grid);
  font-weight: 600;
}

.dx-chart-tooltip [data-slot="chart-tooltip-total-value"] {
  margin-inline-start: auto;
  font-variant-numeric: tabular-nums;
}
```

**s2-radar's block** (three new data-slots — grid rings, fill tint, spokes; everything else radar
draws reuses existing slots — dots, axis labels, hit-bands — unchanged):

```css
/* Radar family (ChartKind::Radar). One closed polygon per series (both
 * filled and stroked by the same element -- unlike Area, there is no
 * separate fill/stroke path pair). fill-opacity is a per-chart configurable
 * value (RadarOptions::fill_opacity), so it is set as a direct `fill-opacity`
 * SVG presentation attribute by the primitive, not here. */
.dx-chart [data-slot="chart-radar-area"] {
  fill: var(--series-color);
  stroke: var(--series-color);
  stroke-width: 2;
}

/* Grid rings (RadarGrid::{Polygon,Circle,CircleFill,Fill,CircleNoLines,Custom}),
 * both the `<circle>` and closed polygon `<path>` shapes. */
.dx-chart [data-slot="chart-grid-ring"] {
  fill: none;
  stroke: var(--dx-chart-grid);
  stroke-width: 1;
}

/* RadarGrid::{CircleFill,Fill}: the primitive sets
 * `--grid-fill-color: var(--color-<first-series-key>)` inline on the
 * `g[data-slot="chart-grid"]` wrapper (same inherited-custom-property
 * technique as `--series-color`), and `data-fill="true"` on each ring that
 * should show it. */
.dx-chart [data-slot="chart-grid-ring"][data-fill="true"] {
  fill: var(--grid-fill-color);
  fill-opacity: 0.2;
}

.dx-chart [data-slot="chart-grid-spoke"] {
  stroke: var(--dx-chart-grid);
  stroke-width: 1;
}
```

## 5. Traps found the hard way

**`error[E0659]: "component" is ambiguous` — hits every thin gallery wrapper that composes
`chart`.** Root cause, by construction, not a one-off typo (found by `s2-area`, hit independently
and confirmed by `s2-line`): `preview/src/components/mod.rs`'s `examples!` macro expands every
registered component (yours included) to
`mod $name { pub(crate) mod component; pub use component::*; mod variants { .. } }`. Every
component module therefore owns a submodule literally named `component` (`pub(crate)`,
crate-visible). A blanket glob `pub use crate::components::chart::*;` re-exports `chart`'s own
`component` submodule *name* too, not just its contents, which collides with your own package's
identically-named, macro-declared `pub(crate) mod component;` in the same scope. Confirmed to bite
`area_chart`, `bar_chart` (implicitly, by the ownership map), `line_chart` (hit and fixed, per its
own DONE note), `radial_chart`/`pie_chart`, `chart_tooltip`, `radar_chart` (pre-emptively fixed in
its own scratchpad draft's doc comment) — every gallery doing the "thin wrapper over `chart`"
pattern the common brief describes. **Fix, verbatim, subsumes every instance of this class:** import
one level deeper, from the sibling's own `component` submodule specifically:
`pub use crate::components::chart::component::*;` (correct) instead of
`pub use crate::components::chart::*;` (ambiguous, do not use). **A second, equally valid pattern
already exists in this tree**, worth knowing about: `bar_chart/component.rs` sidesteps the whole
class by using *named*, not glob, imports —
`pub use crate::components::chart::{Chart, ChartConfig, ChartContainer, ChartDatum, ChartKind, ChartLegend, ChartTooltip};`
— named imports never trigger E0659 regardless of depth, at the cost of needing to add each new
type by hand (e.g. `BarOptions`, once `bar.rs`'s uncommitted primitive work is finished and
committed).

**`playwright.config.ts`'s `grepInvert: /mobile/` silently drops any test whose title contains the
lowercase substring "mobile" — not just mobile-viewport tests.** The regex
(`playwright/playwright.config.ts` lines 43/49/55, confirmed still present) is applied to every
chromium/webkit/firefox project to skip viewport tests, but it matches any test title containing
that substring, case-sensitively, with **no error, warning, or "skipped" marker** — `--list` just
prints one fewer test than the file's own `test(` count. Every stage-2 chart lane's sample data
names a series "mobile" (`chart-api.md`'s own convention), so any spec whose test titles echo that
series name in prose (not just as a `data-series` selector string, which is unaffected) is at risk.
**This is not hypothetical — there is a live victim in the tree right now:**
`playwright/bar_chart.spec.ts:160`,
`test("interactive variant toggled to mobile has no automatically detectable a11y issues", ...)` —
lowercase "mobile," will be silently excluded from every run of this file until either the title or
the config changes. `playwright/line_chart.spec.ts:133`,
`test("interactive: Mobile selected has no automatically detectable a11y issues", ...)` is a
near-miss: capitalized "Mobile" does **not** match the case-sensitive regex, so it currently runs —
but only by capitalization accident, not by design. **Mitigation until the regex itself is fixed
upstream** (e.g. scoped to a project/tag rather than a bare title substring): before trusting a
green run of any chart spec, compare `npx playwright test --list <file> | wc -l` against
`grep -c 'test(' <file>` — a mismatch means a test is being silently dropped, exactly as `s2-area`
found this by that method, not by a failure.

**Disk-pressure protocol**: six lanes running `dx serve`/cargo builds concurrently, each against
its own `target-lane-s2-*` (a full, uncached cold wasm32 build — hundreds of crates, ~1GB+ each),
drove `df -h /home/user` from 8.1G to 4.7G available in about five minutes on 2026-09-19, and
further down to 2.3-3.6G during the worst of the crunch. Four lanes independently made the same
call and it worked: `s2-area` killed its own in-flight build and `rm -rf`'d `target-lane-s2-area`
(1.4G reclaimed) before its first cold build finished; `s2-tooltip` freed `target-lane-s2-tooltip`
(2.6G) while idle, waiting on `s2-refactor`; `s2-line` freed `target-lane-s2-line` (3.2G,
2.4G → 5.4G available immediately after) rather than start a first cold build into an already-tight
shared resource; `s2-radar` deferred starting its own build entirely while the alert was active.
**Practical rule for a resumed lane:** `rm -rf` your own `$CARGO_TARGET_DIR` the moment you're done
verifying, per the environment brief's own instruction, rather than leaving a server up while
writing a report — every GB reclaimed early helps every other concurrent lane's build actually
finish, and a container-wide ENOSPC failure is not a bug in whichever lane happens to trip it.

**A third, previously undocumented registration collision point, found by direct inspection while
writing this handoff** (beyond the two files the task brief names — see §6): `preview/src/
components/mod.rs`'s `category_of`-style match also has a `| "resizable" => ComponentCategory::
DataDisplay,` arm that `area_chart`, `bar_chart`, and `chart_tooltip` have each independently edited
to prepend their own name (`| "area_chart" | "resizable" => ...`, `| "bar_chart" | "resizable" =>
...`, `| "chart_tooltip" | "resizable" => ...`), while `line_chart` left it untouched entirely.
Because all three edits start from the same original one-name line, cherry-picking them in sequence
onto one branch will conflict on this line exactly as it would on the two files below. **The
cleanest resolution, not requiring a three-way textual reconciliation**: this file's own trailing
`_ => ComponentCategory::DataDisplay,` catch-all already resolves every unmatched name — including
every chart-family gallery name — to the identical `DataDisplay` category, so the three edits are
functionally redundant with the file's own default. Drop back to the original, unmodified single-
name arm (`| "resizable" => ComponentCategory::DataDisplay,`) and let the catch-all cover every
chart gallery, rather than reconciling three independent inserts to the same line.

## 6. Integration checklist

**Cherry-pick order.** Land `s2-refactor`'s `b1cc636` first — the integration branch
(`claude/roadmap-evaluation-6eiahc`, currently `03927e3`) has **never received it**; every stage-2
worktree branches from `9a6cb0c`, an ancestor of the `1642100` merge, not from current HEAD. After
that, order by how finished/low-risk each lane is, since ownership is disjoint by file (§3) and
order therefore mostly only matters for the handful of shared/out-of-ownership touch points:
`s2-area` (already finished and clean — nothing left to do but pick its four commits), `s2-tooltip`
(primitive done, gallery drafted, needs its own uncommitted work committed first), `s2-line` (close
to done, same caveat), `s2-bar` (primitive work still genuinely in progress), `s2-radar` (needs its
scratchpad-only gallery draft copied into the worktree and committed *before* anything else — see
§7 — otherwise there is nothing of its gallery to cherry-pick), `s2-polar` last (least far along;
its one committed commit is low-risk, but its uncommitted `chart.rs` edit is the one most likely to
need reconciling against whatever the other lanes left in that refactor-owned file by the time it
lands).

**`preview/src/components/chart/style.css` will conflict with `03927e3`, not with stage-2's own
base.** `03927e3` (current HEAD, landed *after* every stage-2 worktree was created) rewrote this
file's tooltip/legend layout from flexbox to CSS grid (137 lines changed) specifically so
`ChartTooltip` no longer overlaps a top-aligned `ChartLegend` — none of the seven stage-2 lanes has
ever seen this. None of the seven lanes' own commits modify this shared file directly yet (§4(e)'s
two CSS blocks are still pending appends, not yet committed anywhere), so there is no historical
conflict today — but whoever applies §4(e) needs to append onto the *current*, grid-based file
content, not onto a stage-2 worktree's stale flexbox copy of it. Both blocks are purely additive
(new selectors on new data-slots: `chart-swatch[data-indicator]`, `chart-icon`,
`chart-tooltip-total*`, `chart-grid-ring`/`-spoke`, `chart-radar-area`) and don't touch the grid
layout rule at all, so a clean append is expected, not a real merge conflict — just don't build it
from the wrong base file.

**The two files every gallery lane appends one line to, and how those conflicts resolve (keep both
sides):**
- `preview/src/components/mod.rs`'s `examples!` list — one line per gallery, alphabetical position (`area_chart[...]`, `bar_chart[...]`, `line_chart[...]`, `pie_chart[...]`/`radial_chart[...]`, `radar_chart[...]`, `chart_tooltip[...]`). Seven lanes touching this file is expected; each one only ever adds its own single line, so cherry-picks land as trivial adjacent-line inserts.
- The root `component.json`'s `members` array — one string per gallery, same append-only shape.
- (Found by this handoff, not in the original brief — see §5's third bullet): the `category_of` `| "resizable" => ...` arm. Recommended resolution: revert every lane's edit to it and rely on the file's own `_ => ComponentCategory::DataDisplay` catch-all instead of reconciling three independent inserts.

**Additive re-exports flagged as out of ownership, for review:**
- `s2-area`'s `5688459`/`da962e2` (already committed) added `Curve`, `ChartIcon`, `ChartSeries`, `StackMode`, `AreaOptions` to the shared `preview/src/components/chart/component.rs`'s re-export list — additive names only, no logic change, but not this lane's own file.
- `s2-line`'s uncommitted widening of `components/series/mod.rs` (refactor-owned-forever) from `pub use line::LineOptions;` to `pub use line::{DotContext, DotRenderer, LineLabels, LineOptions};` — additive, touches only that one family's own line.
- `s2-polar`'s uncommitted `components/chart.rs` edit (refactor-owned-forever) wiring `ChartKind::Pie`'s own `table_rows_pie`/Percent-column table shape.
- Every remaining gallery lane (`bar_chart`, `line_chart`, `pie_chart`/`radial_chart`, `radar_chart`, `chart_tooltip`) will most likely need to add its own `*Options` (or nested type) re-export to the shared `chart/component.rs` the same way `s2-area` did, once each lane's own primitive work is committed — or follow `bar_chart/component.rs`'s own named-import pattern instead (§5) and avoid touching the shared file at all.

**Oracle rules still to be added, `playwright/oracle/tier3-radix/chart.spec.ts`:** none of the seven
lanes has added to this file — the brief deliberately defers polar/radar-specific oracle rules to
integration, once those two families are far enough along to know what a kind-agnostic rule even
looks like for a polar chart (the existing R1-R8 rules are Cartesian-shaped). `s2-area` explicitly
declined to add anything here itself, on the grounds that Area's own new behaviors (gradient defs,
percent stacking) are family-specific, not a Recharts-parity rule the way the existing rules are —
the same reasoning likely extends to Bar/Line's own remaining features. Pie/Radar are the two
families most likely to need a genuinely new rule (e.g., "every arc's start angle equals the
previous arc's end angle," already asserted as a plain Playwright test in the drafted
`pie_chart.spec.ts`, §2 — promoting it to the oracle file is a judgment call for whoever integrates,
not decided here).

**Full gate set** (`CLAUDE.md`, binding, CI frozen so nothing else catches a red gate):
`scripts/check-preview-composition.sh`, `scripts/check-cfg-axis.sh`,
`scripts/check-dx-class-prefix.sh`, `scripts/check-css-literals.sh`,
`scripts/check-hooks-in-closures.sh`, `scripts/check-self-subscribing-effects.sh`,
`scripts/check-css-logical-properties.sh`, `cargo fmt --all -- --check`,
`cargo clippy --workspace --tests --examples -- -D warnings`, `cargo test --workspace`,
`cd preview && npx stylelint "src/**/*.css"`, plus rustdoc
(`RUSTDOCFLAGS="-Dwarnings --document-private-items" cargo doc -p <crate> --no-deps --document-private-items`,
`dioxus-primitives` also with `--features web`) for every crate touched. Every lane in §2 that
reported gates green ran them in an isolated `CARGO_TARGET_DIR`; a resumed lane should assume it
needs to re-run the full set itself rather than trust a snapshot taken before its own uncommitted
changes existed.

## 7. Preservation — DONE 2026-09-20 (this section was written before it was)

> **Current state, verified against `git ls-remote origin` on 2026-09-20.** Every branch below is
> on the remote. The rest of this section is preserved as written, in its original
> "nothing has been done" voice, because it is the record of what was at risk and why — read it as
> history, not as current status.
>
> | Lane | Branch `worktree-agent-<id>` | Tip | What the tip is |
> |---|---|---|---|
> | s2-refactor | `af6c70a282d507c23` | `b1cc636` | real work — series rendering split per family, stage-2 extension points reserved |
> | s2-area | `aef4de610bdb8e2cf` | `85375d3` | real work — `stacked_expand`, gradient and icons variants |
> | s2-bar | `a1519b1536ba3cc38` | `9deda40` | WIP checkpoint of this lane's uncommitted edits |
> | s2-line | `ab986dd399489fca7` | `074ca38` | WIP checkpoint of this lane's uncommitted edits |
> | s2-polar | `a73bafbe9f257480b` | `4847eb9` | WIP checkpoint of this lane's uncommitted edits |
> | s2-radar | `a54aab42e9a00e001` | `63de8f8` | the scratchpad-only 14-variant gallery, copied in and committed (registration deliberately NOT applied — the draft stays inert and unverified) |
> | s2-tooltip | `ae1ca1ec628383540` | `6e4911c` | WIP checkpoint of this lane's uncommitted edits |
>
> A WIP checkpoint is a throwaway commit made solely to make a push capture the working tree; it is
> not reviewed, not gated, and not intended to be merged as-is. Nothing here has been integrated —
> §2's per-lane status and §6's integration checklist still govern what it would take to land any
> of it.

Every worktree in §2 is a local git worktree under this container's own
`/home/user/dioxus-components/.claude/worktrees/`, on a local-only branch
(`worktree-agent-<id>`) that has never been pushed anywhere. Every uncommitted diff listed in §2 is
sitting only in that worktree's working tree, on this container's disk. **`s2-radar`'s drafted
14-variant gallery is the single highest-risk item in this whole document**: it exists only in the
session scratchpad (`$S/stage2/s2-radar/radar_chart/` and `radar_chart.spec.ts`), which this task's
own framing treats as more ephemeral than the worktrees themselves — it is not even uncommitted
worktree content, it is content that has never been copied into any git-tracked location at all. If
this container is reclaimed, everything not written into the repo is lost — the worktrees, their
uncommitted edits, and (first and worst) the scratchpad draft.

**None of the following has been run.** They are the exact commands that would preserve this work,
in order of preference:

```sh
# Cheapest, per branch: push each stage-2 worktree branch to the fork remote
# (creates a durable ref even though these branches are not yet integrated
# onto claude/roadmap-evaluation-6eiahc):
git -C /home/user/dioxus-components/.claude/worktrees/agent-af6c70a282d507c23 push origin worktree-agent-af6c70a282d507c23
git -C /home/user/dioxus-components/.claude/worktrees/agent-aef4de610bdb8e2cf push origin worktree-agent-aef4de610bdb8e2cf
git -C /home/user/dioxus-components/.claude/worktrees/agent-a1519b1536ba3cc38 push origin worktree-agent-a1519b1536ba3cc38
git -C /home/user/dioxus-components/.claude/worktrees/agent-ab986dd399489fca7 push origin worktree-agent-ab986dd399489fca7
git -C /home/user/dioxus-components/.claude/worktrees/agent-a73bafbe9f257480b push origin worktree-agent-a73bafbe9f257480b
git -C /home/user/dioxus-components/.claude/worktrees/agent-a54aab42e9a00e001 push origin worktree-agent-a54aab42e9a00e001
git -C /home/user/dioxus-components/.claude/worktrees/agent-ae1ca1ec628383540 push origin worktree-agent-ae1ca1ec628383540

# Note: every push above preserves only COMMITTED work. Every lane except
# s2-refactor and s2-radar also has real uncommitted working-tree edits
# (s2-area's worktree is clean, but see the other five in §2) that a push
# alone will NOT capture -- commit them first (even as a throwaway WIP
# commit) or use `git bundle` below, which can include the working tree by
# committing to a throwaway branch first.

# Alternative/supplement, no remote required, one file per worktree, safe
# even for uncommitted-but-staged content once staged:
git -C /home/user/dioxus-components/.claude/worktrees/agent-a1519b1536ba3cc38 bundle create /home/user/dioxus-components/.claude/worktrees/s2-bar.bundle --all
# (repeat per worktree; a bundle is a single file that can recreate the
# branch and its full history elsewhere with `git clone <bundle>` or
# `git fetch <bundle>`.)

# Highest priority, cheapest, and the one item that is not "just" a worktree
# push: copy the scratchpad-only radar gallery into its worktree and commit
# it, so it stops depending on the scratchpad's own survival at all:
cp -r "$S/stage2/s2-radar/radar_chart" /home/user/dioxus-components/.claude/worktrees/agent-a54aab42e9a00e001/preview/src/components/radar_chart
cp "$S/stage2/s2-radar/radar_chart.spec.ts" /home/user/dioxus-components/.claude/worktrees/agent-a54aab42e9a00e001/playwright/radar_chart.spec.ts
# then register (two lines, per $S/stage2/s2-radar/variant-plan.md, §2 above)
# and commit.
```

**Update 2026-09-20 (main loop), superseding the two paragraphs this replaces:** the repository
owner gave the go-ahead ("2. push"), and all of the above has now been executed. The scratchpad-only
radar gallery was copied into `worktree-agent-a54aab42e9a00e001` and committed as `63de8f8` first
(registration deliberately NOT applied, so the draft stays inert and unverified); the four lanes
that still had uncommitted working-tree edits — s2-bar, s2-line, s2-polar, s2-tooltip — each got a
WIP checkpoint commit so the push would capture them; then every one of the seven branches was
pushed to `origin`. No bundles were created: with the branches on the remote they are redundant.
The table under this section's heading lists each branch and its tip SHA, verified against
`git ls-remote origin`.

What this does and does not buy: every lane's work now survives this container being reclaimed, and
can be fetched from a fresh clone. It is still unintegrated, ungated and unreviewed — see §2 and
§6.
