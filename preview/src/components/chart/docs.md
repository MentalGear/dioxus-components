Chart draws themed, accessible SVG charts from plain data. It covers area, bar, line, pie, radar and radial-bar charts with one set of building blocks: describe your series in a `ChartConfig`, supply one `ChartDatum` per category, and compose `ChartContainer`, `Chart`, and optionally `ChartTooltip` and `ChartLegend`. There is no charting library behind it and no injected markup — every mark is ordinary Dioxus RSX, so charts server-render and hydrate like any other component.

The other chart pages are galleries built on this same package: [Area chart](/component/area_chart/), [Bar chart](/component/bar_chart/), [Line chart](/component/line_chart/), [Pie chart](/component/pie_chart/), [Radar chart](/component/radar_chart/), [Radial chart](/component/radial_chart/), and [Chart tooltip](/component/chart_tooltip/). Install the `chart` package once and use any of them.

## Quick start

```rust
// Series, in draw order (also the tooltip row order; the legend lists them by
// key, as Recharts' legend does). `color` is
// any CSS color or a `var(--token)`; the theme ships `--dx-chart-1` to
// `--dx-chart-8`, which adapt to light and dark mode.
let config = ChartConfig::new()
    .series("desktop", "Desktop", "var(--dx-chart-1)")
    .series("mobile", "Mobile", "var(--dx-chart-2)");

// One ChartDatum per x-axis category. `values` has one entry per series, in
// the same order as `config`. `None` means "no value": a gap in a line or
// area, no bar, and an em dash in the data table -- never a made-up zero.
let data = vec![
    ChartDatum { label: "January".into(), values: vec![Some(186.0), Some(80.0)], ..Default::default() },
    ChartDatum { label: "February".into(), values: vec![Some(305.0), Some(200.0)], ..Default::default() },
    // ...
];

ChartContainer {
    // Sets each series' `--color-<key>` CSS variable as an inline style on
    // this element, which carries `data-chart="<id>"` as the instance id.
    // `id` is generated when omitted.
    config,
    data,
    kind: ChartKind::Area, // Area | Bar | Line | Pie | Radar | RadialBar

    // The drawing surface: grid, axes, marks, and a hidden data table.
    // `aria_label` is required and becomes the chart's accessible name.
    Chart {
        aria_label: "Visitors by month, desktop and mobile",
        x_label: "Month", // header of the hidden table's first column
    }

    // Optional: hover/keyboard tooltip.
    ChartTooltip {}

    // Optional: one swatch and label per series.
    ChartLegend {}
}
```

`Chart`, `ChartTooltip` and `ChartLegend` are independent siblings inside the container. Leave out whichever you do not need; a single-series chart often wants no legend. As in shadcn/ui, a legend takes its room from the chart's own box rather than adding to it: an area, bar, line or pie chart with a legend is exactly as tall as one without, and its plot is shorter by the legend's height. A legend that wraps onto more rows tells `Chart` its size up front with `legend_size` (in px), so the server render already reserves the right amount.

## Props you will use most

On `Chart`:

- `aria_label` (required) and `description` — the accessible name, and an optional longer description exposed as the SVG's `<desc>`.
- `stacked` — stack series instead of overlaying (area) or grouping (bar), in config order: the first series is the bottom layer (Recharts stacks in declaration order). Ignored for other kinds.
- `curve` — `Curve::Natural`, `Curve::Monotone` (default), `Curve::Linear`, `Curve::Step` (midpoint steps), `Curve::StepBefore` or `Curve::StepAfter` — Recharts' `type` values. Applies to area and line.
- `show_grid` (default on), `show_x_axis` (default on), `show_y_axis` (default off), `y_tick_count` (default 5) and `tick_margin` (default 8: px between a tick label and the plot, beyond the 6px tick size). The value axis uses Recharts' tick algorithm: exactly `y_tick_count` round values, and the axis runs from the first to the last of them, so the top gridline is the plot's top edge. A shown x axis reserves a 30px band under the plot and a shown y axis a 60px band left of it, as Recharts does. On horizontal bars the y axis holds the categories and the x axis the values.
- `margin` — Recharts' `margin` in px (`ChartMargin`, default 5 on every side). A partial Recharts margin replaces the whole default, so `margin={{ left: 12, right: 12 }}` is `ChartMargin { left: 12.0, right: 12.0, ..ChartMargin::NONE }`. Sides may be negative.
- `cursor` (default on) — the hover cursor: a 1px line through the active point on area and line charts, a muted band behind the active category on bar charts. shadcn's demos usually set `cursor={false}`; so do ours.
- `x_label` — the name of the x-axis column in the hidden data table. Defaults to `"Category"`.
- `x_tick_format` — format x-axis labels. By default a label is cut to its first three characters (`"January"` becomes `"Jan"`), so supply a formatter for anything that is not a month name.
- `min_tick_gap` — the smallest gap in px between two x-axis labels (default 5; see below).
- `width`, `height` and `aspect` — the size. A chart is always exactly as wide as its container and draws at 1 unit = 1 CSS pixel, so every length — margins, bar radius, stroke widths, dot radii, label offsets — is the pixel size you wrote, and text is never scaled. Its height is `height` when given (a fixed-height, fluid-width chart, like shadcn's `h-[250px]` interactive charts), otherwise the width divided by `aspect` (default 16/9 for area, bar and line, like shadcn's `aspect-video`; 1 for pie, radar and radial). `width` (default 369 for area, bar and line, 250 for the others) is only the size of the server render and the first client render, before the container has been measured, so hydration matches; that render already fills the container — an aspect-ratio chart is drawn as a uniform scale of the final one, a fixed-height chart is stretched across at its final height with its text hidden until the measured render (`data-measured="false"` marks it). Pie, radar and radial charts are squares at most 250px wide, centered, like shadcn's `aspect-square max-h-[250px]`; give the container a `max-inline-size` for another size. Axis text is anchored to its marks and stays left-to-right under `dir="rtl"` (the drawing itself is not mirrored).
- `animate` (default on) — after the chart is first measured in the browser, bars grow from the baseline and lines and areas are revealed left to right, once (Recharts' load animation). Never on the server render, so without JavaScript the final chart shows, and never under `prefers-reduced-motion`.
- `keyboard` — keyboard stepping through data points, on by default.
- `default_index` — show the tooltip already open at this data point (shadcn's `defaultIndex`), on the server render and the first client render, until the user hovers, taps or presses a key; once it closes it stays closed. Read on mount only, and ignored when it is past the end of the data. The tooltip is hidden until the browser has measured the chart (it needs the box to flip and clamp), then shown at the data point, with the cursor line and the active dots. Place `ChartTooltip` after `Chart` in the container so it sees the index on the server render. The tooltip gallery uses it so each variant is visible without hovering.
- `area`, `bar`, `line`, `pie`, `radar`, `radial` — per-kind options structs, such as `LineOptions { dots: true, .. }`. Each is described on its gallery page.

On `ChartContainer`: `config`, `data`, `kind` and an optional `id`. Extra attributes you pass land on the container element.

A `ChartDatum` can also carry a `color` to override the series color for that single point. Bar, line (dots), pie and radial charts honor it.

## Colors

Series colors are CSS custom properties. The container sets `--color-<key>` for every series as an inline style on its own element, where `<key>` is the series key lowercased with any character outside `a-z`, `0-9`, `_` and `-` replaced by `-`. Marks, legend swatches and tooltip swatches all read those variables, and so can your own CSS inside the container (`fill: var(--color-desktop)`). Because each chart sets its own variables on its own element, several charts on one page never interfere.

Pie and radial charts color each slice or ring from its datum's `color`, falling back to `--dx-chart-1` to `--dx-chart-8` by position when the datum has none. The legend for a single-series pie or radial chart lists one entry per slice or ring.

## Accessibility

A chart is a picture, so it comes with a real data table for anyone who cannot see it.

- **Screen readers.** The SVG has `role="img"`, an `aria-label` from `aria_label`, and a `<title>` (plus `<desc>` when you pass `description`). Alongside it, a visually hidden `<table>` lists every category and every series' value, with the series labels as column headers and the `x_label` as the first column's header. Missing values read as an em dash. Pie charts add a percent-of-total column; radial charts list the first series' value for each ring. Screen reader users browse this table with their normal table navigation.
- **Keyboard.** With `keyboard` on, the chart's wrapper is focusable (`tabindex="0"`, `role="group"`, `aria-roledescription="chart"`, named by `aria_label`). `ArrowRight` and `ArrowLeft` move the active data point one step (clamped at the ends, and swapped in right-to-left layouts), `Home` and `End` jump to the first and last point, and `Escape` clears it. The active point drives the same tooltip and highlight as hovering, so sighted keyboard users see the same thing as mouse users; with no pointer involved, the tooltip hangs off the active data point and flips or clamps the same way it does for a pointer. Tabbing into the chart with the keyboard opens the tooltip at the first point straight away (like shadcn's `accessibilityLayer`); focus that a click or tap caused does not, since the pointer already picks its own point. Tabbing away closes it. The first arrow press with nothing active selects the first point (`ArrowRight`) or last point (`ArrowLeft`).
- **Tooltip.** The tooltip is hidden from assistive technology on purpose; it is a convenience for sighted users and the data table is the accessible route to the same numbers.
- **Tooltip position and pointer.** The tooltip follows the pointer the way shadcn/Recharts tooltips do. On area, bar and line charts it snaps horizontally to the nearest category's point and tracks the pointer vertically (a horizontal bar chart swaps the axes), sitting 10px right of and below that point; near the chart's right or bottom edge it flips to the left of or above the point, and it is always kept inside the chart box. It closes when the pointer leaves the plot. Radar and radial charts follow the pointer on both axes; a pie anchors at the hovered slice instead, and its row shows the slice's name, value and color. The position eases between points (200ms, off under `prefers-reduced-motion`), but a tooltip that has just appeared is already at its target rather than sliding in. `data-side-x` and `data-side-y` on the tooltip say which side of its point it is on.
- **Active dot.** On an area chart, and on a line chart that is not already drawing its own dots, the active data point gets a dot on every series that has a value there (radius 4, the series color, ringed in the card's surface color so it reads in light and dark mode) -- Recharts' `activeDot`. A stacked area's dot sits on its band's top edge. A line chart with `dots` on enlarges its own active dot instead. The marks are `data-slot="chart-active-dot"` inside `data-slot="chart-active-dots"`.
- **Scaled and zoomed pages.** The pointer position is mapped onto the chart's own layout pixels, so the tooltip still lands at the pointer inside an ancestor with `transform: scale(..)` or CSS `zoom`. A pointer event whose position is still being measured when the tooltip closes (the pointer leaves, focus moves) can no longer reopen it.
- **Touch.** A tap shows the tooltip for the tapped category and it stays after the finger lifts; tapping elsewhere (or anywhere outside the chart) closes it. On area, bar and line charts dragging a finger along the chart scrubs between categories while a drag across the other axis still scrolls the page. On pie, radar and radial charts a tap works but a drag does not move between slices. Charts only claim the pan direction they scrub along (`touch-action: pan-y pinch-zoom`, or `pan-x pinch-zoom` for horizontal bars), so a pinch still zooms the page.
- **Axis text.** Tick labels are the muted text color (`--secondary-color-5`: 4.95:1 on the light card, 7.18:1 on the dark one); values and labels the chart draws keep the stronger body color.
- **Legend and swatches.** Each color swatch has `role="graphics-symbol"` and the series label as its accessible name; the label text beside it is ordinary text.
- **Names matter.** Write an `aria_label` that says what the chart shows (`"Visitors by month, desktop and mobile"`), and set `x_label` to what the first column is (`"Month"`, `"Date"`, `"Product"`).

## Dense x-axes

With many points (say 90 daily values) a label under every category would be an unreadable smear, so labels are thinned the way Recharts' default `interval="preserveEnd"` thins them: the last category is always labelled, and walking back from it, a label is drawn only where it clears the previous one by `min_tick_gap` px and fits inside the chart. A narrow chart (a phone) therefore shows fewer labels than a wide one, and longer labels fewer than short ones. shadcn's 91-day charts set `minTickGap={32}` and so do ours (`min_tick_gap: 32.0`). Only the labels are thinned; every point stays hoverable and stays in the data table.

Label widths are estimated from typical UI-font metrics rather than measured, so the server and the browser agree; an unusually wide font can still crowd, in which case raise `min_tick_gap` or shorten the labels with `x_tick_format`.

## Motion and visibility

The load animation plays once, when the chart is first actually on screen: at least 40% of it visible (or 40% of the viewport height for a chart taller than the screen) with the tab visible. A chart below the fold, in a skipped `content-visibility` card or in a background tab does not animate until it is seen, and one already in view at mount animates as soon as it has been laid out. There is no pre-roll margin on purpose: an entrance that starts before the chart is visible is one nobody saw. Set `animate: false` to turn the animation off entirely. See `dioxus_primitives::activity::use_entered_view_when`.

The hover transitions use `--dx-motion-duration-base` and `--dx-motion-duration-slow`; the load animation durations are literals in `chart/style.css`. See [Motion](/docs#motion) for the tokens, reduced motion and the per-component table.
