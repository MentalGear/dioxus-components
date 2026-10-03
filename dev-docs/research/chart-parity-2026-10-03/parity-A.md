# Parity A: AREA / LINE / BAR / TOOLTIP charts vs shadcn new-york-v4

> **Snapshot 2026-10-03.** Evidence files this report cites by relative name (screenshots, JSON probe dumps, probe scripts) were scratch artifacts of the session and are **not committed**; the findings and numbers are kept. See [README.md](./README.md).

Method: registry TSX for all 39 charts in scope (`parityA/reg/*.tsx`), rendered shadcn views probed with Playwright (`parityA/sp-*.json`: grid, path `d`, bar rects, dots), our built docs (`/charts/{area,line,bar,tooltip}`) probed the same way (`parityA/ours-probe.json`), side-by-side PNGs in `parityA/pairs/`, plus an independent JS re-implementation of d3 `curveMonotoneX`/`curveNatural` and Recharts' `getNiceTickValues` (`parityA/d3curves.mjs`, `parityA/domains.mjs`) that I validated against shadcn's rendered paths/grids (max error 0.002 px / exact tick match) before using them as the oracle.

Legend for root-cause class: (P) port error in our demo, (E) engine semantics differ from Recharts, (D) deliberate.

---

## 0. Answers to the three priority items

### 0.1 Interactive datasets and totals (confirmed: fabricated)

The three registry interactive charts share one fixed dataset: 91 rows 2024-04-01..2024-06-30, **Desktop total 24,828, Mobile total 25,010, max 499 / 530**, spiky (day-to-day swings of 100-300).

| chart | ours | verdict |
|---|---|---|
| area-interactive | `ROWS` copied from registry; 0 of 91 rows differ, totals 24,828 / 25,010 | data OK. **Off by one in the range filter** (P): shadcn keeps `date >= Jun 30 - N days` = 91 / 31 / 8 rows; ours `all_data.iter().rev().take(days)` = 90 / 30 / 7 rows (`area_chart/variants/interactive/mod.rs:151`). First visible day is Apr 2 instead of Apr 1. |
| line-interactive | `generate_data()` = `230 + 160 sin(0.13 i) + 0.7 i` (desktop), `210 + 150 sin(0.16 i + 0.8) + 0.5 i` (mobile) (`line_chart/variants/interactive/mod.rs:43-56`). Totals **24,169 / 22,717**, max 432 / 401 | (P) fabricated; shape is two smooth humps instead of ~45 spikes |
| bar-interactive | `222 + 120 sin(0.11 i) + 0.9 i`, `150 + 90 sin(0.17 i + 1.3) + 0.6 i` (`bar_chart/variants/interactive/mod.rs:46-47`). Totals **25,923 / 16,584**, max 407 / 285 | (P) fabricated |

Other port differences found on the interactive trio:
- line/bar-interactive series colors: shadcn config has `desktop: chart-2`, `mobile: chart-1` (swapped), single active series drawn with `var(--color-${activeChart})`; ours draws every series with `var(--dx-chart-1)` (`line .../interactive/mod.rs:93`). (P)
- tooltip: shadcn label is `labelFormatter` with year ("May 16, 2024") and row name comes from `nameKey="views"` -> "Page Views"; tooltip width 150px. Ours shows "May 8" and "Desktop". The year is lost because labels were pre-formatted to "Mon D" strings. (P)
- area-interactive fill: shadcn gradient (`#fillDesktop` 5% 0.8 -> 95% 0.1) with Recharts' default `fillOpacity` 0.6 (measured `fill-opacity: 0.6` on the area path); ours flat color at 0.4, **no gradient** (`AreaOptions::gradient` not set in the interactive demo). (P)
- line/bar-interactive cursor: shadcn solid 1px `border`-colored line / muted rect band; ours dashed 4 4 line (`chart/style.css` `chart-cursor-line`). (D/E, see E8)
- y domain/ticks: shadcn 5 ticks `0,150,300,450,600` (desktop, max 499) / area stacked `0,250,500,750,1000`; ours 6 ticks to 500/1000 (see E2).
- x ticks: shadcn shows 18 labels ("Apr 3, Apr 8, ... Jun 24, Jun 30": `minTickGap=32`, last point always shown); ours 12 labels every 8th point, last point never labelled (see E7).
- header stat buttons (30px bold numbers, `bg-muted/50` active, border-l) are a UI lane item, visible in `pairs/h-line-*.png`.

### 0.2 Aspect ratio / "squashed" small charts (confirmed)

Every small shadcn chart is `ChartContainer` default `aspect-video` (16:9): content box 369x208 at the 419px card. Ours: `Chart` defaults `width: 600, height: 300` (`primitives/src/chart/components/chart.rs:77,83`), i.e. **2:1**, rendered 350x175 in our 400px card. Plot area:

| | shadcn (area-default) | ours (area main) |
|---|---|---|
| svg px | 369 x 208 | 350 x 175 |
| plot top..bottom | y 0..178 (axis band 30) | 4.7..151 (8/24 user units x 0.583) |
| plot height px | 178 | 146 |
| plot left..right px | 12..357 | 4.7..345 |

Peak-to-trough of the default data (Feb 305 vs Apr 73): shadcn 129 px, ours 96.5 px = **75%**. Decomposition: plot height 0.82 (aspect + smaller axis band) x y-domain headroom 0.91 (see E2: shadcn's max 305 -> top of axis 320 = 95% of plot; ours -> 350 = 87%) = 0.75. Hero charts (area/line/bar interactive) are correct: fixed `height: 250`, `fit_width`.

Second-order effect of the same cause (E5): the whole viewBox is scaled by k = 350/600 = 0.583, and only *text* is compensated (`--dx-chart-text-scale`). Everything else shrinks: line stroke 2 user units -> 1.17 px (shadcn `strokeWidth=2` = 2 px; ours is 58%), grid 1 -> 0.58 px, dots `r=3` -> bbox 3.5 px diameter (shadcn r=3 = 6 px; `dots-colors` r=5), active dot, bar `rx` 2 user units -> 1.2 px. Only the heroes (k = 1) render 1:1.

### 0.3 X-axis tick label styling / spacing

- Font: ours renders 12 px (20.57 user units x 0.583), weight 400, fill rgb(112,112,112) vs shadcn 12 px, 400, `#666`. Family Geist in both. **OK.**
- Formatter: default 3-char truncation == `value.slice(0, 3)`. **OK** for month charts. Not OK where shadcn uses full names: bar-horizontal/mixed/active use `chartConfig[value].label` ("Chrome") and ours truncates to "Chr/Saf/Fir/Edg/Oth" and clips at the left edge (P, see bar tables).
- Vertical distance: shadcn tick text baseline 14 px (area, `tickMargin=8`) / 16 px (bar, `tickMargin=10`) below the axis; ours ~16 px. Close enough.
- Horizontal positions differ: shadcn area/line point scale puts the first label *centered on the plot's left edge* (x=12) and last on the right edge (x=357); ours labels sit at band centers inset by 9.7% of plot width (E1).
- Bar charts: shadcn centers labels on bars, as ours.
- Density on the heroes: see 0.1 / E7.

---

## 1. Engine (E) classes, with the precise change

Evidence is numeric: for the 6-point default data, normalised (x as fraction of plot width, y as fraction of plot height, 0 at the baseline):

```
shadcn area/line default  x 0.000 0.200 0.400 0.600 0.800 1.000 | y 0.581 0.953 0.741 0.228 0.653 0.669
ours   area/line default  x 0.097 0.258 0.419 0.581 0.742 0.903 | y 0.531 0.871 0.677 0.209 0.597 0.611
```

### E1 Area/Line use a band scale (inset) where Recharts uses a point scale (edge to edge)
- `primitives/src/chart/components/layout.rs:198-203`: `x_scale = BandScale{ count: n, range: (plot_x0, plot_x1), padding: BAND_PADDING (0.2) }`, `xs = x_scale.center(i)`. For Area/Line the first/last point sit `0.1 * step + step/2` inside the plot edge (9.7% of plot width for 6 points, 0.7% for 91). Recharts category axis for Area/Line/Line has no padding: point i at `plot_x0 + i/(n-1) * width` (shadcn: 12, 81, 150, ... 357).
- Fix: for `ChartKind::Area | Line`, `xs[i] = plot_x0 + i * (plot_x1 - plot_x0) / (n-1)` (n==1 -> center), keep the band for Bar. Hit bands (`chart.rs:811-823`, nearest-point tooltip) must follow the same point positions (shadcn: tooltip snaps to the point; hit region is plot rect, no padding).
- Subsumes: every Area and Line variant (20 charts), the hero first/last-tick offset, the dots/labels positions.

### E2 y-domain and tick generation are not Recharts'
- `engine/scale.rs:354` `nice_domain` -> d3 `.nice()` with 10 target ticks iterated to a fixed point; `layout.rs:206-211` then takes `ticks(5)` of that domain. Result: domain and grid are decoupled (top grid line often below domain top; 6-8 grid lines) and the headroom differs. Recharts (`getNiceTickValues(domain [0,max], tickCount 5)`, YAxis default): `step = ceil((max-min)/(tickCount-1) / 10^d / 0.05) * 0.05 * 10^d` (0.1 granularity when d == 1), ticks are exactly `tickCount` values and **the domain is the first/last tick**, so the top grid line is the plot top.
- Measured on our rendered grids vs shadcn's:

| data max | shadcn ticks (verified on rendered grid) | ours domain / ticks |
|---|---|---|
| 305 (default area/line/bar) | 0,80,160,240,320 | 0..350 / 0,50,...,350 (8 lines) |
| 505 stacked | 0,150,300,450,600 | 0..550 / 0,100,...,500 (6 lines, top line 50 below plot top) |
| 950 (tooltip charts) | 0,250,500,750,1000 | 0..1000 / 0,200,...,1000 (6 lines) |
| negative -209..214 | -300,-150,0,150,300 | -250..250 / -200..200 (domain beyond ticks) |
| 275 (dots-colors, mixed) | 0,70,140,210,280 | 0..280 / 0,50,...,250 (top 30 above last line) |
| 499 (line/bar interactive) | 0,150,300,450,600 | 0..500 / 0,100,..,500 |
| expand 0..1 | 0,.25,.5,.75,1 | 0..1 / 0,.2,...,1 |
| area-axes `tickCount=3`, max 505 | 0,300,600 | 0,250,500 on domain 0..550 |

- Fix: replace `nice_domain` + `y_scale.ticks(count)` pair by a single `recharts_ticks(min, max, tick_count) -> (domain, ticks)` (port of `getNiceTickValues`/`calculateStep`; `domains.mjs` is a working reference) and use `ticks.first/last` as the domain. Min is `min(0, data min)` (Recharts `[0,'auto']` -> `dataMin` only when negative); `StackMode::Expand` -> domain `[0,1]` ticks 0,.25,...,1. Keep d3's nicer for non-chart uses.
- Subsumes: every cartesian variant (grid count, amplitude, axes tick labels, negative bar domain).

### E3 Curves: no `natural`, `step` is step-after, default is monotone
- `engine/curve.rs:131` `enum Curve { Linear, Monotone, Step }`. shadcn uses `natural` in 13 of 14 smooth charts (area default/stacked/expand/legend/icons/gradient/axes/interactive, line default/dots/custom/colors/label/label-custom); `monotone` only in line-multiple and line-interactive; `linear`, `step` in their own variants.
- `Monotone` verified: ours vs d3 `curveMonotoneX` max control-point difference 0.001 px on 5 and 90 segments, and shadcn's own path matches the same d3 within 0.002. So line-multiple/line-interactive math is correct.
- **Add `Curve::Natural`**: d3 `curveNatural` (tridiagonal solve per axis, `controlPoints` as in d3-shape `natural.js`); verified that it reproduces shadcn's `chart-area-default` and `chart-line-default` paths to 0.001 px (my JS in `d3curves.mjs`). Note natural overshoots the data (shadcn path dips to y = -0.057 above the plot); monotone cannot.
- **`Curve::Step` is wrong**: ours (`curve.rs:184`) steps *after* (horizontal to the next x, then vertical). Recharts `type="step"` = `curveStep` = **midpoint**: shadcn's anchors are x = 0, 0.1, 0.1, 0.3, 0.3, 0.5, ... (vertical risers halfway between categories, half-width first/last runs) vs ours 0.097, 0.258, 0.258, 0.419, ... Fix: `Step` -> midpoint; add `StepAfter`/`StepBefore` if wanted.
- Default: `chart.rs:132` `default = Curve::Monotone`; shadcn default for Area/Line is `natural` -> demos using the default (area `main`, line `main`, `dots`, `dots_custom`, `dots_colors`, `label`, `label_custom`, stacked/legend/icons/gradient/axes/expand, interactive area) render a different smooth shape. Visible: ours has a flat plateau at Feb and May-Jun, shadcn has rounded humps (`pairs/area-default.png`).

### E4 Bar sizing and radius
- `series/bar.rs:97,243,352` `GROUP_PADDING = 0.15` is applied even for a single, non-stacked series, so the lone bar is `0.8 * (1-0.15)/(1+0.15) = 59%` of its category pitch; shadcn (Recharts barCategoryGap 10% each side) 78.5% (47 px of 59.8 px band; hero: 11 px of 13.9). Stacked ours = 80% (OK). Multiple: ours 29.8 of 94 (0.32 each), shadcn 21 of 59.8 (0.35 each, barGap 4 px). Fix: inner band only when `series_count > 1`; use Recharts' fixed 4px barGap between grouped bars rather than a ratio; outer band padding 0.1 each side (`BAND_PADDING` 0.2 is `paddingInner=paddingOuter=0.2/...` semantic; Recharts' `barCategoryGap=10%` -> gap 0.1*step each side).
- **No per-bar radius** (`bar.rs:42` documents it): rect `rx = var(--dx-radius-xs)` = 2 user units (~1.2 px at k = 0.583). shadcn: radius 8 (default, label, active), 4 (multiple, label-custom), 5 (horizontal, mixed), `[0,0,4,4]`/`[4,4,0,0]` per-corner on stacked segments (tooltip charts and bar-stacked), 0 on interactive/negative. Needs a `radius` prop (uniform or 4-corner) rendering a `path`, scaled with k or fixed px.
- Bar active (`active_index`): ours dims all other bars to opacity 0.3 (`chart/style.css:206`); shadcn keeps the others unchanged and draws the active bar with `fill-opacity 0.8`, dashed stroke (`stroke-dasharray 4`, strokeWidth 2, stroke = own fill). (E/D)

### E5 Fixed 2:1 viewBox, uniform scaling shrinks non-text geometry
- `chart.rs:77/83` default `width 600 / height 300`; `ChartContainer` css wrapper gives `width:100%; height:auto`. Only text is compensated. See 0.2. Construction: default to `fit_width` (measured px viewBox, k = 1) with `aspect-ratio: 16/9` on the container (`aspect-video`), as the heroes already do with fixed `height: 250`. That removes the whole class (aspect, stroke widths, dot radii, grid, radii, tick offsets) without touching each variant. `fit_width` today only supports a fixed height; extend it to derive height from an aspect ratio after measuring width.

### E6 Layout margins are constants, not per-chart Recharts margins
- `layout.rs:27-32`: top 8, right 8, left 8 (40 with y axis), bottom 24 with x axis. Recharts v3: a `margin={{left:12,right:12}}` **replaces** the default (top/bottom = 0), no margin prop = 5 on all sides, x axis band 30 px, y axis width 60 (area-axes uses `left: -20` so y labels sit inside the plot rect starting at x=40).

| chart | shadcn plot rect (px) | ours (px, k = 0.583) |
|---|---|---|
| area/line default etc. | x 12..357, y 0..178 | x 4.7..345, y 4.7..151 |
| bar default etc. | x 5..364, y 5..173 | same as above |
| bar-label / line-label | top margin 20 (labels above top bar) | 4.7 |
| expand | top 12 | 4.7 |
| dots-colors / label-custom | left/right 24, top 24, **no x axis** (no `XAxis`) | ours shows x axis and margins 8 |
| area-axes | x 40..357 with y labels at 0..40 | x 40*... + labelled ticks outside |
| hero line/bar | x 12..1274 | x 8..1222 |
| hero area | default margin 5 | 8 |

Fix: expose `margin` (top/right/bottom/left) on `Chart`, default 5/5/5/5, and let demos pass shadcn's value; or compute margins from props.

### E7 X tick thinning
- `chart.rs:191` `max_x_ticks = 12`; `layout.rs:463` `x_tick_step = ceil(n / max)` (fixed stride from index 0). Recharts `interval="preserveEnd"` + `minTickGap=32`: picks the largest stride whose labels (measured width + 32 px) do not overlap, anchored so the **last** tick is always drawn; heroes show 18 labels at 1286 px, 6 at 419 px (ours: 12 and 6). Add `min_tick_gap` (default 5 per Recharts, demos pass 32) and anchor to the end.

### E8 Hover cursor cannot be turned off, style differs
- `chart.rs:804-860`: cursor group always drawn for the active index; style `chart/style.css` dashed line (4 4) for area/line and `fill-opacity .1` rect for bars. shadcn: `cursor={false}` on every default area/line/bar chart and all 9 tooltip charts; interactive line: solid 1px `border` line; interactive bar and bar-stacked: `muted` rect band (bar-multiple, bar-active etc. `cursor={false}`). Needs `cursor: bool` (or `ChartTooltip { cursor: false }`) and restyle to `stroke: var(--border); stroke-dasharray: none` / `fill: muted`.

### E9 Stack order follows config order
- Our stacked charts stack in `ChartConfig` order (first series at the bottom): `layout.rs`/`stack.rs` consume `values` in series order. shadcn stacks in **JSX Area order**, which for the area demos is the reverse of the config: `mobile`, then `desktop` on top (stacked/legend/icons/gradient/axes/interactive); expand: `other` bottom, `mobile`, `desktop` on top. So ours puts desktop at the bottom, shadcn at the top (compare `stacked` curves: shadcn top series y .443 .842 .595 ... vs ours bottom series .338 .555 .431). Because our API couples draw/stack order to config order, a faithful port must reorder config (and then legend order flips too, which matches Recharts: the legend reads the item order). (P, with E rooted in the single-order API; `Area` series-order prop would be the construction).

---

## 2. Per-chart tables

Notation: "x0/x1" = first/last point as fraction of plot width. "amp" = vertical amplitude of the data relative to shadcn. All ours data columns were compared with the registry TSX; data matches unless stated.

### 2.1 Area (10)

| chart | shadcn | ours | difference | class |
|---|---|---|---|---|
| area-default | AreaChart margin L/R 12, natural, fillOpacity .4, `indicator="line"` tooltip, cursor false, plot 12..357 x 0..178, 5 grid lines 0..320 | `main`: data identical; `Curve::Monotone` default; plot inset; 8 grid lines to 350; tooltip default | x0/x1 0.097/0.903 vs 0/1; y(Feb) .871 vs .953; shape monotone not natural; aspect 2:1; area stroke 1.17 px vs 1 px; dashed cursor; 8 vs 5 grid lines | E1 E2 E3 E5 E8; tooltip indicator check P |
| area-linear | `type="linear"`, tooltip `indicator="dot" hideLabel`, cursor false | `curve: Linear`, `hide_label: true` | same inset/y/aspect differences; curve math identical | E1 E2 E5 E8 |
| area-step | `type="step"` (curveStep, midpoint), hideLabel | `Curve::Step` (after) | shadcn step corners at x .1,.3,.5,.7,.9 with half-width end runs; ours at band centers .258,.419... and an extra right-edge stub | E3 (+E1 E2 E5) |
| area-stacked | `AreaChart` L/R 12, natural, stackId a, order mobile then desktop, 2-series data (max 505), tooltip dot | `stacked: true`, Monotone, config order desktop then mobile | stack order reversed; domain 0..550 w/ 6 grid lines vs 0..600 w/ 5; shape | E1 E2 E3 E9 |
| area-stacked-expand | `stackOffset="expand"`, margin top 12, order other, mobile, desktop; other `fillOpacity .1`; tooltip line; ticks 0,25,...,100% | `StackMode::Expand`, config order desktop, mobile, other (other on top, fill .4), 6 grid lines (0,.2..1) | bottom band is desktop in ours vs other in shadcn; `other` fill opacity .1 vs .4; ticks | E2 E3 E9 + P (fillOpacity .1) + E6 (top margin) |
| area-legend | stacked natural + `ChartLegendContent`, plot height 150 (legend takes ~28) | `stacked`, `ChartLegend {}`, Monotone | as stacked; legend present both | E1 E2 E3 E9 |
| area-icons | as legend, config icons `TrendingDown`/`TrendingUp` | icons set, same | as legend | E1 E2 E3 E9 |
| area-gradient | stacked natural, gradient 5% .8 -> 95% .1 **with** `fillOpacity .4`, tooltip default | `AreaOptions{gradient:true}` (same stops, fill-opacity .4 default) | gradient OK; same stack order/curve/inset issues | E1 E2 E3 E9 |
| area-axes | `margin left -20`, `YAxis tickCount=3 tickLine/axisLine false tickMargin 8`; labels 0,300,600; plot x 40..357 | `show_y_axis`, `y_tick_count: 3`: labels 0,250,500 on domain 0..550, plot x 68.6*... | y labels/gridlines at different values; ours reserves a left gutter (MARGIN_LEFT_WITH_AXIS 40 user = 23 px) vs shadcn y-axis inside a -20 margin; top gridline not at plot top | E2 E6 + E1 E3 E9 |
| area-interactive | (hero 1286x250) 91 rows, margin default 5, gradient + Recharts default fillOpacity .6, natural, stackId a (mobile then desktop), `minTickGap 32`, select 90d/30d/7d (91/31/8 rows), tooltip labelFormatter "Mon D", dot | data identical, 90/30/7 rows, flat fill .4 no gradient, Monotone, 12 ticks every 8th, config order desktop bottom | off-by-one range (P), missing gradient + fill .6 (P), curve (E3), stack order (E9), tick density (E7), inset (E1 0.7%), 6 grid lines vs 5 (E2), margins 8 vs 5 (E6) | P E1 E2 E3 E6 E7 E9 |

### 2.2 Line (10)

| chart | shadcn | ours | difference | class |
|---|---|---|---|---|
| line-default | `natural`, strokeWidth 2, no dots, margin L/R 12, tooltip hideLabel cursor false | `main`: Monotone, stroke 2 user units | y(Feb) .871 vs .953, x0 .097 vs 0, monotone vs natural, line 1.17 px vs 2 px, aspect | E1 E2 E3 E5 E8 |
| line-linear | `linear` | `Curve::Linear` | inset/y/stroke/aspect only | E1 E2 E5 E8 |
| line-step | `step` (midpoint) | `Curve::Step` (after) | step geometry (x .1,.3,... vs .258,.419...) | E3 + E1 E2 E5 |
| line-multiple | two series **monotone**, tooltip with label, cursor false | Monotone | curve math equal (0.0007 px); y(desktop Feb) .871 vs .953; mobile .571 vs .625 | E1 E2 E5 E8 |
| line-dots | `natural`, `dot {fill}` r=3 default, `activeDot r=6`, hideLabel | `dots: true` r=3 user units (1.75 px radius) | curve; dot half size; active dot size | E3 E5 + E1 E2 |
| line-dots-custom | `GitCommitVertical` icon (24px, bg fill) as dot, natural | diamond `rect` 7px custom dot (documented) | different glyph, curve | D + E3 |
| line-dots-colors | `LineChart margin top 24 L/R 24`, **no XAxis**, natural, `Dot r=5` colored by `payload.fill` (chrome..other), tooltip `nameKey=visitors hideLabel indicator=line`; ticks 0,70,...,280 | `dots: true`, per-datum color via `ChartDatum::color`, ours shows an **x axis with "Chrome/Safari..."**, margins 8, Monotone; ticks to 250 on domain 280 | missing margins (24), x axis extra, dot r 5 vs 3 (1.75 px), curve | P (x axis shown, dot radius) E3 E6 E2 |
| line-label | data = 6 months with mobile column (unused); margin top 20; **grid + x axis kept**; `natural`, dot, `LabelList top offset 12 fontSize 12`, tooltip indicator line | `show_grid: false, show_y_axis: false`, `labels: Value`, Monotone | grid hidden in ours (P, comment claims labels replace it, shadcn keeps it); labels same; curve; margin top 20 | P E3 E6 |
| line-label-custom | **browser data** (chrome 275, safari 200, firefox 187, edge 173, other 90); margin top 24 L/R 24; grid kept, no x axis; labels = browser **label** (Chrome...) above points; natural | month data (Jan..Jun, 186..214); no grid, no axes, labels = first 3 letters of month, Monotone | **wrong dataset** (P), grid hidden (P), label text (P), curve | P E3 |
| line-interactive | (hero) fixed spiky 91-row dataset, `monotone`, strokeWidth 2, dot false, margin L/R 12, tooltip `nameKey=views` + year label, colors desktop=chart-2 / mobile=chart-1, header stat buttons 24,828 / 25,010 | sine data (24,169 / 22,717), both series chart-1, Monotone (math OK), dashed cursor, 12 ticks | **dataset fabricated** (P), colors (P), tooltip text (P), cursor style (E8), ticks (E7), inset (E1), ticks 0..500 vs 0..600 (E2) | P E1 E2 E7 E8 |

### 2.3 Bar (11 ours, 10 in shadcn)

| chart | shadcn | ours | difference | class |
|---|---|---|---|---|
| bar-default | `radius 8`, `tickMargin 10`, hideLabel, cursor false, margin default 5, plot 5..364 x 5..173, 5 lines to 320 | bar width 55.7/94.2 = 59% of pitch (shadcn 78.5%), `rx` 2 (1.2 px), 8 grid lines to 350, dashed? (rect cursor shown on hover) | thinner bars, square corners, wrong ticks, aspect | E2 E4 E5 E8 |
| bar-horizontal | **6 month data** (186,305,237,73,209,214), `layout=vertical`, `XAxis hide`, `YAxis` month slice(0,3), radius 5, margin left -20 | **5 browser data** (Chrome 275...), `horizontal`, axes hidden, labels "Chr/Saf/..." clipped at left edge, bar from x=8 | wrong dataset (P), clipped/truncated labels (P/E), radius | P E4 |
| bar-multiple | desktop+mobile, radius 4, `indicator="dashed"`, cursor false | same data, Dashed | bar width 29.8 of 94 each vs 21 of 59.8; radius; ticks | E2 E4 E5 E8 |
| bar-stacked (title "Stacked + Legend") | stacked, radius [0,0,4,4] bottom / [4,4,0,0] top, `ChartLegend`, **cursor shown** (muted band), hideLabel | `Stacked` (no legend) and `Stacked + Legend` (extra variant, hide_label) | uniform tiny rx; domain 0..550 vs 0..600; ours has an extra non-shadcn variant (D) | E2 E4 E8 |
| bar-label | `margin top 20`, **grid + x axis kept**, radius 8, `LabelList top offset 12 size 12` | `show_grid: false, show_y_axis: false`, `value_labels` | grid hidden (P); square corners; top margin | P E4 E6 |
| bar-label-custom | **6 months, vertical layout**, `desktop` chart-2, `LabelList dataKey=month insideLeft offset 8` (fill background) + `desktop` right, margin right 16, grid horizontal=false, `YAxis hide`, `XAxis hide`, radius 4, tooltip indicator line | **5 browsers** horizontal, labels inside | wrong dataset (P), color (chart-1 vs chart-2) | P |
| bar-mixed | 5 browsers, vertical layout, radius 5, `YAxis` labels via `chartConfig[value].label` ("Chrome"), x hidden, per-bar fill | same data/order; labels truncated "Chr"; axis starts at x=8 (shadcn bars start at x=60 after the y-axis width) | label text/space (P), colors from ours palette (tokens), radius | P E4 |
| bar-active | **vertical** bars, 5 browsers `chrome 187, safari 200, firefox 275, edge 173, other 90`, `ACTIVE_INDEX=2` (Firefox is the tallest, 275), grid vertical=false, radius 8, strokeWidth 2, active: `fillOpacity .8` + dashed outline | **horizontal**, order `chrome 275, safari 200, firefox 187...` (the mixed dataset), active_index 2 (Firefox 187 is third), others dimmed to .3 | wrong orientation (P), wrong values/order (P), highlight style (E4) | P E4 |
| bar-negative | data `186, 205, -207, 173, -209, 214`; **no XAxis** (month labels via `LabelList top` of each bar, below for negatives), `Cell` fills chart-1 (positive) / chart-2 (negative), tooltip hideLabel hideIndicator; ticks -300..300 | data `186, 305, -237, 73, -209, 214`; x axis shown; negative color chart-5 (pink in our palette); domain -250..250 | wrong values (P), axis vs bar labels (P), color (P), domain (E2) | P E2 |
| bar-interactive | hero: 91 bars, desktop chart-2 / mobile chart-1, margin L/R 12, cursor (muted band), tooltip w-150 `nameKey=views` + year, `minTickGap 32` | sine data (25,923 / 16,584), both chart-1, bar 7.87 of 13.3 pitch (59% vs 79%), 12 ticks | dataset fabricated (P), colors (P), width (E4), ticks (E7) | P E2 E4 E7 |
| (mixed, label_custom, interactive...) | | | see above | |

### 2.4 Tooltip (9)

All nine shadcn tooltip charts are the **same** stacked `BarChart` (`running`, `swimming`; 2024-07-15..20; `XAxis` weekday short, `tickLine/axisLine false tickMargin 10`; **no CartesianGrid, no YAxis**; radius `[0,0,4,4]` / `[4,4,0,0]`; `cursor={false}`; `defaultIndex={1}`). Ours: same data and series, `stacked: true`, weekday formatter OK (`short_weekday`), but:

| chart | shadcn props | ours | difference | class |
|---|---|---|---|---|
| tooltip-default | `ChartTooltipContent` defaults, defaultIndex 1 | `ChartTooltip {}`, `default_index: 2` | wrong open index (open on Tue-vs-Wed: shadcn Jul 16, ours Jul 17), grid drawn (7 lines) vs none, domain 0..1000 w/ 6 ticks (hidden in shadcn), square bars | P (default_index, show_grid) E4 |
| tooltip-indicator-line | `indicator="line"` | `Line` | same | P E4 |
| tooltip-indicator-none | `hideIndicator` | `hide_indicator` | same | P E4 |
| tooltip-label-none | `hideIndicator hideLabel` | same | same | P E4 |
| tooltip-label-custom | `labelKey="activities"` (config label "Activities"), indicator line | `label_key: Some("Activities")` literal string, not a config key | semantically a different mechanism (value, not key) but same output | D |
| tooltip-label-formatter | `labelFormatter` -> "July 16, 2024" | `label_format: long_date` | equal output | -- |
| tooltip-formatter | `hideLabel`, custom row `Running ... 450 kcal` (min-w 130) | `formatter` closure returning row | equal intent | -- |
| tooltip-icons | `hideLabel`, config icons Footprints/Waves in tooltip rows | `ChartSeries::icon` | equal intent | -- |
| tooltip-advanced | `hideLabel className="w-[180px]"`, formatter with colored dot + "kcal" + extra **Total** row (basis-full, border-t) at index 1 | `formatter` with `TooltipRow::is_last`/`total` | equal intent | -- |

Not measured in this lane: tooltip placement/flip arithmetic (`pointX+10`, 400 ms transform transition), covered by the shadcn spec; do the pie/radar/radial lane compare separately.

---

## 3. Port (P) errors, collected

1. line/bar-interactive: fabricated sine data -> import the registry rows (`reg/chart-{line,bar}-interactive.tsx`, same 91-row array as area-interactive; reuse one shared const).
2. area-interactive: `take(days)` -> `date >= last - days` (91/31/8); add gradient + `fill_opacity 0.6`.
3. line/bar-interactive: swapped colors (desktop chart-2, mobile chart-1), tooltip name "Page Views", year in label, tooltip width 150px, `min_tick_gap 32`.
4. bar-horizontal: use the 6-month dataset; bar-label-custom: 6 months with `desktop` (chart-2) and `LabelList` month inside-left + value right; bar-active: vertical, `187,200,275,173,90`, active index 2 (Firefox) with fill .8 + dashed outline; bar-negative: values `186,205,-207,173,-209,214`, no x axis, month labels as bar labels, chart-1/chart-2; bar-label/line-label: keep grid + x axis, top margin 20; line-label-custom: browser data, browser labels, grid, no x axis; line-dots-colors: no x axis, margins 24; bar-mixed/horizontal: full labels (`Chrome`).
5. tooltip charts: `default_index: 1`, `show_grid: false`, `show_y_axis: false`.
6. area stacked/legend/icons/gradient/axes/interactive/expand: reverse config order so mobile (or other) is the bottom layer; `other` fill .1 in expand; top margin 12 in expand.
7. area-axes: shadcn y-axis within `left: -20`, labels 0/300/600 (follows E2/E6).
8. Cursor: set off for all demos that are `cursor={false}` in shadcn (after E8).

---

## 4. By-construction fixes

### 4.1 Port parity cannot drift again: generate demos from the registry
All P items 1-5 are one class: demo data/props were hand-typed or invented instead of derived. Construction: vendor the registry JSONs (`preview/tests/shadcn-registry/chart-*.json`, ~170 KB for all 39 in scope) and add a test (`cargo test -p preview` or a node script) that, for every demo in `preview/src/components/{area,line,bar}_chart/variants` and `chart_tooltip/variants`, parses the registry TSX `chartData` + `chartConfig` + the JSX props (`type`, `stackId`, `stackOffset`, `radius`, `layout`, `margin`, `hideLabel`, `indicator`, `cursor`, `defaultIndex`, presence of `CartesianGrid`/`XAxis`/`YAxis`/`LabelList`) and asserts them against the demo's source constants. Cheaper and sturdier variant: a codegen script emitting `const ROWS` modules from the registry and failing CI if the committed file differs (the same pattern as `check-*.sh` gates). Subsumes: P1, P2 (data), P4 datasets, P5 and most of P6 (config order).
Add a data-checksum gate: registry totals (24,828 / 25,010; 91 rows) asserted in the interactive demos.

### 4.2 Engine parity: golden files from shadcn's rendered DOM
`parityA/sp-*.json` holds shadcn's rendered gridlines, ticks, bar rects and path anchors for all 39 charts. Turn it into fixtures and a Playwright (or SSR) check per variant that renders ours at the same plot box and asserts normalized geometry within 1%: anchors' (x fraction, y fraction), grid count, bar width / band, curve control points for `natural`/`monotone`/`step`. That would have caught E1-E4, E6 in one run (the y/x fractions above fail by 5-10%).
Unit-level goldens for the engine (no browser): `recharts_ticks([0,305],5) == [0,80,160,240,320]`, `[0,505]->[0,150,...,600]`, `[-209,214]->[-300..300]`, `[0,950]->[0,250,...,1000]`, `[0,499]->[0,150,...,600]`; natural/step control points from the `chart-area-default` / `chart-area-step` shadcn paths (anchors 12,81,...,357).

### 4.3 One scaling model for the whole class E5/E6
Replace the default fixed 600x300 viewBox + text-only compensation with the existing measured `fit_width` path (k = 1) and an aspect-ratio container (16:9 default, like `aspect-video`), plus Recharts-style `margin`. After that, stroke widths, dot radii, radii, grid widths and axis offsets are in real CSS pixels by construction, which the heroes already demonstrate.

### 4.4 Summary of what each construction subsumes

| construction | removes |
|---|---|
| point scale for Area/Line (E1) | inset of all 20 area/line charts, first/last tick offset, hit-band offset |
| Recharts tick algorithm (E2) | all grid-count/ amplitude / negative-domain / y-axis label differences (every cartesian chart) |
| `Natural` + midpoint `Step` + default natural (E3) | 13 smooth charts' shape, area-step, line-step |
| bar sizing + radius (E4) | all 12 bar charts + 9 tooltip charts |
| fit_width + aspect + margin (E5, E6) | squashed look, thin strokes/dots, margin offsets |
| min_tick_gap / preserveEnd (E7), `cursor` prop (E8), stack order (E9) | hero tick density, hover cursor on 30+ charts, stacked area order |
| registry-derived demo data + parity test (4.1) | all P-type data errors (not E-type) |

Not subsumed: D items (diamond custom dot instead of the lucide icon, tooltip `label_key` as literal, the extra non-shadcn `Bar - Stacked` variant) and the palette difference (`--dx-chart-N` colors are saturated blue/orange/green/pink in ours vs shadcn's blue ramp; visible in `pairs/*.png`: bars light blue vs dark blue; belongs to the tokens/design lane).

---

## 5. Artifacts

- `parityA/reg/*.tsx|json`: registry source of all 39 charts.
- `parityA/sp-{area,line,bar,tip}.json`: shadcn DOM probes. `parityA/ours-probe.json`: ours.
- `parityA/pairs/*.png`: side by side (ours left, shadcn right): area-default, area-step, area-expand, line-default, line-dotscolors, bar-default, bar-horizontal, bar-active, bar-negative, bar-label, h-line-ours/h-line-shadcn.
- `parityA/d3curves.mjs`, `chkcurve.mjs`, `domains.mjs`, `cmp1.mjs`, `totals.mjs`, `cmpArea.mjs`: reproducible checks.
