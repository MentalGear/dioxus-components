Line Chart is shadcn's ten-demo `chart-line-*` gallery, ported onto this repo's own `Chart`
primitive (`ChartKind::Line`) rather than Recharts. There is nothing to install beyond the shared
`chart` package (`dx components add chart`) — every demo below is composed entirely from
`ChartContainer`, `Chart`, and `ChartTooltip`; see `chart`'s own docs page for the full
`ChartConfig`/`ChartDatum` model, the accessibility contract (hidden data table, `role="img"`,
keyboard stepping), and the color-token system.

## Basic usage

```rust
let config = ChartConfig::new().series("desktop", "Desktop", "var(--dx-chart-1)");
let data = vec![
    ChartDatum { label: "January".into(), values: vec![Some(186.0)] },
    ChartDatum { label: "February".into(), values: vec![Some(305.0)] },
    // ...
];

ChartContainer { config, data, kind: ChartKind::Line,
    Chart {
        aria_label: "Visitors by month, desktop",
        x_label: "Month",
    }
    ChartTooltip { hide_label: true } // a single series' tooltip doesn't need to repeat its name
}
```

## Curve

`Chart`'s `curve` prop selects the line's interpolation (`Curve::Linear`, `Curve::Monotone` —
the default, a smooth monotonicity-preserving cubic — or `Curve::Step`, a step-after line).
shadcn's own `type="natural"` (a natural cubic spline) is ported as `Curve::Monotone` throughout
this gallery — the closest existing curve, and visually indistinguishable for these demos' data;
see `primitives/src/chart/engine/curve.rs` for the exact algorithm and its own citation.

## Dots

`Chart { line: LineOptions { dots: true, .. } }` draws a small circle at every defined data
point, filled with that series' color (or `ChartDatum::color` per point, when set) and doubled in
radius when hovered/keyboard-focused. `LineOptions::dot` replaces the default circle entirely with
a caller-supplied renderer, given a [`DotContext`] (already-scaled position, raw value, whether
this point is active) for every defined point.

## Labels

`LineOptions::labels` draws a text label above each defined point: `LineLabels::Value` (the
point's own value, formatted like the hidden table's cells) or `LineLabels::Custom` (a
caller-supplied callback given the point's index, e.g. to label by category instead of value).

## Variants

- **main** (`chart-line-default.tsx`) — one series, no dots, the baseline shape every other
  variant below starts from.
- **linear** (`chart-line-linear.tsx`) — same data, `Curve::Linear`.
- **step** (`chart-line-step.tsx`) — same data, `Curve::Step`.
- **multiple** (`chart-line-multiple.tsx`) — two series (desktop/mobile), legend-free tooltip
  showing both rows.
- **dots** (`chart-line-dots.tsx`) — `LineOptions::dots: true`.
- **dots_colors** (`chart-line-dots-colors.tsx`) — one series over browser categories, each
  point's own dot colored individually via `ChartDatum::color`.
- **dots_custom** (`chart-line-dots-custom.tsx`) — `LineOptions::dot` draws a diamond in place of
  the default circle.
- **label** (`chart-line-label.tsx`) — `LineLabels::Value`, grid and y-axis hidden.
- **label_custom** (`chart-line-label-custom.tsx`) — `LineLabels::Custom` labels each point by its
  own category name instead of its value.
- **interactive** (`chart-line-interactive.tsx`) — a two-button header (a `CardAction`) toggles
  which of two series the chart actually draws, each button showing that series' running total.
