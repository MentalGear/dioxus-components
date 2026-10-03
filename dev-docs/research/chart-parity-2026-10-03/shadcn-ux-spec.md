# shadcn/ui Charts gallery and docs: UX spec

> **Snapshot 2026-10-03.** Evidence files this report cites by relative name (screenshots, JSON probe dumps, probe scripts) were scratch artifacts of the session and are **not committed**; the findings and numbers are kept. See [README.md](./README.md).

Source: live https://ui.shadcn.com on 2026-10-03, driven with Playwright/Chromium at 1440 and 390 widths, light and dark. Recharts v3 (the docs say "now uses Recharts v3"), Tailwind v4, Next.js.

Method notes:
- Chromium could not load ui.shadcn.com assets directly through the sandbox proxy (`ERR_TOO_MANY_RETRIES`). Every request was routed through `curl` instead (`lib.mjs`). This only affects how I fetched pages, not what the site does.
- Every gallery chart is an `<iframe src="/view/new-york-v4/chart-xxx" loading="lazy" height=460>`. I measured tooltips by opening those view pages directly at 419px wide (the 3-column card width at 1440) or 1336px wide (full-width cards).
- Raw data: `tip-*.json` (hover traces), `survey-*.json` (every chart), `tokens.json`, `dom1.json`.

---

## 1. Page and IA structure of /charts/*

Routes: `/charts/area`, `/bar`, `/line`, `/pie`, `/radar`, `/radial`, `/tooltip`. Each route is a separate page with the same chrome. The tab links point to `/charts/<type>#charts` (hash anchor), so switching tabs is a navigation, not client-side tab state.

Top to bottom at 1440:
1. **Sticky site header**, 64px tall, `position: sticky; top: 0`. Left: Home, Docs, Components, Blocks, Charts, Directory, Typeset, Create. Right: search field, GitHub stars (125k), theme toggle, black "+ New" button. At 390 it collapses to a "Menu" button, the star count, the theme toggle and "New".
2. **Hero, centered**:
   - A pill linking to "New Questionnaire component ->".
   - H1 "Beautiful Charts & Graphs": 48px, weight 600, letter-spacing -2.4px, line-height 52.8px, Geist. It wraps to 2 lines at 390.
   - Intro: "A collection of ready-to-use chart components built with Recharts. From basic charts to rich data displays, copy and paste into your apps." About 18px, 2 lines at desktop, 4 lines at 390.
   - Two pill buttons: **Browse Charts** (solid, black in light mode, white in dark) and **Documentation** (muted fill).
   - Vertical rhythm at 1440: pill at y≈155, H1 at y≈208, intro at y≈265-293, buttons at y≈349, tab row at y≈476.
3. **Chart-type nav row**: plain text links, one per chart type, no underline and no background.
   - Items: Area Charts, Bar Charts, Line Charts, Pie Charts, Radar Charts, Radial Charts, Tooltips.
   - Style: 16px, weight 500, `h-7 px-4` (28px tall, 16px horizontal padding, so item gaps are 32px).
   - Muted gray by default; active is black (light) or white (dark), with `data-active=true`. Hover darkens to primary. There is no indicator bar, only the color change.
   - At 390 the row scrolls horizontally and is cut off at "Line Charts".
4. **Gallery section**, with an `sr-only` `<h2>` ("Area Charts"). There are no visible section headings. The grid is `grid gap-10 md:grid-cols-2 md:gap-6 lg:grid-cols-3 xl:gap-10 items-stretch`.
5. **Footer**: "Built by shadcn at Vercel. The source code is available on GitHub." 14px, muted, centered.

### Gallery grid

| Viewport | Columns | Gap | Page gutter | Cell width | Note |
|---|---|---|---|---|---|
| 390 | 1 | 40px | 24px | 342 | |
| 768 | 2 | 24px | 24px | 348 | hero chart spans 2 columns (720) |
| 1024 | 3 | 24px | 40px | 299 | hero chart spans 3 columns (944) |
| 1440 | 3 | 40px | 52px | 419 | hero chart spans 3 columns (1336) |

- The first card in Area, Bar and Line is the "Interactive" chart. It uses `md:col-span-2 lg:col-span-3`, so it is full width.
- Pie, Radar, Radial and Tooltip have no hero chart. They are all 1/3-width cards.
- Cell structure: a `.themes-wrapper` flex column containing a **toolbar row** (about 44px) above an **iframe** (460px tall).
- At 1440 the row pitch is 544px (504 + 40 gap). The iframe is 460px tall but the card inside is only about 388px (default) or 409px (interactive), so each cell ends in blank space. In the gallery this is what makes rows look airy.
- Cards per page: Area 10, Bar 10, Line 10, Pie 11, Radar 12, Radial 6, Tooltip 9. Each page ends with 2-3 empty dashed-border placeholder boxes (visible in the screenshots). They look like a lazy-load or empty filler artifact. Do not copy them.
- Iframes start at `opacity-0` and fade in over 300ms (`transition-opacity duration-300`) after load. Below-the-fold charts only render on scroll (`loading=lazy`).
- Theme: the whole /charts area is a "blue" theme. `--chart-1..5` are the same in light and dark (see section 5).

---

## 2. Per-card chrome

Toolbar row above each card (`relative z-20 flex justify-end px-3 py-2.5`):
- **Left:** a lucide chart-type icon (area, bar and so on, 0.9rem) plus the type name ("Area Chart"). 13px, muted.
- **Right:** a ghost icon button "Copy" (24x24, `rounded-[6px]`, tabler copy icon, `hover:bg-muted`), a 1px x 16px vertical separator (hidden below `md`), and an outline button "View Code" (small).
- There is no "Open in v0" on the card. It appears only inside the code view.

**Copy click:** the full TSX source of that chart goes to the clipboard (verified: 2451 chars, starting `"use client"` + imports). The hover tooltip reads "Copy code" (black pill above the button).

**View Code click:**
- On desktop (>= 768) a right-side **Sheet** slides in. It is `inset-y-0 right-0 w-3/4 sm:max-w-sm md:w-[700px] md:max-w-[700px]`, with a 50% black overlay. Open animation is 500ms slide-from-right and close is 300ms. A tiny close X sits at the top right.
- On mobile (390) it becomes a **bottom Drawer** (vaul): it starts about 170px from the top, has a grabber bar of 100x8px, and uses `transform 0.5s cubic-bezier(0.32,0.72,0,1)`.
- The content has a single file view, no tabs:
  - A figure with a 48px caption bar: a TS file icon, the filename (`chart-area-default`), a ghost copy button (tooltip "Copy code"), and a black **"Open in v0"** button.
  - Below it, line-numbered Shiki-highlighted TSX: red keywords, blue strings and numbers, mono font, about 15px, 23px row pitch.
  - The code starts with `"use client"`, then imports, then `export const description`, `chartData`, `chartConfig` (colors as `var(--chart-1)`) and the component.
- The sheet also has a `chart-wrapper` slot (`hidden sm:block`, max-h 35vh) intended for a live preview. It rendered empty in my run.
- Screenshots: `viewcode-1440.png`, `viewcode-390.png`, `copy-hover.png`.

**Card body** (the chart view page, `bg-background`, `shadcn-card shadow-none` inside the gallery):
- Border radius 14px, 1px border, `py-6`, internal `gap-6`.
- Header `px-6`:
  - Title 16px/600, line-height 16px.
  - Description 14px, muted-foreground.
  - Examples: "Area Chart - Linear", "Showing total visitors for the last 6 months".
- Content `px-6`. The chart sits inside, `aspect-video` (369x208 at 1440) or `aspect-square max-h-[250px]` for pie, radar and radial.
- **Footer** (`px-6`) has two variants.
  - Area charts: row, 16px. Line 1 is "Trending up by 5.2% this month" + a trending-up icon (font-medium, leading-none). Line 2 is "January - June 2024" in muted text.
  - Most other charts: a column with `gap-2` and `text-sm`. The same two lines, but line 2 is "Showing total visitors for the last 6 months". Pie, radial and some radar charts center-align it.
- **Interactive hero cards** have a different header:
  - `py-5`, a bottom border, no top padding, and a chart that is `aspect-auto h-[250px]`.
  - Area-interactive: a Select at the right of the header ("Last 3 months", options Last 3 months / Last 30 days / Last 7 days). It has a check mark on the selected option, and the dropdown is a popover with `shadow-md` and rounded corners.
  - Bar-interactive and Line-interactive: two stat buttons at the right of the header ("Desktop 24,828" and "Mobile 25,010"). Each is a big number (about 30px, bold, estimated from the screenshot) over a small muted label, with a left border between them. The active one has `bg-muted/50`.
  - On mobile these stat buttons drop under the title with a top border.

---

## 3. Tooltip behavior (measured)

**Implementation:** the stock Recharts v3 `<Tooltip>` wrapper with shadcn's `ChartTooltipContent`.
- The wrapper's inline style: `position:absolute; top:0; left:0; pointer-events:none; visibility:visible|hidden; transition: transform 400ms; transform: translate(Xpx, Ypx)`.
- Classes `recharts-tooltip-wrapper-{left|right}` and `-{top|bottom}` record the chosen placement.
- Computed transition: `transform 0.4s ease`.
- The wrapper is a **child of `.recharts-wrapper`** (the chart's own box), so the tooltip is clipped to and constrained by the chart area. It never leaves the card.

### Positioning rule per chart type

All numbers below are in coordinates relative to the chart box, from the 419px card (chart box 369x208, plot x 12..357, plot y 0..178). Mouse y is `my`.

**Area, Line, Radar and similar category charts**
- **x snaps to the nearest data point** (Area/Line) or the hovered category's axis position. The tooltip x is `pointX + 10`.
  - Area default points sit at x = 12, 81, 150, 219, 288, 357. Tooltip translate-x was 22, 91, 160, 229, then 150 (flipped), 219 (flipped).
  - The switch to the next point happens at the **midpoint between points** (about 46, 115, 184, 253 and 322 mouse-x).
- **y follows the mouse continuously: `ty = mouseY + 10`.** Verified at mouse y = 40, 70, 100, 114: ty = 50, 80, 110, 124 exactly.
- **Horizontal flip:** when `pointX + 10 + tooltipWidth` would exceed the chart width (369), it flips to the left of the point: `tx = pointX - tooltipWidth - 10`.
  - Example with width 128: May (x=288) gives 150 (288-128-10); June (x=357) gives 219.
  - The class changes to `-left`. The flip is instant (a different translate target, which then animates like any other move).
- **Vertical flip:** when `mouseY + 10 + tooltipHeight` overflows (about 178, the plot bottom), it flips above the cursor: `ty = mouseY - tooltipHeight - 10`.
  - A 66px tall tooltip: mouse y=100 gives ty=110 (below), mouse y=130 gives ty=54 (above).
  - Class `-top` means "placed above the cursor".
- Tooltip sizes at 419 width: 128 wide (min-w 8rem). Heights: 44 with label + 1 series, 26 without label, 66 with 2 series + label, 48 for radar (label + 1 row).
- **Hit region:** the tooltip is only visible while the mouse is inside the plot area. Mouse x at 0 and 360 (left of the first point margin or right of the last) was hidden, and y beyond about 178 was hidden. There is no hit-test padding.

**Bar charts**
- x is the center of the hovered bar's category band (+10): tx = 44.9, 104.8, 164.6, 224.4, then flip 136.25 and 196.1.
- y follows the mouse (+10) as above.
- The default shadcn examples use `cursor={false}`. Only bar-interactive, bar-stacked and line-interactive show a cursor (see below).

**Pie / donut**
- **Does not follow the mouse.** It is anchored to the **slice's centroid** (tx = 98.9, ty = 166.65 for the whole Firefox slice), so it jumps slice to slice with the 400ms transition.
- Within one slice, mouse movement makes no difference. The tooltip shows the slice name and value (no label row).
- It flips left for slices on the right (class `-left`).
- Visible only over a slice. A hole or outside the ring is hidden.

**Radar**
- It follows the mouse along the spoke of the active angle: tx and ty change by about 10 per 12px of mouse x in my trace.
- The active dot sits on the data point of the hovered angle, and the content shows the angle label ("February") plus the value.
- Flips left at the right half; clamped to a minimum of x=5 (observed `tx = 5`).

**Radial:** like pie (one tooltip per bar, anchored near the bar end). Radial-text and radial-shape had no tooltip.

### Animation and timing
- Moving between points: `transform 400ms ease`.
  - Measured x of the tooltip going 185 to 254: 22ms 186.8, 72ms 200, 122ms 219, 172ms 234, 222ms 243, 272ms 249, 322ms 252, 372ms 253.7, 422ms 254. That is about 90% of the move in 250ms, settled by about 420ms.
- **Fly-in quirk:** after the mouse has left and re-enters (and on first hover), the wrapper starts from translate(0,0) and slides diagonally from the chart's top-left corner to the target over 400ms. Measured at +40ms intervals, bounding box x went 41, 97, 157, 187, 218, 231 while the target was 229 (translate-y similar). This looks sloppy. **Recommendation: do not copy it; snap to the first position and animate subsequent moves.**
- **Appear:** no delay; `visibility: visible` on the first mousemove inside the plot. There is no opacity fade, only the transform slide.
- **Disappear:** immediate on mouseleave (`visibility: hidden` within the first 50ms sample; the content is cleared). No fade-out.
- `pointer-events: none` on the wrapper.

### Cursor, band and active dots
- **Area, Line, Radar:** active dot appears at each hovered series point: `<circle r="4" stroke-width="2" stroke="#fff">` filled with `var(--color-KEY)`. The `#fff` stroke is overridden to transparent via `[&_.recharts-dot[stroke='#fff']]:stroke-transparent`, so there is effectively no white ring. `line-dots` examples use r=6 for the active dot (dot r=3, stroke-width 2).
- **Cursor on most examples: none** (`cursor={false}`). Exceptions:
  - `chart-bar-interactive`: a rectangle band, `fill: muted` (lab 96.5% in light, about 15% in dark), width 4px (one band per bar).
  - `chart-bar-stacked`: a band 60x139px behind the hovered stack, fill muted (visible in `hover-chart-bar-stacked-dark.png`).
  - `chart-line-interactive`: a 1px vertical line, `stroke: border`, full plot height (220px), no dash.
- Bars themselves do not change on hover (fill, opacity and stroke are identical before and after).
- **Static tooltips:** the /charts/tooltip gallery shows each tooltip already open (via `defaultIndex`), so the gallery looks populated in static screenshots (`gallery-tooltip-1440-light-full.png`).

### Tooltip visual
`grid min-w-[8rem] gap-1.5 rounded-lg border border-border/50 bg-background px-2.5 py-1.5 text-xs shadow-xl`
- Label: `font-medium` 12px.
- Series name: muted-foreground.
- Value: `font-mono font-medium tabular-nums` right-aligned.
- Indicator `dot`: 10x10, 2px radius.
- Indicator `line`: `w-1` bar.
- Indicator `dashed`: dashed border.
- Row: `flex gap-2`, `justify-between`, `leading-none`.
- Dark mode: the tooltip is `bg-background` (nearly black, lab 2.75%), darker than the card (lab 7.8%).
- Variants in `/charts/tooltip`: default, line indicator, no indicator, custom label (from config), label formatter, no label, formatter (suffix "kcal"), icons, advanced (with Total row).

---

## 4. Other interaction UX

- **Legend** (`ChartLegendContent`): centered row `flex items-center justify-center gap-4 pt-3`. Each item has an 8x8 `rounded-[2px]` swatch in the series color (or an icon at 12x12 if configured), `gap-1.5`, 12px text. It is **not interactive**: cursor is default, hover does nothing, and clicking does not toggle the series (verified: the area count is unchanged after a click). It sits inside the chart box at the bottom, under the x-axis.
- **Interactive charts**
  - Area-interactive: the Select re-slices the data (3 months = 91 points, 30 days, 7 days = Jun 24..Jun 30). The axis ticks regenerate (verified: 7 ticks Jun 24..Jun 30). I did not measure any re-animation.
  - Bar-interactive and Line-interactive: the two header stat buttons switch the active series, `data-active`. The bar fill changes from `var(--color-desktop)` to `var(--color-mobile)` (single series, 91 bars). Stats are computed totals. Hover on the stat button shows a `bg-muted/50` fill.
  - Pie-interactive: has a Select with January to May (options verified). I did not test what it changes.
- **Active-shape highlight:** `bar-active` (static: the highlighted bar has a dashed outline, seen in the gallery screenshot, not measured), `pie-donut-active` (outer ring pop-out), `radial-shape`. These are static highlights from `activeIndex`, not hover reactions.
- **Load animation:** Recharts default 400ms ease (bars grow from the baseline; heights at +170ms 50%, +370ms about 97%, +570ms 100%). Lines draw via stroke-dasharray (the `line-dots` curve carries `stroke-dasharray: 478.09px 478.09px`). Areas reveal left to right. Radar and pie animate too.
- **Keyboard / a11y:** the examples use `accessibilityLayer`.
  - The `<svg role="application" tabindex="0">` is focusable. Focus ring: 50%-opacity `ring` outline on the svg.
  - Tab focuses the chart and **immediately shows the tooltip at the first point** (January, translate(22px,114px)).
  - ArrowRight and ArrowLeft step through points (Feb, Mar, Apr, then back to Mar). The tooltip y stays at about the vertical center (114) for keyboard.
  - There is no aria-label by default. `<title>` and `<desc>` are empty.
  - Docs: "turn on the `accessibilityLayer` prop ... adds keyboard access and screen reader support".
- **Hover on cards:** `.themes-wrapper` has `hover:z-30 transition-all duration-200` only; there is no visible lift.

---

## 5. Visual tokens

**Chart colors** (the /charts pages are themed blue; the values are the same in light and dark):

| Token | Value | Notes |
|---|---|---|
| `--chart-1` | `lab(77.5 -6.5 -36.4)` | Tailwind blue-300 (about `#90c5ff`) |
| `--chart-2` | `lab(54.2 13.3 -74.7)` | about blue-500 |
| `--chart-3` | `lab(44.1 29.0 -86.0)` | about blue-600 |
| `--chart-4` | `lab(36.9 35.1 -85.7)` | about blue-700 |
| `--chart-5` | `lab(30.3 27.8 -70.3)` | about blue-800 |

- The CSS bundle defines `--chart-1: var(--color-<hue>-300)` for several themes, so the blue ramp is a theme, not the default. The docs default palette is oklch orange/teal/etc.
- Series reference `var(--color-KEY)`, set per chart by `ChartContainer` from `chartConfig`.

**Theme surfaces**

| | Light | Dark |
|---|---|---|
| `--background` | white | `lab(2.75%)` |
| `--card` | white | `lab(7.8%)` |
| `--border` | `lab(90.95%)` (#e5e5e5) | `white / 10%` |
| `--muted` | `lab(96.5%)` | `lab(15.2%)` |
| `--muted-foreground` | `lab(48.5%)` | `lab(66.1%)` |
| `--radius` | 0.625rem (card = 14px) | same |

**Grid:** horizontal only (`CartesianGrid vertical={false}`), 1px solid. The raw stroke is `#ccc`, overridden to `border/50` (about `#e5e5e5` at 50%). No dash. 5 lines for the standard chart (3 when a y axis is shown). The grid starts at x=12 and spans the plot width; y spans 0..178 of 208.

**Axes:**
- X axis: `axisLine=false`, `tickLine=false`, `tickMargin=8`. Ticks are 12px, weight 400, fill `muted-foreground` (rendered `#666`), Geist, `text-anchor: middle`.
- Y axis is hidden by default. The "Axes" example shows 0 / 300 / 600 ticks.
- Hero charts show about 15 date labels ("Apr 2", "Apr 7", ...) over 91 points, so the tick gap is about 32px (assumed from the labels, not read from the source).
- Plot margins: 12px left and right (default charts); the interactive charts use 5px for the plot and 12px for the margin.
- Radar: polar grid concentric polygons, stroke `border`, angle tick labels 12px at `#808080`.

**Series:**
- Area:
  - Curve is `stroke-width 1`, `type="natural"` (smooth).
  - Fill is `fill-opacity 0.4` of the series color (default), or **gradient fills** in the gradient/interactive variants: `linearGradient 0,0,0,1` from 5% at 0.8 opacity to 95% at 0.1 opacity.
  - Stacked areas use `stackId`.
  - Interactive uses a fill-opacity of 0.6 on the curve.
  - Variants: linear, step, stacked, stacked-expand (100%), legend, icons, gradient, axes.
- Bar: `radius=8` (vertical default), `radius=5` (horizontal), other variants not measured; the gap between bars is about 13px (bar 47px in a 60px band). Mixed uses a per-bar fill. Bar labels are 12px.
- Line: `stroke-width 2`, dot r=3 (when shown), active dot r=6 in the dots variants, r=4 otherwise.
- Pie: sectors with `stroke-width 5` and white stroke, forced transparent by CSS (gaps via `paddingAngle` in some variants, none in separator-none). Donut inner radius not measured.
- Radial: background track sector `fill: muted` (`#eee` raw), `max-h-[250px]`.

**Card:** 14px radius, 1px border, `py-6`, `gap-6`, header and content `px-6`; footer text is 14px with `gap-2`. In the gallery the cards have `shadow-none`, only the border.

**Fonts:** Geist (body and charts), a mono font for the tooltip value.

---

## 6. /docs/components/chart page structure

`/docs/components/chart` redirects to `/docs/components/base/chart` (200). The page has a library switcher (Base UI / React Aria / Radix UI) and a "Copy Page" button with prev/next arrows. The left sidebar is the component list, with "Chart" highlighted. The right rail is "On This Page" plus a Vercel ad card. The content column is about 640px wide.

Outline:
1. **Chart** (H1), "Beautiful charts. Built using Recharts. Copy and paste into your apps." An "Updated: now uses Recharts v3" callout. A live preview card (Bar Chart - Interactive, with Desktop 7,324 / Mobile 7,250 stat buttons) with Copy and View Code buttons.
2. **Component**: composition philosophy, "We do not wrap Recharts", "The components are yours."
3. **Updating to Recharts v3**: use `var(--chart-1)` not `hsl(var(--chart-1))`; `defaultIndex` is for initial tooltip state only; remove `layout` from `<Bar>` when `BarChart` defines it; keep a height, `min-h-*` or `aspect-*` on `ChartContainer`.
4. **Installation**: CLI / Manual tabs.
5. **Your First Chart**: Start by defining your data, Define your chart config, Build your chart (note: set a `min-h-[VALUE]` on `ChartContainer`).
   - Add a Grid
   - Add an Axis
   - Add Tooltip
   - Add Legend
   - Each step has a live preview and a "View Code" button. "Done. You've built your first chart! What's next?" links to Themes and Colors, Tooltip, Legend.
6. **Chart Config**: labels, icons, colors; decoupled from data.
7. **Theming**:
   - CSS Variables (define in globals.css, add to `chartConfig`)
   - hex, hsl or oklch
   - Using Colors (`var(--color-KEY)`): Components, Chart Data, Tailwind
8. **Tooltip**: anatomy diagram (Label / Name / Indicator / Value); `hideLabel`, `hideIndicator`, `indicator` (dot / line / dashed), `labelKey`, `nameKey`; Props table; Colors; Custom.
9. **Legend**: `ChartLegend` + `ChartLegendContent`, Colors, Custom (`nameKey`).
10. **Accessibility**: the `accessibilityLayer` prop.
11. **RTL**: with an Arabic toggle example.

---

## Rebuild checklist (key decisions for the Dioxus gallery)

1. Page = sticky header, centered hero, chart-type link row, a 1/2/3-column grid (24-40px gap, 24/40/52 gutters), and one full-width hero card first.
2. Each card has a 44px toolbar (type icon + label left, Copy + View Code right). View Code opens a 700px right sheet on desktop and a bottom drawer on mobile, with filename, copy and "Open in" actions and line-numbered highlighted source.
3. Tooltip rule (cartesian): x snaps to the nearest data point, y follows the pointer, offset +10px on both axes. Flip horizontally when `x + 10 + width > chartWidth` and vertically when `y + 10 + height > plotBottom`. Animate the position with `transform 400ms ease`. Appear and disappear instantly, with no fade. Hide when the pointer is outside the plot. Do not copy the fly-in from (0,0) on first show.
4. Pie and radial: anchor to the slice centroid, not the pointer. Radar: follow the pointer along the active spoke.
5. Most examples have no cursor line or band, only active dots (r=4). Opt in to the band only for stacked and interactive bars, and the vertical line for the interactive line.
6. Horizontal-only 1px grid at `border/50`. Axis lines and ticks hidden, tick labels 12px muted with 8px margin. Bar radius 8. Area fill-opacity .4, or a gradient from .8 to .1.
7. Legend is passive (swatch, label, centered, 12px). Keyboard: the chart focuses with Tab and arrow keys step through points while showing the tooltip.

## Screenshots and raw data (names only; files not committed)

- Gallery, area: `gallery-area-1440-light-full.png`, `gallery-area-1440-dark-top.png`, `gallery-area-390-light-top.png`, `gallery-area-390-dark-full.png`, `gallery-area-1440-dark-full.png`, `gallery-area-390-light-full.png`
- Gallery, other types: `gallery-bar-1440-light-full.png`, `gallery-line-1440-light-full.png`, `gallery-pie-1440-light-full.png`, `gallery-radar-1440-light-full.png`, `gallery-radial-1440-light-full.png`, `gallery-tooltip-1440-light-full.png`
- View code: `viewcode-1440.png` (sheet), `viewcode-390.png` (drawer), `copy-hover.png`
- Hover and tooltip: `hover-chart-area-default-{light,dark}.png`, `hover-chart-bar-stacked-{light,dark}.png`, `hover-chart-line-interactive-{light,dark}.png`, `hover-chart-pie-donut-{light,dark}.png`, `hover-chart-radar-default-{light,dark}.png`, `hover-chart-tooltip-advanced-{light,dark}.png`, `hover-chart-area-interactive-*.png`, `hover-chart-bar-interactive-*.png`
- Interactive and keyboard: `area-interactive-select.png`, `area-interactive-7d.png`, `bar-interactive-hoverbtn.png`, `bar-interactive-mobile-active.png`, `pie-interactive-select.png`, `kbd-area-focus.png`, `kbd-area-arrow.png`
- Docs: `docs-chart-1440-light.png`, `docs-chart-1440-light-mid.png`
- Raw data: `tip-chart-area-default-{x,y}.json`, `tip-chart-bar-default-x.json`, `tip-chart-line-default-x.json`, `tip-chart-pie-simple-x.json`, `tip-chart-radar-default-x.json`, `tip-chart-area-stacked-y.json`, `survey-*.json`, `tokens.json`, `dom1.json`
