Radial charts draw each value as a bar wrapped around a circle, which suits a handful of progress-style measures or a single gauge. They are the other arc-based member of the [Chart](/component/chart/) family: `kind: ChartKind::RadialBar` plus a `RadialOptions` value. Like [Pie chart](/component/pie_chart/), there is nothing extra to install; `chart` is everything a radial chart needs.

## Quick start

```rust
let config = ChartConfig::new()
    .series("visitors", "Visitors", "var(--dx-chart-1)");

// Each ChartDatum is its own concentric ring, with the first one innermost.
// Without a `color`, rings use --dx-chart-1, --dx-chart-2, ... by position.
let data = vec![
    ChartDatum { label: "Chrome".into(), values: vec![Some(275.0)], color: Some("var(--dx-chart-1)".into()) },
    ChartDatum { label: "Safari".into(), values: vec![Some(200.0)], color: Some("var(--dx-chart-2)".into()) },
    // ...
];

ChartContainer { config, data, kind: ChartKind::RadialBar,
    Chart {
        aria_label: "Visitors by browser",
        radial: RadialOptions {
            inner_radius: 30.0, // where the innermost ring starts
            outer_radius: 110.0, // 0.0 sizes it from the chart automatically
            start_angle: 0.0,   // radians; 0.0 is twelve o'clock, clockwise
            end_angle: std::f64::consts::TAU, // may exceed a full turn
            corner_radius: 0.0,
            grid: false,        // a muted background track behind each ring
            labels: PieLabels::None,
            stacked: false,     // true stacks all series into one ring
            center_text: None,  // Some((primary, secondary)) in the hole
        },
    }
    ChartTooltip {}
}
```

Each ring is scaled against the largest value in the data, so the biggest value fills the whole sweep from `start_angle` to `end_angle`.

## Options

- `inner_radius` should normally be above zero; otherwise the innermost ring starts at a single point and its length is hard to judge. `outer_radius` defaults to `0.0`, which fits the ring stack to the chart's size.
- `start_angle` and `end_angle` set the sweep in radians, where `0.0` is twelve o'clock and angles increase clockwise. Shift both to rotate the chart (a start of `FRAC_PI_2` is three o'clock, `-FRAC_PI_2` nine o'clock), or make the sweep longer than a full turn so the ends of the first and last rings do not touch.
- `grid: true` draws a muted full-sweep track behind every ring.
- `labels` takes the same `PieLabels` values as the pie chart (`None`, `Value`, `Percent`, `List`) and prints each label along its ring, starting just inside the ring's start.
- `center_text` writes a two-line total in the hole, which is the usual way to make a gauge. It needs `inner_radius` above zero.
- `stacked: true` puts every configured series' value for the first datum into one ring, end to end. It only makes sense with two or more series.

Rings are `path[data-slot="chart-arc"]` elements with `data-index`, `data-start-angle` and `data-end-angle`, the same shape the pie chart uses, so one stylesheet can cover both.

## Demos

- **Simple** — five rings, one per browser.
- **Label** — each ring's category name drawn on its arc.
- **Grid** — `grid: true`.
- **Text** — one ring with a total in the hole, gauge-style.
- **Shape** — one ring with rounded ends through `corner_radius`.
- **Stacked** — two series stacked into one ring with `stacked: true`.

## Accessibility

Radial charts share the [Chart](/component/chart/) behavior: the SVG is a named image (`role="img"`), and a visually hidden data table lists every ring's category and value for assistive technology. The chart's wrapper is focusable, and `ArrowLeft`, `ArrowRight`, `Home`, `End` and `Escape` move through the rings.
