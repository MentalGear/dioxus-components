# Visual parity: final6 build vs shadcn (1440 light and dark)

> **Snapshot 2026-10-03.** Evidence files this report cites by relative name (screenshots, JSON probe dumps, probe scripts) were scratch artifacts of the session and are **not committed**; the findings and numbers are kept. See [README.md](./README.md).

Build: `docs/` = target-final SSG build of the working tree (88 routes, 0 `<style><!--node-id`), served from `final6/site` on :8801.
Evidence: `final6/pairs/<tab>-<variant>.png` (shadcn trimmed | ours light | ours dark), `final6/sheets2/s-<tab>-N.png` (3 pairs per sheet), raw cards in `final6/shots/<tab>-<variant>-<scheme>-1440.png`, phone pages in `final6/shots/phone-<tab>-390.png`, checks in `final6/checks2.json`.
shadcn references: `parityA/shots/s-chart-*.png` (Cartesian and tooltip), `parityB/shadcn/chart-*.png` (polar).

## Differences that apply to every card (not repeated per line)
- **Palette (deliberate theme):** our `--dx-chart-1..5` are blue, orange, green, amber and pink. shadcn's current site uses five blues. Series 2 and later therefore read orange or green where shadcn's read as a darker blue (bar/line interactive bars, multiple, stacked, every pie and radial). Geometry is unaffected.
- **Card width at 1440:** ours is 400px (chart 350px) and shadcn's is 419px (chart 369px). Cause: the site container (`--dx-content-width` minus 40px padding = 1280px, vs shadcn's 1336px). Every 16:9 chart is a uniform 0.95 scale of shadcn's. Gallery layout, out of scope here.

## Area
- default (main): matches (natural curve, 5 gridlines, 197/208 aspect).
- linear: matches.
- step: matches.
- stacked: matches (mobile at the bottom, desktop on top).
- stacked_expand: matches (other/mobile/desktop, 0..1 domain, top margin 12).
- legend: **now matches.** The legend sits inside the 16:9 box, the plot shrinks by 28px, Recharts drops the second gridline, and the legend reads Desktop, Mobile. Before this round the legend was below the box (plot 28px taller, card 28px taller).
- icons: **now matches**, same as legend. Small leftover: our legend icon glyphs are muted grey, shadcn's are foreground.
- gradient: **now matches.** The gradient fades from 0.8 to 0.1. Before this round it was painted as a flat colour because a stylesheet `fill` overrode the `url(#..)` attribute.
- axes: matches (y ticks 0/300/600, `left: -20`).
- interactive: matches shape, legend inside the 250px box, and the gradient (now fading). The x tick labels start "Apr 4" where shadcn's start "Apr 2". Cause: the chart is 1230px wide vs shadcn's 1334px, so preserveEnd thins the labels differently.

## Bar
- default (main): matches.
- horizontal: matches.
- multiple: matches.
- stacked + legend: **now matches.** The legend is inside the box, the plot is 28px shorter, and the gridline at 40 is dropped as in shadcn.
- stacked (bare, ours only): no shadcn equivalent.
- label: matches.
- label_custom: matches (in-bar labels, value labels outside, vertical gridlines behind the bars).
- mixed: matches (palette aside).
- active: matches (Firefox dashed outline).
- negative: matches.
- interactive: **header now matches** (title on the left, two full-height toggle cells with `text-3xl` totals and left rules, header rule, card `py-0`). Before this round the toggles were stacked under the title because the card's `display: grid` won on equal specificity. Bars are orange because the series is `--chart-2` (palette).

## Line
- default (main), linear, step, multiple, dots, dots_custom, dots_colors, label, label_custom: all match. Dots-colors and label-custom colours differ only by palette.
- interactive: **header now matches** (same structure as bar; before this it was a compact bordered toggle pill in `CardAction`). Data, totals (24,828 / 25,010) and the monotone shape match. The first x tick reads "Apr 6" vs "Apr 3" (width-driven thinning, as for the area chart).

## Pie
- simple (main), separator_none, label, label_custom, label_list, donut, donut_active, donut_text: match. Slices run counter-clockwise from 3 o'clock with Recharts radii. Label-list text uses the background colour as in shadcn. Axe passes.
- legend: **now matches.** The 300px square has the legend inside it, wrapped to two rows (52px), so the pie has r 95.2 and centre y 124. The legend wraps through `--dx-chart-legend-item-basis` and `legend_size: 52`.
- stacked: matches structurally (inner disc and outer ring).
- interactive: matches. Leftovers: shadcn's select trigger shows a colour swatch and ours does not. Pre-hydration, the Select renders one `<option>`, the description wraps, and the card is 4px taller at 1440 (20px at 390) until wasm loads. That is a Select component SSR issue, not the chart.

## Radar
- default, dots, lines_only, multiple, radius, label_custom, grid_custom, grid_none, grid_circle, grid_circle_no_lines, grid_circle_fill, grid_fill: match (r 96, centre 125, spokes and rings). shadcn clips "Februa"; ours shows the whole word.
- legend / icons: the chart geometry matches (r 96, centre 125). The legend sits about 12px lower than shadcn's: ours is below the 250px square (box 250x278), while shadcn's overhangs the box bottom by 10px (`margin bottom -10`, `mt-8`). Recharts centres Radar on the whole box (`formatAxisMap`), so Radar/RadialBar deliberately keep their full square instead of giving room to the legend.

## Radial
- simple (main): matches. Partial arcs from 3 o'clock counter-clockwise over full muted tracks.
- label: matches. Arcs start at 6 o'clock and names run along each arc.
- grid: matches (circles plus spokes, no tracks).
- text: matches (250 degree gauge over a full muted ring, "200 Visitors").
- shape: matches (100 degree ring with square ends, "1,260"). The spec was updated: shadcn has no `cornerRadius`.
- stacked: half-donut, rounded ends and "1,830" match. **Remaining, deliberate (Lane L):** mobile spans 0 to 56 degrees (570/1830 of the half turn) where shadcn's spans 0 to 81 degrees. Recharts clamps to the 1260 single-value domain, which clips desktop.

## Tooltip
- default, indicator_line, indicator_none, label_none, label_custom, label_formatter, formatter, icons, advanced: content and indicators match. The tooltip is now shadcn's 128px border-box width; it was 152px. Each tooltip opens on `defaultIndex` 1. shadcn's reference screenshots place the open tooltip at varying heights, apparently from the capture's mouse position, so tooltip y is not compared.

## Behaviour checks (final build, `checks2.json`)
- **Hydration jump, SSR (no JS) vs hydrated + measured:**
  - Line, radar, radial and tooltip tabs: no movement.
  - Cartesian 16:9 cards: 0.3px (aspect-height rounding).
  - The four legend cards: 1.7px at 1440 and 6px at 390, before wasm only. Cause: the legend's 28px is constant while the pre-measure svg (369x180) scales with the card. Fixing it needs container-query units on `.dx-chart`.
  - Pie interactive: 4px at 1440, 20px at 390 (the Select SSR issue above).
  - Heroes (fixed 250px): no movement.
- **Load animation:** about 169 chart animations at load, none still running after 2.5s, none replayed by hovering. With `prefers-reduced-motion: reduce`: 0.
- **Tooltip follows the pointer at the first and last points** (line interactive, area, bar, line). It opens at pointer + 10px, flips left at the last point, and stays inside the chart (`shots/follow-*.png`). The pointer must be at or inside the plot edge: hovering the 12px margin left of the first point closes it, as in Recharts' `isInRange`.
- **Polar squares:** every pie, radar and radial chart is 250x250 (viewBox 0 0 250 250), except:
  - pie legend 300x300 (svg 300x248 plus a 52px legend);
  - pie interactive 300x300 (shadcn `max-h-[300px]`);
  - radar legend and icons 250x278 (legend below the square).
- **Phone (390):** no document overflow on any tab and no cell overflow; the tooltip-tab cards are now inside their cells. Interactive headers stack with the toggles side by side. `shots/phone-*.png`.
