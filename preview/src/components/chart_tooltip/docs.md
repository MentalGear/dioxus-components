Chart Tooltip gathers the tooltip and legend *options* the `chart` package's `ChartTooltip` and
`ChartLegend` expose — the indicator shape, hiding the label or indicator, a custom label, a label
or value formatter, per-series icons, and a fully custom row renderer — one demo per option,
mirroring shadcn's own nine `chart-tooltip-*` gallery entries. It installs no primitive of its own:
every demo below composes the same `Chart`/`ChartContainer`/`ChartTooltip` pieces `chart` ships.

## Indicator shape

```rust
ChartTooltip { indicator: TooltipIndicator::Line } // Dot (default) | Line | Dashed | None
```

`Dot` draws a small square swatch, matching every other chart demo's default. `Line` and `Dashed`
draw a full-row-height bar or dashed rule instead — useful when a row's own value already reads as
a magnitude and the swatch mainly needs to carry color, not shape. `None` (or the separate
`hide_indicator: true` flag, kept for parity with shadcn's own distinct `hideIndicator` prop) omits
the indicator entirely.

## Hiding the label or indicator

```rust
ChartTooltip { hide_label: true, hide_indicator: true }
```

Either can be set independently. With both set, a row shows only its series' name and value.

## Overriding the label or a row's name

```rust
ChartTooltip { label_key: Some("Activities".into()) }
```

`label_key` replaces the label row's text with a literal string instead of the active datum's own
category label — useful for a static heading ("Activities") rather than a per-datum date. `name_key`
does the same for every row's displayed name. Both are this crate's simplified equivalent of
shadcn's `labelKey`/`nameKey`, which instead point at a second, free-form lookup into `ChartConfig`
— a mechanism this crate's own strongly-typed, positional `ChartConfig` has no equivalent of, so the
same *observable* effect is exposed directly as a literal string instead.

## Formatting the label or a value

```rust
ChartTooltip {
    label_format: |raw: String| /* e.g. */ format!("{raw} (spelled out)"),
    value_format: |v: f64| format!("{v} kcal"),
}
```

`label_format` transforms the (possibly `label_key`-overridden) label text; `value_format`
transforms each row's own numeric value. Both leave the indicator/name markup untouched — for that,
use `formatter` instead.

## Fully custom rows

```rust
ChartTooltip {
    formatter: |row: TooltipRow| rsx! {
        span { "{row.label}" }
        span { "{row.value:?} kcal" }
        if row.is_last {
            div { "data-slot": "chart-tooltip-total", "Total: {row.total} kcal" }
        }
    },
}
```

`formatter` replaces a row's entire inner content (indicator, name, and value together) with
whatever `Element` it returns, called once per configured series with a [`TooltipRow`] carrying
that row's key, resolved label, value, color, position, and (since this crate has no equivalent of
shadcn's free-form per-datum payload object) a precomputed cross-series total — enough to build the
`advanced` demo's own trailing "Total" row without recomputing it from raw data.

## Icons

```rust
ChartConfig::new().series_with_icon("running", "Running", "var(--dx-chart-1)", || rsx! { Footprints {} })
```

A series with an icon renders that icon in place of its indicator swatch, in both the tooltip and
the legend (`ChartLegend`'s own `hide_icon: true` opts a legend back out of icons while keeping a
series' icon in the tooltip). The icon sits in a `role="graphics-symbol"` wrapper with the series'
label as its accessible name, exactly like the swatch it replaces.

## The nine demos

| Demo | What it shows |
| --- | --- |
| Default | `ChartTooltipContent`'s own defaults: dot indicator, label and every value shown. |
| Line Indicator | `indicator: TooltipIndicator::Line`. |
| No Indicator | `hide_indicator: true`. |
| No Label | `hide_indicator: true, hide_label: true`. |
| Custom label | `label_key`, pointing the label row at a fixed heading. |
| Label Formatter | `label_format`, spelling the date out in full. |
| Formatter | `formatter`, replacing each row's value with a custom "N kcal" layout. |
| Icons | Per-series icons instead of swatches. |
| Advanced | `formatter` again, this time appending a computed total row after the last series. |

Every demo ports its data, series, and copy from shadcn's own
`registry/new-york-v4/charts/chart-tooltip-*.tsx` sources (cited in each variant's own doc comment)
— a six-day running/swimming calorie dataset, rendered as a stacked bar chart.
