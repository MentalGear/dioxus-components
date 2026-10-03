# Chart parity with shadcn / Recharts — 2026-10-03 snapshot

Research behind PRs #79, #80 and #81 (`dev-docs/backlog.md` rows 117-119): how the Chart component and its `/charts` gallery differed from `ui.shadcn.com/charts` (new-york-v4, Recharts 3.8.0), the root cause of each difference, and what was built to close it. **A point-in-time snapshot** (measured 2026-10-03, main @ `8155cc6` for "ours before", `62de3cb` for "ours after"); re-measure before relying on a number.

## Files

| File | What it is |
|---|---|
| [`shadcn-ux-spec.md`](./shadcn-ux-spec.md) | Measured spec of shadcn's `/charts` gallery UX: IA, hero, toolbar, Copy / View Code sheet and drawer, tooltip rules (offsets, flip, motion, per-chart-type anchoring), cursor/active-dot conventions, keyboard, tokens. Input to PR #80. |
| [`ours-audit.md`](./ours-audit.md) | Audit of our chart docs and tooltip *before* PR #80: file/line anchors, build pipeline notes, the old tooltip's behavior (fixed anchor per category), the five divergent demo footer patterns. |
| [`parity-A.md`](./parity-A.md) | Area / line / bar / tooltip charts vs shadcn: fabricated interactive datasets, `getNiceTickValues`, curves (natural/step), bar sizing, with a root-cause class per finding (P port error, E engine semantics, D deliberate). Proposed construction: vendored registry fixtures + geometry tests. Input to PR #81. |
| [`parity-B.md`](./parity-B.md) | Pie / radar / radial vs shadcn: Recharts angle convention (degrees, 0 = 3 o'clock, counter-clockwise), radius semantics, radial sweep, tracks, grid, active slice, outside labels. Input to PR #81. |
| [`sizing-contract.md`](./sizing-contract.md) | The agreed sizing contract (1 unit = 1 CSS px; 16:9 cartesian, <=250px square polar; `width`/`height`/`aspect`/`margin`) that the implementation and the demo lane both coded against. |
| [`shadcn-vs-ours.md`](./shadcn-vs-ours.md) | Mid-flight UX side-by-side (gallery IA, hero, tooltip, interactions) with a matches / deliberate difference / gap verdict per row. |
| [`visual-parity.md`](./visual-parity.md) | Final visual comparison on the finished build (1440 light and dark, 390 phone) and the behavior checks. Source of the remaining-differences list in `backlog.md` rows 117-119 and 120-128. |

## Method

- **shadcn side:** the registry JSON / TSX for every chart in scope (39 cartesian + tooltip, plus the polar set; the demos and fixtures now live in `preview/tests/shadcn/`), and the *rendered* DOM of the live site: SVG path `d`, gridlines, bar rects, arc geometry, hover traces. Chromium could not load `ui.shadcn.com` assets directly through the sandbox proxy (`ERR_TOO_MANY_RETRIES`), so every request was routed through `curl` (Playwright request routing) — this only changes how pages were fetched, not what the site does.
- **Our side:** a locally served static SSG build of `docs/`, probed with the same scripts, plus Playwright/Chromium for behavior.
- **Oracles:** independent JS re-implementations of d3 `curveMonotoneX` / `curveNatural` and Recharts `getNiceTickValues`, validated against shadcn's rendered paths and grids (max error 0.002 px, exact tick match) before being used as references.
- **Result:** the by-construction guard is the committed fixtures plus `preview/src/{chart,polar,chart_tooltip}_parity.rs` (see `scripts/extract-shadcn-*fixtures.mjs`), not these reports.

## What is not here

The screenshots, JSON probe dumps and probe scripts these reports cite by file name were scratch artifacts of the session and were deliberately not committed (binary images; reproducible from the method above). The reports keep every finding and number.
