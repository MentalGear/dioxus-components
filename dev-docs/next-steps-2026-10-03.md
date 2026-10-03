# Next steps — 2026-10-03 (paste-ready thread briefs)

Written at the end of the chart-parity session (`backlog.md` rows 117-119). Order: row 116 (build/dev loop) first, then row 111 (theme); the open chart follow-ups are rows 120-128. Each brief below is self-contained and can be pasted into a fresh thread.

Repo: MentalGear/dioxus-components, `main` @ `62de3cb` (PRs #79, #80, #81 merged: chart display + docs UX, the shadcn-style charts gallery with pointer-following tooltip, and Recharts/shadcn projection parity).

---

## Thread 1 — Build/dev-loop efficiency (backlog row 116)

**Goal:** cut the verify loop (release SSG build 11–18 min on this 4-core sandbox; cold lanes; cargo-lock and dx-output contention between parallel lanes) several-fold, without changing the deployed artifact.

**Read first:** `dev-docs/backlog.md` row 116 (measured plan), CLAUDE.md (isolated absolute `CARGO_TARGET_DIR`, rows 98/100), `scripts/deploy-preview.sh`, `playwright/ssg.local.config.ts`.

**Evidence from the 2026-10-03 chart session:**
- Fresh container had no `dx` and no `wasm32-unknown-unknown`: `cargo install dioxus-cli --version 0.7.9 --locked` took ~21 min; dx's own tool downloader (esbuild, binaryen/wasm-opt) failed TLS (`UnknownIssuer`, ignores the proxy CA bundle) and had to be seeded by curl into `/root/.local/share/.dx/tools/`.
- Release SSG builds ~11–13 min warm; parallel lanes sharing one target dir hit "Text file busy" and stale cached artifacts (a mirror tree reused the main tree's compiled primitives).
- Disk filled twice (5 lane target dirs ≈ 18 GB); `deploy-preview.sh` hard-codes `$repo_root/target/...`, which broke when `target` wasn't a plain symlink.
- `main-thread.spec.ts` timing oracles need release builds (debug gives false reds).

**Plan (measure before/after each step):**
1. Cheap profile for the SSG *server* binary (never shipped): no LTO, codegen-units 16, low opt-level; wasm keeps `opt-level=z` + LTO. Verify which profile name dioxus-cli 0.7.9 uses for the server build.
2. Verification lanes build debug SSG; release only for deploy + one pre-merge run (+ the timing specs).
3. Warm, base-path-free target template copied per lane; prune stale target dirs; make `deploy-preview.sh` honor `CARGO_TARGET_DIR`.
4. `lld` as host linker.
5. SessionStart hook (use the `session-start-hook` skill): install dx 0.7.9 (or prebuilt binary), wasm32 target, seed dx tools, `npm ci` for playwright + stylelint (preview has no lockfile — add one), `cargo fetch`.
6. Loop hygiene: one full suite pre-merge; shorter per-test timeout.

**Done when:** measured before/after table for each step, deploy artifact byte-identical in behavior (all gates + 373-spec targeted run green), CLAUDE.md updated.

---

## Thread 2 — Theme migration to shadcn's system (backlog row 111) — after Thread 1

**Goal:** semantic role tokens (shadcn names: background, foreground, card, popover, primary(-foreground), secondary, muted, accent, destructive, border, input, ring, chart-1..5, sidebar-*, radius), then the Nova style.

**Read first:** `dev-docs/backlog.md` row 111, `dev-docs/research/theme-2026-09-26/theme-vs-base-nova.md`, `preview/assets/dx-components-theme.css`, `playwright/computed-style-snapshot.spec.ts`.

**Facts:** ~424 ramp references (`--primary-color-*`, `--secondary-color-*`) in 64/72 component stylesheets, used inconsistently ("border" = 5 different steps). Keep the space-toggle light/dark mechanism and plain CSS.

**Plan:** (A) `--dx-<role>` tokens with shadcn-name fallback (`--dx-border: var(--border, <ramp step>)`), repoint sites by intended role, proven value-preserving by an empty computed-style snapshot diff; keep ramp names as aliases. Global `box-sizing: border-box` belongs here (two instances already). (B) Nova foundations: radius swap, then card/button/input/badge/tabs/dialog/select shadow + density. (C) remaining components, axe colour-contrast re-run.

**Chart tie-in:** the one deliberate shadcn difference left in the charts gallery is the palette (ours multi-hue `--dx-chart-1..8`, shadcn's /charts uses a blue ramp). With role tokens the charts page can adopt shadcn's chart theme as a token-level choice; the chart parity tests (`preview/src/{chart,polar,chart_tooltip}_parity.rs`) must stay green.
