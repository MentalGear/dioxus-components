# Chart sizing contract (Lane C owns the implementation; Lane L codes against it)

> **Snapshot 2026-10-03.** Evidence files this report cites by relative name (screenshots, JSON probe dumps, probe scripts) were scratch artifacts of the session and are **not committed**; the findings and numbers are kept. See [README.md](./README.md).

Recharts `ResponsiveContainer` semantics: **1 user unit = 1 CSS px**, always.

## `ChartProps` (primitives/src/chart/components/chart.rs)

| prop | type | meaning |
|---|---|---|
| `width` | `Option<f64>` (pass a plain `f64`) | INITIAL width in px, used only before the container is measured (SSR + first client render). Default: **369** for Area/Bar/Line, **250** for Pie/Radar/RadialBar. |
| `height` | `Option<f64>` (pass a plain `f64`) | FIXED height in px ("fit" mode, the old `fit_width` hero behaviour): width = measured container width, height = this. `None` (default) = aspect mode. |
| `aspect` | `Option<f64>` | width / height in aspect mode. Default **16/9** for Area/Bar/Line (shadcn `aspect-video`), **1.0** for Pie/Radar/RadialBar (shadcn `aspect-square`). |
| `margin` | `ChartMargin { top, right, bottom, left }` (px, may be negative) | Recharts `margin`. `Default` = 5 on every side (Recharts default). A partial Recharts margin (`margin={{left:12,right:12}}`) REPLACES the default, so write `ChartMargin { left: 12.0, right: 12.0, ..ChartMargin::NONE }` (NONE = all 0). |
| `fit_width` | **removed** | folded into `height: Some(..)`. |

Resolved size per render (`SeriesRenderContext::width` / `::height`, px):
- measured: `width = round(container content-box width)` (min 100), `height = props.height.unwrap_or(width / aspect)`.
- unmeasured (SSR, first client render -- identical, no hydration mismatch): `width = props.width.unwrap_or(kind default)`, `height = props.height.unwrap_or(width / aspect)`.
- svg: `viewBox="0 0 W H"`, CSS `width: 100%`; aspect mode `height: auto` (pre-measure render is a uniform scale of the final one, no distortion); fixed-height mode `height: Hpx` (pre-measure stretched with `preserveAspectRatio="none"`, text hidden until measured -- unchanged from the old fit mode).

**Polar size cap** = shadcn `aspect-square max-h-[250px] mx-auto`: done in CSS on the container, not in Rust:
`.dx-chart:is([data-kind="pie"], [data-kind="radar"], [data-kind="radial-bar"]) { max-inline-size: 250px; margin-inline: auto; }` (Lane C adds it to style.css). So a polar chart's measured width = min(card content width, 250) and, with aspect 1, it is a 250x250 square in a normal card. A demo that wants shadcn's `max-h-[300px]` passes `style: "max-inline-size: 300px"` on its `ChartContainer` (caller style is kept). No props needed for the default case: **polar demos should drop their `width: 300.0, height: 300.0`** (under the new contract `height: 300.0` means fixed 300px height with a fluid width -- not what you want).

## `SeriesRenderContext` (components/layout.rs) -- what series renderers read

- `ctx.width`, `ctx.height`: the resolved px size above (the svg box). Compute radii from these.
- `ctx.margin: ChartMargin`: the resolved margin.
- `ctx.plot_x0/plot_x1/plot_y0/plot_y1`: the box inset by `margin` (Cartesian kinds additionally subtract the x-axis band (30px, Recharts XAxis height) at the bottom when the x axis is shown, and the y-axis width (60px, Recharts YAxis width) at the left when the y axis is shown). For polar kinds it is exactly the box inset by `margin` (default 5) -- i.e. Recharts' polar `offset` box: `cx = (plot_x0+plot_x1)/2`, `cy = (plot_y0+plot_y1)/2`, `maxRadius = min(plot_x1-plot_x0, plot_y1-plot_y0)/2` (Pie default outerRadius = 0.8 * maxRadius = 96 in a 250 box).
- `ctx.text_scale`: **removed** (all geometry is plain px; nothing reads it any more).
- CSS: `--dx-chart-text-scale` is no longer set; do not use it in new rules (Lane C removes the existing uses in its own rule blocks; Lane L please drop it from the pie/radar/radial blocks: `font-size: var(--dx-text-xs)` etc.).

## Tooltip

`ChartLayout::anchor_percent` stays "(left%, top%) of the svg box (= the chart wrapper)". Unchanged API.

## Load animation (Lane C)

After the first measured client render the svg gets `data-animate="true"`; SSR/first render has no attribute (final state for no-JS). CSS (style.css, Lane C) animates Cartesian marks once; reduced motion = none. Lane L may add polar rules keyed on `[data-slot="chart-svg"][data-animate="true"]` in its own blocks if wanted.

## Addenda (Lane C, after implementation)

- Polar hover hit test (`components/pointer.rs`, `HitTest::Sectors`) now measures angles/radii around the **plot rect's center** (`(plot_x0+plot_x1)/2, (plot_y0+plot_y1)/2`), not the svg center -- so a demo with an asymmetric `margin` (radar-legend's `top: -40, bottom: -10`) hovers correctly as long as the renderer also centers on the plot rect.
- Aspect-mode height is rounded to a whole px (Recharts reads the container's integer `clientHeight`): 369 wide -> 208 tall; polar 250 -> 250.
- `nice_ticks(min, max, tick_count)` (Recharts `getNiceTickValues`, exactly `tick_count` values, domain = first..last) is public in `engine::scale` / `dioxus_primitives::chart::nice_ticks` if the radar radius axis wants it (`nice_ticks(0.0, 305.0, 5) == [0, 80, 160, 240, 320]`).
