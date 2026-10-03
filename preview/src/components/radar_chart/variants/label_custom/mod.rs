use super::super::component::*;
use crate::components::card::{
    Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle,
};
use dioxus::prelude::*;
use dioxus_icons::lucide::TrendingUp;

/// Each month labelled with its two values over its name (`RadarOptions::ticks`),
/// inside a 10px margin.
fn generate_data() -> Vec<ChartDatum> {
    const MONTHS: [(&str, f64, f64); 6] = [
        ("January", 186.0, 80.0),
        ("February", 305.0, 200.0),
        ("March", 237.0, 120.0),
        ("April", 73.0, 190.0),
        ("May", 209.0, 130.0),
        ("June", 214.0, 140.0),
    ];
    MONTHS
        .iter()
        .map(|(label, desktop, mobile)| ChartDatum {
            label: label.to_string(),
            values: vec![Some(*desktop), Some(*mobile)],
            ..Default::default()
        })
        .collect()
}

#[component]
pub fn Demo() -> Element {
    let config = ChartConfig::new()
        .series("desktop", "Desktop", "var(--dx-chart-1)")
        .series("mobile", "Mobile", "var(--dx-chart-2)");
    let data = generate_data();
    // shadcn's custom tick: "desktop/mobile" (the slash muted) over the month.
    let ticks: Vec<RadarTick> = data
        .iter()
        .map(|d| {
            let value = |s: usize| format!("{:.0}", d.values[s].unwrap_or(0.0));
            RadarTick {
                parts: vec![
                    (value(0), false),
                    ("/".to_string(), true),
                    (value(1), false),
                ],
                caption: Some(d.label.clone()),
            }
        })
        .collect();

    rsx! {
        Card {
            CardHeader {
                CardTitle { "Radar Chart - Custom Label" }
                CardDescription { "Showing total visitors for the last 6 months" }
            }
            CardContent {
                ChartContainer { config, data, kind: ChartKind::Radar,
                    Chart {
                        margin: ChartMargin::all(10.0),
                        aria_label: "Total visitors by month, desktop and mobile",
                        radar: RadarOptions {
                            fill_opacity: vec![0.6],
                            ticks: Some(ticks),
                            ..Default::default()
                        },
                    }
                    ChartTooltip { indicator: TooltipIndicator::Line }
                }
            }
            CardFooter { class: "dx-chart-footer",
                div { class: "dx-chart-footer-trend",
                    "Trending up by 5.2% this month"
                    TrendingUp { size: "16px" }
                }
                div { class: "dx-chart-footer-caption", "January - June 2024" }
            }
        }
    }
}
