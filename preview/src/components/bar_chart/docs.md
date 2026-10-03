Bar charts compare values across categories. This page is a gallery of bar configurations built from the [Chart](/component/?name=chart) package (`ChartContainer`, `Chart`, `ChartTooltip`, `ChartLegend`) and `Card` for the surrounding frame. Installing this component adds `chart` and `card` for you.

## Usage

```rust
let config = ChartConfig::new().series("desktop", "Desktop", "var(--dx-chart-1)");
let data = vec![
    ChartDatum { label: "January".into(), values: vec![Some(186.0)], ..Default::default() },
    // ...
];

ChartContainer { config, data, kind: ChartKind::Bar,
    Chart { aria_label: "Visitors by month, desktop" }
    ChartTooltip { hide_label: true }
}
```

## Multiple series

Add a second series to the `ChartConfig` and a second value to every `ChartDatum`. Bars for the same category are drawn side by side with no extra prop. Set `Chart { stacked: true }` to stack them instead, and add a `ChartLegend` once there is more than one series so each segment can be identified. A missing value (`None`) simply leaves that bar out.

## Options

Bar-specific settings live in `bar: BarOptions { .. }` on `Chart`:

- `horizontal: true` draws categories top to bottom and values left to right. Hide both default axes (`show_x_axis: false`, `show_y_axis: false`), because in this layout the chart draws its own category labels at the left edge.
- `value_labels: true` prints each bar's value just beyond its end.
- `inside_labels: true` also prints the category name inside the bar near its start. It takes precedence over `value_labels` and suits a horizontal chart with the axes hidden. Both label options are ignored when the chart is stacked.
- `active_index: Some(i)` highlights one bar: it gets `data-active="true"` and every other bar `data-active="false"`, which the theme uses to dim the rest.

For per-bar colors, set `ChartDatum::color` on each datum, for example one color for positive values and another for negative ones. Bar charts always draw a zero line, so mixed positive and negative values have a visible baseline.

## Variants

- **Default** — one series.
- **Multiple** — two series grouped side by side.
- **Stacked** and **Stacked with legend** — the same two series stacked.
- **Horizontal** — `horizontal: true` with the axes hidden.
- **Mixed** — a horizontal chart where every bar has its own color through `ChartDatum::color`.
- **Label** — `value_labels: true`.
- **Custom label** — `inside_labels: true` on a horizontal chart.
- **Active** — one highlighted bar through `active_index`.
- **Negative** — positive and negative values colored per datum around a zero line.
- **Interactive** — a header lets the reader switch which of two series is drawn. To build the same, keep the full dataset in a signal, build a one-series `ChartConfig` for the chosen series, and map each `ChartDatum` down to that series' value before passing both to `ChartContainer`.

## Accessibility

Bar charts follow the shared [Chart](/component/?name=chart) behavior: a named SVG image, a hidden data table with every value, and arrow-key, `Home`, `End` and `Escape` stepping through bars on the focusable wrapper.
