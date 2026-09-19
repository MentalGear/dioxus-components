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

`show_dots: true` draws a small circle at every defined data point, filled with that series'
color. `Chart`'s default row already grew a `desktop`-only demo (`dots`) — per-point custom
colors, a caller-supplied dot renderer, and value/category labels above each point are gallery
variants layered on top of this same `show_dots` mechanism (see each variant's own doc comment
once landed).

## Variants

- **main** (`chart-line-default.tsx`) — one series, no dots, the baseline shape every other
  variant below starts from.
- **linear** (`chart-line-linear.tsx`) — same data, `Curve::Linear`.
- **step** (`chart-line-step.tsx`) — same data, `Curve::Step`.
- **multiple** (`chart-line-multiple.tsx`) — two series (desktop/mobile), legend-free tooltip
  showing both rows.
- **dots** (`chart-line-dots.tsx`) — `show_dots: true`.
- **interactive** (`chart-line-interactive.tsx`) — a two-button header (a `CardAction`) toggles
  which of two series the chart actually draws, each button showing that series' running total.
