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
        curve: Curve::Natural, // Natural | Monotone (default) | Linear | Step | ...
        stacked: false,        // true stacks every configured series
        // shadcn's `margin={{ left: 12, right: 12 }}` and `cursor={false}`:
        margin: ChartMargin { left: 12.0, right: 12.0, ..ChartMargin::NONE },
        cursor: false,
    }
    ChartTooltip {}
    ChartLegend {} // optional
}
```

## Options

- `curve` on `Chart` picks the interpolation: `Curve::Natural` is the smooth spline most shadcn demos use (it can overshoot a point slightly between data points), `Curve::Monotone` (the default) is smooth and never overshoots a data point, `Curve::Linear` draws straight segments, and `Curve::Step` draws midpoint steps -- each value holds until halfway to the next point (`StepBefore`/`StepAfter` put the riser at a point instead).
- `stacked: true` on `Chart` draws each series on top of the previous one, in config order -- the first series is the bottom layer, as in Recharts, where the first `<Area>` is -- so the top edge is the running total.
- `area: AreaOptions { .. }` holds the area-specific settings:
  - `gradient: true` fills each series with a gradient that is opaque near the line and fades toward the baseline.
  - `fill_opacity` sets the fill opacity (default `0.4`, shadcn's usual `fillOpacity`; Recharts' own default is `0.6`), and `series_fill_opacity` overrides it per series. The line itself stays fully opaque.
  - `connect_nulls: true` draws straight across a missing (`None`) value instead of breaking the area. It applies to unstacked charts only.
  - `stack_mode: StackMode::Expand` stacks as percentages, so every category reaches the same total height regardless of its raw total. Pair it with `stacked: true`.
- `show_y_axis: true` and `y_tick_count` show the y-axis and set how many ticks it gets (exactly that many round values, as Recharts picks them). The y-axis is off by default; the tooltip and the data table carry exact values.
- A series can have an icon: set `icon: Some(ChartIcon(Callback::new(|()| rsx! { TrendingUp {} })))` on a `ChartSeries`. `ChartLegend` and `ChartTooltip` show it in place of the color swatch.

## Demos

Each demo is a port of the shadcn/ui chart of the same name -- same data, series order, curve, margins, axes and tooltip -- checked against shadcn's own source by a test.

- **Default** — one series on a natural curve.
- **Linear** and **Step** — the same data with `Curve::Linear` and `Curve::Step`.
- **Stacked** — two series with `stacked: true`.
- **Legend** — a stacked chart with a `ChartLegend` below it.
- **Axes** — both axes shown, with `y_tick_count: 3`.
- **Interactive** — 91 days of two-series data in a fixed 250px-tall chart (`height: 250.0`), with a `Select` that narrows it to the last 3 months, 30 or 7 days (shadcn keeps the dates on or after June 30 minus the range: 91, 31 or 8 days). With this many points, `min_tick_gap` thins the x-axis labels (keeping the last one), as in shadcn.
- **Stacked, expand** — three series stacked as percentages with `StackMode::Expand`.
- **Gradient** — `AreaOptions { gradient: true, .. }`.
- **Icons** — per-series icons shown in the legend and tooltip.

## Accessibility

Area charts have the same accessibility behavior as every [Chart](/component/chart/): the SVG is a named image (`role="img"` with a `<title>`), a visually hidden data table lists every value, and a focusable wrapper lets keyboard users step through points with the arrow keys, `Home`, `End` and `Escape`. Legend swatches have `role="graphics-symbol"` and the series label as their name. See the Chart page for details.
