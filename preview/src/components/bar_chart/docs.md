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

## Coming in a follow-up

This gallery currently ships the variants that need no new primitive feature: the default,
multiple-series, stacked (with and without a legend), and interactive demos above. A horizontal
orientation, negative values with a drawn zero line, per-datum colors, value/inside labels, and a
single highlighted ("active") bar are tracked as this component's own follow-up commits (see
`primitives/src/chart/components/series/bar.rs`'s own module doc once it lands) and will be added
to this gallery and this file as they ship.
