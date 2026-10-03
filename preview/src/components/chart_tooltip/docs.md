`ChartTooltip` and `ChartLegend` are the optional pieces of a [Chart](/component/chart/) that tell a reader what each series and value means. This page shows each option on its own: the indicator shape, hiding the label or indicator, a fixed heading, label and value formatters, per-series icons, and fully custom rows. Nothing extra is installed; it is the same `ChartTooltip` that ships with `chart`.

Place `ChartTooltip {}` as a sibling of `Chart` inside the `ChartContainer`. It shows the label and one row per series for the active data point, which is the point under the pointer or the one the keyboard has stepped to. It is hidden from assistive technology (`aria-hidden`) because the chart's data table is the accessible source of the same numbers.

## Indicator shape

```rust
ChartTooltip { indicator: TooltipIndicator::Line } // Dot (default) | Line | Dashed | None
```

`Dot` draws a small square swatch beside each row. `Line` and `Dashed` draw a full-height bar or dashed rule instead, which reads better when the row's value is already a magnitude and the swatch only needs to carry color. `None`, or `hide_indicator: true`, leaves the indicator out.

## Hiding the label or indicator

```rust
ChartTooltip { hide_label: true, hide_indicator: true }
```

Set either on its own or both together. With both set, each row shows only the series name and its value.

## Overriding the label or a row's name

```rust
ChartTooltip { label_key: Some("Activities".into()) }
```

`label_key` replaces the label row's text with a fixed string instead of the active category, which suits a static heading ("Activities") better than a per-point date. `name_key` does the same for every row's name, though with more than one series every row would then show the same text.

## Formatting the label or a value

```rust
ChartTooltip {
    label_format: |raw: String| format!("{raw} (spelled out)"),
    value_format: |v: f64| format!("{v} kcal"),
}
```

`label_format` transforms the label text (after `label_key`, if set) and `value_format` transforms each row's number. Neither changes the indicator or name markup; use `formatter` for that. Without `value_format`, values are shown with up to two decimals.

## Fully custom rows

```rust
ChartTooltip {
    formatter: |row: TooltipRow| rsx! {
        span { "{row.label}" }
        span {
            if let Some(v) = row.value { "{v} kcal" } else { "—" }
        }
        if row.is_last {
            div { "Total: {row.total} kcal" }
        }
    },
}
```

`formatter` replaces the inside of every row (indicator, name and value) with whatever `Element` you return. It is called once per series with a `TooltipRow`: the series `key`, its resolved `label`, the `value` (`None` for a gap), the `color` as a `var(--color-<key>)` reference, the row's `index`, an `is_last` flag, and `total`, the sum of every series' value at that point. The total and flag make it easy to append a "Total" line after the last row without recomputing anything.

## Series icons

A series can carry an icon that replaces its color swatch in both the tooltip and the legend. Set it on the series in your config:

```rust
let mut config = ChartConfig::new()
    .series("running", "Running", "var(--dx-chart-1)")
    .series("swimming", "Swimming", "var(--dx-chart-2)");
config.series[0].icon = Some(ChartIcon(Callback::new(|()| rsx! { Footprints {} })));
```

The icon is wrapped in a `role="graphics-symbol"` element named after the series, exactly like the swatch it replaces. In the tooltip an icon is always shown when present, whatever `indicator` or `hide_indicator` say. `ChartLegend { hide_icon: true }` falls back to plain swatches in the legend while the tooltip keeps its icons.

## Legend options

`ChartLegend` shows one swatch (or icon) and label per series. `vertical_align` (`LegendAlign::Top` or `LegendAlign::Bottom`, the default) sets which side of the chart it belongs on through a `data-align` attribute, and `hide_icon` is described above. A single-series pie or radial chart lists one entry per slice or ring instead.

## Demos

- **Default** — dot indicator, label and every value.
- **Line indicator** — `indicator: TooltipIndicator::Line`.
- **No indicator** — `hide_indicator: true`.
- **No label** — `hide_indicator: true, hide_label: true`.
- **Custom label** — `label_key` as a fixed heading.
- **Label formatter** — `label_format` spelling the date out in full.
- **Formatter** — `formatter` showing each value as "N kcal".
- **Icons** — per-series icons instead of swatches.
- **Advanced** — `formatter` again, adding a computed total row after the last series.

All demos use a six-day running and swimming calorie dataset drawn as a stacked bar chart.
