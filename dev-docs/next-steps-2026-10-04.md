# Next steps — 2026-10-04 (paste-ready thread briefs)

Written at the end of the build-loop and theme-foundations round (`backlog.md` rows 116 and 111). Order: row 111 phase C first, then the open small rows. Each brief is self-contained and can be pasted into a fresh thread. **Round 2 (same date) is recorded in the next section; its refreshed pick-up list supersedes the thread order below, and Threads 1 and 2 stay as the detailed briefs they feed.**

Repo: MentalGear/dioxus-components, `main` @ `d9bcfda` (17 commits since `2241da8`). Landed this round: row 116 (cheap server profile, `scripts/build-ssg.sh`, `scripts/lane-target.sh`, SessionStart hook, the debug-wasm `stderr` link fix), rows 122/123, row 111 phases A and B plus a scoped global border-box, the Navbar light-dismiss fix (row 40). Full-suite and gate numbers for the final tip are in `backlog.md` rows 111/116 (filled 2026-10-04).

Environment rules that apply to every thread (all in CLAUDE.md): build SSG only through `scripts/build-ssg.sh [debug|release] [--base-path P]` with an absolute, isolated `CARGO_TARGET_DIR` (`scripts/lane-target.sh create <name>` copies a warm tree; a 6.1 GB copy takes ~24 s, no reflink here); functional specs may run on debug SSG builds, timing specs (`oracle/tier2-html/main-thread.spec.ts`, any latency threshold) only on release; a fresh container is bootstrapped by `scripts/session-setup.sh` — if `dx` is missing, run it; run every gate before every commit.


---

## Round 2 outcome (2026-10-04, branch `claude/zealous-davinci-mtusif`, base `main` @ `44a103e`; uncommitted when written)

Lanes ran with disjoint file sets (CLAUDE.md's lane rule). New backlog rows 139-151; research notes in `dev-docs/research/*-2026-10-04.md` (static reads of `44a103e` plus the dirty tree; `scroll-jank-2026-10-04.md` sections 0-6 describe HEAD and its section 7 the fixes).

| Lane | What landed | Recorded in |
|---|---|---|
| Navigation Menu | panel, featured card and list fixes | `component-backlog.md`, `navigation_menu.spec.ts` |
| Data Table, Resizable | checkbox centring and a layout-shift fix (checkbox is now `display: flex`); resizable grip hit area and an RTL pointer-drag direction bug | `data_table.spec.ts`, `resizable.spec.ts` |
| Menu-item indicator | one spacing construction across select, combobox, native-select, dropdown, context-menu, command (the `--dx-menu-item-indicator-*` tokens) | `playwright/menu-indicator-gap.spec.ts`; audit section 2 (C2) for what is still per-component |
| Menu checkable items | `CheckboxItem` / `RadioGroup` / `RadioItem` for dropdown, context-menu and menubar from `primitives/src/menu_item.rs` | `oracle/tier1-apg/menu-roles.spec.ts`, per-host specs; open edge: row 140 |
| Overlay scrim | one construction across popover, dialog, alert-dialog, sheet, drawer (popover demo -> shadcn's Dimensions form, `PopoverTrigger` aria) | `playwright/assert-backdrop-fade.ts`; open: row 141, audit section 2 (C1: Command dialog) |
| Theme picker | shadcn presets 9 bases x 17 accents x 6 radii, no-flash pre-paint script, phone Drawer in the mobile nav sheet; preset contrast fixes | row 111 round-2 addendum; open: row 142 |
| Gates and head | `scripts/run-gates.sh` (three clippy arms), `check-eager-head-document.sh`, `check-uncleared-intervals.sh`, `check-blocking-scroll-listeners.sh`; `eager_head.rs` | rows 116 (addendum), 139, 151, 150; CLAUDE.md |
| Scroll jank | `use_interval`, blocks mounted as real components, no blocking wheel/touch listener on `#main`, `content-visibility` on cards and variant sections | `research/scroll-jank-2026-10-04.md` section 7; rows 149, 150 |
| Chat kit | message, bubble, marker, attachment, `dx-effects.css` (scroll-fade, shimmer); `message_scroller` stage 1 (`Virtualization::{None, ContentVisibility}`, windowing deferred by the owner) | `component-backlog.md`; rows 146, 148 |

**What this round taught about how to work (each is a row, not a reminder):** (1) a "gates green" is a claim about one tree: row 139's was taken before the commit that broke clippy, so quote `run-gates.sh`'s tree line with every green; (2) `cargo clippy --workspace` does not lint the `web` or `server` arms, so a lint-clean workspace said nothing about shipped code -- three arms now; (3) a `Callback::call` runs hooks on the scope that CREATED the callback, so any block rendered through one is a leak candidate -- the gate for it is unbuilt (row 150's candidate); (4) a `document::Link` can be recorded and never inserted when its subtree is torn down in the turn it first renders (row 151) -- the third occurrence of that class.

**Phase C's `before` snapshot is now stale.** Thread 1 below says to capture it at `main` @ `d9bcfda`; round 2 changed computed styles under it (scrim, indicator spacing, presets, destructive fill/hover, link-button ink, checkbox `display: flex`). Re-capture at the round-2 tip (format 2, light and dark, A/A byte-identical) before the next theme or unification lane starts; it cannot be recovered afterwards.

**Verification of the round-2 tree** (gates via `scripts/run-gates.sh` -- the summary's tree line and diff hash; release no-base-path SSG build via `scripts/build-ssg.sh release`; full Playwright suite diffed against the 2026-10-04 baseline of `20e2fa4`, 1414 passed / 32 failed / 13 skipped; scroll re-measure on the release build via `node scripts/measure-scroll.mjs`, against `research/scroll-jank-2026-10-04.md` section 7.6's HEAD row and its pending release row): **done 2026-10-04, green after one real fix; full account in `backlog.md` row 116's round-2 verification paragraph.** Gates: `run-gates.sh` ALL GREEN 23/23 (diff-hash `63cf2ec50538`, again `98e08d3c4c41` after the fixes). Release no-base-path SSG build: 14 m 14 s cold, 93 pages, STALE_HTML=0, both head scripts and both new stylesheets on every page. Full suite (1786 tests): 1729 / 44 / 13 on the handed-over tree, **1761 passed / 12 failed / 13 skipped** after the fixes, the 12 being 6 known (rows 113, 21 x2, 106 Rule 8, 114, the Rule 4c selftest) and 6 load flakes green on a `--workers=1` rerun. The one real regression was `content-visibility: auto` on `.dx-component-variant` (carousels and charts that measure on mount measured 0 inside skipped variants: 33 tests red); the rule is gone, the home cards keep theirs. Three specs fixed (`drawer.spec.ts` handle pressed mid slide-in, `navigation_menu.spec.ts` plain `goto`, `top-layer-ink.spec.ts` old popover demo text) plus `scroll-main-thread.spec.ts` test 6. Scroll, release, median of 3, before (pre-round) -> after: idle 178 -> 45 ms/s, idle layouts 71 -> 3 per 3 s, scroll busy 6,745 -> 2,089 ms, layouts 387 -> 170, tasks > 16 ms 8 -> 0, wheel p95 57.8 -> 49.0 ms, live intervals 74 -> 0 (and flat over a 150 s warm-up: 160 -> 0).

### Refreshed pick-up list (supersedes the thread order below)

1. **Unification top 3 (row 145; `research/unification-audit-2026-10-04.md` sections 1 and 7).** Serial first: the **theme baseline** lane (owns `preview/assets/dx-components-theme.css`: `font-family: inherit`, reduced-motion block, `touch-action`, half-step spacing tokens, the shared `::backdrop` block, the overlay and indicator tokens; near-zero visual change except fonts and the Command scrim). Then the **floating-surface recipe** (12 surfaces; `top_layer.rs` offsets only) and the **state recipes** (focus, invalid, disabled; **fixes Combobox's and Textarea's missing focus ring**) -- they collide on `select`, `combobox`, `popover` and `date_picker` CSS, so floating owns those four files and state takes the rest, or run state second. Each lane brings its own gate. Do row 135's overlay and hover snapshot pass first, or the floating-surface lane has no proof.
2. **`VirtualList` content-visibility mode** (row 143; caution from round-2 verification: a skipped subtree has no layout box, so anything that measures on mount, as the carousel and chart tooltip did inside skipped variants, must re-measure on un-skip first): default for N <= ~5k, windowing above; quantize the scroll signal; drop the O(N) per-resize clone; find-in-page and 10k-row specs. Disjoint from everything above (`primitives/src/virtual_list.rs`, `virtual/`).
3. **Opt-in `hidden="until-found"`** for Collapsible, Accordion and Tabs (row 144); delete `tabs/style.css:118`'s `display: none` for the opt-in; verify the Ctrl+F path by hand.
4. **`message_scroller` stage 2** (row 146): visibility hook, `scroll_margin`/`align`, RTL pass, a release-build timing spec. Windowing stays deferred by the owner.
5. **Small, mostly independent:** row 149 (`scroll_lock.rs` add-on-lock/remove-on-unlock; S), row 148 (a "Chat" sidebar category; S), row 140 (menubar pointer-close focus; reproduce first), row 142 (phone theme picker on `/demos` and `/charts`), row 147 (Safari 18-25 find-in-page caveat into two comments and one docs page), row 141 (deferred dialog close; M, needs Firefox and WebKit to prove), and row 150's candidate `.call(())` gate (measure false positives first).
6. **Still open from before round 2:** theme phase C (merges with item 1: audit row 5 is phase C as one rule; plus the axe colour-contrast pass over the docs, which the round-2 preset sweep is NOT), the quick and measure-first rows of Thread 2 below, the catalog rows 136-138 (Questionnaire after the chat kit has proved out, `Direction` docs page, Toast depth), and the user-gated rows.

---

## Thread 1 — Theme phase C (backlog row 111) — the big one

**Goal:** finish the shadcn Nova migration for every component outside the seven already done (card, button, input, badge, tabs, dialog, select), without regressing the contrast, layout or behaviour of anything.

**Read first:** `dev-docs/backlog.md` row 111 (the 2026-10-04 addendum is the state of play), `dev-docs/research/theme-2026-10-03/role-token-map.md` (token set, section 10 reconciliation, 10.5 owner recommendations), `border-box.md` (what border-box changed and how to read the snapshot), `preview/assets/dx-components-theme.css`, `playwright/computed-style-snapshot.spec.ts`, and the Nova sources at `shadcn-ui/ui@295a1f1` (`registry/styles/style-nova.css`, `registry/bases/base/ui/*`). Commits `99f00b1` (the pattern for one component: Nova rules transcribed to plain CSS and `--dx-` tokens) and `0e6d3ac`.

**Do first (they make phase C provable):**
1. Row 135: extend the snapshot with an open-state pass for overlays (`alert_dialog`, `command`, `context_menu`, `dialog`, `drawer`, `sheet`, `top_layer`) and a hover pass, with the same A/A byte-identical gate. Phase A's 0-diff proof does not cover them.
2. Capture a fresh `before` snapshot from `main` @ `d9bcfda` (light and dark, format 2) — it cannot be recovered afterwards.

**Plan:**
- Per-component Nova pass in disjoint lanes (e.g. accordion/alert/avatar/breadcrumb; calendar/date_picker/checkbox/radio/switch/slider; menus and popovers; sidebar/navigation_menu/pagination/table; charts last — their parity tests `preview/src/{chart,polar,chart_tooltip}_parity.rs` must stay green, they compare geometry not colour). One reviewer pass per lane over the before/after snapshot; every difference must be an intended Nova change, listed with old -> new.
- Axe colour-contrast re-run over the docs, light and dark (the theme file's own rule; not run in phase B). Rows 134 (tinted destructive, ~2.7:1 in dark) and the dark `--dx-primary` `#e5e5e5` (was `#fafafa` through the ramp) are the known suspects.
- Close the leftovers: `test-harness/assets/dx-components-theme.css` is a stale copy (differs from `preview/assets/dx-components-theme.css`); 40 remaining `var(--primary|secondary-color...)` refs (`sheet`, `carousel`, `toggle`, `toggle_group`, `main.css`, `top_layer.rs`) get a role or a stated reason; row 132 (`.dx-tabs-content` padding), row 133 (InputGroup invalid/disabled).
- Owner questions to ask before building, not after: the chart palette (shadcn's blue ramp vs our multi-hue `--dx-chart-1..8`, row 119 tie-in), and whether accent dark moves `#3e3e3e` -> `#262626` (role-token-map.md 10.5).

**Deliberately not taken, keep that way unless the owner changes it:** shadcn's grey focus ring (~2.6:1 on white), Nova's tinted destructive button, the `#171717` dark card/popover.

**Done when:** snapshot diff per lane reviewed, every `scripts/check-*.sh` plus fmt/clippy/test/stylelint green (CLAUDE.md gate list, including `check-css-vars-defined.sh` and the radius check in `check-css-literals.sh`), axe colour-contrast clean or each exception recorded, full Playwright suite diffed against the `d9bcfda` baseline, row 111 addendum appended.

---

## Thread 2 — Open small rows (quick ones first)

One thread can take the **quick** group in a single pass; the rest each need a measurement before a fix. All file sets are disjoint across rows unless noted.

**Quick (S, mostly mechanical):**
- **Row 129** — `preview/src/components/navbar/component.json` lacks the theme `globalAssets` entry. Add it, drop the kept `box-sizing: border-box` at `navbar/style.css:3`, and add a check that every component with a `style.css` lists the theme.
- **Row 120** — duplicate `id="dnd-instructions"`: per-instance id from `use_unique_id` through the list's context in `primitives/src/drag_and_drop_list.rs`, plus a duplicate-id assertion over the home page in `preview.spec.ts`.
- **Row 127** — fix the 7 `preview/assets/main.css` stylelint errors, widen the gate glob to `"src/**/*.css" "assets/**/*.css"` in CLAUDE.md, confirm the widened glob reports a seeded error first.
- **Row 125** — chart nits: radar legend offset, pie-interactive Select swatch, area `icons` legend glyph colour; extend the demo-level parity tests.
- **Rows 130, 131** — upstream filings (arborium `stderr`; dioxus-cli `--ssg` stale `public/`), text and repro are in the rows; needs the owner's account or approval to post.

**Needs a measurement first (S-M):**
- **Row 121** — `navigation_menu.spec.ts:128` fails on every SSG run: read the failing sample values; decide sampling race (row 112 family) vs a real width change; do not widen the threshold.
- **Row 124** — chart pre-hydration layout jump (container-query units on `.dx-chart`) and the `Select` SSR rendering one `<option>`.
- **Row 126** — `Link` `aria_current` duplicate attribute under a base path: reproduce on a `--base-path` SSG build and count attributes per `<a>` before touching `Link` or the gallery's plain anchors.
- **Row 132** — `.dx-tabs-content` padding consumers (phase C overlaps).
- **Row 133** — InputGroup invalid/disabled styling (phase C overlaps).

**Verified, no task left:** row 40's Navbar fix `d9bcfda` is green on the release build of `b9e9580` (opening-gesture probe 2/2, `navbar.spec.ts` 5/5, keyboard matrix 77 passed, 1 skipped, `top-layer.spec.ts` only the known row 106 red) -- recorded in `backlog.md` row 40.

Rows 122 and 123 have no leftovers (landed, `8c99edd`).

**Done when:** each row's own fix-by-construction is in, its gate or test fails first (red-first rule), all gates green, the row gets a dated addendum.
