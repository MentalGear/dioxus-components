# Our side: chart docs audit (main @ 8155cc6, read-only research)

> **Snapshot 2026-10-03.** Evidence files this report cites by relative name (screenshots, JSON probe dumps, probe scripts) were scratch artifacts of the session and are **not committed**; the findings and numbers are kept. See [README.md](./README.md).

Scope: `/home/user/dioxus-components`. All anchors are `file:line` as of the checkout. Browser measurements were taken against a local static server of `docs/` (a small throwaway node static server, `/dioxus-components/*` mapped to `docs/*`, port 8765). Scripts and screenshots are in the same folder (`measure.mjs`, `measure2.mjs`, `tokens.mjs`, `shots*.mjs`, `*-card-hover.png`).

---

## 1. Code map: routing, composition, demo discovery

### 1.1 Router
- `preview/src/main.rs:198-263` `#[derive(Routable)] pub enum Route`. Layout nesting:
  - `#[layout(AppLayout)]` (l.200) wraps EVERY route. It mounts `GlobalHead` once (CSS/fonts) and a theme-seed effect. `AppLayout` is at l.384.
  - `#[layout(NavigationLayout)]` (l.201) adds `hero.css` and `Footer`. `NavigationLayout` is at l.399.
  - Routes inside both layouts: `Home /` (l.202), `Docs /docs` (l.207), `Demos /demos` (l.209), `ComponentDemo /component/?:name` (legacy, l.221), `ComponentDemoPath /component/:name/` (canonical, l.233).
  - `#[end_layout]` at l.239. After it come the chrome-less routes: `ComponentBlockDemo` (legacy, l.243), `ComponentBlockDemoPath /component/block/:name/:variant/` (l.255), `EmailClientDashboard /dashboard/email-client` (l.261).
- Every variant carries `dark_mode: Option<bool>`. Routes with an iframe mode also carry `iframe`. Adding a variant means touching three `match` blocks: `Route::iframe()` (l.266-277), `Route::dark_mode()` (l.284-295), and the `Navbar`'s `in_component` matches! (l.443-446). The helper constructors are `Route::home/docs/demos/component/component_block` (l.302-345).
- A new `/charts` route would look like `#[route("/charts/?:tab&:dark_mode")] Charts { tab: Option<String>, dark_mode: Option<bool> }`, placed between `Demos` and `ComponentDemo` inside both layouts. Cheap deep-linkable tabs are a path segment (`/charts/:tab/`) so they are SSG-enumerable.

### 1.2 SSG static routes
- `preview/src/main.rs:138-166` `server_static_routes()` (`#[cfg(feature = "server")]`):
  - It starts from `Route::static_routes()`. This only returns all-literal-segment routes (`/`, `/docs`, `/demos`, `/dashboard/email-client`, and the bare legacy shells). The reason is documented at l.119-137.
  - It then pushes one string per `components::DEMOS` entry (`ComponentDemoPath`, l.143-151) and one per Block-type variant (l.152-161).
- It is served by an axum shim, `POST /api/static_routes` (l.101-113). This works around a dx-cli `--base-path` bug.
- To register `/charts`:
  - A literal `/charts` (no params) is picked up automatically by `Route::static_routes()`.
  - A `/charts/:tab/` route needs an explicit `for tab in [...] { routes.push(Route::Charts{tab: Some(..), ..}.to_string()) }` loop in `server_static_routes`, in the same style as l.143-161.
- Deploy guards that will bite a new route:
  - `scripts/deploy-preview.sh:80-115` fails the build if any `index.html` contains `dx-component-demo-not-found`, and checks every route has its bootstrap script.
  - `scripts/check-internal-hrefs.sh`: internal hrefs must be `Route`-built, never literals.
  - `docs/` (the local build) was produced by `dx build --ssg`. Expect `docs/charts/index.html` once registered.

### 1.3 Component page template (what a chart page is today)
- `preview/src/main.rs:1373-1400` `ComponentDemoPath` looks the name up in `components::DEMOS` (l.1377) and renders `ComponentHighlight` (l.1455-1533), wrapped in `DocsLayout` (l.1101-1266, the sidebar and `Navbar`).
- `ComponentHighlight` renders, in order:
  1. Header with `h1`, the `description` and a `dx components add <name>` copy button (l.1480-1488).
  2. Main variant, via `ComponentVariantHighlight` with `main_variant: true`.
  3. "Installation" section (`ManualComponentInstallation`, with `main.rs`/`style.css`/theme tabs, l.1547).
  4. "Usage notes", the baked `docs.md` HTML (`dangerous_inner_html: prefix_internal_hrefs(docs)`, l.1509).
  5. "Variants": every other variant stacked vertically, each under `h3.dx-component-variant-title` (title from `variant_title()`, l.1572).
- `ComponentVariantHighlight` (l.1597-1680) is the per-demo card:
  - A `Tabs { default_value: "Demo" }` (l.1634) with `TabTrigger "DEMO"` / `"CODE"` (l.1642-1643).
  - Demo tab: `TabContent` with `class: "dx-component-preview-frame"`, `id: component-preview-frame` for main or `component-preview-frame-<variant>` for the others (l.1620-1626, 1652-1660), containing `Comp {}`.
  - Code tab: `CodeBlock { source: highlighted }` (l.1668-1676).
  - Playwright specs target `#component-preview-frame`, so any gallery must keep unique ids per card.
- The Charts sidebar group is auto-built from `category_of()` (`preview/src/components/mod.rs:79-102`; the chart names are at l.98-99). Order is `demos_in_category` (l.110-116, which puts `chart` first). The sidebar icon is `ChartColumn` (main.rs:1229-1232). A `/charts` link would be added next to "Home"/"Overview" in `DocsLayout`'s "Start" group (main.rs:1165-1212), plus a new `DocsNavActive::Charts` variant (l.1050).
- Top nav links are at l.490-491 (Docs / Demos), and the footer duplicates them at l.563-564.

### 1.4 How variants and demos are discovered
- `preview/src/components/mod.rs:119-209` `macro_rules! examples!`: this is the single registry.
  - Invocation is at l.211-295, one line per component, e.g. `area_chart[linear, step, stacked, stacked_expand, gradient, legend, axes, icons, interactive]` at l.234. A `main` variant is implicit.
  - For each name it emits `mod <name> { mod component; mod variants { mod main; mod <v>; ... } }` (l.120-134).
  - It also builds a `ComponentDemoData` entry in the static `DEMOS` (l.136):
    - `name`
    - `description`: `include_str!(OUT_DIR/<name>/description.txt)`
    - `docs`: `include_str!(OUT_DIR/<name>/docs.html)`
    - `component`/`style`: code of `component.rs` and `style.css`
    - `variants`: per variant, `{ name, rs_highlighted: dioxus_code::code!("/src/components/<name>/variants/<v>/mod.rs"), component: <name>::variants::<v>::Demo }`
  - Variants are therefore an explicit list in the macro call, not a directory glob. A new variant directory is invisible until it is added to that list (and `mod.rs` is touched, which per `build.rs` note l.30-45 does not rerun the build script for new `.md`/`component.json`).
  - `ComponentType::Block` (variants rendered in an iframe at `/component/block/:name/:variant/`) is opt-in with `name(block)[...]`. Only `sidebar` uses it today.
- Data structs: `ComponentDemoData` (main.rs:53-61) and `ComponentVariantDemoData` (main.rs:64-70). The variant's `component` is a plain `fn() -> Element`, so a gallery can render any variant from any route with `let Comp = variant.component; Comp {}`. `ComponentGalleryPreview` (main.rs:2859-2930) already does this for the homepage grid.
- `preview/build.rs:1-202`:
  - Walks `src/components/*`, and for each `*.md` runs pulldown-cmark with GFM tables, with a fenced-code highlight pass through `dioxus_code` (`render_code_block_html`, l.~137). Output is `OUT_DIR/<name>/docs.html`.
  - Extracts `component.json` `description` to `description.txt`.
  - It only reads `.md` and `component.json`, never `variants/**/*.rs`.
- `component.json` per chart package (e.g. `preview/src/components/area_chart/component.json`): `{name, description, authors, exclude:[variants,docs.md,component.json], componentDependencies:["chart"], globalAssets:[theme css]}`. `chart/component.json` is the real installable one (cargoDependencies on `dioxus-primitives`). The `*_chart` packages are docs-only galleries whose `component.rs` is a one-line `pub use crate::components::chart::component::*;` (`area_chart/component.rs:1-39`, comment explains the `component` name collision).

### 1.5 DEMO / CODE tab and highlighting
- Tabs UI: the `Tabs/TabList/TabTrigger/TabContent` primitives (`preview/src/components/tabs`). `TabsVariant::Ghost`, horizontal (main.rs:1632-1640). The header row is `.dx-component-tabs-header` (`preview/assets/main.css:876`).
- Code highlighting:
  - `HighlightedCode` (main.rs:588-590) wraps `dioxus_code::advanced::HighlightedSource`, produced at COMPILE time by `dioxus_code::code!(path)`. The whole `variants/<v>/mod.rs` file is shown, helpers and tests included.
  - `CodeBlock` (main.rs:593-603) renders a `.dx-code-block` plus `PreviewCode` (l.605-615, theme `GITHUB_LIGHT`/`GITHUB_DARK` system pair, class `dx-preview-code-theme`) plus `CopyButton`.
  - `CopyButton` (l.739-759) is an absolutely positioned `button.dx-copy-button` (`position/top/right` at the call site, l.600). It copies via an inline `onclick` JS string (`navigator.clipboard.writeText(visible pre innerText)`) and flips to a check icon through a Rust `onclick` signal. It never resets (no timeout), so the check icon stays.
  - CSS tab: `CssHighlight` (l.643+) is embedded in release and lazy-fetched in debug builds.
- A gallery card needs only: `variant.name`, `variant.rs_highlighted`, `variant.component`. Reusing `ComponentVariantHighlight` verbatim already gives DEMO/CODE tabs plus copy.
  - It is `fn` in `main.rs` (private, same crate), so a new `Charts` page in `main.rs` can call it directly.
  - Differences from shadcn's cards: the whole tabbed preview frame (dotted-background canvas, `.dx-component-preview-frame`) wraps the Card. A shadcn-style gallery would want a lighter "view code" toggle and a bare card.

### 1.6 Other places a new route touches
- `preview/src/components/mod.rs` `category_of()` / `ComponentCategory::Charts`: sidebar grouping.
- `preview/assets/main.css:1485-1510`: chart-in-frame sizing (`width:100%; max-width:40rem`), only applied to children of `.dx-component-preview-frame`. A gallery grid outside the frame needs its own sizing.
- Gates from CLAUDE.md that new route code must pass: `scripts/check-preview-composition.sh` (demos import only via `crate::components::*`), `check-dx-class-prefix.sh` (CSS classes need the component's `dx-<name>` prefix, so a page-level gallery CSS belongs in `main.css`, not a component), `check-demo-wrapper-width.sh`, `check-internal-hrefs.sh`.
- Playwright specs that exist for charts: `playwright/{chart,area_chart,bar_chart,line_chart,pie_chart,radar_chart,radial_chart,chart_tooltip}.spec.ts`. They target `#component-preview-frame`/`-<variant>` on `/component/<name>/`, so keep those pages if a gallery is added alongside rather than replacing them.

---

## 2. Tooltip mechanics today

### 2.1 Code
- `primitives/src/chart/components/tooltip.rs:195-` `ChartTooltip`. Always rendered. `data-state="open|closed"`, hidden by CSS `display:none` when closed (`preview/src/components/chart/style.css:~245`).
- Position (tooltip.rs:206-246): `active_index` -> `layout.anchor_percent[i]` -> inline `style="left:X%;top:Y%"` (position_style at l.238). Fallback `(50.0, 50.0)` (l.214) whenever no anchor exists.
- The percentages are of the svg viewBox, valid because `.dx-chart` is a CSS grid with `[data-slot=chart]` and the tooltip sharing one grid area (`style.css` `.dx-chart` rule at the top of the file).
- CSS placement (style.css, `.dx-chart-tooltip`): `position:absolute; transform: translate(-50%,-100%); pointer-events:none; white-space:nowrap; min-width:8rem`. With a TOP legend only (`:has(> [data-slot=chart-legend][data-align=top])`) the transform becomes `translateX(-50%)` (tooltip hangs below the anchor).
- Anchor computation, Cartesian only: `primitives/src/chart/components/chart.rs:380-406`. For each datum `x = x_scale.center(i)`, `y = y_scale.scale(max series value at i)` (the stacked top when stacked), converted to percent. The `ChartLayout` struct is in `primitives/src/chart/context.rs:~40` (`anchor_percent: Vec<(f64,f64)>`). Nothing writes it for Pie, Radar or RadialBar (an empty vec, so the tooltip falls to 50/50).
- Pointer events that update `active_index` (a `Signal<Option<usize>>` created in `container.rs:103`):
  - Cartesian: `onpointerenter` on one transparent `rect[data-slot=chart-hit-band]` per datum (chart.rs:573-590, handler at l.586). Bands cover the plot area only (`plot_y0..plot_y1`, margins excluded).
  - Horizontal bars: their own hit bands, `series/bar.rs:495-525` (handler l.520).
  - Pie: `onpointerenter` on each arc (`series/pie.rs:322`).
  - Radar: hit path per category (`series/radar.rs:355-366`).
  - Radial: arcs (`series/radial.rs:237` for rings, `:323` for stacked).
  - Clear: `onpointerleave` on the `<svg>` (chart.rs:515). That is the only way a hover closes.
  - There is NO `onpointermove` / `onmousemove` anywhere in the tooltip path. The tooltip never reads mouse coordinates.
- Keyboard (chart.rs:471-496, `keyboard: bool` default true, chart.rs:173): the `[data-slot=chart]` wrapper is `tabindex=0 role=group aria-roledescription=chart`. ArrowRight/Left step (clamped, RTL-aware via `direction.resolve_horizontal`), Home/End jump to the first/last, Escape closes. Focus ring is `box-shadow: var(--dx-ring)` on `:focus-visible` (style.css, `.dx-chart [data-slot=chart]:focus-visible`).
- Tooltip root is `aria-hidden="true"`; the hidden `<table>` is the AT path (chart.rs doc + tooltip.rs doc).
- Active feedback marks: cursor line for area/line (`chart-cursor-line`, dashed 4 4, chart.rs ~555), grey translucent `chart-cursor-rect` for bars (opacity 0.1), line dot grows from r=3 to r=6 on the active point (`series/line.rs:209`), pie/radial arc `transform: scale(1.05)` with a 120ms transition (style.css, `[data-slot=chart-arc]`).
- Transitions on the tooltip: none. Computed `transition` is `all 0s ease 0s`, so it snaps between anchors. Edge flipping or clamping: none (fixed `translate(-50%,-100%)`).
- Props worth knowing for the redesign (tooltip.rs:78-153): `label_format`, `value_format`, `hide_label`, `hide_indicator`, `indicator: TooltipIndicator::{Dot,Line,Dashed,None}`, `label_key`, `name_key`, `formatter: Callback<TooltipRow, Element>` (rows carry key/label/value/color/index/is_last/total), `children` (full custom content).

### 2.2 Browser measurements (1280x900, Chromium, light)
Area chart svg: x=411.5, w=592. Tooltip: 152x48 (label + one row). Hit bands: first one starts about 24px into the svg (mouse at x=435 = closed; at x=1001 = closed for the right margin). Mouse swept across in 12 steps at mid height, then vertically at x=40%.

| Observation | Result |
|---|---|
| Horizontal | Tooltip snaps between 6 discrete anchors (one per datum). Every pair of adjacent mouse positions inside the same band gives the identical bbox. Tooltip centre x = `anchor.x` (e.g. 475.2, 568.1, 661.0, 754.0, 846.9, 939.8), not the mouse x. |
| Vertical (area, bar) | Mouse moved from y=514 to y=751 at fixed x: the bbox did not change at all (585.0, 544.6, centre 661.0). No y tracking. |
| Vertical position | `bottom = anchor y` = top of the tallest series value at that datum (e.g. Apr (low value) -> tooltip bottom 716.5; Feb (peak) -> 541.2). The tooltip floats up from the data point, independent of the cursor. It sits above the anchor, but when the anchor is low the tooltip body overlaps other marks (`area_chart-card-hover.png`: it covers the area fill; `bar_chart-card-hover.png`: it overlaps the cursor band, though the active bar's own top is hidden behind it). |
| Edges | Index 0 tooltip left edge is 399 (svg left 411.5): hangs 12px outside the svg on the left; last index right edge 1015.8 vs svg right 1003.5: 12px outside on the right. At 390px viewport (svg 54..323): left tooltip x=6.9 (22px past the card edge at 29), right tooltip right edge 370 (inside viewport). No flip/clamp, so at narrower widths it can leave the card and, on the left, approach the viewport edge. |
| Leave | `pointerleave` on svg -> `data-state=closed` within 150ms. |
| Keyboard | Focus the wrapper, ArrowRight x1 -> index 0 tooltip, ArrowRight x2 -> index 1, End -> last, Escape -> closed. Tooltip uses the same anchors. Works identically on pie/radar/radial (state flips open, but see the polar behaviour below). |
| Touch (hasTouch context, tap on the middle of the area chart) | Tooltip state `closed` after the tap (pointerenter fires but pointerleave on release closes it). No persistent touch tooltip. svg `touch-action: auto`. |

Bar chart: identical anchors (anchor is the tallest bar's top, same percentages), tooltip 152x30 (`hide_label: true`, one row, no heading), a grey cursor band behind the active bar.

Polar families (pie, radar, radial): confirmed in the browser that the tooltip is always pinned to the chart centre, regardless of where the mouse is.
- Pie: all eight sample points around the ring give the identical bbox (630.5,617.4 152x30), centred on the svg centre (707,647), and style `left:50%;top:50%`. Hovering the 5 different slices only changes the content. The tooltip shows the SERIES label ("Visitors 275"), not the slice name ("Chrome"), because `ChartTooltip` iterates `config.series` (documented limitation, `series/pie.rs:28-46`). `hide_label: true` in the default demo.
- Radar: identical (630.5,556 152x48, `50%/50%`) for every vertex hover. Radar's own file documents it (`series/radar.rs:42-49`).
- Radial: identical (630.5,574.4 152x30, `50%/50%`). The centre hole is closed (no hit area).
- The legend for single-series pie is also wrong for the same reason (one swatch per series, not per slice, `pie.rs:28-46`).

Verdict for the shadcn-style rebuild: our tooltip is an anchored data-point tooltip with a percentage position from the svg box, no cursor following, no y-tracking, no flipping, no animation, and polar families have no real positioning at all. Following the mouse (shadcn/Recharts default) would need either an `onpointermove` that stores client coords relative to `[data-slot=chart]` (a new signal in `ChartContext`, replacing `anchor_percent` for the non-anchored mode) or switching the tooltip to a per-hit-band `onpointermove` writing a `(x%,y%)` pair. The percent-of-svg-box containing-block trick in `style.css` (`.dx-chart` grid-area) already gives the right coordinate frame for that.

---

## 3. Card chrome and visual tokens

### 3.1 Card chrome in our demos
- Component: `preview/src/components/card/style.css:1-60`. Measured (light): `border-radius 16px` (`--dx-radius-2xl`), `1px solid rgb(229,229,229)`, `padding 24px 0`, `box-shadow 0 2px 10px rgb(0 0 0 / 10%)`, white background, rendered 642px wide (`width:100%; max-width:40rem` from `main.css:1485-1510`). Title: 16px/600 (`.dx-card-title`), description 14px muted (`--secondary-color-5`), `CardContent` padding 0 24px, `CardFooter` is a flex row.
- Structure per demo: `Card { CardHeader { CardTitle, CardDescription [, CardAction] } CardContent { ChartContainer { Chart, ChartTooltip [, ChartLegend] } } CardFooter {...} }`. Same as shadcn's (header title and description, optional action, content, footer).
- The footer is NOT standardized. Five patterns:
  1. area: inline `style` flex/grid with a `TrendingUp` icon (9 of 10 variants; inline styles, no class).
  2. line: inline-style column variant (5 variants have footers).
  3. bar: `dx-bar-chart-footer*` classes (`bar_chart/style.css:23-41`).
  4. pie/radial: `dx-chart-footer`, `dx-chart-footer-trend`, `dx-chart-footer-caption` (`chart/style.css`, end of file).
  5. radar: all 14 variants use bare `div`s in a default `CardFooter` flex row. Visible defect: in `radar_chart-card-hover.png`, "Trending up by 5.2% this month [icon]January - June 2024" renders side by side with no gap instead of two stacked lines.
  - chart_tooltip and line (5 of 10) variants have no footer at all. This is a good candidate for one shared `ChartCardFooter` or a `dx-chart-footer` with an icon slot, replacing all five patterns, per the CLAUDE.md "same problem more than once" rule.
- The demo sits inside the docs "preview frame" (`.dx-component-preview-frame`, `main.css:944-960`: dotted radial-gradient background, 1px border, 0.5rem radius, 1.75rem padding), under a Ghost tab bar `DEMO | CODE`. shadcn's gallery shows a bare card in a grid with no frame and no tabs.

### 3.2 Visual tokens (measured and from source)
| Token | Ours | Source |
|---|---|---|
| Grid lines | horizontal only, SOLID (`stroke-dasharray: none`), 1px, `--dx-chart-grid` = `--primary-color-6` (rgb 229,229,229 light); 8 lines for the area default (zero line extra on bars) | `layout.rs:295-318`, `style.css` `[data-slot=chart-grid] line` |
| Axis lines | none (no axis line or tick marks; only tick LABELS, 12px, `--dx-chart-axis-text`; x-axis ticks are first 3 chars by default; y-axis off by default) | `layout.rs:320-380`, `chart.rs:136-147` |
| Svg | default viewBox 600x300 (`chart.rs` width/height props), text-scale compensation var `--dx-chart-text-scale` | style.css `chart-svg` |
| Bars | `rx: var(--dx-radius-xs)` = 2px on ALL four corners, uniform (no per-corner radius, documented gap: `series/bar.rs:42-44`, `bar_chart/variants/stacked_legend/mod.rs:8-23`). shadcn uses `radius=8` default / `[4,4,0,0]` stacked | style.css |
| Bar band | band width 55.7 of ~92 pitch (about 0.6) | measured |
| Area | fill = series colour at `fill-opacity 0.4`, 2px stroke line (`monotone` default, `Curve::{Monotone,Linear,Step}`), optional gradient via `AreaOptions.gradient` (stop opacities 0.8 to 0.1, `series/area.rs:63-125`) | style.css, `series/area.rs` |
| Line | 2px stroke, round caps; dots only when `dots` is set (`LineOptions.dot_radius` default 3, active dot r doubles to 6, stroke `--primary-color-1` 1.5px) | `series/line.rs:80-112, 209`, style.css |
| Cursor | area/line: dashed line `4 4`, 1px, `--dx-chart-axis-text`; bars: translucent rect, opacity 0.1 | style.css |
| Palette | `--dx-chart-1..8` in `preview/assets/dx-components-theme.css:234-241`: blue `#2a78d6`, orange `#eb6834`, green `#1baf7a`, amber `#eda100`, pink `#e87ba4`, green `#008300`, purple `#4a3aa7`, red `#e34948` (light; dark variants differ) | theme css |
| Tooltip surface | `--primary-color-2` bg, `--dx-radius-lg` 8px, `--dx-shadow-md`, 12px text, 8rem min width, padding `--dx-space-2` x `--dx-space-3`; label 600; swatch 10px square with `--dx-radius-xs` corner (shadcn: 10px rounded-2px as well); line indicator 4px wide; dashed `1.5px dashed` | style.css |
| Legend | `ul.dx-chart-legend` centred, 14px muted, 10px square swatch or 12px icon; `data-align=top|bottom` | style.css, `legend.rs` |
| Radar | polygon fill `fill-opacity` via `RadarOptions.fill_opacity` (default demo 0.6), 2px stroke, grid rings + spokes `--dx-chart-grid` | style.css |
| Arcs | `transform: scale(1.05)` on hover/active, 120ms ease; labels have a surface-colour halo (`paint-order: stroke`) | style.css |
| Donut centre text | primary `--dx-text-2xl` 700, secondary `--dx-text-sm` | style.css |

### 3.3 Animation
- Entry animation: none. `document.getAnimations().length === 0` on area, bar and line pages after load. No `@keyframes` in any chart CSS. The only chart transition in the CSS is the arc scale (120ms, `style.css`, `[data-slot=chart-arc]`). Recharts animates on mount (shadcn default). To match shadcn we would add a CSS `clip-path`/`stroke-dashoffset`/`transform: scaleY` load animation (reduced-motion aware).
- Tooltip: no transition, no fade (`display:none` toggle).

### 3.4 Legend interactions
- None. `legend.rs` has no `onclick`/`onpointer*`; items are non-interactive `li`s (series swatch + label, `role=graphics-symbol` on the swatch). No series toggling, no hover-to-highlight. (`bar_chart` main and `area_chart` main variants have no legend at all; legends appear in the `legend`, `stacked_legend`, etc. variants.)
- Interactive demos are driven by external controls instead: `Select` for time range (area), `Select` for month (pie), a custom "pick a series" button group for bar/line (`bar_chart/variants/interactive/mod.rs:93`, `line_chart/variants/interactive/mod.rs:117`).

---

## 4. Inventory (variants per type; `main` = default/first variant)

Key: OK = exists and cites the matching upstream demo (every variant doc comment names its `chart-*.tsx` source). "~" = exists with a documented simplification. GAP = missing.

### Area (`area_chart`, 10 variants, `components/mod.rs:234`)
| shadcn | ours | note |
|---|---|---|
| default | `main` | OK |
| linear | `linear` | OK |
| step | `step` | OK (its doc comment says the series icon is not ported, but an `icons` variant exists) |
| legend | `legend` | OK |
| stacked | `stacked` | OK |
| stacked-expanded | `stacked_expand` | OK (`AreaOptions.stack_mode`) |
| icons | `icons` | OK |
| gradient | `gradient` | OK (`AreaOptions.gradient`) |
| axes | `axes` | OK |
| interactive | `interactive` | OK (stacked, `Select` range 90/30/7 days; 228 lines) |
Gaps: none.

### Bar (`bar_chart`, 11 variants, mod.rs:238)
| shadcn | ours | note |
|---|---|---|
| default | `main` | OK |
| horizontal | `horizontal` | OK |
| multiple | `multiple` | ~ shadcn uses a DASHED tooltip indicator. `TooltipIndicator::Dashed` now exists but this demo still uses the default dot (its doc comment is stale and says there is no indicator prop). Easy fix. |
| stacked + legend | `stacked`, `stacked_legend` | ~ no per-corner radius (uniform `rx` 2px; shadcn rounds only the stack's outer corners) |
| label | `label` | OK (`BarOptions.value_labels`) |
| custom label | `label_custom` | OK (`inside_labels`) |
| mixed | `mixed` | OK |
| active | `active` | OK (`active_index`, dims others via `:has()`) |
| negative | `negative` | OK |
| interactive | `interactive` | OK |
Gaps: none by count; fidelity gaps are per-corner radius and the dashed indicator.

### Line (`line_chart`, 10 variants, mod.rs:269)
| shadcn | ours |
|---|---|
| default | `main` |
| linear | `linear` |
| step | `step` |
| multiple | `multiple` |
| dots | `dots` |
| custom dots | `dots_custom` |
| dots colors | `dots_colors` |
| label | `label` |
| custom label | `label_custom` |
| interactive | `interactive` |
Gaps: none. All OK. Five of the ten variants have no `CardFooter` (see 3.1).

### Pie (`pie_chart`, 11 variants, mod.rs:275)
| shadcn | ours | note |
|---|---|---|
| simple | `main` | OK, but the tooltip shows the series name ("Visitors") instead of the slice, pinned at the centre |
| separator none | `separator_none` | ~ visually identical to `main` (we draw no separator at all, so there is nothing to remove; shadcn's default pie HAS a white separator stroke, so `main` differs from shadcn instead) |
| label | `label` | ~ labels centred inside slices (no outside leader lines) |
| custom label | `label_custom` | OK |
| label list | `label_list` | OK |
| legend | `legend` | ~ `ChartLegend` is per series, not per slice (documented `pie.rs:28-46`), so the demo needs its own legend handling |
| donut | `donut` | OK |
| donut active | `donut_active` | OK |
| donut with text | `donut_text` | OK |
| stacked | `stacked` | OK (concentric rings) |
| interactive | `interactive` | OK (`Select` of months) |
Gaps: per-slice tooltip names and legend (a class of the same root cause as the polar tooltip positioning).

### Radar (`radar_chart`, 14 variants, mod.rs:278)
| shadcn | ours | note |
|---|---|---|
| default | `main` | OK |
| dots | `dots` | OK |
| lines only | `lines_only` | OK |
| custom label | `label_custom` | ~ standard category labels only; shadcn renders a custom multi-line tick (documented in the variant) |
| grid custom | `grid_custom` | OK |
| grid none | `grid_none` | OK |
| grid circle | `grid_circle` | OK |
| grid circle no lines | `grid_circle_no_lines` | OK |
| grid circle fill | `grid_circle_fill` | OK |
| grid fill | `grid_fill` | OK |
| multiple | `multiple` | OK |
| legend | `legend` | OK |
| icons | `icons` | OK |
| radius axis | `radius` | ~ no PolarRadiusAxis; documented deviation |
Gaps: radius axis overlay, custom tick renderer. All 14 variants use an unstyled footer (3.1).

### Radial (`radial_chart`, 6 variants, mod.rs:279)
| shadcn | ours |
|---|---|
| simple | `main` |
| label | `label` |
| grid | `grid` |
| text | `text` |
| shape | `shape` |
| stacked | `stacked` |
Gaps: none by count. Tooltip is pinned at the centre (2.2).

### Tooltip (`chart_tooltip`, 9 variants, mod.rs:246; all built on `ChartKind::Bar`)
| shadcn | ours |
|---|---|
| default | `main` |
| line indicator | `indicator_line` |
| no indicator | `indicator_none` |
| (extra, not in shadcn's list) | `label_none` |
| custom label | `label_custom` |
| label formatter | `label_formatter` |
| formatter | `formatter` |
| icons | `icons` |
| advanced | `advanced` |
Gaps: none. Note there is no `dashed` indicator demo anywhere even though the primitive supports it (a bar `multiple` use case, see above).

### Generic `chart` package (not in shadcn's gallery): `main` (area), `bar`, `line`, `stacked` (`mod.rs:245`).

Totals: 10 + 11 + 10 + 11 + 14 + 6 + 9 = 71 variant demos (plus the 4 `chart` ones). Every shadcn gallery name has a counterpart, so the rebuild is about presentation (gallery layout, card chrome, tooltip behaviour), not missing demos.

### Cross-cutting fidelity gaps worth fixing as classes
1. Tooltip not anchored to the cursor, and polar tooltip pinned at 50/50 (same root: `ChartTooltip` only knows Cartesian `anchor_percent`). Affects pie (11 variants), radar (14), radial (6), 31 demos.
2. Pie and radial tooltip/legend are per series, not per slice/datum (pie `main`, `legend`, etc.).
3. No touch support: tooltip closes on release.
4. No entry animation, no tooltip fade/slide.
5. Uniform 2px bar radius (shadcn: 4 to 8, per-corner on stacks).
6. Card footer is five different patterns, with radar unstyled.
7. Legend is static (no series toggle).
8. `bar_chart/multiple` does not use `TooltipIndicator::Dashed`; stale doc comments in `bar_chart/multiple`, `area_chart/step`.
