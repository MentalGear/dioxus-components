Bar charts compare values across categories. This page is a gallery of bar configurations built from the [Chart](/component/chart/) package (`ChartContainer`, `Chart`, `ChartTooltip`, `ChartLegend`) and `Card` for the surrounding frame. Installing this component adds `chart` and `card` for you.

## Quick start

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

- `horizontal: true` draws categories top to bottom and values left to right (Recharts' `layout="vertical"`). The axes swap: the y axis (`show_y_axis: true`) holds the category labels and the x axis the values, so shadcn's horizontal demos use `show_x_axis: false`.
- `radius` rounds the bars, in px: `BarRadius::all(8.0)`, or per corner with `BarRadius::corners(top_left, top_right, bottom_right, bottom_left)`. `series_radius` sets it per series, which is how a stack rounds only its outer ends (`[0, 0, 4, 4]` below, `[4, 4, 0, 0]` on top).
- `category_gap` (default `0.1`) and `bar_gap` (default `4`) size the bars as Recharts does: 10% of each category's band is left on either side, and grouped bars sit 4px apart.
- `value_labels: true` prints each bar's value 12px past its end, and `category_labels: true` prints its category name 5px past its end in the bar's own color (above a positive bar, below a negative one).
- `inside_labels: true` prints the category name inside the bar near its start and the value past its end. It takes precedence over the other two and suits a horizontal chart with the axes hidden. The label options are ignored when the chart is stacked.
- `active_index: Some(i)` highlights one bar: it gets `data-active="true"`, which the theme draws as shadcn's active bar -- 0.8 fill opacity and a dashed outline in the bar's own color -- and the other bars stay as they are.

For per-bar colors, set `ChartDatum::color` on each datum, for example one color for positive values and another for negative ones; the value axis then spans both signs, with round ticks on either side of zero.

## Demos

Each demo is a port of the shadcn/ui chart of the same name -- same data, series order, radius, margins, axes, labels and tooltip -- checked against shadcn's own source by a test.

- **Default** — one series, 8px-rounded bars.
- **Multiple** — two series grouped side by side.
- **Stacked** and **Stacked with legend** — the same two series stacked, rounding only the stack's outer ends.
- **Horizontal** — `horizontal: true` with the months on the y axis.
- **Mixed** — a horizontal chart where every bar has its own color through `ChartDatum::color`, with full browser names on the y axis.
- **Label** — `value_labels: true`.
- **Custom label** — `inside_labels: true` on a horizontal chart with no axes.
- **Active** — one highlighted bar through `active_index`.
- **Negative** — positive and negative values colored per datum, each labelled with its month (`category_labels: true`) instead of an x axis.
- **Interactive** — a header lets the reader switch which of two series is drawn. To build the same, keep the full dataset in a signal, build a one-series `ChartConfig` for the chosen series, and map each `ChartDatum` down to that series' value before passing both to `ChartContainer`.

## Accessibility

Bar charts follow the shared [Chart](/component/chart/) behavior: a named SVG image, a hidden data table with every value, and arrow-key, `Home`, `End` and `Escape` stepping through bars on the focusable wrapper.
