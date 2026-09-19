Area Chart is a gallery of [`Chart`](/component/?name=chart&) configured with `kind: ChartKind::Area` — shadcn/ui's own `chart-area-*` demo spread, ported one-for-one. There is no `area_chart` primitive and no new themed wrapper: every piece below (`ChartContainer`, `Chart`, `ChartTooltip`, `ChartLegend`) is the `chart` package's own, installed as this component's one dependency (`componentDependencies: ["chart"]`). This page exists to show the shapes an area chart actually takes in practice — a single series, several curve interpolations, stacked and percent-stacked totals, gradient fills, and an interactive range picker — the same variety shadcn's own docs site demonstrates.

## Component structure

```rust
let config = ChartConfig::new().series("desktop", "Desktop", "var(--dx-chart-1)");
let data = vec![
    ChartDatum { label: "January".into(), values: vec![Some(186.0)] },
    // ...
];

ChartContainer { config, data, kind: ChartKind::Area,
    Chart {
        aria_label: "Visitors by month",
        curve: Curve::Monotone, // Monotone (shadcn's "natural") | Linear | Step
        stacked: false,         // true stacks every configured series
    }
    ChartTooltip {}
    ChartLegend {} // optional
}
```

## Variants

- **Default** (`chart-area-default`) — one series, the default `Curve::Monotone` interpolation (matches shadcn's `type="natural"`), filled to the baseline.
- **Linear** (`chart-area-linear`) — the same data with `curve: Curve::Linear` — straight segments between points, no smoothing.
- **Step** (`chart-area-step`) — `curve: Curve::Step` — a step-after path (a horizontal run to the next point's x, then a vertical rise/drop to its y).
- **Stacked** (`chart-area-stacked`) — two series, `stacked: true`: each datum's values sum, drawn as one filled region atop the other rather than overlapping.
- **Legend** (`chart-area-legend`) — a stacked area chart with a `ChartLegend` below it.
- **Axes** (`chart-area-axes`) — both axes shown (`show_y_axis: true` alongside the default `show_x_axis`), with a reduced y-tick count (`y_tick_count: 3`) matching shadcn's own `tickCount={3}`.
- **Interactive** (`chart-area-interactive`) — 90 days of two-series data, a `Select` narrows the visible range to the last 90/30/7 days, with a `ChartLegend`.
- **Stacked, expand** (`chart-area-stacked-expand`) — three series stacked as *percentages of each datum's total* (`area: AreaOptions { stack_mode: StackMode::Expand, .. }`) — every datum's stack reaches exactly 100%, regardless of its raw total. Renders with `show_grid: false`: the shared grid still reflects the *raw* domain, not the percent one, until a follow-up makes it stack-mode-aware (see the component's own source comment).
- **Gradient** (`chart-area-gradient`) — `area: AreaOptions { gradient: true, .. }`: each series' fill is a top-to-bottom `<linearGradient>` (opaque near the line, fading toward the baseline) instead of a flat, uniform fill-opacity.
- **Icons** (`chart-area-icons`) — each series' `ChartSeries.icon` is set (`TrendingDown`/`TrendingUp`). Not yet visually wired up — `ChartLegend`/`ChartTooltip` don't read this field yet — so today this renders identically to `legend`; the config is ready for the moment they do.

## Accessibility

Nothing about `kind: ChartKind::Area` changes `Chart`'s own accessibility contract — see the [Chart](/component/?name=chart&) page: `role="img"` + `<title>`/`<desc>` on the svg, a real hidden `<table>` mirroring every datum, `role="graphics-symbol"` legend/tooltip swatches, and optional arrow-key stepping of the active point.
