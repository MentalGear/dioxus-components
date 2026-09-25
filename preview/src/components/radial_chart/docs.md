Radial chart is the other arc-based member of the [Chart](/component/?name=chart) family:
`kind: ChartKind::RadialBar` plus a `RadialOptions` prop. Like [Pie chart](/component/?name=
pie_chart), there is no separate installable package — `chart` is everything a radial bar chart
needs.

## Component structure

```rust
let config = ChartConfig::new()
    .series("visitors", "Visitors", "var(--dx-chart-1)");

// Unlike Pie, each ChartDatum here is its own concentric RING (innermost =
// index 0, the same "series 0 innermost" convention Pie's own stacked-ring
// mode uses) -- not an angular slice of one ring.
let data = vec![
    ChartDatum { label: "Chrome".into(), values: vec![Some(275.0)], color: Some("var(--dx-chart-1)".into()) },
    ChartDatum { label: "Safari".into(), values: vec![Some(200.0)], color: Some("var(--dx-chart-2)".into()) },
    // ...
];

ChartContainer {
    config,
    data,
    kind: ChartKind::RadialBar,

    Chart {
        aria_label: "Visitors by browser",
        radial: RadialOptions {
            inner_radius: 30.0,
            outer_radius: 110.0,
            start_angle: 0.0,
            end_angle: std::f64::consts::TAU, // may exceed a full turn, e.g. shadcn's 380°
            corner_radius: 0.0,
            grid: false,   // a muted background track behind every ring
            labels: PieLabels::None,
            stacked: false, // true stacks every configured series into ONE ring instead
        },
    }
    ChartTooltip {}
}
```

Each ring renders the same `path[data-slot="chart-arc"][data-index][data-series]` shape Pie chart
does (plus `data-start-angle`/`data-end-angle`), so the two families share one mental model even
though what a "slice" represents differs (an angular share of the whole, versus a bar wrapped
around a circle).

## The six variants

Each ports one of shadcn/ui's own `chart-radial-*.tsx` demos:

- **Simple** (`main`) — five rings, one per browser, each with its own background track.
- **Label** — each ring's own category name, drawn inside its own arc.
- **Grid** — a background track shown as its own element (`grid: true`) rather than each bar's own
  `background`; visually the same idea (shadcn spells these two differently — a `RadialBar`
  `background` prop versus a `PolarGrid` element — this crate unifies both as one `grid: bool`).
- **Text** — a single ring with a two-line total centered in its hole, gauge-style.
- **Shape** — a single ring with rounded corners (`corner_radius > 0.0`).
- **Stacked** — two series (mobile, desktop) stacked cumulatively into one ring rather than each
  getting its own (`stacked: true`).

## Accessibility

Same contract as every other chart kind: `role="img"` + accessible name on the SVG, and a real
hidden `table[data-slot="chart-data"]` listing every ring's category and value for assistive tech.
