# Adopting fixes from the `dioxus-components` fork network

**Scan date:** 2026-08-29
**Baseline:** `bf007c1` — `Tag Group component (#271)`, 2026-06-29, the tip of both `DioxusLabs/dioxus-components@main` and this fork
**Verification level:** static only. Every claim below was checked by reading diffs and the current code on `main`, and every cherry-pick result was executed. **Nothing here was compiled, and no test suite was run.** Treat "merges cleanly" as "git applied it", not "it works".

---

## TL;DR

The fork network holds **four correctness fixes that were never submitted upstream** and are still live bugs on `main`. Three of them come from one person's product fork, one from another; none has a pull request. All four are small, touch only `primitives/`, and apply to today's `main`.

| # | Fix | Source | Files | Cherry-pick |
|---|---|---|---|---|
| 1 | `RangeSlider` thumbs swap identity when they collide | `sarendipitee@42b56dd3` | `slider.rs` | clean |
| 2 | `use_animated_open` unmounts mid-race / stale promise overwrites state | `jcgruenhage@6f0a69f0` | `lib.rs` | clean |
| 3 | Popover self-dismisses when you click a non-focusable area inside it | `sarendipitee@f63ee07e` | `lib.rs`, `popover.rs` | clean (minus one test hunk) |
| 4 | `VirtualList` holds a signal borrow across a call that reads it | `sarendipitee@799a4ff3` | `virtual_list.rs` | clean (primitives hunk) |

Beyond those, **11 candidates are conditional** (§6) and **~45 are rejected** (§7) — mostly already merged upstream in a better form, or personal build hacks.

One decision has to be made before adopting #2: two forks patch the same function in **incompatible** ways, and the one with an open upstream PR is the weaker of the two. See §5.

---

## 1. Scope

This repository is a fork of `DioxusLabs/dioxus-components` created 2026-08-29, sitting exactly on upstream's `main` — zero divergence. Upstream `main` has not moved since 2026-06-29, so anything found in the network is genuinely unmerged rather than merely unreleased.

The question this document answers: **across all 81 sibling forks, what work exists that upstream does not have, and which parts of it are worth taking?**

## 2. Method

```bash
# 1. Enumerate the fork network (the GitHub API is not reachable from this
#    environment; the network members page is)
#    https://github.com/DioxusLabs/dioxus-components/network/members

# 2. Cheap first pass: list every fork's refs and drop any SHA that already
#    exists anywhere in upstream's 637-commit history (all branches, not just main)
git ls-remote --heads --tags https://github.com/<owner>/<repo>

# 3. Fetch what survives into an upstream clone under a private namespace
git fetch --no-tags https://github.com/<owner>/<repo> \
    "+refs/heads/<branch>:refs/mined/<owner>/<branch>"

# 4. Drop patches already upstream by patch-id (catches verbatim merges)
git cherry origin/main refs/mined/<owner>/<branch>   # '-' == already upstream

# 5. For everything left: read the diff, then read the CURRENT code on main to
#    decide whether the bug is still real, and test applicability
git diff origin/main...refs/mined/<owner>/<branch>
git merge-tree --write-tree origin/main refs/mined/<owner>/<branch>
git cherry-pick --no-commit <sha>          # in a throwaway worktree
```

Step 4 is necessary but **not sufficient**: a squash-merged multi-commit branch keeps a different patch-id, so `git cherry` still calls it novel. Every candidate was therefore additionally checked against the current source of `main` — which is what caught, for example, `ealmloff/fix-initial-value-select` landing verbatim (down to a stray-space typo now sitting at `select/components/group.rs:174`).

### Why every branch, not just each fork's `main`

In a fork-and-PR workflow the fork's `main` stays a mirror of upstream while the work sits on topic branches. Of the 111 refs carrying novel commits, only **18** are a fork's `main`; **93** are topic branches, and **31 forks have an upstream-identical `main` but a topic branch with unique work**. A main-only sweep would have missed every Tier 1 fix in this document.

## 3. What the scan covered

| | |
|---|---|
| Forks in the network | 81 (+ this one) |
| Reachable | 80 (`LinusCentrostrom/dioxus-components` failed to resolve) |
| Refs carrying commits absent from upstream | 111 (`gh-pages` build branches excluded) |
| — of those, fork `main` | 18 |
| — topic branches | 93 |
| Forks with at least one novel ref | 58 |
| Branches discarded as patch-identical to upstream | 10 |
| Candidates analysed in depth | ~60 |
| **Recommended for adoption** | **4** |
| Conditional | 11 |

Five branch names (`docs`, `style`, `consistant-props`, `slider-fixes`, `fix-tab-keyboard-focus`) appear identically in 5–6 forks each — they are copies of upstream branches that were deleted after merging, not independent work, and collapse to one candidate each.

---

## 4. Tier 1 — adopt

All four are authored by people who never opened a PR for them (verified: `jcgruenhage` has no PRs against upstream; `sarendipitee` has four, none of which are these three commits). They are invisible to anyone reading upstream's PR queue.

### 1. `RangeSlider` thumbs swap identity on collision

- **Source:** `sarendipitee/dioxus-components@42b56dd3`, Saren, 2026-08-08 — `fix(slider): preserve thumb identity at collision`
- **Files:** `primitives/src/slider.rs` · **cherry-pick: clean**

`set_thumb` passes both thumb positions through `ordered_range(...)`, which *sorts* the pair. When you drag one thumb past the other, the sort silently reassigns which thumb is `start` and which is `end` — so the thumb under the user's finger changes identity mid-drag and the drag continues against the wrong end of the range.

Still live on `main`, `primitives/src/slider.rs:264-265`:

```rust
let next = match idx {
    0 => ordered_range(v, cur.end),
    _ => ordered_range(cur.start, v),
};
```

The fix clamps instead of reordering: `v.min(cur.end)..cur.end` / `cur.start..v.max(cur.start)`.

**Risk:** low — one function, no API change. **Missing:** no test; a Playwright case dragging one thumb past the other is worth adding.

### 2. `use_animated_open` unmounts under a race

- **Source:** `jcgruenhage/dioxus-components@6f0a69f0`, JC Grünhage, 2026-05-23 — `fix(animations): keep closing element in DOM 250ms before unmount`
- **Files:** `primitives/src/lib.rs` · **cherry-pick: clean** (+27/−10)

`use_animated_open` is shared by `Dialog`, `AlertDialog`, `Popover`, `Combobox`, `DropdownMenu`, `ContextMenu`, `HoverCard`, `Tooltip`, `Accordion`, `Listbox`, `Menubar` and `Navbar` — so anything wrong in it is wrong everywhere. `primitives/src/lib.rs:244-277` on `main` is byte-for-byte the pre-fix version, including a dead `animating` signal that is declared without `mut`, never written, and therefore always `false` while still being OR'd into the hook's return value.

The commit fixes three things at once: it waits a frame before querying animations, holds the closing element in the DOM ~250 ms after the animation ends (it is `opacity: 0` / `pointer-events: none` throughout, so users see nothing) so observers such as Playwright and screen readers get a predictable window, and — most importantly — it *declines to send* when the animation promise rejects, because a rejection means a newer open/close cycle is already in flight and resolving the stale `recv()` would let it overwrite fresher state. It was root-caused against an intermittent WebKit failure in `combobox.spec.ts:131`.

**Risk:** medium — it changes unmount timing for every animated overlay. The 250 ms hold is a deliberate trade of DOM-residency for observability and should be reviewed as a policy, not just a patch. **Conflicts with an open upstream PR — see §5 before adopting.**

### 3. Popover dismisses itself when you click inside it

- **Source:** `sarendipitee/dioxus-components@f63ee07e`, Saren, 2026-06-15 — `Fix Popover dismiss when clicked inside content`
- **Files:** `primitives/src/lib.rs`, `primitives/src/popover.rs` · **cherry-pick: clean for both primitives files**; only the fork's own `playwright/popover.spec.ts` hunk conflicts (drop it and write the test against current selectors)

`use_outside_dismiss` (`primitives/src/lib.rs:214-229`) registers **one** handler for both `pointerdown` and `focusin`, dismissing whenever `!root.contains(e.target)`. That is correct for pointer events and wrong for focus ones: clicking a non-focusable region inside the popover blurs the active control, and the browser moves focus to the nearest focusable *ancestor* — which is outside `root` while still containing it. The shared handler reads that as focus leaving and closes the popover the user just clicked into.

The fix splits the handler into `onPointer` / `onFocus` and gives the focus path an ancestor-containment check.

**Risk:** low-medium — `use_outside_dismiss` is also used by `Dialog` (`dialog.rs:231`), so the behaviour change is not popover-only. That is an argument for adopting it (same latent bug) but means dialog light-dismiss should be re-tested too.

### 4. `VirtualList` holds a signal borrow across a call that reads it

- **Source:** `sarendipitee/dioxus-components@799a4ff3`, Saren, 2026-06-28 — `Fix virtual_list resize memoization`
- **Files:** `primitives/src/virtual_list.rs` · **cherry-pick of the primitives hunk: clean** (the full commit also touches a fork-only `data_table` file — split it)

`primitives/src/virtual_list.rs:198-205` keeps a `peek()` guard alive across the call that uses it:

```rust
let m = measurements.peek();
let adjustment = resize_item(&state, &m, idx, measured);
drop(m);
```

The `drop` comes after the call, so the borrow is held for its whole duration — a re-entrant read/write of the same signal inside `resize_item` panics rather than misbehaving quietly. The fix clones the snapshot before the call.

**Risk:** low — a defensive change in one closure.

---

## 5. Decide first: two forks fix `use_animated_open` in opposite directions

`ziimakc` and `jcgruenhage` independently found the same rejection and drew opposite conclusions. They cannot both be applied — they touch the same lines — and **the one with the open upstream PR is the weaker of the two.**

**`ziimakc@573cc1e9` (upstream PR #291, open)** swallows the rejection per animation:

```js
Promise.all(element.getAnimations().map((animation) => animation.finished.catch(() => {})))
```

`Promise.all` then always resolves, `dioxus.send(true)` always fires, and the awaiting task always runs `show_in_dom.set(open)`.

**`jcgruenhage@6f0a69f0`** catches at the end of the chain and deliberately sends *nothing*, with the reasoning in a comment: an aborted animation means a newer cycle is in flight, so resolving the stale `recv()` would overwrite newer state.

The distinction matters because `open` is captured by value when the effect runs. Take a rapid toggle — close starts, then the user reopens before the close animation ends:

1. The close-path task is spawned with `open == false` and awaits the animation.
2. Reopening cancels the animation and re-runs the effect, which sets `show_in_dom = true`.
3. Under `ziimakc`'s patch the swallowed rejection lets the **stale** task resolve and run `show_in_dom.set(false)` — the element vanishes although it is open, and no further effect run will correct it, because `open` is already `true`.

So PR #291 converts a stuck-open element into an element that disappears while open. `jcgruenhage`'s version avoids that, at the cost of leaving one case unhandled: an animation cancelled with no subsequent cycle never resolves, and the element stays mounted in its `data-state="closed"` form (hidden by CSS — a leaked node rather than a visible defect).

**Recommendation:** take `jcgruenhage@6f0a69f0`. If you want the residual case covered too, the correct shape is a per-cycle generation counter — send on rejection *only* when the cycle that spawned the task is still current — rather than either patch as written. If PR #291 lands upstream first, this fork will need to reconcile; the analysis above is the argument to bring to that thread.

---

## 6. Tier 2 — conditional

Each of these is real but carries a blocker. None should be merged as written.

| Candidate | What it gives you | Blocker |
|---|---|---|
| `matchish@df16e449` — `pub mod portal` (`lib.rs`) | `use_portal`/`PortalId` are unreachable by consumers today (`lib.rs:40` still `mod portal;`). One line, **clean** | Public-API commitment; wants docs + a stability decision first |
| `catan2001@a7a6a9f1` — `class` prop on `ToggleProps` | Fills a real inconsistency: `radio_group.rs:208`, `alert_dialog.rs`, `dialog.rs`, `tabs.rs`, `navbar.rs`, `popover.rs` all expose `class`; `toggle.rs:37` only has `extends = GlobalAttributes`. **Clean** | Defaults the class to `"toggle"`, ignoring the `dx-` prefix convention; no docs or test |
| `jaysonmaw@3c43ce75` — public `CalendarDayState` + `use_calendar_day_state`/`use_calendar_day_attributes` | Answers open upstream issue #199 ("the APIs to build each component should be public") for `CalendarDay` | Conflicts in `calendar.rs` (needs rebase); uses `.unwrap()` and an `unreachable!()` reachable by misuse outside a Calendar context |
| `Torvex-UG@045f7dfd` — dialog light-dismiss via backdrop `onclick` (upstream PR #286, open) | Replaces the fragile `getElementById` + `contains()` JS listener with plain event bubbling. **Clean** | Repro never independently confirmed; narrows dismissal to `click` only, dropping the `focusin` path. Overlaps Tier 1 #3, which fixes the same helper's real defect instead — **prefer #3 and re-evaluate this afterwards** |
| `Torvex-UG@ffbed418` — `pub use_global_escape_listener` | Lets consumers join the escape-key stack | **Would fail CI as written.** The crate sets `#![warn(missing_docs)]` (`lib.rs:2`) and CI runs clippy with `-D warnings`; the new `pub fn` has no doc comment. Trivial to fix — add one |
| `arkret-org@3510aeee` — idempotency guard on the focus-trap script | Real problem: `dialog.rs:135-138` (and `alert_dialog.rs`, `popover.rs`) each emit `document::Script { src: FOCUS_TRAP_JS }`, and re-running a classic script that declares a top-level `class FocusTrap` throws | **Patches the generated artifact.** `primitives/build.rs` regenerates `src/js/focus-trap.js` from `src/ts/focus-trap.ts` via `lazy_js_bundle`, and the `.ts` is untouched — the next rebuild silently drops the fix. Re-implement in the TypeScript source |
| `Wervice@a704c517` — tooltip fade-out CSS (upstream PR #293, open) | Symmetric close animation; safe because the content is `position: absolute` and unmount is gated by `use_animated_open`'s `render()` (`tooltip.rs:286-290`). **Clean** | **Increases exposure to Tier 1 #2**: tooltips are toggled fast, which is exactly what cancels animations. Land only after the `use_animated_open` fix |
| `ealmloff@fe23e524` — `default-features = false` for `dioxus` (upstream PR #273, open) | Still absent from `main` (`Cargo.toml:9` is a bare `dioxus = "0.7.8"`); would stop forcing dioxus's full default feature set on every consumer. **Clean** | Repoints `dioxus-sdk-time` at a personal git fork — unpublishable. Drop that hunk and re-check whether the upstream crate still needs it |
| `sarendipitee@56c4e811` — `.peek().clone()` in slider `clamp_for` | Avoids a reactive read (`(self.thumbs)()`, `slider.rs:~785/795`) inside a per-pointer-move effect (`slider.rs:370`) that also writes the value driving it | Surrounding code was refactored upstream; reapply the one-line change by hand |
| `sarendipitee@3b22d34f` — pre-render preview code snippets at build time (was upstream PR #281, **closed unmerged**) | Removes the runtime `dioxus-code` dependency in favour of build-time highlighting; aimed at the wasm/SSG target CI builds. **Clean** | Failure mode not reproduced here, and upstream closed the PR — find out why before reviving. Costs runtime highlighting (fine for static demos) |
| `nickelser@d0e39952` — hoist `FOCUS_TRAP_JS` out of each overlay to the app root | Plausible SSR-hydration fix, complements the focus-trap idempotency item above | **Breaking as shipped**: conflicts with current `dialog.rs`/`popover.rs`, and no `document::Script` exists anywhere in `preview/`, so this repo's own showcase would lose focus trapping (modal defaults to on). Needs a rebase plus the consumer migration |

**Feature-shaped finds** (out of scope for a fix pass, recorded so they are not re-discovered): nested submenus — genuinely absent upstream, `grep -c Submenu primitives/src/*.rs` is 0 everywhere (`molikto@ed681439`, with more mature versions in the `sarendipitee` and `dignifiedquire` forks); Sidebar (three independent implementations — `molikto`, `zhiyanzhaijie/feat-sidebar`, `sarendipitee`; pick one); `RecycleList` (`haywoodfu`); `DragAndDropBoard`, a generic `DateBackend`, and `ComboboxMulti` (`jcgruenhage`).

---

## 7. Tier 3 — rejected

Roughly 45 candidates, in five groups.

**Already upstream, verified in code (~20).** Every substantive branch in the maintainer fork `ealmloff/components` had landed: keyboard nav (`calendar.rs:615-662`, `accordion.rs:446-467`), the focus trap (`src/js/focus-trap.js` + `lib.rs:60`), mobile pointer handling (`menubar.rs:407,414`), the Safari `display:contents` toast bug (now `ToastList`/`ToastListItem`), `Memo<T>` context fields, the dark-mode query param (`main.rs:136-159`), the calendar month/year selectors, the `Button` preview component, and a documentation pass now enforced by `#![warn(missing_docs)]`. Also here: `WhaleFromMars` (drag-and-drop index resolution, byte-identical at `drag_and_drop_list.rs:46,60`), `jaysonmaw/patch-1` and `patch-2` (merged as #193 and #195), `AnttiJalomaki/fix-toolbar`, `p-jackson/slider-reactive-min-max-step` (upstream's version additionally handles `RangeSlider`), `guiemrabassa/multiselect` (shipped as `SelectMulti`), `zhiyanzhaijie/feat-pagination`, and three forks whose `main` tree is byte-identical to ours (`Tumypmyp`, `ddudenin`, plus `matta`/`p-jackson`/`wheregmis` single-commit branches).

**Superseded by later architecture (~8).** `wheregmis/click_out_onfocusout` (per-component focus state → shared `CollectionState`), `knoxfighter/select-focus-disabled` and `combo-box-try-1` (patch `primitives/src/focus.rs`, a file that **no longer exists** — replaced by `selectable.rs`/`selection.rs`/`listbox.rs`/`collection.rs`; the shipped primitive is `combobox`, not `combo_box`), the `DanielWarloch` cluster, and `haywoodfu`'s `VirtualList` scroll fixes (they repair the fork's own superseded engine, not `main`'s).

**Personal build hacks (~8).** `MichiBab` pins dioxus to `path = "../dioxus/packages/dioxus"`; `Integritetsbyran`, `FastTrackStudios` and `ealmloff/fix-features` point `dioxus-sdk-time` at personal forks; `Integritetsbyran` also drops `preview` from the workspace; `Mettwasser`, `abeni-csa`, `PupsieCo` and `DanielWarloch/main` are version-pin churn from resolution trouble long since moot. `AnttiJalomaki/wip-backup` contains nothing but a personal `.claude/settings.json`.

**Whole-repo rewrites, not patches (3).** `dignifiedquire/main` (245 commits) is a ground-up shadcn/Tailwind + Radix-parity reimplementation with an in-repo `floating-ui` port — nothing is cherry-pickable, but its accessibility audit trail is a useful reference. `SPRAGE/main` is a Copilot-generated repackaging into a flat single crate (202 files renamed). `lavaeater/0.8` is a Dioxus 0.8 alpha bump plus a vendored `dioxus-icons` dump (3,684 files); keep it bookmarked for a future 0.8 migration, since it documents the `dioxus-sdk-time` incompatibility.

**Not real fixes (~6).** `sumitpsm/basic-fix` (upstream PR #288) collapses a `format!` string and cites a `data-node` attribute that exists nowhere in the repo; whitespace inside a `style` attribute is inert. `SFSeeger/main` bundles the same calendar API as `jaysonmaw` with a silent **regression** — it reverts `OffsetDateTime::now_local_date()` back to `UtcDateTime::now().date()`, undoing the shipped timezone fix (`calendar.rs:492,496,748,752`, helper at `lib.rs:327-337`). `Fakhir-Israr-200219/table` (upstream PR #254) is a preview-only demo over hardcoded data with no primitive underneath. `hovinen`'s branch (upstream PR #289) adds only a `#[cfg(test)]` module. `dignifiedquire@2e913cd8`'s global-Escape fix does not reproduce here — upstream's menus use element-scoped `onkeydown` and its dialogs gate content behind `render()`; worth remembering as a latent risk if more global-listener consumers appear.

---

## 8. Suggested sequence

Adopt in three batches so each is independently revertable, and validate between them with what CI runs — `cargo clippy --workspace --examples --tests --all-features --all-targets -- -D warnings`, `cargo fmt --check`, and the Playwright suite.

```bash
git remote add mining https://github.com/DioxusLabs/dioxus-components   # object source

# Batch 1 — independent, lowest risk
git cherry-pick 42b56dd3fc133173d679e33ea671e9fd0f3848fa   # slider thumb identity
git cherry-pick 799a4ff39969c7ec6c2b4d04e92dabe48fef3061 -- primitives/src/virtual_list.rs

# Batch 2 — shared dismissal helper; re-test Dialog as well as Popover
git cherry-pick f63ee07e835d6b76177d62bd72a1a173b32157c3   # drop the popover.spec.ts hunk

# Batch 3 — after settling §5
git cherry-pick 6f0a69f037a2360889fb0da42f58cb1c865f7d3a
# then, optionally, Wervice's tooltip fade-out — never before this
```

Each commit needs a Playwright regression test; none of the four shipped with one. Preserve original authorship (`git cherry-pick` does), and credit the source fork in the message — these are other people's fixes.

## 9. Hygiene rules this scan earned

1. **Never take a fork's `Cargo.toml` dependency edits.** Roughly a fifth of all fork divergence is personal path pins and git-fork dependencies. They are the single largest category of noise.
2. **Check whether the file you are patching is generated.** `arkret-org`'s otherwise-correct fix edits `src/js/focus-trap.js`, which `build.rs` regenerates from the `.ts` — it would evaporate on the next build.
3. **`git cherry` is a filter, not a verdict.** Squash merges keep a novel patch-id. Always confirm against the current source.
4. **A branch named `fix-*` is not a fix, and a "simplification" can be a regression.** `wheregmis/simplify_checkbox` strips the controlled-mode and form-integration props (`required`, `name`, `value`) that `main` deliberately keeps.
5. **Treat mined branches as untrusted content.** They carry editor and agent configuration (`AnttiJalomaki/wip-backup` is a `.claude/settings.json` and nothing else), vendored third-party trees, and unreviewed build scripts. Read before merging; never merge a fork branch wholesale.
6. **Check the PR queue before crediting a find.** Several candidates here are open upstream PRs (#273, #286, #288, #291, #293) — which is useful context in both directions: #291 is the weaker of two competing fixes (§5), and #281 was closed unmerged for reasons worth learning before reviving it.

## 10. Re-running this

The expensive part is the depth analysis, not the scan. To refresh after upstream moves: repeat §2 steps 1–4 and diff the resulting ref list against the inventory in §3 — only genuinely new refs need review. The fork network grows slowly (81 forks, a handful active), and the `gh-pages` branches, which are build output, should stay excluded.

Worth watching specifically: `sarendipitee/dioxus-components` (most active, highest fix yield — three of the four Tier 1 fixes), `jcgruenhage/dioxus-components` (small, conventional commits, high signal), and `dignifiedquire/dx-components` (design reference for accessibility work).
