use super::super::component::*;
use crate::components::card::{Card, CardContent, CardDescription, CardHeader, CardTitle};
use dioxus::prelude::*;

/// Port of shadcn's `chart-line-dots-custom.tsx`: `LineOptions::dot`
/// replaces the default filled circle with a caller-supplied renderer --
/// here, a small diamond (`<rect>` rotated 45deg) at every defined point,
/// reading [`DotContext::active`] to grow slightly when hovered/keyboard-
/// focused, the same enlargement the default circle gets for free
/// (`series::line::render_dot`'s own doc).
fn generate_data() -> Vec<ChartDatum> {
    const MONTHS: [(&str, f64); 6] = [
        ("January", 186.0),
        ("February", 305.0),
        ("March", 237.0),
        ("April", 73.0),
        ("May", 209.0),
        ("June", 214.0),
    ];
    MONTHS
        .iter()
        .map(|(label, desktop)| ChartDatum {
            label: label.to_string(),
            values: vec![Some(*desktop)],
            ..Default::default()
        })
        .collect()
}

#[component]
pub fn Demo() -> Element {
    let config = ChartConfig::new().series("desktop", "Desktop", "var(--dx-chart-1)");
    let dot = DotRenderer(Callback::new(|ctx: DotContext| {
        let size: f64 = if ctx.active { 10.0 } else { 7.0 };
        let half = size / 2.0;
        rsx! {
            rect {
                key: "{ctx.index}",
                "data-slot": "chart-custom-dot",
                "data-index": "{ctx.index}",
                x: "{ctx.cx - half}",
                y: "{ctx.cy - half}",
                width: "{size}",
                height: "{size}",
                fill: "var(--series-color)",
                transform: "rotate(45 {ctx.cx} {ctx.cy})",
            }
        }
    }));

    rsx! {
        Card {
            CardHeader {
                CardTitle { "Line Chart - Custom Dots" }
                CardDescription { "January - June 2024" }
            }
            CardContent {
                ChartContainer { config, data: generate_data(), kind: ChartKind::Line,
                    Chart {
                        aria_label: "Visitors by month, desktop, diamond-shaped points",
                        x_label: "Month",
                        line: LineOptions { dot: Some(dot), ..Default::default() },
                    }
                    ChartTooltip { hide_label: true }
                }
            }
        }
    }
}
