use super::super::component::*;
use dioxus::prelude::*;

/// Two series on a scale where stacking is the legible choice, shared by the
/// stacked area and stacked bar chart below so both show the same data.
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
        // A styled `p`, not a real heading: this demo is rendered on pages
        // where the ambient heading level isn't fixed, so a real `h*` could
        // skip a level.
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
