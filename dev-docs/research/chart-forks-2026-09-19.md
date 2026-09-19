# Chart — fork network & wider Dioxus chart landscape research (chart-forks lane, 2026-09-19)

Copied verbatim from the chart-forks research lane's own report; committed by the integration lane as part of the Chart round's docs pass, per `dev-docs/backlog.md` row 88 and `component-backlog.md`'s Chart row.

# Chart -- fork network & wider Dioxus chart landscape research (lane: chart-forks)

**Role:** RESEARCH only, no repo edits. **Date:** 2026-09-19. **Base:** `main` = `0ae53fc`.
**User ask, verbatim:** "check the other Dioxus components forks whether they have charts and what we can use or learn from them."
**Reused as read:** `dev-docs/research/chart-2026-09-19.md` (the approved research -- shadcn `chart.tsx`, recharts `accessibilityLayer`, `dioxus-charts` 0.4.0, `dignifiedquire/dx-components`, `charts-rs`, `plotters`, `poloto`, `charming`, `plotly` -- all already read in full there; **not re-derived here**, only cited, per the task's own instruction to read it as already-approved). This report's job is everything that report did **not** cover: the actual DioxusLabs/components fork network, the wider Dioxus ecosystem beyond the crates that report picked, the two named sibling-framework libraries (leptos-chartistry, yew-chart), and the canonical d3-shape/d3-array math the primitive lane's `scale.rs` is being written against.

Every claim below is cited to a file path + git SHA this session actually read or a command this session actually ran. `$S` = `/tmp/claude-0/-home-user-dioxus-components/0c0a1bde-8dc9-51c3-91e6-e93c0c00fb9e/scratchpad`; new clones went to `$S/refs/` alongside the ones already there (`ui`, `primitives`, `recharts`, `dioxus-charts`, `dx-components`, `embla-carousel`, `charts-rs`, `plotters`, `poloto-project`, `charming`, `plotly.rs`).

---

## 1. Roster -- every fork/crate checked, this lane and the prior one combined

| Name | Has charts? | Rendering approach | Licence | Last commit | Verdict |
|---|---|---|---|---|---|
| shadcn/ui `chart.tsx` | yes (source of the family) | React children-as-config over Recharts | MIT | 2026-09-17 (`a87a63b2`) | Not portable as one API (prior report 1.4) -- cited, not re-read |
| recharts | yes (engine shadcn wraps) | React/SVG + `accessibilityLayer` | MIT | 2026-09-18 (`86ad3632`) | a11y/behaviour oracle source -- cited, not re-read |
| `dioxus-community/dioxus-charts` 0.4.0 | yes (bar/line/pie only) | Real RSX `<line>`/`<text>`/`<g>` | MIT OR Apache-2.0 | 2026-04-05 (`01aadb2`) | Right shape, zero a11y/theming, no area/radar/radial -- cited, not re-read |
| `dignifiedquire/dx-components` | **zero chart files** | -- | MIT OR Apache-2.0 | 2026-08-21 (`5af3cc2`) | Confirmed absent -- cited, not re-read |
| `vicanso/charts-rs`, `plotters-rs/plotters`, `tiby312/poloto-project` | yes | Opaque SVG **string** output | Apache-2.0 / MIT / MIT | see prior report | Needs `dangerous_inner_html`, no real DOM -- cited, not re-read |
| `yuankunzhang/charming`, `plotly/plotly.rs` | yes | Wraps a JS engine (ECharts/Plotly.js) | MIT/Apache-2.0 dual, MIT | see prior report | Ships/loads a JS bundle -- disqualified, cited not re-read |
| **26 forks of `DioxusLabs/components`** (roster in section 2) | **zero, all 26** | -- | -- | various, several years stale | **New this lane** -- confirmed by GitHub code search, not assumed |
| `rust-ui/dioxus-ui` | **yes -- all 6 families** (area x11, bar, line, pie, radar, radial) | Empty `<div data-name>` + **externally-loaded ApexCharts JS** | MIT | 2026-09-10 (`95798e6`) | **New this lane** -- architecture disqualified (JS engine, no SSR content); variant catalogue useful as a checklist only |
| `DorianPinaud/plotters-dioxus` | yes (generic, via `plotters`) | **Raster PNG -> base64 `<img>`** | MIT | 2024-02-26 (`493d74f`), Dioxus 0.3-era API | **New this lane** -- stale, API-incompatible, a third "opaque output" failure mode |
| `feral-dot-io/leptos-chartistry` | yes (line/bar/stack, full axis/tick/legend/tooltip machinery) | Real Leptos view / SVG, computed in Rust | **MPL-2.0** | 2026-01-23 (`a98bae1`), active | **New this lane** -- best-engineered sibling; re-derive clean-room only (licence), several concrete learn/use points below |
| `titanclass/yew-chart` | yes (line/scatter/bar/radar via composable primitives) | Real Yew view / SVG | Apache-2.0 | 2024-09-27 (`494a50a`), quiet ~2y | **New this lane** -- simplest sibling; licence-compatible, thin |
| `MBeliou/shadcn-dioxus` | **no component**, only shadcn's `--chart-N` CSS tokens | -- | -- | -- | **New this lane** -- checked, nothing to lift |
| `AurimarL/rustcn-ui`, `MadsLudvig/shadcn-dioxus` | no | -- | -- | -- | **New this lane** -- checked, nothing |
| `d3/d3-shape`, `d3/d3-array` | n/a (drawing-math library, not a chart) | -- | **ISC** | 2023-10-06 (both) -- mature/stable, not abandoned | **New this lane** -- the canonical reference, section 4 |

---

## 2. The actual DioxusLabs/components fork network -- checked exhaustively, not sampled

Two independent methods, both confirming the same result:

1. **`WebFetch https://github.com/DioxusLabs/components/forks`** returned the full roster GitHub shows (25 forks): `sarendipitee/dioxus-components`, `aaelony/dioxus-components`, `MichiBab/components`, `molikto/dioxus-components`, `nickelser/dioxus-components`, `npatsakula/components`, `oscar370/components`, `p-jackson/dioxus-labs-components`, `paxwort/dioxus-components`, `PupsieCo/components`, `QazCetelic/components`, `Raflos10/components`, `ryo33/components`, `sagikazarmark/dioxus-components`, `SFSeeger/dioxus-components`, `SPRAGE/components`, `sumitpsm/dioxus-components`, `timothebot/components`, `Torvex-UG/dioxus-components`, `Traverse-Research/components`, `TrildaDevCenter-CrossPlatformDev/components`, `Tumypmyp/components`, `tv42/dioxus-components`, `Wervice/dioxus-components`, `WhaleFromMars/components`. (`https://api.github.com/repos/DioxusLabs/components/forks?per_page=100` returned HTTP 403 through the proxy, as the brief predicted -- the HTML page worked.) Added `jcgruenhage/dioxus-components` (26th) from this repo's own `dev-docs/lifting-from-forks.md`/`adopt-fork-fixes-results.md`, a known contributor fork not shown on that page (likely older, off the default sort).
2. **`mcp__github__search_code`, one `repo:<owner>/<repo> chart` query per repo, all 26, this session, 2026-09-19.** Every single query returned `"total_count":0`. Sample (all 26 gave the identical shape of result):
   ```json
   {"incomplete_results":false,"items":[],"total_count":0}
   ```
   This is a stronger and cheaper method than cloning each fork: GitHub's code index, not a local grep, so it also catches any branch/path this session didn't think to look for by name.

**Finding: none of the 26 forks of `DioxusLabs/components` contain the word "chart" anywhere in their code, let alone a chart component.** This extends the prior report's single-fork finding (`dignifiedquire/dx-components`, zero chart files, confirmed by directory listing) to the *entire* known fork network, confirmed by a different, broader method (GitHub's own code search index vs a local `find`). The prior report's line "no adoptable Dioxus port exists anywhere in the fork network this repo's own backlog research has already surveyed" is now verified for the whole network, not just the one fork that research happened to clone.

---

## 3. Wider Dioxus landscape -- deep reads

### 3a. `rust-ui/dioxus-ui` -- full shadcn chart family, but it is an ApexCharts wrapper wearing Dioxus RSX

Cloned to `$S/refs/rust-ui-dioxus-ui`, `95798e61325775372b78f668933263946aaad06a` (2026-09-10). MIT (`LICENSE`, Max Wells). This is the repo behind `rust-ui.com`, and its docs site literally advertises "Beautiful Dioxus Rust UI chart components" for **Area, Bar, Line, Pie, Radar, Radial** -- the complete shadcn family, plus 11 numbered Area variants (`area_chart_01.rs`...`area_chart_11.rs` + a placeholder) matching shadcn's own gradient/stacked/step/legend demo spread. This is a real, functioning registry, not a mockup -- but reading the actual `.rs` files shows why it's not a reference for *how to draw a chart*:

`app_crates/registry/src/charts/bar_chart_01.rs` (49 lines, read in full) -- the entire component body:
```rust
rsx! {
    section { class: "flex flex-col gap-9 px-6 mx-auto w-full max-w-4xl rounded-lg border border-border bg-card",
        p { class: "mt-4 text-xs font-bold text-card-foreground", "Overview" }
        div {
            id: "barChart01",
            class: "w-full h-[400px]",
            "data-name": "BarChart",
            "data-chart-values": data_json,
            "data-chart-labels": labels_json,
        }
    }
}
```
That `div` is **empty** -- no SVG, no marks, nothing Dioxus renders. `line_chart_01.rs`, `pie_chart_01.rs`, `radar_chart_01.rs`, `radial_chart_01.rs` (each read in full, 41-49 lines) are byte-for-byte the same shape, only the `id`/`data-name` differ. All the actual drawing lives in `public/app_components/chart_init.js` (534 lines) plus a vendored `public/cdn/apexcharts.5.3.6.min.js`. Its own header comment, read verbatim:
```
1: // ApexCharts initializer for Leptos SPA
...
7:  if (window.__chartInitDone) return;
22:  function loadApexCharts() {
23:    if (typeof ApexCharts !== "undefined" || apexChartsLoading) return Promise.resolve();
102:    if (options && typeof ApexCharts !== "undefined") {
103:      const chart = new ApexCharts(container, options);
264:  const observer = new MutationObserver(() => {
```
Two findings: (1) it dynamically loads ApexCharts from a CDN if not already present, then constructs `new ApexCharts(container, options)` and lets a `MutationObserver` re-init charts after SPA navigation -- the same category of architecture this repo's own approved research already disqualified `charming`/`plotly` for (ships/loads a JS charting engine), just with the JS loaded lazily at runtime instead of bundled; the empty `<div>` also means **no server-rendered content and no fallback for JS-disabled/pre-hydration**, the opposite of this repo's hydration-parity requirement. (2) the comment says **"for Leptos SPA"**, in a repo whose whole purpose is a *Dioxus* registry -- direct evidence this chart layer is an unmodified mechanical mirror of `rust-ui/ui` (the sibling Leptos registry), not Dioxus-idiomatic code at all, despite living under `rust-ui/dioxus-ui`. Either way it proves the Dioxus port never adapted the *mechanism*, only the component syntax.

The one thing worth taking (a checklist, not code): `app_crates/registry/src/ui/charts.rs`'s `AreaChart` props -- `curve: ChartCurve {Smooth, Straight, Stepline}`, `stack_type: Option<String>`, `gradient: Option<bool>`, `show_yaxis`, `show_grid`, `json_annotations` -- independently converges on almost the same prop surface our `chart-api.md` contract already specifies (`curve: Curve`, `stacked: bool`, `show_grid`, `show_y_axis`), a weak positive signal the contract's shape is the natural one, nothing to change because of it.

### 3b. `DorianPinaud/plotters-dioxus` -- a third "opaque output" failure mode: raster image, not string-SVG

Found via `mcp__github__search_code query:"plotters-dioxus filename:Cargo.toml"` (the crates.io/lib.rs pages 403'd through the proxy). Cloned to `$S/refs/plotters-dioxus`, `493d74fd1d2fd1a1a6f7d03c4d630d7fd9a3f33a` (2024-02-26 -- **~2.5 years stale as of today**). MIT (`LICENSE`, Dorian Pinaud). 264 lines total across the whole crate -- read in full.

`plotters-dioxus/src/plotter.rs`'s `Plotters` component renders a `plotters::BitMapBackend` into an in-memory RGB buffer, PNG-encodes it, and emits:
```rust
render!(img {
    onclick: |e| { ... },
    ...
    src: "data:image/png;base64,{buffer_base64}",
})
```
This is a **raster bitmap embedded as a base64 data URI `<img>`**, not SVG at all -- a third architecture beyond the "opaque SVG string" class the prior report already covered for `charts-rs`/`plotters-svg`/`poloto`, and strictly worse for this repo's needs: an `<img>` has no internal structure whatsoever (not even the string-SVG candidates' unreachable-but-present `<path>`/`<circle>` elements), so hover/click coordinates can only ever be raw pixel offsets on the image. The demo (`demo/src/main.rs`, 103 lines, read in full) proves this is a real, working technique, not a strawman: it calls `scatter_ctx.as_coord_spec().reverse_translate((click_coord.x as i32, click_coord.y as i32))` to turn a click's pixel position back into a data-space coordinate, then redraws a crosshair line through it. That is coordinate-math-driven hover in its purest, most manual form -- every other interactive candidate surveyed in either report needs either this technique or a real DOM element per mark; there is no third option once the substrate is opaque. Also notable: the crate's API is written against pre-0.4 Dioxus (`Scope<'a, Props>`, `cx.props`, a `render!` macro, lifetime-parameterised `#[derive(Props)]`) -- it would not compile against this repo's Dioxus 0.7 without a full rewrite, on top of the architecture mismatch. Not adoptable on any axis; useful only as confirmation that "opaque output implies no free interactivity/a11y" is a property of the *rendering substrate*, not of any one crate or format.

### 3c. `feral-dot-io/leptos-chartistry` -- the best-engineered sibling, MPL-2.0

Cloned to `$S/refs/leptos-chartistry`, `a98bae1cb4371184d5e2ae26f2c769bcd45643c6` (2026-01-23, actively maintained). **Licence: Mozilla Public License 2.0** (`LICENSE.txt`, verbatim header "Mozilla Public License Version 2.0"). Small dependency footprint: `chrono`, `leptos`, `leptos-use` (wraps `ResizeObserver`/mouse-position browser APIs), `web-sys` -- a real Leptos component library, not a wrapper over a JS chart engine, with genuine axis/tick/legend/tooltip/stacking machinery (`leptos-chartistry/src/{ticks,series,layout,overlay,colours}/`).

**Licence nuance worth flagging explicitly, since the task's own MIT/Apache/BSD-ok-GPL-not framing doesn't have a slot for it:** MPL-2.0 is file-level weak copyleft, not GPL, but it is *not* simply "OK with attribution" for a crate declared `license = "MIT OR Apache-2.0"` (this repo's own declared terms, confirmed in `lifting-from-forks.md` section 1 for every other fork checked so far). Copying an MPL-2.0 file's actual content into this codebase would put that content under MPL-2.0 obligations (source availability for that file, licence notice) that this repo's own Cargo.toml doesn't carry and presumably doesn't want to add for one file. **Verdict: everything below is a clean-room re-derivation candidate -- read for the algorithm/shape, reimplement independently, cite in the commit message -- never copy-paste, even with attribution.** This is a stricter bar than every other source in this report.

**a11y: zero, confirmed by grep, not assumed.** `grep -rniE "aria|role=|tabindex" leptos-chartistry/src` gives 6 hits, every one a false positive on the substring ("vari*a*ble", "Two-*Varia*ble Graph"). Zero real ARIA/role/tabindex/keyboard anywhere in ~50 source files of an otherwise sophisticated, actively-maintained chart library. This matters as a **cross-library confirmation**, not just a data point: `dioxus-charts` (prior report) had zero a11y too; so does `yew-chart` (section 3d); so does `rust-ui/dioxus-ui`'s ApexCharts wrapper (ApexCharts itself has no first-class a11y story either, though this session didn't independently verify that claim about ApexCharts specifically). The pattern across every non-JS-ecosystem chart library surveyed across both reports is the same: **no a11y at all is the norm, not the exception** -- this repo's own `role="img"` + hidden `<table>` + optional keyboard-crosshair contract (already decided, `chart-2026-09-19.md` section 2.5/6.4) is ahead of the field, not behind it. Validates the existing decision; argues for no change.

**Tick generation (`ticks/gen/aligned_floats.rs`, 310 lines, read in full) is a genuinely different algorithm from d3's, and the difference is a usable idea, not just a contrast.** It does not use d3's magnitude/1-2-5-step approach at all. Instead (`find_precision`, `generate_count`): it determines a decimal `scale` via `log10`, estimates how many ticks of that precision's *rendered label width* fit in the available pixel `span` (via a `Span` trait that measures consumed width), and only then evenly divides the range into that many steps -- i.e., **tick count is derived from estimated label width vs. available space**, not chosen from a fixed candidate set of "nice" step multiples. This is a real, working answer to exactly the "tick-label collision/thinning algorithm" the task asked about, and it's a technique, not code under a restrictive licence -- safe to reuse as a *design idea* regardless of MPL-2.0. See section 6 recommendation.

**Monotone cubic interpolation (`series/line/interpolation.rs`, 183 lines, read in full) cites the same paper as d3-shape, but a materially different formula -- see section 4, the single most load-bearing finding in this report.**

**Stacking (`series/stack.rs`, 134 lines, read in full)** is architecturally different from our contract's batch pure function: each `Line` in a `Stack` carries a `previous: Vec<Arc<dyn GetYValue>>` and sums itself plus every prior line *per datum, on access* (`stacked_value`), rather than precomputing `(y0, y1)` pairs for a whole `Vec<Vec<Option<f64>>>` up front the way `chart-api.md`'s `stack()` does. Reactive-accessor-per-point vs. batch-precompute is a real design fork; our contract's batch shape is simpler to unit-test in isolation (matches `slider.rs`'s existing pure-fn convention) and is not worth changing on the strength of this one counter-example.

**`use_watched_node.rs` (105 lines, read in full) documents a real, concrete resize pitfall in its own code comment**, quoted verbatim:
```rust
// Outer chart bounds -- dimensions for our root element inside the document
// Note <svg> has issues around observing size changes. So wrap in a <div>
// Note also that the box_ option doesn't seem to work for us so wrap in another <div>
let (bounds, set_bounds) = signal::<Option<Bounds>>(None);
use_resize_observer_with_options(node, ...)
```
This is a **direct validation of this repo's own contract choice**, not a gap to fix: our `Chart` uses a fixed logical `viewBox` scaled by CSS (`width:100%; height:auto`), so it never needs a `ResizeObserver` on an `<svg>` at all -- the exact bug class this comment is warning about is unreachable by construction, not avoided by discipline. Worth keeping in the report as evidence for *why* the contract's approach is right, in case anyone is tempted to add JS-measured responsive sizing later.

**Tooltip positioning (`overlay/tooltip.rs`, 327 lines) tracks the live mouse pixel position continuously** via `use_watched_node`'s `mouse_page` signal and a reactive CSS `calc()` (`style:right = move || format!("calc(100% - {}px + {}px)", state.mouse_page.get().0, cursor_distance.get())`), i.e. free-hover-anywhere positioning, not index-snapped. Its background colour is a **hardcoded literal**, quoted verbatim: `style="position: absolute; ...; background-color: #fff; ..."` -- no CSS variable, no dark-mode counterpart anywhere in the file. This is a concrete, citable "don't do this" for our contract's own `--dx-chart-*`/theme-token-driven tooltip CSS (already the plan) -- worth keeping as evidence, not a reason to change anything already decided.

**Theming (`colours/scheme.rs`, 321 lines) computes colours entirely in Rust** (`ColourScheme::by_index`/`interpolate`, gradient stop generation for SVG `<linearGradient>`) with no CSS custom properties anywhere in the file. Changing the palette means a Rust-side recompute/re-render, not a CSS cascade change. Contrasts directly with, and validates, this repo's `var(--dx-chart-N)`/`var(--color-<key>)` design (`chart-api.md`'s whole point is that a dark-mode toggle needs zero Rust re-render).

### 3d. `titanclass/yew-chart` -- the simplest sibling

Cloned to `$S/refs/yew-chart`, `494a50a4245af4712936167b680a0ecd806b79be` (2024-09-27, quiet ~2 years but not abandoned-looking -- a stable, low-churn "toolkit of composable SVG primitives" rather than a pre-built-chart library). **Licence: Apache-2.0** (`Cargo.toml` `license = "Apache-2.0"`, `LICENSE` verbatim Apache header) -- compatible with this repo's MIT OR Apache-2.0 terms, verbatim-copy-with-attribution is legally fine here (not that there's much worth copying -- see below).

`src/linear_axis_scale.rs` (237 lines, read in full) is deliberately minimal: `LinearScale::new(range: Range<f32>, step: f32)` takes an **explicit, caller-supplied step** -- there is no "nice tick" algorithm anywhere in this file or crate (confirmed by reading the whole file; no `log10`/magnitude logic exists). `ticks()` returns `Vec<Tick { location: NormalisedValue, label }>` where `NormalisedValue` is a plain `0.0..1.0` fraction, not a pixel coordinate -- normalisation and pixel-mapping are two separate steps, a small, clean decoupling worth noting (easier to unit-test the math independent of any concrete viewBox size) but not adopted since it doesn't change our contract's own `LinearScale::scale`/`ticks` shape materially.

Tooltips (`src/series.rs`, grepped + spot-read) are the simplest possible mechanism in the whole survey: a native SVG `<title>` child per mark plus an `onmouseover` callback:
```rust
<circle ... onmouseover={onmouseover(&props.onmouseover, title)}/>
<title>{tt(data_x, data_y1)}</title>
```
Zero custom positioning math, zero floating overlay, but also zero styling control and the browser's native (slow, non-uniform) hover-tooltip delay -- not a fit for shadcn-parity visuals, useful only as the "free baseline" contrast to our contract's own `ChartTooltip` overlay. Zero ARIA/role/tabindex anywhere either (grep, 0 hits) -- same universal-no-a11y pattern as every other library in this report.

### 3e. Dioxus "shadcn port" projects -- checked, nothing to lift

`mcp__github__search_code repo:MBeliou/shadcn-dioxus chart` gives 3 hits, all CSS/docs (`packages/web/tailwind.css`, `packages/web/assets/tailwind.css`, `packages/web/content/theming.md`) -- this is shadcn's own `--chart-1..5` theme-colour tokens carried over as boilerplate, **not a Chart component**. `mcp__github__search_code repo:AurimarL/rustcn-ui chart` and `repo:MadsLudvig/shadcn-dioxus chart` give 0 hits each. None of the three Dioxus "shadcn port" projects surfaced by web search have a Chart component.

---

## 4. The canonical math -- d3-shape / d3-array, and what recharts actually calls

Cloned `d3/d3-shape` (`a82254af78f08799c71d7ab25df557c4872a3c51`, 2023-10-06) and `d3/d3-array` (`be0ae0d2b36ab91b833294ad2cfc5d5905acbd0f`, 2023-10-06) -- both mature/stable (not abandoned; d3-shape/d3-array are finished, low-churn foundational libraries, the same "quiet but not a red flag" reading this repo's own research already gives `plotters`). **Licence: ISC** for both (`package.json`: `"license": "ISC"`; `LICENSE`: "Copyright 2010-2022/2023 Mike Bostock / Permission to use, copy, modify, and/or distribute this software for any purpose with or without fee is hereby granted, provided that the above copyright notice and this permission notice appear in all copies.") -- ISC is in the same permissive family as MIT/BSD for our purposes: safe to port with attribution.

**Confirmed recharts calls these directly, not a reimplementation:** `recharts/src/shape/Curve.tsx` (this session's own read, this lane):
```ts
import {
  ..., curveMonotoneX, curveMonotoneY, curveNatural,
  curveStep, curveStepAfter, curveStepBefore, ...
} from 'victory-vendor/d3-shape';
```
`victory-vendor/d3-shape` is a republished/vendored copy of the real d3-shape package (maintained for the Victory chart ecosystem), not a rewrite -- so shadcn -> recharts -> d3-shape's actual Bostock code is a straight, unbroken chain. Also: recharts imports **`curveStepAfter`** specifically among the three step variants d3 offers, which independently confirms `chart-api.md`'s own choice of "Step = step-after" as the recharts-matching one, not an arbitrary pick.

### 4.1 Monotone cubic -- `d3-shape/src/curve/monotone.js`, and a real discrepancy with leptos-chartistry

Read in full (105 lines). The tangent-slope function, with its own citation:
```js
// Calculate the slopes of the tangents (Hermite-type interpolation) based on
// the following paper: Steffen, M. 1990. A Simple Method for Monotonic
// Interpolation in One Dimension. Astronomy and Astrophysics, Vol. 239, NO.
// NOV(II), P. 443, 1990.
function slope3(that, x2, y2) {
  var h0 = that._x1 - that._x0, h1 = x2 - that._x1,
      s0 = (that._y1 - that._y0) / (h0 || h1 < 0 && -0),
      s1 = (y2 - that._y1) / (h1 || h0 < 0 && -0),
      p = (s0 * h1 + s1 * h0) / (h0 + h1);
  return (sign(s0) + sign(s1)) * Math.min(Math.abs(s0), Math.abs(s1), 0.5 * Math.abs(p)) || 0;
}
```
Then a cubic Bezier is drawn through Hermite-to-Bezier control points (`point()`, `dx = (x1-x0)/3`, control points at `x0+dx, y0+dx*t0` / `x1-dx, y1-dx*t1`).

**This is the same paper leptos-chartistry cites** (section 3c), but its Rust transcription (`series/line/interpolation.rs::tangent`, quoted in full earlier) computes:
```rust
(slope_prev.signum() + slope.signum()) * slope_prev.abs().min(0.5 * para.abs())
```
-- a **two-way** `min(|s0|, 0.5|p|)`, missing d3-shape's third clamp term `Math.abs(s1)`. This is not a stylistic difference: d3's three-way min is what gives Steffen's method its monotonicity guarantee in general (clamping against *both* neighbouring slopes, not just the incoming one); dropping the `|s1|` term can let the tangent overshoot near a local extremum in cases the three-way version would catch. **This is the single most load-bearing, non-obvious finding in this report** -- anyone reading leptos-chartistry "for shape" (as its MPL-2.0 licence requires anyway) and transcribing its formula would silently produce curves that are *not* bit-for-bit what shadcn/recharts draws, even though both call themselves "monotone" and cite the same 1990 paper. Flagged for the primitive lane in `chart-lanes.md`, not written into the contract (the contract already correctly says "Monotone = monotone cubic (recharts 'monotone')" -- this is an implementation-precision note for whoever writes `scale.rs`'s curve math, not a contract change).

### 4.2 Step -- `d3-shape/src/curve/step.js`, read in full (54 lines)

One `Step(context, t)` implementation parameterised by `t` in `{0, 0.5, 1}`, exported three ways:
```js
export default function(context) { return new Step(context, 0.5); }      // curveStep
export function stepBefore(context) { return new Step(context, 0); }      // curveStepBefore
export function stepAfter(context) { return new Step(context, 1); }       // curveStepAfter -- what recharts imports
```
The corner position is `x1 = this._x*(1-t) + x*t`, then two line segments (`lineTo(x1, prevY)`, `lineTo(x1, newY)`) -- a single formula covering all three variants. `chart-api.md` commits to exactly one (`Step = step-after`), matching recharts' own choice -- reasonable MVP scope; the `t`-parameterised family is a natural, cheap future-extension point if `step`/`step-before` are ever wanted (leptos-chartistry independently exposes four named variants -- `Horizontal`/`HorizontalMiddle`/`Vertical`/`VerticalMiddle` -- over the same design space, corroborating that this is the natural generalisation, not a d3-specific quirk).

### 4.3 Ticks -- `d3-array/src/ticks.js`, read in full (56 lines) -- THE reference for `scale.rs`'s `LinearScale::ticks`/`nice_domain`

`ticks`, `tickIncrement`, and `tickStep` all live in this one file (not split across separate files, correcting an assumption the task's own phrasing could suggest):
```js
const e10 = Math.sqrt(50), e5 = Math.sqrt(10), e2 = Math.sqrt(2);
function tickSpec(start, stop, count) {
  const step = (stop - start) / Math.max(0, count),
      power = Math.floor(Math.log10(step)),
      error = step / Math.pow(10, power),
      factor = error >= e10 ? 10 : error >= e5 ? 5 : error >= e2 ? 2 : 1;
  ...
}
export default function ticks(start, stop, count) { ... }
export function tickIncrement(start, stop, count) { ... }
export function tickStep(start, stop, count) { ... }
```
`e10`/`e5`/`e2` are the geometric-mean cutoffs between consecutive members of the 1, 2, 5, 10, 20, 50, 100... sequence (`sqrt(1*2) is about 1.41`, `sqrt(2*5) is about 3.16`, `sqrt(5*10) is about 7.07`) -- this is the precise, canonical algorithm `chart-api.md`'s own comment ("d3 'nice' ticks: 1/2/5 steps") already names, now with exact arithmetic to port. This is a *completely different algorithm* from leptos-chartistry's label-width-driven approach (section 3c) -- both are legitimate, but they answer different questions (d3: "what step size looks clean at this magnitude", independent of pixel space; leptos-chartistry: "how many ticks of this precision actually fit"). `chart-api.md` wants the first; the second is worth layering on top as an optional thinning pass (section 6), not a replacement.

---

## 5. USE -- licence-permitting, worth porting (algorithm, not verbatim text -- all three are JS to Rust)

1. **`d3-array/src/ticks.js`'s `ticks`/`tickIncrement`/`tickStep`** (ISC, Mike Bostock, `d3/d3-array@be0ae0d2`). Port the exact `tickSpec` arithmetic (the `e10`/`e5`/`e2` factor selection) into `scale.rs`'s `LinearScale::ticks`/`nice_domain`. Not a verbatim file copy (different language), but a faithful line-by-line algorithm port -- ISC's terms and this repo's own `lifting-from-forks.md` section 1 doctrine both treat that as clean, attribution in the commit message (cite file + SHA, as here) is sufficient; no provenance header needed since no literal text crosses over.
2. **`d3-shape/src/curve/monotone.js`'s `slope3`/`slope2`/`point`** (ISC, same repo family, `d3/d3-shape@a82254af`). Port the **three-way min** tangent formula exactly (section 4.1) -- this is the one place where getting the arithmetic right instead of approximating from a secondary source (leptos-chartistry) actually matters for shadcn/recharts visual parity.
3. **`d3-shape/src/curve/step.js`'s `t`-parameterised `Step`** (ISC, same). Only the `t=1` (step-after) case is needed for MVP per the contract, but porting the general `t`-parameterised formula costs nothing extra and leaves `step`/`step-before` as a one-line addition later.

None of these are "copy the file" (source language differs); all three are explicitly **clean-room algorithm ports with attribution**, the cheapest-and-correct category per this repo's own `lifting-from-forks.md` categorisation ("concept ports need no header, but crediting the source in the commit message is honest and cheap") -- except here the "concept" is exact arithmetic, so the commit message should quote the source formula, not just name the source.

**Nothing from leptos-chartistry, yew-chart, rust-ui/dioxus-ui, or plotters-dioxus is a USE candidate as code** -- MPL-2.0 (leptos-chartistry) requires clean-room re-derivation even though the ideas are good (section 6 covers what to take as *ideas*); yew-chart's Apache-2.0 code is licence-clean but too thin/simple to be worth lifting (no nice-tick algorithm, no theming, no a11y -- see section 3d); rust-ui/dioxus-ui and plotters-dioxus are architecturally wrong (JS engine / raster image) regardless of licence.

## 6. LEARN -- design decisions to copy or avoid, with citations

**Avoid (pitfalls confirmed in the wild):**
- **The ApexCharts-JS-wrapper pattern** (`rust-ui/dioxus-ui`, section 3a): empty `<div data-name>` + externally-loaded JS chart engine + `MutationObserver` re-init. Fails hydration parity (no SSR content) and the no-JS-bundle spirit, same class the prior report already ruled out for `charming`/`plotly` -- this is a second, independent confirmation from a totally different codebase, not a new argument.
- **The raster-bitmap-`<img>` pattern** (`plotters-dioxus`, section 3b): a third "opaque output" failure mode beyond opaque-SVG-string -- zero DOM structure at all, coordinate-math-only interactivity via `reverse_translate`. Confirms the prior report's "opaque output implies no free interactivity/a11y" finding is a property of the *rendering substrate* (string-SVG, raster image, or JS-engine DOM), not of any one crate.
- **Hardcoding a tooltip's background colour** (`leptos-chartistry/src/overlay/tooltip.rs`, `background-color: #fff` literal, no CSS variable, no dark-mode counterpart) -- cited as the concrete "don't do this" our own `--dx-chart-*`-token tooltip CSS already avoids.
- **`ResizeObserver`-ing an `<svg>` directly** -- `leptos-chartistry/src/use_watched_node.rs`'s own code comment documents real, working-around-in-production trouble ("`<svg>` has issues around observing size changes... wrap in a `<div>`... twice") -- our contract's fixed-`viewBox` + CSS-scaled responsive sizing makes this pitfall unreachable by construction, not avoided by discipline. Validates the existing decision; no `ResizeObserver` should be introduced for this component.

**Copy the idea, not the code:**
- **Tick count/thinning driven by available label width**, not just data magnitude (`leptos-chartistry/src/ticks/gen/aligned_floats.rs`, section 3c) -- a genuine, working answer to "how do I avoid overlapping tick labels" that's independent of MPL-2.0 (it's a technique, no text to copy). Worth layering as an *optional* pass on top of d3's magnitude-based `ticks()` (section 4.3): compute d3's nice ticks first, then drop every other one (or recompute with a smaller `count`) if the estimated rendered label width at the current `viewBox` scale would overlap. Not urgent for MVP -- `chart-api.md` already has one crude, working mitigation (`x_tick_format`'s "first 3 chars" default, matching shadcn) -- but worth a code comment pointing at this as the natural stage-2+ upgrade path.
- **Decoupling scale normalisation (0..1) from pixel mapping** (`yew-chart/src/linear_axis_scale.rs`'s `NormalisedValue`, section 3d) -- a small, clean testability idea; not worth restructuring the existing contract's `LinearScale::scale() -> f64` (pixel-space) shape over, just worth knowing it's a legitimate alternative if `scale.rs`'s tests ever feel awkward.
- **Recharts' own choice of `curveStepAfter`** (confirmed by this session's read of `Curve.tsx`, section 4) independently corroborates `chart-api.md`'s "Step = step-after" pick -- not a new idea, but a direct confirmation worth recording so a future reader doesn't wonder why "after" specifically was chosen.

**Cross-library pattern, not a single library's lesson:**
- **Every chart library surveyed across both reports -- `dioxus-charts`, `leptos-chartistry`, `yew-chart`, and (so far as ApexCharts' own DOM goes) `rust-ui/dioxus-ui` -- has zero accessibility semantics.** Grep-confirmed at 0 real hits for `aria|role=|tabindex` in both `leptos-chartistry` and `yew-chart` this session, matching the prior report's identical `dioxus-charts` grep. This is not a gap specific to any one project; it is the norm outside the shadcn/recharts (JS-ecosystem) lineage. This repo's own `role="img"` + hidden `<table>` + optional keyboard contract (already decided) is therefore ahead of every peer surveyed, not catching up to one -- good evidence to keep the decision as-is, not a reason to relax it.

## 7. Recommended changes to the contract

**Nothing found in this lane argues for a hard change to `chart-api.md`.** Everything that touches the contract's own decisions (a11y approach, no-`ResizeObserver` responsive sizing, CSS-variable theming, `Step = step-after`) came out **validated** by what the wider landscape does and doesn't do well, not contradicted. The one genuinely new fact -- the Steffen-monotone two-way-vs-three-way-min discrepancy (section 4.1) -- is an implementation-precision matter for whoever writes `scale.rs`'s curve math, not a change to the contract's own text (which already correctly says "recharts 'monotone'"); it's routed to `chart-lanes.md` below as a citation for the primitive lane, per this task's own instruction not to instruct either lane to change the contract.

If the main loop wants one optional, non-urgent idea on record: consider a documented stage-2+ note that `y_tick_count`/tick-label thinning could become width-aware (section 6) rather than a fixed count -- soft, not urgent, and the contract's existing `x_tick_format` truncation already covers the MVP case shadcn itself ships.

---

## 8. Verification ledger

**Verified by execution/reading this lane, 2026-09-19:**
- All 26 known forks of `DioxusLabs/components` return zero "chart" hits via `mcp__github__search_code`, one query per repo (section 2).
- Fork roster (25 of the 26) independently confirmed via `WebFetch` of the live GitHub forks page (section 2); the 26th (`jcgruenhage`) sourced from this repo's own prior fork-mining docs.
- `rust-ui/dioxus-ui`'s six chart-family `.rs` files are empty `<div>`s driving an externally-loaded ApexCharts via a 534-line JS file -- read in full, not sampled (section 3a).
- `plotters-dioxus`'s entire 264-line source (`lib.rs`+`plotter.rs`+demo `main.rs`) read in full; renders to a base64 PNG `<img>`, pre-0.4 Dioxus API (section 3b).
- `leptos-chartistry`'s tick-generation, interpolation, stacking, resize-watching, tooltip, and colour-scheme modules read in full (1,853 combined lines across the 11 files listed in section 3c); zero a11y confirmed by grep across the whole `src/` tree; MPL-2.0 confirmed from `LICENSE.txt`'s own header text.
- `yew-chart`'s scale, axis, and series modules read/grepped (its `linear_axis_scale.rs` in full); zero a11y confirmed by grep; Apache-2.0 confirmed from `Cargo.toml` and `LICENSE`.
- `d3-shape`'s `monotone.js` and `step.js` read in full; `d3-array`'s `ticks.js` read in full; both packages' `"license": "ISC"` confirmed from `package.json`.
- `recharts/src/shape/Curve.tsx` read (this session, this lane) confirming it imports `curveMonotoneX`/`curveStepAfter`/etc. from `victory-vendor/d3-shape` rather than reimplementing them.
- `MBeliou/shadcn-dioxus`, `AurimarL/rustcn-ui`, `MadsLudvig/shadcn-dioxus` checked via `mcp__github__search_code`; only CSS theme tokens found in the first, nothing in the other two.
- Web searches run, per the task's list: "dioxus chart component 2026", "dioxus charts svg rust", "dioxus shadcn chart", "dioxus components fork github chart", "dioxus plotters wasm chart", "dioxus-charts crates.io", "charming dioxus rust chart library", "leptos-chartistry svg chart leptos", "yew-chart svg rust yew" -- all nine returned results, surfacing `plotters-dioxus`, `rust-ui.com`'s chart pages, and the shadcn-for-dioxus projects as the new leads this report chased down.

**Not independently re-verified this lane (cited from the prior, already-approved report instead):** shadcn `chart.tsx`, recharts' `accessibilityLayer`/APG/WAI-ARIA Graphics Module findings, `dioxus-charts`' full source, `dignifiedquire/dx-components`'s zero-chart-files finding, `charts-rs`/`plotters`/`poloto`/`charming`/`plotly`'s architecture verdicts -- all already read in full by `dev-docs/research/chart-2026-09-19.md`, re-citing rather than re-cloning per this task's own framing of that report as "the approved research."

**No repo files were edited. No server was started. No build was run** (pure research/reading lane, no cargo/dx invocation needed or made). All clones live under `$S/refs/` for the other lanes/the main loop to re-read if wanted: `rust-ui-dioxus-ui/`, `plotters-dioxus/`, `leptos-chartistry/`, `yew-chart/`, `d3-shape/`, `d3-array/`.
