Pie chart is the arc-based member of the [Chart](/component/?name=chart) family: the same
`ChartContainer`/`Chart`/`ChartTooltip`/`ChartLegend` pieces, `kind: ChartKind::Pie`, and a
`PieOptions` prop that shapes how the slices themselves draw. There is no separate installable
package — `dx components add chart` is everything a pie or donut chart needs; this page is a
gallery of that one package's polar mode.

## Component structure

```rust
let config = ChartConfig::new()
    .series("visitors", "Visitors", "var(--dx-chart-1)");

// One ChartDatum per slice (a category, e.g. a browser) -- `color`
// overrides the position-based `--dx-chart-N` fallback per slice, the
// same way shadcn's own demos give each row its own `fill`.
let data = vec![
    ChartDatum { label: "Chrome".into(), values: vec![Some(275.0)], color: Some("var(--dx-chart-1)".into()) },
    ChartDatum { label: "Safari".into(), values: vec![Some(200.0)], color: Some("var(--dx-chart-2)".into()) },
    // ...
];

ChartContainer {
    config,
    data,
    kind: ChartKind::Pie,

    Chart {
        aria_label: "Visitors by browser",
        pie: PieOptions {
            inner_radius: 0.0,   // > 0.0 draws a donut
            pad_angle: 0.0,      // radians between slices
            corner_radius: 0.0,
            labels: PieLabels::None, // None | Value | Percent | List(Vec<String>)
            active_index: None,      // force a slice "active" regardless of hover
            center_text: None,       // Some((primary, secondary)) -- donuts only
            ..Default::default()
        },
    }
    ChartTooltip {}
    ChartLegend {}
}
```

Every slice renders as `path[data-slot="chart-arc"][data-index][data-series]`, with `data-start-
angle`/`data-end-angle` (radians) alongside the `d` path for anything that needs the raw layout
numbers rather than re-deriving them from the path. Hovering a slice (or, when `active_index` is
set, a caller-forced slice) grows its outer radius and sets `data-active="true"` — plain CSS driven
off that attribute, no re-render of the path itself needed for the common case.

## The eleven variants

Each ports one of shadcn/ui's own `chart-pie-*.tsx` demos (`registry/new-york-v4/registry/new-
york-v4/charts/`) — same dataset, same config, same card copy:

- **Simple** (`main`) — a plain 5-slice pie.
- **Separator none** — no stroke between slices.
- **Label** — each slice's own value, centered in the slice (this crate always centers a label at
  the slice's own centroid, per `engine::polar::centroid` — shadcn's default instead floats the
  label outside the pie on a leader line; a documented simplification, not a missing feature).
- **Label list** — each slice's category name instead of its value.
- **Label custom** — a caller-supplied label list is exactly the same mechanism as "Label list";
  this variant just supplies different text.
- **Legend** — a swatch + label per slice below the chart.
- **Donut** — `inner_radius > 0.0`.
- **Donut active** — a permanently-active first slice (`active_index: Some(0)`).
- **Donut text** — a two-line total centered in the hole.
- **Stacked** — two data sets drawn as two concentric rings (two `Chart`-level series, one ring
  each), matching shadcn's two `<Pie>` elements sharing one `<PieChart>`.
- **Interactive** — a `Select` drives `active_index` and the donut's center text together.

## Accessibility

Same contract as every other chart kind (see the [Chart](/component/?name=chart) docs): the SVG is
`role="img"` with an accessible name, and the real, visually-hidden `table[data-slot="chart-data"]`
lists every slice's category, value, and share of the total — screen reader users get the data
table, not an attempt to describe forty pixels of colored wedge in prose.
