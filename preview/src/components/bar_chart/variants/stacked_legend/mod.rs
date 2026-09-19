use super::super::component::*;
use dioxus::prelude::*;

/// Port of shadcn's `chart-bar-stacked.tsx` ("Bar Chart - Stacked +
/// Legend"): the same two-series data as the `stacked` variant, plus
/// `ChartLegend`.
///
/// Simplification, stated plainly: shadcn gives each stacked segment a
/// different corner radius depending on its position in the stack --
/// `desktop` (the bottom segment) rounds only its *bottom* corners
/// (`radius={[0, 0, 4, 4]}`), `mobile` (the top segment) rounds only its
/// *top* corners (`radius={[4, 4, 0, 0]}`) -- so only the stack's outer
/// silhouette is rounded and the seam between the two segments stays
/// square. The current engine draws every bar as a plain SVG `rect` with a
/// single, uniform `rx` (`chart/style.css`'s `[data-slot="chart-bar"] { rx:
/// var(--dx-radius-xs); }`), which rounds all four corners of every segment
/// identically -- visually close (both segments are still rounded) but not
/// per-corner-accurate at the internal seam. Per-corner rounding needs a
/// hand-built rounded-rect path (a plain `rect` has no per-corner `rx`),
/// which belongs in `engine/geometry.rs`/`components/series/bar.rs`
/// (`BarOptions.radius`, this lane's own primitive ownership per
/// `$S/stage2-common.md`) -- tracked as a follow-up enhancement to this
/// exact variant, not a blocking gap: the chart already stacks, labels, and
/// legends correctly without it.
fn generate_data() -> Vec<ChartDatum> {
    const ROWS: [(&str, f64, f64); 6] = [
        ("January", 186.0, 80.0),
        ("February", 305.0, 200.0),
        ("March", 237.0, 120.0),
        ("April", 73.0, 190.0),
        ("May", 209.0, 130.0),
        ("June", 214.0, 140.0),
    ];
    ROWS.iter()
        .map(|(label, desktop, mobile)| ChartDatum {
            label: label.to_string(),
            values: vec![Some(*desktop), Some(*mobile)],
        })
        .collect()
}

#[component]
pub fn Demo() -> Element {
    let config = ChartConfig::new()
        .series("desktop", "Desktop", "var(--dx-chart-1)")
        .series("mobile", "Mobile", "var(--dx-chart-2)");

    rsx! {
        Card {
            CardHeader {
                CardTitle { "Bar Chart - Stacked + Legend" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config, data: generate_data(), kind: ChartKind::Bar,
                    Chart { aria_label: "Visitors by month, desktop and mobile, stacked", stacked: true }
                    ChartTooltip { hide_label: true }
                    ChartLegend {}
                }
            }
            BarChartTrendFooter {}
        }
    }
}
