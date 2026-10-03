Radar charts compare several measures for the same item by laying categories out around a circle and drawing one polygon per series. This page covers what is specific to `kind: ChartKind::Radar`: the grid options, how series are drawn, and hover. Everything else (`ChartConfig`, colors, the data table, keyboard behavior) is shared with every chart; see [Chart](/component/?name=chart).

## Usage

```rust
let config = ChartConfig::new()
    .series("desktop", "Desktop", "var(--dx-chart-1)")
    .series("mobile", "Mobile", "var(--dx-chart-2)");

// One ChartDatum per category, which becomes one axis around the circle.
let data = vec![
    ChartDatum { label: "January".into(), values: vec![Some(186.0), Some(80.0)], ..Default::default() },
    // ...
];

ChartContainer { config, data, kind: ChartKind::Radar,
    Chart {
        aria_label: "Visitors by month, desktop and mobile",
        radar: RadarOptions {
            grid: RadarGrid::Polygon, // Polygon | Circle | CircleFill | CircleNoLines | Fill | None | Custom { .. }
            fill_opacity: 0.6,
            dots: false,       // a circle at every vertex
            lines_only: false, // stroke only, no fill
            outer_radius: 0.8, // fraction of the available radius
            axis_labels: true, // category names around the rim
        },
    }
    ChartTooltip {}
    ChartLegend {}
}
```

All of these are the defaults, so `radar: RadarOptions::default()` (or omitting the prop) gives the same chart.

## Grid

`RadarGrid` chooses the rings drawn at each value tick:

- `Polygon` (default) — straight-sided rings with a spoke to each category.
- `Circle` — circular rings, still with spokes.
- `CircleFill` and `Fill` — circular or polygon rings with the interior tinted in the first series' color.
- `CircleNoLines` — circular rings with no spokes.
- `None` — no grid.
- `Custom { values, spokes }` — rings at exactly the values you give, in your data's own units, and a flag for whether spokes are drawn.

## How series are drawn

Each series is one closed polygon (`path[data-slot="chart-radar-area"]`) that is both filled and stroked. `fill_opacity` sets the fill, and `lines_only: true` forces it to `0` so only the outline shows. `dots: true` adds a circle (`circle[data-slot="chart-dot"]`) at every vertex.

A category with no value for a series (`None`) is skipped: its two neighbors are joined directly instead of drawing a made-up zero. `outer_radius` is the outermost ring's size as a fraction of the available space, and `axis_labels: false` removes the category names around the rim.

## Hover

Each category owns an angular wedge reaching from the center to the outer edge. Hovering anywhere in a wedge activates that category, and the tooltip shows every series' value for it. Keyboard stepping activates categories in order.

## Variants

The gallery shows the default polygon grid, `dots`, `lines_only`, `multiple` series, a `legend`, series icons, a variant with the category labels hidden (`axis_labels: false`), and each grid type (`grid_circle`, `grid_circle_fill`, `grid_circle_no_lines`, `grid_fill`, `grid_none`, `grid_custom`).

## Accessibility

The hidden data table lists every category and every series' exact value regardless of the visual options above. The SVG is a named image and the wrapper is keyboard focusable; see [Chart](/component/?name=chart) for the full behavior.
