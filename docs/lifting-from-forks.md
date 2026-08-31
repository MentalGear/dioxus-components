# How to lift useful work out of the fork network

A practical playbook for moving code from a fork of `DioxusLabs/dioxus-components` into this repo without importing someone's architecture, their build hacks, or a fix that evaporates on the next build.

Companions: [`adopt-fork-fixes-results.md`](./adopt-fork-fixes-results.md) (which bug fixes exist) and [`capability-gaps.md`](./capability-gaps.md) (which capabilities are missing). This document is the *how*.

---

## 1. Legal position

Verified on the scan date: upstream and all three source forks (`dignifiedquire`, `sarendipitee`, `jcgruenhage`) ship both `LICENSE-APACHE` and `LICENSE-MIT`, and `dignifiedquire`'s `primitives/Cargo.toml` declares `license = "MIT OR Apache-2.0"` — identical terms to this repo.

So lifting is permitted. What both licences require is that attribution survives. In practice:

- **Cherry-picked commits** carry it automatically — `git cherry-pick` preserves the original author, and `git commit -C <sha>` does the same for a partial apply. Don't rewrite authorship.
- **Lifted files** need a provenance header, since a copied file otherwise loses all trace of who wrote it:

  ```rust
  //! Ported from dignifiedquire/dx-components (MIT OR Apache-2.0),
  //! primitives/src/scroll_lock.rs @ <sha>. Adapted: <what you changed>.
  ```
- **Concept ports** (you read their approach and wrote your own) need no header, but crediting the source in the commit message is honest and cheap.

Re-verify licences before lifting — a fork can relicense at any time.

## 2. Getting the code where you can see it

The fork network is not reachable through the GitHub API in most sandboxes, but anonymous git reads work. Fetch fork branches into a private ref namespace of an upstream clone, so nothing pollutes your working branches:

```bash
git clone https://github.com/DioxusLabs/dioxus-components upstream-mirror
cd upstream-mirror

# One fork, one branch, into refs/mined/<owner>/<branch>
git fetch --no-tags https://github.com/<owner>/<repo> \
  "+refs/heads/<branch>:refs/mined/<owner>/<branch>"

# Read anything without checking it out
git show refs/mined/<owner>/<branch>:primitives/src/scroll_lock.rs
git grep -n "<pattern>" refs/mined/<owner>/<branch> -- 'primitives/*'
git ls-tree -r --name-only refs/mined/<owner>/<branch> -- primitives/src
```

To enumerate the whole network, read `https://github.com/DioxusLabs/dioxus-components/network/members` — it lists every fork on one page. Skip `gh-pages` branches; they are build output.

## 3. Choose the lift shape

Three shapes, in increasing cost. Pick the cheapest one that actually works.

### Shape A — cherry-pick a commit

For discrete fixes whose commit touches only files you want.

```bash
git remote add fork-<owner> https://github.com/<owner>/<repo>
git fetch --no-tags fork-<owner> <branch>
git cherry-pick <sha>
```

When the commit also touches files you *don't* want (a fork-only module, their own test edits), take the paths you need — note that `git cherry-pick` accepts **no pathspec**:

```bash
git show <sha> -- primitives/src/<file>.rs | git apply --index
git commit -C <sha>          # keeps the original author
```

### Shape B — lift a module

For self-contained files: `scroll_lock.rs`, `aria_hidden.rs`, `typeahead.rs`, `direction.rs`.

```bash
git show refs/mined/<owner>/<branch>:primitives/src/<mod>.rs > primitives/src/<mod>.rs
```

Then: add the provenance header, add `mod <mod>;` to `lib.rs`, map any helper names (§5), wire the call sites, and write the oracle test *first* (§6).

### Shape C — port the concept

When the source is entangled with fork-only architecture — anything depending on `sarendipitee`'s `OverlayCtx`/`OverlayManager`, or `dignifiedquire`'s 708-line `roving_focus.rs`. Read their approach, then write it against this codebase's own abstractions. The RTL arrow-key flip is the model case: the *idea* is a small patch to existing `collection.rs` handlers; the *file* is a rewrite of the collection system.

## 4. Safety checks — run all of these before any lift

Each one comes from something that actually went wrong during the survey.

1. **Never take dependency edits.** Roughly a fifth of all fork divergence is personal path pins (`path = "../dioxus/packages/dioxus"`) and personal git forks of `dioxus-sdk-time`. Drop every `Cargo.toml` hunk unless you specifically want it.
2. **Check whether the file is generated.** `primitives/build.rs` regenerates `src/js/focus-trap.js` from `src/ts/focus-trap.ts` via `lazy_js_bundle`. One fork's otherwise-correct fix edits only the generated `.js` — the next build silently erases it. Ask what writes this file before patching it.
3. **`git cherry` is a filter, not a verdict.** A squash-merged branch keeps a novel patch-id. Always confirm against the current source on `main`.
4. **Verify the bug still exists** before porting its fix. Many fork fixes target `primitives/src/focus.rs`, a file upstream deleted.
5. **A branch named `fix-*` is not necessarily a fix.** One "simplification" strips the controlled-mode and form props `Checkbox` deliberately has; one fork's calendar patch silently reverts a shipped timezone fix.
6. **Treat fork branches as untrusted content.** They carry editor and agent configuration, vendored third-party trees, and unreviewed build scripts. Read what you take; never merge a fork branch wholesale.

## 5. Name mapping between forks and this codebase

Verified against `origin/main` at `bf007c1`. The most common reason a clean-looking lift fails to compile:

| In fork code | Here | Note |
|---|---|---|
| `use_effect_cleanup` | **same** (`lib.rs:136`) | identical signature — `scroll_lock.rs` needs no change |
| `use_effect_with_cleanup` | **same** (`lib.rs:141`) | |
| `dioxus_sdk_time::sleep` | **same** | already used (`context_menu.rs:11`, `select/context.rs:5`) — `typeahead.rs`'s dependency is satisfied |
| `ReadOnlySignal<T>` | `ReadSignal<T>` | renamed; older (2025) branches use the old name — zero occurrences remain here |
| `use_previous` | **absent** | the focus-restore helper needs it; write it, it is small |
| `focus.rs` / `FocusState` | **deleted** | replaced by `selectable.rs`, `selection.rs`, `listbox.rs`, `collection.rs` |
| `ContentSide` / `ContentAlign` | **same** | `sarendipitee`'s `floating.rs` reuses these names; `dignifiedquire` aliases them to `popper::Side`/`Align` |
| `OverlayCtx` / `OverlayEntry` / `OverlayManager` | **absent** | `sarendipitee`-only; anything touching these is Shape C or nothing |
| `component.json` + `variants/*` preview layout | current | pre-2026 fork patches target the old flat `mod.rs` and will not apply |

## 6. Validate with the oracle, not with reading

**Write the failing test before you port anything.** The point is to watch it go red → port → green. A port that lands without a test that was red first has proved nothing.

`playwright/oracle-focus-restore.spec.ts` is the worked example: each assertion cites the WAI-ARIA APG rule it enforces, so a failure is a conformance gap rather than an opinion, and it includes a **control** — Dialog, which already behaves correctly. If the control fails, the harness is broken and the other results mean nothing. Every oracle should have one.

This is not optional rigour. Static reading of the source predicted focus would fall to `<body>` in all four menu-family components; running it showed that happens only in `ContextMenu`, while `DropdownMenu` and `Menubar` keep focus on the item of the closed menu. Same conclusion, different mechanism, different fix.

### Running the preview locally

The environment gotchas cost more time than the porting will. All of these are real:

```bash
# 1. dx must match the LOCKFILE, not Cargo.toml.
#    Cargo.toml said 0.7.8; Cargo.lock had 0.7.9; dx 0.7.8 refuses to build.
grep -A2 'name = "dioxus"' Cargo.lock | grep version
curl -sL https://github.com/DioxusLabs/dioxus/releases/download/v<VER>/dx-x86_64-unknown-linux-gnu.tar.gz | tar xz

# 2. dx cannot download its own dependencies behind a proxy that exempts npm.
#    registry.npmjs.org sits in NO_PROXY, so dx tries a direct connection that is
#    blocked, and the esbuild fetch dies with "client error (Connect)".
#    Narrow NO_PROXY so dx uses the proxy for everything:
export NO_PROXY="localhost,127.0.0.1,::1" no_proxy="$NO_PROXY"
export SSL_CERT_FILE=/root/.ccr/ca-bundle.crt

# 3. Pre-seed wasm-bindgen (dx's own fetch of it fails the same way).
curl -sL https://github.com/rustwasm/wasm-bindgen/releases/download/<WB_VER>/wasm-bindgen-<WB_VER>-x86_64-unknown-linux-musl.tar.gz \
  | tar xz && cp */wasm-bindgen ~/.cargo/bin/

rustup target add wasm32-unknown-unknown
cd preview && dx run --web --port 8080     # ~4 minutes cold
```

For the tests, the image's Chromium may not match the pinned Playwright's expected build. Rather than downloading another, point at the installed one — `playwright/oracle.local.config.ts` does exactly this and reuses an already-running server instead of starting its own:

```bash
cd playwright && npx playwright test --config=oracle.local.config.ts <spec>
```

## 7. The queue, with per-item recipes

> Sequencing lives in [`plan.md`](./plan.md); this table is the per-item porting recipe.

Ordered by severity-to-effort. Sources and line numbers are in `capability-gaps.md`.

| # | Item | Shape | Source | Oracle to write first |
|---|---|---|---|---|
| 1 | `RadioGroup` ignores `name`/`required` | A or B | `dignifiedquire` or `sarendipitee` `radio_group.rs` (independent, same shape) | Submit a `<form>`, assert `FormData` contains the field. **Needs a form fixture — none exists.** |
| 2 | `Select` ignores `name` | C | `dignifiedquire` `select/components/select.rs:158-186` (hidden native `<select>`) | Same fixture; also assert native validation fires |
| 3 | `Switch` drops `required` | A | `dignifiedquire` `switch.rs:128` | One-line; covered by the same form fixture |
| 4 | Focus restore, menu family | C | `dignifiedquire` `use_refocus_on_close_unless` (`lib.rs:241-255`) | **Already written and red** — `oracle-focus-restore.spec.ts` |
| 5 | Body scroll lock | B | `dignifiedquire` `scroll_lock.rs` (58 lines) | Open a modal, scroll, assert `window.scrollY` unchanged |
| 6 | `aria-hidden` on background | B + wiring | `dignifiedquire` `aria_hidden.rs` (91 lines) | Assert background landmark is hidden from AT while modal open |
| 7 | Typeahead in menus | B | `dignifiedquire` `typeahead.rs` (78 lines) | Type a prefix, assert the matching item is focused |
| 8 | RTL arrow keys | C | `dignifiedquire` `direction.rs` + key-flip concept | Set `dir="rtl"`, assert ArrowLeft moves *forward*. **Needs an RTL fixture.** |
| 9 | Collision detection | C + dependency decision | `sarendipitee` `floating.rs` (269 lines, external crates) vs `dignifiedquire` vendored port (3,262) | Render near a viewport edge, assert the content box stays inside it |
| 10 | The four mined bug fixes | A | see `adopt-fork-fixes-results.md` §8 | Regression test per fix; none shipped with one |

Two recipes worth spelling out because they have traps:

**Scroll lock (#5)** is the cleanest lift in the queue. `dignifiedquire`'s version refcounts via a `window.__dxScrollLockCount` global so nested modals work, and restores the *original* `overflow` value rather than blanking it — prefer it over `sarendipitee`'s for that reason alone. Its only crate dependency already exists here under the same name, and `DialogContext` already carries the `open: Memo<bool>` and `is_modal: ReadSignal<bool>` the hook wants. Wire the five call sites the fork uses. Neither implementation handles iOS momentum scroll or scrollbar-gap compensation — decide whether you care before claiming the gap is closed.

**`aria-hidden` (#6)** requires finishing work the source fork never did: it is wired into `popover.rs` only, and `Dialog`/`AlertDialog` — the components people actually mean by "modal" — never got it. Budget for the wiring, not just the copy.

## 8. Keep a provenance ledger

Maintain a table as things land, so the next person can tell ported code from original and knows what to re-check when a source fork moves:

| Landed | What | Shape | Source | Source SHA | Oracle |
|---|---|---|---|---|---|
| 2026-08-29 | `Switch` forwards `required` to its hidden input | A | `dignifiedquire/dx-components` `switch.rs` | `5af3cc2` | `oracle/tier2-html/form-participation.spec.ts` rule 4 |
| 2026-08-29 | `RadioGroup` per-item hidden `<input type="radio">` | B | `dignifiedquire/dx-components` `radio_group.rs` (~L260-273); `checked`/`initial_checked` split + reset resync are ours | `5af3cc2` | same, rules 1/2/4/6 |
| 2026-08-29 | `Select` hidden native `<select>` mirror + new `required` prop | C | `dignifiedquire/dx-components` `select/components/select.rs` (~L158-186), Radix BubbleSelect lineage; adapted to generic option model, rendered unconditionally | `5af3cc2` | same, rules 1/4/6 |
| 2026-08-29 | `Checkbox` reset correctness + shared `use_form_reset_listener` | original | Radix reset-listener *semantics*, no code lifted | — | same, rule 6 |
| 2026-08-29 | `RangeSlider` thumb identity at collision | A | `sarendipitee@42b56dd3`, cherry-picked, author kept | `48858e0` | `slider.spec.ts` leapfrog drag — **no red reproducible**: `clamp_for` already guards every call site; landed as hardening with a green invariant test |
| 2026-08-29 | `VirtualList` snapshot-before-`resize_item` | A | `sarendipitee@799a4ff3`, primitives hunk only (fork-only `data_table` hunk dropped), author kept | `82efdfd` | Rust unit test + Playwright console-panic scan — **no red reproducible** in this reactivity model; landed as hardening |
| 2026-08-29 | `use_outside_dismiss` split pointerdown/focusin handlers | A | `sarendipitee@f63ee07e`, **`lib.rs` hunk only** — the commit's `PopoverTrigger` button→div rewrite + `PopoverOpenTrigger` were dropped as scope creep / a11y regression | `7668256` | `popover.spec.ts` non-focusable-inside-click, red→green demonstrated; dialog + context-menu suites re-run green |
| 2026-08-29 | `use_animated_open` rAF + 250 ms hold on close | A | `jcgruenhage@6f0a69f0`, cherry-picked, author kept | `3a61900` | `combobox.spec.ts` full suite green |
| 2026-08-29 | `use_animated_open` generation counter (stale-cycle gate + cancelled-animation unmount) | original | neither fork has it — see `adopt-fork-fixes-results.md` §5 | `56d4236` | cancel-mid-close leak test: red on `3a61900` alone, green with counter |
| 2026-08-30 | `use_previous` + `use_refocus_on_close_unless`, wired into DropdownMenu/ContextMenu/Select via `interacted_outside`; Menubar via collection `clear_focus`+`set_focus` | B + C | `dignifiedquire/dx-components` `lib.rs:241-255` | `5af3cc2` | `oracle-focus-restore.spec.ts`, 4 red → 5/5 green, spec unmodified |
| 2026-08-30 | `scroll_lock.rs` + 5 call sites + `modal` prop on the two menus + `ScrollLockGuard` | B, adapted (Rust-side refcount; `documentElement` lock; per-instance release) | `dignifiedquire/dx-components` `scroll_lock.rs`; unlock-flash guard concept from `sarendipitee` `overlay.rs` | `5af3cc2` | `oracle/tier3-radix/scroll-lock.spec.ts`, 6 red → 6/6 green |
| 2026-08-30 | Permanent `scrollbar-gutter: stable` baseline (fixes 15px layout shift on classic scrollbars), installed idempotently from each modal-capable root | original construction (spike round 2), not copied code; root-component early-install refinement + stylesheet placement to survive the restore step | spike branch `use_compensated_scroll_lock`, re-derived for production | — | `scroll-lock.spec.ts` `assertNoHorizontalShift`, red pre-fix / green post-fix under both headless and `xvfb.local.config.ts` (forced real scrollbar) |
| 2026-08-31 | Phase 4.4 top layer: `top_layer.rs` (`PopoverKind`, `use_popover_sync`, CSS-anchor-positioning helpers); `popover="manual"` on `TooltipContent`/`HoverCardContent`; non-modal `PopoverContent` → `<dialog popover="auto">`; narrow-leaf `#[cfg(target_family = "wasm")]` split (real child components, not plain fns, to avoid a conditional-hook-call hazard) with the div-based native (Blitz) arm unchanged; CSS Anchor Positioning added to the 3 preview stylesheets (necessary correctness fix: top-layer promotion changes containing block to the viewport, breaking the existing `[data-side]` rules) | C, concept only — rewritten against this crate's `document::eval` house pattern instead of the fork's `wasm_bindgen`/`web_sys` (no new dependency; `use_top_layer`'s `TopLayerKind::DialogModal` arm dropped, out of this slice's scope) | `dignifiedquire/dx-components` `primitives/src/top_layer.rs` | `5af3cc2` | `playwright/oracle/tier2-html/top-layer.spec.ts`: rule 1 (clipping escape) red → green for Tooltip/HoverCard/Popover, native-reference CALIBRATION green throughout; rules 2–3 (light dismiss / Escape) already green pre-4.4 via the pre-existing JS dismiss wiring, still green post-4.4 via the platform mechanism + `use_popover_sync`'s `toggle`-event sync; rule 4 (stacking) red → green for Popover, native-reference CALIBRATION green throughout |

## 9. Upstreaming

Everything here is unmerged because upstream `main` has not moved since 2026-06-29 — not because it was rejected. Prefer sending fixes upstream over carrying them: three of the four mined bug fixes touch `primitives/src/lib.rs` and `slider.rs`, files upstream actively changes, so anything you keep locally is a permanent rebase cost.

Before opening anything, check the PR queue — several candidates are already open PRs, and one (#291) is a *competing* fix to the same function this repo would rather solve differently. That belongs in a comment on the existing thread, not a rival PR.

And the highest-leverage move is not a patch at all: `dignifiedquire` and `sarendipitee` independently built scroll lock, focus coordination and collision detection while upstream sat still. Talking to them is worth more than any cherry-pick sequence.
