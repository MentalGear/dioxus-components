Chart draws themed, accessible SVG charts from plain data. It covers area, bar, line, pie, radar and radial-bar charts with one set of building blocks: describe your series in a `ChartConfig`, supply one `ChartDatum` per category, and compose `ChartContainer`, `Chart`, and optionally `ChartTooltip` and `ChartLegend`. There is no charting library behind it and no injected markup — every mark is ordinary Dioxus RSX, so charts server-render and hydrate like any other component.

The other chart pages are galleries built on this same package: [Area chart](/component/area_chart/), [Bar chart](/component/bar_chart/), [Line chart](/component/line_chart/), [Pie chart](/component/pie_chart/), [Radar chart](/component/radar_chart/), [Radial chart](/component/radial_chart/), and [Chart tooltip](/component/chart_tooltip/). Install the `chart` package once and use any of them.

## Quick start

```rust
// Series, in draw order (also the legend and tooltip row order). `color` is
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

`Chart`, `ChartTooltip` and `ChartLegend` are independent siblings inside the container. Leave out whichever you do not need; a single-series chart often wants no legend.

## Props you will use most

On `Chart`:

- `aria_label` (required) and `description` — the accessible name, and an optional longer description exposed as the SVG's `<desc>`.
- `stacked` — stack series instead of overlaying (area) or grouping (bar). Ignored for other kinds.
- `curve` — `Curve::Monotone` (default, smooth), `Curve::Linear` or `Curve::Step`. Applies to area and line.
- `show_grid` (default on), `show_x_axis` (default on), `show_y_axis` (default off) and `y_tick_count` (default 5).
- `x_label` — the name of the x-axis column in the hidden data table. Defaults to `"Category"`.
- `x_tick_format` — format x-axis labels. By default a label is cut to its first three characters (`"January"` becomes `"Jan"`), so supply a formatter for anything that is not a month name.
- `max_x_ticks` — cap on how many x-axis labels are drawn (see below).
- `width` and `height` — the logical size (default 600 by 300): the coordinate space that every length in the chart's props (radii, insets, gaps) is expressed in. The SVG scales to fit its container, so these set the aspect ratio and the layout, not a pixel size. Text keeps its authored size whatever the container: on a container narrower than `width` it is scaled up (to at most 2.5x) instead of shrinking with the drawing, and on a wider one scaled down (to at least 0.5x) instead of growing with it. Axis text is anchored to its marks and stays left-to-right under `dir="rtl"` (the drawing itself is not mirrored).
- `fit_width` (default off) — fixed height, fluid width, like shadcn's `h-[250px]` chart: once mounted, the chart's logical width becomes its container's measured width (1 unit is 1 CSS pixel), `height` is rendered at exactly that many CSS pixels, and text is not scaled. Use it for a full-width chart that must be the same height on a phone and on a desktop, which a fixed aspect ratio cannot do. `width` is still used for the server render and the first client render (so hydration matches); the SVG is already `height` pixels tall then, so the page does not shift when the measured width is adopted. X-axis labels are thinned to the real width. Area, bar (vertical and horizontal) and line charts only; ignored for pie, radar and radial charts, whose radii are authored in the fixed logical space.
- `keyboard` — keyboard stepping through data points, on by default.
- `area`, `bar`, `line`, `pie`, `radar`, `radial` — per-kind options structs, such as `LineOptions { dots: true, .. }`. Each is described on its gallery page.

On `ChartContainer`: `config`, `data`, `kind` and an optional `id`. Extra attributes you pass land on the container element.

A `ChartDatum` can also carry a `color` to override the series color for that single point. Bar, line (dots), pie and radial charts honor it.

## Colors

Series colors are CSS custom properties. The container sets `--color-<key>` for every series as an inline style on its own element, where `<key>` is the series key lowercased with any character outside `a-z`, `0-9`, `_` and `-` replaced by `-`. Marks, legend swatches and tooltip swatches all read those variables, and so can your own CSS inside the container (`fill: var(--color-desktop)`). Because each chart sets its own variables on its own element, several charts on one page never interfere.

Pie and radial charts color each slice or ring from its datum's `color`, falling back to `--dx-chart-1` to `--dx-chart-8` by position when the datum has none. The legend for a single-series pie or radial chart lists one entry per slice or ring.

## Accessibility

A chart is a picture, so it comes with a real data table for anyone who cannot see it.

- **Screen readers.** The SVG has `role="img"`, an `aria-label` from `aria_label`, and a `<title>` (plus `<desc>` when you pass `description`). Alongside it, a visually hidden `<table>` lists every category and every series' value, with the series labels as column headers and the `x_label` as the first column's header. Missing values read as an em dash. Pie charts add a percent-of-total column; radial charts list the first series' value for each ring. Screen reader users browse this table with their normal table navigation.
- **Keyboard.** With `keyboard` on, the chart's wrapper is focusable (`tabindex="0"`, `role="group"`, `aria-roledescription="chart"`, named by `aria_label`). `ArrowRight` and `ArrowLeft` move the active data point one step (clamped at the ends, and swapped in right-to-left layouts), `Home` and `End` jump to the first and last point, and `Escape` clears it. The active point drives the same tooltip and highlight as hovering, so sighted keyboard users see the same thing as mouse users; with no pointer involved, the tooltip hangs off the active data point and flips or clamps the same way it does for a pointer. Tabbing away closes it. The first arrow press with nothing active selects the first point (`ArrowRight`) or last point (`ArrowLeft`).
- **Tooltip.** The tooltip is hidden from assistive technology on purpose; it is a convenience for sighted users and the data table is the accessible route to the same numbers.
- **Tooltip position and pointer.** The tooltip follows the pointer the way shadcn/Recharts tooltips do. On area, bar and line charts it snaps horizontally to the nearest category's point and tracks the pointer vertically (a horizontal bar chart swaps the axes), sitting 10px right of and below that point; near the chart's right or bottom edge it flips to the left of or above the point, and it is always kept inside the chart box. It closes when the pointer leaves the plot. Radar and radial charts follow the pointer on both axes; a pie anchors at the hovered slice instead, and its row shows the slice's name, value and color. The position eases between points (200ms, off under `prefers-reduced-motion`), but a tooltip that has just appeared is already at its target rather than sliding in. `data-side-x` and `data-side-y` on the tooltip say which side of its point it is on.
- **Touch.** A tap shows the tooltip for the tapped category and it stays after the finger lifts; tapping elsewhere (or anywhere outside the chart) closes it. On area, bar and line charts dragging a finger along the chart scrubs between categories while a drag across the other axis still scrolls the page. On pie, radar and radial charts a tap works but a drag does not move between slices.
- **Legend and swatches.** Each color swatch has `role="graphics-symbol"` and the series label as its accessible name; the label text beside it is ordinary text.
- **Names matter.** Write an `aria_label` that says what the chart shows (`"Visitors by month, desktop and mobile"`), and set `x_label` to what the first column is (`"Month"`, `"Date"`, `"Product"`).

## Dense x-axes

With many points (say 90 daily values) a label under every category would be an unreadable smear, so a chart draws at most 12 x-axis labels. `max_x_ticks` (default `12`) sets that upper limit: the chart labels every n-th category, where n is the number of categories divided by the label count, rounded up, always starting with the first. Only the labels are thinned; every point stays hoverable and stays in the data table.

Labels are also thinned to fit the available width. The chart estimates how wide its longest label is and draws only as many as fit with a small gap, so a narrow chart (such as on a phone) shows fewer labels than a wide one, and longer labels show fewer than short ones. The estimate is not an exact measurement, so extremely wide labels can still touch; shorten them with `x_tick_format` or lower `max_x_ticks`.
