RadarChart is the `Radar` family of the shared `chart` engine (`ChartConfig`/`ChartContainer`/`Chart`/
`ChartTooltip`/`ChartLegend` -- see the [Chart](/component/?name=chart) docs for colors, the a11y
contract, and the hidden data table shared by every chart family). This page covers what's specific
to `ChartKind::Radar`: one polygon per series, category axes arranged around a circle, and the grid
options a radar chart offers instead of a Cartesian axis/gridline pair.

## Component structure

```rust
let config = ChartConfig::new()
    .series("desktop", "Desktop", "var(--dx-chart-1)")
    .series("mobile", "Mobile", "var(--dx-chart-2)");

// One ChartDatum per category (an axis around the circle), same shape as
// every other chart family.
let data = vec![
    ChartDatum { label: "January".into(), values: vec![Some(186.0), Some(80.0)] },
    // ...
];

ChartContainer { config, data, kind: ChartKind::Radar,
    Chart {
        aria_label: "Visitors by month, desktop and mobile",
        radar: RadarOptions {
            grid: RadarGrid::Polygon, // Polygon | Circle | CircleFill | CircleNoLines | Fill | None | Custom { .. }
            fill_opacity: 0.6,
            dots: false,       // a circle at every vertex
            lines_only: false, // stroke-only series (fill_opacity forced to 0)
            outer_radius: 0.8, // fraction of the available half-extent
            axis_labels: true, // category labels around the rim
        },
    }
    ChartTooltip {}
    ChartLegend {}
}
```

## Grid

`RadarGrid` picks the ring shape drawn at each nice tick of the magnitude axis:

- `Polygon` (default) -- straight-sided rings through every category's own angle, plus a spoke per
  category. Matches shadcn's default `<PolarGrid />`.
- `Circle` -- circular rings instead of polygon rings, still with spokes.
- `CircleFill` / `Fill` -- circular or polygon rings, with the rings' interior tinted using the
  first configured series' color (matches shadcn's `chart-radar-grid-circle-fill`/`-grid-fill`
  demos, both of which tint the grid with their one series' own color).
- `CircleNoLines` -- circular rings with no spokes.
- `None` -- no grid at all.
- `Custom { values, spokes }` -- draw rings at exactly these values (in the chart's own data
  domain, not pixels), instead of the default nice-tick set, and choose whether spokes are drawn.

## Series rendering

Each series draws as one closed polygon (`path[data-slot="chart-radar-area"][data-series]`) --
there is no separate fill/stroke path pair the way Area does; the same path is both filled
(`fill-opacity`, from `RadarOptions::fill_opacity`) and stroked. Setting `lines_only: true` forces
`fill_opacity` to `0`, leaving only the stroke -- shadcn's `chart-radar-lines-only` demo. `dots: true`
adds a `circle[data-slot="chart-dot"][data-index]` at every defined vertex.

A category with no value for a series (`None`) is skipped -- its vertex is omitted, connecting its
two neighbors directly -- rather than drawn as a fabricated zero-radius point.

## Hover

Each category gets its own angular hit-sector (a pie-slice-shaped wedge from the center to the
outer radius) -- hovering anywhere inside a category's wedge activates it, the same "the hit
target's own shape does the work, no coordinate math" principle the Cartesian families' hit-bands
use. The hidden data table (categories x series) is unaffected by any visual option above; it
always lists every category and every series' exact value.
