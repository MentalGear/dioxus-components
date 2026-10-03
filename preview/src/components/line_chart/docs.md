Line charts show how values change across an ordered set of categories, such as months or dates. This page is a gallery of line configurations built from the [Chart](/component/chart/) package; install `chart` and you have everything. The Chart page explains the shared `ChartConfig` and `ChartDatum` model, colors and accessibility.

## Quick start

```rust
let config = ChartConfig::new().series("desktop", "Desktop", "var(--dx-chart-1)");
let data = vec![
    ChartDatum { label: "January".into(), values: vec![Some(186.0)], ..Default::default() },
    ChartDatum { label: "February".into(), values: vec![Some(305.0)], ..Default::default() },
    // ...
];

ChartContainer { config, data, kind: ChartKind::Line,
    Chart {
        aria_label: "Visitors by month, desktop",
        x_label: "Month",
    }
    ChartTooltip { hide_label: true } // one series does not need its name repeated
}
```

## Curve

`Chart`'s `curve` prop sets the interpolation: `Curve::Monotone` (the default) is a smooth curve that never overshoots a data point, `Curve::Linear` joins points with straight segments, and `Curve::Step` draws a horizontal run to the next point and then a vertical jump. A missing value (`None`) breaks the line at that point.

## Line options

Line-specific settings live in `line: LineOptions { .. }` on `Chart`:

- `dots: true` draws a circle at every defined point, in the series color or in `ChartDatum::color` when a datum sets one. The active point's circle is drawn twice as large. `dot_radius` sets the base radius (default `3`).
- `dot: Some(..)` replaces the circle with your own renderer. It receives a `DotContext` for each defined point, with the scaled position, the raw value and whether the point is active.
- `labels` draws a text label above each point: `LineLabels::Value` shows the point's value, and `LineLabels::Custom(callback)` calls your callback with the point index so you can label by category or anything else.
- `stroke_width` sets the line thickness (default `2`).

## Demos

- **Default** — one series, no dots.
- **Linear** and **Step** — the same data with `Curve::Linear` and `Curve::Step`.
- **Multiple** — two series, with a tooltip that shows both rows.
- **Dots** — `dots: true`.
- **Dots with colors** — one series over browser categories, each dot colored through `ChartDatum::color`.
- **Custom dots** — a diamond drawn through `dot`.
- **Label** — `LineLabels::Value`, with the grid and y-axis hidden.
- **Custom label** — `LineLabels::Custom`, labeling each point with its category.
- **Interactive** — two header buttons toggle which of two series is drawn, each showing that series' total.
