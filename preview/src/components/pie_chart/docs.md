Pie charts show how a whole divides into parts, and donut charts do the same with a hole in the middle that can hold a total. They are the arc-based members of the [Chart](/component/chart/) family: the same `ChartContainer`, `Chart`, `ChartTooltip` and `ChartLegend`, with `kind: ChartKind::Pie` and a `PieOptions` value that shapes the slices. Installing `chart` is everything a pie or donut needs; this page is a gallery of that mode.

## Quick start

```rust
let config = ChartConfig::new()
    .series("visitors", "Visitors", "var(--dx-chart-1)");

// One ChartDatum per slice. `color` sets that slice's color; without it,
// slices use --dx-chart-1, --dx-chart-2, ... by position.
let data = vec![
    ChartDatum { label: "Chrome".into(), values: vec![Some(275.0)], color: Some("var(--dx-chart-1)".into()) },
    ChartDatum { label: "Safari".into(), values: vec![Some(200.0)], color: Some("var(--dx-chart-2)".into()) },
    // ...
];

ChartContainer { config, data, kind: ChartKind::Pie,
    Chart {
        aria_label: "Visitors by browser",
        pie: PieOptions {
            inner_radius: Radius::Px(0.0),      // above 0 draws a donut
            outer_radius: Radius::Percent(80.0), // or Radius::Px(..)
            start_angle: 0.0,                    // degrees, 0 = three o'clock
            end_angle: 360.0,                    // counter-clockwise
            padding_angle: 0.0,                  // degrees between slices
            corner_radius: 0.0,
            labels: PieLabels::None,
            active_index: None,
            active_halo: false,
            center_text: None,
            rings: vec![],
        },
    }
    ChartTooltip { hide_label: true }
}
```

These are the defaults, and they are Recharts' `<Pie>` defaults: every option means what the same-named Recharts prop means, so the numbers in a shadcn/ui or Recharts example carry over unchanged (`innerRadius={60}` is `inner_radius: Radius::Px(60.0)`).

A polar chart is a square at most 250px wide, centred in its container (shadcn's `aspect-square max-h-[250px]`). Lengths are CSS pixels. To make one larger, give the `ChartContainer` a `style: "max-inline-size: 300px"` and the `Chart` a matching initial `width: 300.0`.

With a single series, each datum is one slice and `ChartLegend` lists one entry per slice. With more than one series, each series is its own concentric pie over the same categories; `rings` gives each its `(inner, outer)` radii, and without it the range from `inner_radius` to `outer_radius` is split into equal rings, the first series innermost.

## Options

- `inner_radius` and `outer_radius` are a `Radius`: `Px(n)` pixels, or `Percent(n)` of the *max radius*, half the chart's shorter side less a 5px margin. The default outer radius, `Percent(80.0)`, is 96px in a 250px chart.
- `start_angle` and `end_angle` are degrees, with `0` at three o'clock and angles increasing counter-clockwise. The default `0` to `360` lays the first slice out counter-clockwise from three o'clock, as Recharts does. `end_angle` below `start_angle` runs the other way.
- `labels` prints text per slice:
  - `Outside { line: true }` is Recharts' `label`: the value 20px beyond the rim, anchored away from the centre, with a leader line in the slice's color. `line: false` drops the line.
  - `Value`, `Percent` (one decimal place) and `List(vec![..])` (your own text, one entry per datum) are centred in the slice, like Recharts' `<LabelList>`.
- `active_index: Some(i)` draws slice `i` 10px further out, leaving the hole unchanged. `active_halo: true` adds a separate ring 12 to 25px beyond the rim, as in the interactive demo. Hovering a slice does not grow it; it opens the tooltip and marks the slice `data-active="true"`.
- `center_text: Some(("1,125".into(), "Visitors".into()))` writes a two-line total in a donut's hole.
- Each slice is a `path[data-slot="chart-arc"]` with `data-index`, `data-start-angle` and `data-end-angle` (degrees, as above), and `data-inner-radius` and `data-outer-radius` (pixels).
- The slices sweep in when the chart first appears, and the labels fade in after them. Nothing animates under `prefers-reduced-motion: reduce`.

## Demos

- **Simple**: a plain five-slice pie.
- **Separator none**: the same pie; slices are drawn without a separator stroke.
- **Label**: each slice's value outside the rim with a leader line.
- **Custom label**: the values outside the rim without leader lines.
- **Label list**: each browser's name inside its slice.
- **Legend**: a swatch and name per slice below the chart.
- **Donut**: a 60px hole.
- **Donut active**: the first slice held active.
- **Donut with text**: the total in the hole.
- **Stacked**: two series as two concentric pies, a disc and a ring around it.
- **Interactive**: a `Select` drives `active_index` and the center text together.

## Accessibility

A pie chart has the same accessibility behavior as every [Chart](/component/chart/): the SVG is a named image, and the visually hidden data table lists each slice's category, its value and its percent of the total, so screen reader users read the numbers instead of an attempt to describe wedges. The chart's wrapper is focusable, and `ArrowLeft`, `ArrowRight`, `Home`, `End` and `Escape` move through the slices.
