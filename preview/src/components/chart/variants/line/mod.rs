use super::super::component::*;
use dioxus::prelude::*;

/// A single series, monotone curve (the `Chart`/`Curve` default -- left
/// unset below), with each data point's dot drawn (`show_dots`). Matches
/// shadcn's own `chart-line-dots` demo shape.
fn generate_data() -> Vec<ChartDatum> {
    const MONTHS: [&str; 6] = ["January", "February", "March", "April", "May", "June"];
    MONTHS
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let t = i as f64;
            let desktop = (200.0 + 110.0 * (t * 0.8 + 0.4).sin() + t * 6.0).max(10.0).round();
            ChartDatum {
                label: label.to_string(),
                values: vec![Some(desktop)],
                ..Default::default()
            }
        })
        .collect()
}

#[component]
pub fn Demo() -> Element {
    let config = ChartConfig::new().series("desktop", "Desktop", "var(--dx-chart-1)");

    rsx! {
        ChartContainer { config, data: generate_data(), kind: ChartKind::Line,
            Chart {
                aria_label: "Visitors by month, desktop",
                line: LineOptions { dots: true },
            }
            ChartTooltip {}
        }
    }
}
