Pie charts show how a whole divides into parts, and donut charts do the same with a hole in the middle that can hold a total. They are the arc-based members of the [Chart](/component/?name=chart) family: the same `ChartContainer`, `Chart`, `ChartTooltip` and `ChartLegend`, with `kind: ChartKind::Pie` and a `PieOptions` value that shapes the slices. Installing `chart` is everything a pie or donut needs; this page is a gallery of that mode.

## Usage

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
            inner_radius: 0.0,       // above 0.0 draws a donut (SVG units)
            pad_angle: 0.0,          // gap between slices, in radians
            corner_radius: 0.0,      // rounds slice corners
            labels: PieLabels::None, // None | Value | Percent | List(Vec<String>)
            active_index: None,      // hold one slice in its highlighted state
            center_text: None,       // Some((primary, secondary)), donuts only
            ..Default::default()
        },
    }
    ChartTooltip {}
    ChartLegend {}
}
```

With a single series, each datum is one slice and `ChartLegend` lists one entry per slice. With more than one series, each series is drawn as its own concentric ring (the first series innermost) and the legend lists the series instead.

## Options

- `labels` prints text on each slice, centered in it: `Value` shows the slice's value, `Percent` its share of the total (one decimal place), and `List(vec![..])` shows your own text, one entry per datum by position. A slice with no entry gets no label.
- `inner_radius` greater than `0.0` makes a donut, and `center_text: Some(("1,125".into(), "Visitors".into()))` writes a two-line total in the hole.
- `active_index: Some(i)` keeps slice `i` highlighted (its outer radius grows and it gets `data-active="true"`) regardless of hover. Drive it from a signal to build a picker.
- Hovering a slice, or stepping to it with the keyboard, highlights it the same way.
- Each slice is a `path[data-slot="chart-arc"]` with `data-index`, `data-start-angle` and `data-end-angle` (in radians) if you need the raw layout numbers in your own CSS or scripts.

## Variants

- **Simple** — a plain five-slice pie.
- **Separator none** — the same pie; slices are drawn without a separator stroke.
- **Label** — each slice's value.
- **Label list** and **Custom label** — your own text per slice through `PieLabels::List`.
- **Legend** — a swatch and label per slice below the chart.
- **Donut** — `inner_radius` above zero.
- **Donut active** — the first slice held active.
- **Donut with text** — a two-line total in the hole.
- **Stacked** — two series drawn as two concentric rings.
- **Interactive** — a `Select` drives `active_index` and the center text together.

## Accessibility

A pie chart has the same accessibility behavior as every [Chart](/component/?name=chart): the SVG is a named image, and the visually hidden data table lists each slice's category, its value and its percent of the total, so screen reader users read the numbers instead of an attempt to describe wedges. The chart's wrapper is focusable, and `ArrowLeft`, `ArrowRight`, `Home`, `End` and `Escape` move through the slices.
