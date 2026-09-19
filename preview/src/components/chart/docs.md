Chart is a themed, data-driven SVG chart engine (area, bar, line) built entirely from house
primitives — no third-party charting dependency, no injected/opaque markup. It mirrors shadcn's
own `ChartConfig` → CSS-variable theming idea, adapted to Dioxus's data-driven (not
children-as-configuration) component model: the series/axis data is a **prop**, not something a
parent introspects from nested children.

## Component structure

```rust
// ChartConfig maps a series key to its label and color. Order matters -- it
// is also the legend/tooltip row order. `color` can be any CSS color or a
// `var(--token)` reference; the theme ships `--dx-chart-1..8` for this.
let config = ChartConfig::new()
    .series("desktop", "Desktop", "var(--dx-chart-1)")
    .series("mobile", "Mobile", "var(--dx-chart-2)");

// One ChartDatum per x-axis category. `values` has one entry per series, in
// the same order as `config` -- `None` renders as a gap (line/area) or a
// zero-height bar, never a fabricated zero.
let data = vec![
    ChartDatum { label: "January".into(), values: vec![Some(186.0), Some(80.0)] },
    ChartDatum { label: "February".into(), values: vec![Some(305.0), Some(200.0)] },
    // ...
];

ChartContainer {
    // Scopes the generated `--color-<key>` CSS variables to this instance
    // via `data-chart="<id>"`. Auto-generated if omitted.
    config,
    data,
    kind: ChartKind::Area, // Area | Bar | Line

    // The chart itself: axes, grid, marks, the hidden data table, and the
    // hover hit-bands. `aria_label` is required -- it is the chart's
    // accessible name.
    Chart {
        aria_label: "Visitors by month, desktop and mobile",
        stacked: false,
        show_dots: false, // Line only
    }

    // Optional: the hover tooltip. Rendered even when closed (CSS hides
    // it) so hydration never has to attach it after the fact.
    ChartTooltip {}

    // Optional: a legend with one swatch per configured series.
    ChartLegend {}
}
```

`Chart`, `ChartTooltip`, and `ChartLegend` are independent, explicitly-rendered pieces (matching
shadcn's own `<ChartTooltip content={<ChartTooltipContent />} />` idiom) — omit whichever your
chart doesn't need, or a single/dual-series chart that doesn't need a legend at all.

## Colors

Every series color is applied as a real CSS custom property (`--color-<key>`), scoped by
`data-chart="<id>"` and generated once by `ChartContainer` from its `config` prop — never inlined
per-mark. A series' `color` field can be a literal CSS color or, more usefully, one of this
theme's own categorical tokens (`--dx-chart-1` through `--dx-chart-8`), which are already tuned
for both light and dark mode and for colorblind-safe adjacent contrast.

## Accessibility contract

Chart does not use `role="application"` anywhere — that ARIA escape hatch hands every keystroke to
the widget and strips a screen-reader user of ordinary browse-mode navigation, and neither the APG
nor Radix defines a chart pattern to justify it. Instead:

- The SVG root carries `role="img"`, a required non-empty `aria-label` (`Chart`'s `aria_label`
  prop), and a `<title>`/optional `<desc>` — a single, indivisible graphic, per the WAI-ARIA
  Graphics Module 1.0's own definition of that role.
- A real, visually-hidden `<table>` mirrors the exact same series/category/value data as the
  chart's actual screen-reader-facing path — one row per data point, one column per series,
  natively and correctly keyboard-navigable with zero bespoke widget behavior to get wrong.
- Legend swatches carry `role="graphics-symbol"` (the Graphics Module's own role for an atomic,
  repeated glyph) plus an `aria-label` naming the series.
- Optional arrow-key stepping of the visual tooltip (`Chart`'s `keyboard` prop, on by default) is
  an *additive* sighted-keyboard-user affordance layered on top of the hidden table, cited to
  Recharts' `accessibilityLayer` as a tier-3 opinion — never the only way to reach the data.

## Keyboard

When `keyboard: true` (the default), `Chart`'s own wrapping element (not the SVG itself) is
focusable (`tabindex="0"`, `role="group"`, `aria-roledescription="chart"`):

- `ArrowRight` / `ArrowLeft` step the active data point (clamped, no wraparound); swapped under an
  RTL context.
- `Home` / `End` jump to the first / last data point.
- `Escape` clears the active point and closes the tooltip.

Hovering a data point's invisible hit-band does the same thing via pointer input — both paths
drive the same `active_index` state, so the tooltip and the visual cursor line always agree with
whichever input method is in use.
