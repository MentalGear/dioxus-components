use super::super::component::*;
use dioxus::prelude::*;

/// Two series, deliberately on a scale where stacking (rather than
/// grouping/overlap) is the legible choice -- matches shadcn's own
/// `chart-area-stacked`/`chart-bar-stacked` demo shapes. Shared between the
/// area and bar chart below so the two are visibly the same data, just
/// stacked with a different mark (dev-docs/research/chart-2026-09-19.md
/// §6.7's brief for this variant: "stacked area + stacked bar").
fn generate_data() -> Vec<ChartDatum> {
    const MONTHS: [&str; 6] = ["January", "February", "March", "April", "May", "June"];
    MONTHS
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let t = i as f64;
            let desktop = (140.0 + 70.0 * (t * 0.6 + 0.2).sin() + t * 9.0).max(15.0).round();
            let mobile = (80.0 + 45.0 * (t * 1.1 + 0.9).sin() + t * 5.0).max(10.0).round();
            ChartDatum {
                label: label.to_string(),
                values: vec![Some(desktop), Some(mobile)],
                ..Default::default()
            }
        })
        .collect()
}

#[component]
pub fn Demo() -> Element {
    let config = ChartConfig::new()
        .series("desktop", "Desktop", "var(--dx-chart-1)")
        .series("mobile", "Mobile", "var(--dx-chart-2)");

    rsx! {
        // Styled `p`, not a real heading: this demo is composed both on its
        // own route and inlined into the home page's gallery (every
        // variant's `Demo` renders there too), where the ambient heading
        // level isn't fixed -- introducing a real `h*` here risks a skipped
        // level in one context or the other. Same reasoning + precedent as
        // `tooltip/variants/main/mod.rs`'s own "Tooltip title" label.
        p { style: "margin: 0 0 8px; font-weight: 660;", "Stacked area" }
        ChartContainer { config: config.clone(), data: generate_data(), kind: ChartKind::Area,
            Chart {
                aria_label: "Visitors by month, desktop and mobile, stacked",
                stacked: true,
            }
            ChartTooltip {}
            ChartLegend {}
        }

        p { style: "margin: 24px 0 8px; font-weight: 660;", "Stacked bar" }
        ChartContainer { config, data: generate_data(), kind: ChartKind::Bar,
            Chart {
                aria_label: "Visitors by month, desktop and mobile, stacked",
                stacked: true,
            }
            ChartTooltip {}
            ChartLegend {}
        }
    }
}
