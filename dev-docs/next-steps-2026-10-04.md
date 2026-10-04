# Next steps — 2026-10-04 (paste-ready thread briefs)

Written at the end of the build-loop and theme-foundations round (`backlog.md` rows 116 and 111). Order: row 111 phase C first, then the open small rows. Each brief is self-contained and can be pasted into a fresh thread.

Repo: MentalGear/dioxus-components, `main` @ `d9bcfda` (17 commits since `2241da8`). Landed this round: row 116 (cheap server profile, `scripts/build-ssg.sh`, `scripts/lane-target.sh`, SessionStart hook, the debug-wasm `stderr` link fix), rows 122/123, row 111 phases A and B plus a scoped global border-box, the Navbar light-dismiss fix (row 40). Full-suite and gate numbers for the final tip are in `backlog.md` rows 111/116 (filled 2026-10-04).

Environment rules that apply to every thread (all in CLAUDE.md): build SSG only through `scripts/build-ssg.sh [debug|release] [--base-path P]` with an absolute, isolated `CARGO_TARGET_DIR` (`scripts/lane-target.sh create <name>` copies a warm tree; a 6.1 GB copy takes ~24 s, no reflink here); functional specs may run on debug SSG builds, timing specs (`oracle/tier2-html/main-thread.spec.ts`, any latency threshold) only on release; a fresh container is bootstrapped by `scripts/session-setup.sh` — if `dx` is missing, run it; run every gate before every commit.

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
