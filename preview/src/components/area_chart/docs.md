Area charts show how one or more values change across categories, with the region under the line filled in. Use them for totals over time, or stack several series to show how a whole is made up. This page is a gallery of the shapes an area chart usually takes; every piece it uses (`ChartContainer`, `Chart`, `ChartTooltip`, `ChartLegend`) comes from the [Chart](/component/chart/) package, which is the one package you install.

## Quick start

```rust
let config = ChartConfig::new().series("desktop", "Desktop", "var(--dx-chart-1)");
let data = vec![
    ChartDatum { label: "January".into(), values: vec![Some(186.0)], ..Default::default() },
    // ...
];

ChartContainer { config, data, kind: ChartKind::Area,
    Chart {
        aria_label: "Visitors by month",
        curve: Curve::Monotone, // Monotone (default, smooth) | Linear | Step
        stacked: false,         // true stacks every configured series
    }
    ChartTooltip {}
    ChartLegend {} // optional
}
```

## Options

- `curve` on `Chart` picks the interpolation: `Curve::Monotone` is smooth and never overshoots a data point, `Curve::Linear` draws straight segments, and `Curve::Step` draws a horizontal run to the next point followed by a vertical jump.
- `stacked: true` on `Chart` draws each series on top of the previous one, so the top edge is the running total.
- `area: AreaOptions { .. }` holds the area-specific settings:
  - `gradient: true` fills each series with a gradient that is opaque near the line and fades toward the baseline.
  - `fill_opacity` sets the flat fill opacity (default `0.4`); the line itself stays fully opaque.
  - `connect_nulls: true` draws straight across a missing (`None`) value instead of breaking the area. It applies to unstacked charts only.
  - `stack_mode: StackMode::Expand` stacks as percentages, so every category reaches the same total height regardless of its raw total. Pair it with `stacked: true`.
- `show_y_axis: true` and `y_tick_count` show the y-axis and control how many ticks it gets. The y-axis is off by default; the tooltip and the data table carry exact values.
- A series can have an icon: set `icon: Some(ChartIcon(Callback::new(|()| rsx! { TrendingUp {} })))` on a `ChartSeries`. `ChartLegend` and `ChartTooltip` show it in place of the color swatch.

## Demos

- **Default** — one series with the default smooth curve.
- **Linear** and **Step** — the same data with `Curve::Linear` and `Curve::Step`.
- **Stacked** — two series with `stacked: true`.
- **Legend** — a stacked chart with a `ChartLegend` below it.
- **Axes** — both axes shown, with `y_tick_count: 3`.
- **Interactive** — 90 days of two-series data with a `Select` that narrows the range to the last 90, 30 or 7 days. With this many points, `max_x_ticks` keeps the x-axis readable.
- **Stacked, expand** — three series stacked as percentages with `StackMode::Expand`.
- **Gradient** — `AreaOptions { gradient: true, .. }`.
- **Icons** — per-series icons shown in the legend and tooltip.

## Accessibility

Area charts have the same accessibility behavior as every [Chart](/component/chart/): the SVG is a named image (`role="img"` with a `<title>`), a visually hidden data table lists every value, and a focusable wrapper lets keyboard users step through points with the arrow keys, `Home`, `End` and `Escape`. Legend swatches have `role="graphics-symbol"` and the series label as their name. See the Chart page for details.
