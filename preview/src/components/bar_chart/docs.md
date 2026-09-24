Bar Chart is a gallery of `ChartKind::Bar` configurations built entirely from the installable
`chart` package (see that component's own docs for `ChartConfig`/`ChartContainer`/`Chart`/
`ChartTooltip`/`ChartLegend`) plus `Card` for the surrounding chrome. There is no separate
`bar_chart` primitive -- installing this component pulls in `chart` and `card` as
`componentDependencies` and gives you the demo source below as a starting point.

## A single series

```rust
let config = ChartConfig::new().series("desktop", "Desktop", "var(--dx-chart-1)");
let data = vec![
    ChartDatum { label: "January".into(), values: vec![Some(186.0)] },
    // ...
];

ChartContainer { config, data, kind: ChartKind::Bar,
    Chart { aria_label: "Visitors by month, desktop" }
    ChartTooltip { hide_label: true }
}
```

## Multiple series (grouped)

Add a second series to `ChartConfig` and a second value to every `ChartDatum` -- bars for the same
category draw side by side automatically, no extra prop needed.

## Stacked

`Chart { stacked: true }` stacks every configured series' bars instead of grouping them. Combine
with `ChartLegend` once there is more than one series to identify each stack segment.

## Interactive (toggle which series draws)

Keep the full dataset (every series' values) around, but build a single-series `ChartConfig` from
whichever one the user has picked, and re-map each `ChartDatum` down to just that series' value
before passing it to `ChartContainer`. See this gallery's `interactive` variant for the full
pattern, including per-series running totals shown in the card header.

## Horizontal orientation

`Chart { bar: BarOptions { horizontal: true, .. } }` draws the category axis running top-to-bottom
and values running left-to-right instead of the default vertical layout -- hide both of `Chart`'s
default axes (`show_x_axis`/`show_y_axis: false`) the way shadcn's own horizontal demos do, since
this family draws its own category labels at the plot's left edge when `horizontal` is set.

## Negative values and per-datum color

Set `ChartDatum::color` per datum (e.g. one color for a positive value, another for negative) to
color each bar independently instead of from one flat series color. `ChartKind::Bar` always draws
an explicit zero-line, so a chart mixing positive and negative values has a visible baseline.

## Value and inside labels

`BarOptions::value_labels` draws each bar's own value just outside its far end.
`BarOptions::inside_labels` (takes precedence when both are set) additionally draws the datum's
category name inside the bar near its start -- useful paired with `horizontal` and every axis
hidden, so the labels themselves carry the information an axis normally would.

## A highlighted ("active") bar

`BarOptions::active_index` marks one bar `data-active="true"` (every other bar `"false"`); the
themed stylesheet dims every non-active bar once at least one is marked active, independent of
hover.
