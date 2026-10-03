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
            inner_radius: Radius::Px(30.0),
            outer_radius: Radius::Px(110.0),
            start_angle: 0.0,   // degrees, 0 = three o'clock
            end_angle: 360.0,   // counter-clockwise; may pass a full turn
            background: true,   // a muted track behind each ring
            ..Default::default()
        },
    }
    ChartTooltip { hide_label: true }
}
```

Every option means what the same-named Recharts `RadialBarChart`/`RadialBar` prop means, so a shadcn/ui or Recharts example's numbers carry over unchanged. Each ring sweeps from `start_angle` by `value / max * (end_angle - start_angle)`, where `max` is the largest value in the data, so the biggest value fills the whole sweep and a single-value gauge always does.

A polar chart is a square at most 250px wide, centred in its container. Lengths are CSS pixels.

## Options

- `inner_radius` and `outer_radius` are a `Radius`, `Px(n)` or `Percent(n)` of the max radius (half the chart's shorter side less a 5px margin). The defaults are Recharts': `Px(0.0)` and `Percent(80.0)`. The range between them is split into one band per datum. Each ring is inset 10% of its band on both sides, with its thickness rounded down to whole pixels, so rings from radius 30 to 110 over five data are 12px thick with 4px gaps.
- `start_angle` and `end_angle` are degrees, with `0` at three o'clock and angles increasing counter-clockwise. A start of `-90.0` is six o'clock. A sweep longer than a full turn, such as `-90.0` to `380.0`, keeps the largest ring a closed circle while the others stop short of it.
- `domain_max: Some(v)` makes `v` the value that fills the sweep, instead of the data's maximum.
- `background: true` draws a muted track behind every ring over the full sweep (Recharts' `<RadialBar background>`).
- `grid` is Recharts' `<PolarGrid>`:
  - `RadialGrid::Circles { radial_lines: true }` draws a circle through every ring and a spoke at every "nice" value step (every 20 for values up to 275).
  - `RadialGrid::Annulus { outer, inner }` draws a full muted ring between two radii (px), under a gauge whose bar covers only part of the turn.
- `corner_radius` rounds both ends of every bar, clamped to half its thickness.
- `labels` takes the pie chart's `PieLabels` (`List(vec![..])`, `Value`, `Percent`) and sets each label along its ring, starting 5 degrees past the ring's start (Recharts' `insideStart`).
- `center_text` writes a two-line total in the hole. `center_text_raised: true` lifts both lines into the upper half for a half-turn gauge.
- `stacked: true` puts every configured series' value for the first datum into one ring, end to end, in series order.
- Rings are `path[data-slot="chart-arc"]` elements with `data-index`, `data-start-angle` and `data-end-angle` in degrees, and `data-inner-radius` and `data-outer-radius` in pixels.
- The bars sweep in when the chart first appears; nothing animates under `prefers-reduced-motion: reduce`.

### Stacked rings differ from Recharts on purpose

In a stacked chart, each segment covers its own share of the stack total, so the segments fill the sweep exactly. Recharts keeps the angle scale at the largest *single* value and cuts the overflowing stack off at `end_angle`. Its stacked demo (570 mobile, 1,260 desktop) therefore splits the half turn 45/55, where the real shares are 31/69. This chart draws 31/69.

## Demos

- **Simple**: five rings from three o'clock over muted tracks.
- **Label**: rings from six o'clock over 470 degrees, each named along its arc.
- **Grid**: rings over circles and spokes.
- **Text**: a 250-degree gauge with rounded ends over a full muted ring, the value in the centre.
- **Shape**: a thick 100-degree gauge over a thinner full ring.
- **Stacked**: two series stacked into one half-turn ring with rounded segments.

## Accessibility

Radial charts share the [Chart](/component/chart/) behavior: the SVG is a named image (`role="img"`), and a visually hidden data table lists every ring's category and value for assistive technology. The chart's wrapper is focusable, and `ArrowLeft`, `ArrowRight`, `Home`, `End` and `Escape` move through the rings.
