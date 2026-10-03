# shadcn/ui /charts vs dioxus-components: UX side by side

> **Snapshot 2026-10-03.** Evidence files this report cites by relative name (screenshots, JSON probe dumps, probe scripts) were scratch artifacts of the session and are **not committed**; the findings and numbers are kept. See [README.md](./README.md).

Evidence: shadcn = `./shadcn-ux-spec.md` (measured live 2026-10-03). Ours before = live site (`mentalgear.github.io/dioxus-components`, which predates `98c430f..HEAD`) and `./ours-audit.md` (the same tooltip code). Ours now = the HEAD build on `127.0.0.1:8797`, measured with Playwright/Chromium. Scripts: `s*.mjs`; screenshots: `*.png` in this folder.

Status: **matches** / **deliberate difference** / **gap**.

## Gallery IA

| Area | shadcn | Ours before (live) | Ours now (8797) | Status |
|---|---|---|---|---|
| Route | `/charts/<type>`, tabs link to `#charts` | none (`/charts/` is 404) | `/charts/` (Area) and `/charts/<slug>/` for 7 slugs, all prerendered | matches |
| Hero | pill, H1 48px/600/-2.4px, 2-line intro, Browse Charts (solid) + Documentation (muted) | none | H1 48px/600/-1.92px/52.8px "Beautiful Charts for Dioxus", intro, the same two pill buttons. No announcement pill. | matches (copy differs) |
| Tab row | 7 text links, 16px/500, `h-7 px-4`, muted with a black active tab, no indicator; scrolls horizontally on phones | none | 16px/500, 32px tall, `0 16px` padding, `aria-current=page`, scrolls at 390 | matches |
| Tab navigation scroll | lands on `#charts`, so the tab row stays in view | n/a | SPA push to `/charts/bar/?` scrolls to the **top** (scrollY 291 -> 0); focus drops to `body` | **gap** |
| Grid | 1 / 2 / 3 columns; gap 40/24/24/40; gutter 24/24/40/52 at 390/768/1024/1440 | n/a | 1 / 2 / 3 columns; gap 40/24/24/40; gutter 16/24/40/73 (1280 max width); cell 343/341/294/400 | matches (1440 cell is 400 vs 419; phone gutter 16 vs 24) |
| Hero card | interactive Area/Bar/Line spans the full row | n/a | same (`data-span=full`) | matches |
| Empty dashed filler boxes | 2-3 per page | n/a | none | deliberate difference (good) |
| Iframe lazy-load and fade-in | `loading=lazy`, 300ms opacity fade | n/a | inline render; the whole page is SSR'd (area page 186 KB of HTML) | deliberate difference |
| Page title | "Beautiful Charts & Graphs" etc. | n/a | "Dioxus Componentsdioxus \| ..." (generic) | gap (nit) |

## Card chrome

| Area | shadcn | Ours before (live) | Ours now (8797) | Status |
|---|---|---|---|---|
| Card frame | bare card in the grid, 14px radius, 1px border, no shadow | card inside a dotted "preview frame" with DEMO / CODE tabs, 16px radius, shadow | bare card, 16px radius, 1px border, no shadow | matches |
| Toolbar | type icon + label on the left; Copy (ghost 24px, "Copy code" tooltip), separator, outline "View Code" on the right | DEMO / CODE tabs and a copy button inside the code tab | same layout and the same tooltip | matches |
| Copy | full self-contained TSX on the clipboard | copies the visible `<pre>` text; check icon never resets | copies the variant's `mod.rs` and resets after 1.6s. But the text starts `use super::super::component::*;` (does not resolve in a user's app), and 32 of the variant files carry internal `$S/refs/...` research comments. | **gap** (copy-paste fidelity) |
| View Code, desktop | 700px right Sheet, 50% overlay, filename bar + copy + "Open in v0", line-numbered Shiki | inline CODE tab | 700px Sheet, filename bar + copy, highlighted source. Esc, overlay and X close it, and focus returns to "View Code". | matches, except no line numbers and no "Open in" |
| View Code, phone | vaul bottom drawer with a grabber | inline CODE tab | bottom Drawer with a grabber; drag-down and an overlay tap dismiss it; horizontal and vertical code swipes work (scroll-lock fix: 0 -> 243px) | matches |
| Footer | "Trending up ... + icon" over a muted caption | five divergent patterns; radar's two lines ran together | one `dx-chart-footer` construction everywhere | matches |
| Interactive header (bar/line) | stat buttons separated by a left border, active `bg-muted/50` | stat buttons drawn with UA borders; black text on the dark card | color fixed; the UA `2px outset` border is still on top, right and bottom (`bar_chart/style.css:52` never resets `border`) | **gap** |
| Hero chart height | `h-[250px] w-full` at every width | about 460px tall at 1440 and a 40px plot at 390 | `fit_width` gives 250px at every width; ticks thin to the real width (12 -> 9 -> 5). Before hydration (SSR/no-JS, about 2.8 MB gz of wasm) the chart is drawn 700 wide, left-aligned in a 1230 box at 1440 and shrunk to ~5px text at 390, then jumps (`hero-*-nojs.png`). | matches after hydration; **gap** before it |
| Pie card size | `aspect-square max-h-[250px]` | 600x300 viewBox, small | unchanged: the pie sits in a 350x175 box (about 150px across) | gap |
| Tooltips tab | every card shows its tooltip already open (`defaultIndex`) | n/a | nine identical-looking bar charts until you hover one | **gap** |

## Tooltip

| Area | shadcn | Ours before (live) | Ours now (8797) | Status |
|---|---|---|---|---|
| X snap | snaps to the nearest point/category, switching at the midpoint | snapped to the hovered hit-band's center | nearest category from coordinates, switching at the midpoint; `tx = pointX + 10` exact (January 47.6 = 37.6 + 10) | matches |
| Y follow | `ty = mouseY + 10` | fixed at the top of the tallest value | `ty = mouseY + 9..10` (pointer y is truncated to an integer `clientY`, so up to 1px short) | matches |
| Horizontal flip | `x - w - 10` when `x + 10 + w > chartWidth` | none (overhung the card by 12px) | same rule (April at 202.5 -> 40.5) | matches |
| Vertical flip | above the cursor when it would overflow the plot bottom | none | same rule against the chart box (y=140 -> 81) | matches |
| Clamp | wrapper is clipped to `.recharts-wrapper` | none | `place_tooltip` clamps into the chart box | matches |
| Hidden outside the plot | yes | the margin bands were dead | yes (x=3.5 and x=346 closed, y >= 157 closed) | matches |
| Motion | `transform 400ms ease` | none | `transform 200ms ease-out`, about 180ms to settle | deliberate difference |
| Fly-in from (0,0) | yes (a quirk) | n/a | none: the first visible frame is already at the target (hidden until measured) | deliberate difference (better) |
| Appear / disappear | instant, no fade | instant | instant | matches |
| Reduced motion | not handled | n/a | `transition: none` | better |
| Pie | anchored at the slice centroid, jumps between slices, shows slice name and value | pinned at the chart center; showed the series label "Visitors" | centroid anchor, slice name, value and color; multi-ring pie follows the pointer | matches |
| Radar | follows the pointer along the active spoke | pinned at center | follows the pointer; the sector is the angular wedge out to the outer radius | matches |
| Radial | anchored near the bar end | pinned at center | follows the pointer inside the ring's swept arc | deliberate difference (minor) |
| Touch | Recharts: tap shows, no persistence guarantees | a tap opened then closed it at once | tap opens and stays; horizontal drag scrubs (Feb -> Jun); vertical drag scrolls the page (171px); a tap elsewhere or on another chart closes it | better. But `touch-action: pan-y` also turns off pinch-zoom on every chart. |
| Keyboard | Tab focuses and shows the first point immediately; arrows step | arrows step, anchored at the old position | arrows, Home, End and Esc step. The tooltip hangs off the data point with the same flip rules; Tab away closes it. Focus alone does not open it. | matches, except no open-on-focus |
| Page scroll / RTL | n/a | n/a | correct after scrolling (fresh rect per event); correct under `dir=rtl` (physical frame) | matches |
| Scaled ancestor (`transform: scale`, CSS `zoom`) | n/a (Recharts uses offsets) | n/a | wrong: screen-px pointer delta used as local px (scale 0.6: tooltip lands above-left of the pointer; zoom 1.5: 1.5x too far) | gap (edge) |
| Visual | `min-w-8rem`, `border-border/50`, `rounded-lg`, `px-2.5 py-1.5`, `shadow-xl`, mono value | 152px wide, 8px radius, shadow | 128px min-width, 8px radius, `8px 12px` padding, no border, a soft shadow, tabular-nums but not mono | near match |

## Cursor, dots, legend, load animation

| Area | shadcn | Ours before (live) | Ours now (8797) | Status |
|---|---|---|---|---|
| Cursor | none by default; a muted band only on bar-interactive and stacked bars; a 1px solid line on line-interactive | dashed line (area/line), 10% band (bars) | unchanged: a dashed 4/4 line on every area/line chart and a 10% band on every bar chart; horizontal bars now get a row band | gap (deliberate?) |
| Active dots | r=4 dot on each hovered series point (area/line/radar) | line dot grows from 3 to 6 only when dots are on | unchanged: no active dot on area or plain line charts | gap |
| Legend | passive, centered, 8px rounded swatch, 12px | passive, 10px swatch | unchanged (10x10, 2px radius, passive) | matches |
| Load animation | 400ms grow/draw | none | none | gap |

## Tokens

| Area | shadcn | Ours before (live) | Ours now (8797) | Status |
|---|---|---|---|---|
| Palette | blue ramp theme on /charts | multi-hue `--dx-chart-1..5` | unchanged | deliberate difference |
| Grid | horizontal only, 1px, `border/50` | horizontal, 1px, #e5e5e5 | unchanged | matches |
| Axis ticks | 12px, muted `#666`, no lines | 12px effective, near-black | unchanged (fill rgb 17,17,17) | gap (minor) |
| Bar radius | 8 (vertical), 5 (horizontal) | 2px | unchanged (2px) | gap |
| Area fill | 0.4, or a 0.8 -> 0.1 gradient | 0.4; gradient variant exists | unchanged (the gradient variant reads almost flat in the 1440 screenshot) | near match |

## Interactive charts and docs

| Area | shadcn | Ours before (live) | Ours now (8797) | Status |
|---|---|---|---|---|
| Area range Select | 3m / 30d / 7d re-slice | present (inside a 460px hero) | present. Before hydration it reads "Select an option". | matches |
| Bar/line series toggle | stat buttons switch the series | present; the line stylesheet was unlinked, ticks read "Apr Apr Apr" | fixed (ticks "Apr 1, Apr 9, ..."); the bar UA border remains | near match |
| Docs page | `/docs/components/chart`: install, first chart, config, theming, tooltip, legend, a11y, RTL | `/component/chart/` with usage notes, install and variants | the same plus tooltip position/touch notes, `fit_width` docs, and a "Browse all charts" link | matches in substance |
| Accessibility | `accessibilityLayer`: focusable svg, arrows, no label by default | named `role=group` wrapper, arrows, hidden data table | unchanged plus `tabindex=-1` on non-keyboard charts (for tap dismissal); gallery tabs use `aria-current`; sheet/drawer have a title and description and return focus | better than shadcn |
