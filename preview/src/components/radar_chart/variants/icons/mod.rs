use super::super::component::*;
use crate::components::card::{
    Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle,
};
use dioxus::prelude::*;
use dioxus_icons::lucide::{ArrowDownFromLine, ArrowUpFromLine, TrendingUp};

/// Each series has an icon (`ChartSeries::icon`) shown next to its legend
/// swatch.
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
    // `ChartConfig::series(key, label, color)` leaves `icon` unset; set it
    // afterwards on the public `ChartSeries.icon` field.
    let mut config = ChartConfig::new()
        .series("desktop", "Desktop", "var(--dx-chart-1)")
        .series("mobile", "Mobile", "var(--dx-chart-2)");
    config.series[0].icon = Some(ChartIcon(Callback::new(|()| {
        rsx! {
            ArrowDownFromLine {}
        }
    })));
    config.series[1].icon = Some(ChartIcon(Callback::new(|()| {
        rsx! {
            ArrowUpFromLine {}
        }
    })));

    rsx! {
        Card {
            CardHeader {
                CardTitle { "Radar Chart - Icons" }
                CardDescription { "Showing total visitors for the last 6 months" }
            }
            CardContent {
                ChartContainer { config, data: generate_data(), kind: ChartKind::Radar,
                    Chart {
                        aria_label: "Total visitors by month, desktop and mobile",
                        radar: RadarOptions { fill_opacity: vec![0.6], ..Default::default() },
                    }
                    ChartTooltip { indicator: TooltipIndicator::Line }
                    ChartLegend {}
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
