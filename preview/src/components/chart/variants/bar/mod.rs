use super::super::component::*;
use dioxus::prelude::*;

/// Three series over six months, grouped side by side per month (not stacked).
fn generate_data() -> Vec<ChartDatum> {
    const MONTHS: [&str; 6] = ["January", "February", "March", "April", "May", "June"];
    MONTHS
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let t = i as f64;
            let desktop = (180.0 + 90.0 * (t * 0.7).sin() + t * 12.0).max(20.0).round();
            let mobile = (90.0 + 60.0 * (t * 0.9 + 0.6).sin() + t * 8.0).max(15.0).round();
            let tablet = (40.0 + 25.0 * (t * 1.3 + 1.1).sin() + t * 3.0).max(5.0).round();
            ChartDatum {
                label: label.to_string(),
                values: vec![Some(desktop), Some(mobile), Some(tablet)],
                ..Default::default()
            }
        })
        .collect()
}

#[component]
pub fn Demo() -> Element {
    let config = ChartConfig::new()
        .series("desktop", "Desktop", "var(--dx-chart-1)")
        .series("mobile", "Mobile", "var(--dx-chart-2)")
        .series("tablet", "Tablet", "var(--dx-chart-3)");

    rsx! {
        ChartContainer { config, data: generate_data(), kind: ChartKind::Bar,
            Chart { aria_label: "Visitors by month, desktop, mobile, and tablet" }
            ChartTooltip {}
            // The legend aligned to the top instead of the default bottom.
            ChartLegend { vertical_align: LegendAlign::Top }
        }
    }
}
