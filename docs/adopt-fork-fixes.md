# Adopting functional fixes from sibling forks

Two community forks of this component library carry changes that were never upstreamed. This is a task brief
for mining their commits and adopting the **functional fixes** here, shaped as clean, upstream-ready commits.

## Goal

Adopt functional fixes (correctness, accessibility, behavior, new primitives, performance — **including CSS
that fixes a real bug**). **Do not** adopt pure visual restyles (redesigns to a different look).

## Forks to mine

```sh
git remote add fork-dq https://github.com/dignifiedquire/dx-components
git remote add fork-sr https://github.com/sarendipitee/dioxus-components
git fetch fork-dq && git fetch fork-sr
```

## Baseline

This repo's `main` is the baseline (the current upstream point). Isolate each fork's own commits by diffing
its default branch against the merge-base with `main`:

```sh
git log --oneline $(git merge-base main fork-dq/main)..fork-dq/main
git log --oneline $(git merge-base main fork-sr/main)..fork-sr/main
```

## Already done — do not re-derive

Branch `fix/preview-a11y-ux` already carries three `preview/` CSS fixes:
- input/textarea **16px font-size floor under `pointer: coarse`** (prevents iOS Safari focus auto-zoom),
- accordion animation switched to **`grid-template-rows: 0fr ↔ 1fr` with `transition … ease`** (was broken keyframes),
- dropdown/select overlay **`max-width: calc(100vw - 2rem)`** (prevents small-screen horizontal overflow).

## Classification — read the diff, not the commit message

- **TAKE (functional):** correctness/bug fixes; accessibility (ARIA, roles, keyboard, focus management);
  behavior/interaction; new primitives/components; performance; and CSS that fixes a bug (overflow, animation
  jank, focus zoom, layout breakage).
- **SKIP (restyle):** visual redesign — color / spacing / token / typography changes that only alter
  appearance.
- **FLAG:** ambiguous; a commit that mixes a fix with a restyle (extract only the functional delta); or a
  change already present in `main` / already upstreamed.

## Process

1. For each fork, list the divergent commits (above) and read each with `git show <sha>`.
2. Classify each. For TAKE candidates, confirm the fix still applies to `main` and isn't already present.
3. Produce a **categorized + batched table** — columns: `commit · fork · category · what it fixes · take? ·
   batch`. Categories: a11y / correctness / behavior / new-component / perf / tooling. Batch by independence
   and component, low-risk first. Save it as `docs/adopt-fork-fixes-results.md`.
4. After the table is reviewed, apply **batch by batch** — each batch its own commit (one fix per commit
   where practical) — validating with the repo's own checks (`stylelint`, `cargo check`/`clippy`/`fmt`,
   Playwright where feasible) **before** committing.

## Constraints

- **Keep every commit neutral and upstream-ready.** This is a public library; commit messages describe only
  the fix. Each adopted fix should be shaped so it can be submitted to the true upstream
  (`DioxusLabs/dioxus-components`).
- Extract **only the functional delta** from a commit that mixes a fix with a restyle.
- Don't widen scope; keep the fork thin over upstream and upstream fixes promptly.

## Output

The results table (`docs/adopt-fork-fixes-results.md`), then the batched commits / PRs.
