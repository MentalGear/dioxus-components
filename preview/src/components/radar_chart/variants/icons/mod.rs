use super::super::component::*;
use crate::components::card::{
    Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle,
};
use dioxus::prelude::*;
use dioxus_icons::lucide::{ArrowDownFromLine, ArrowUpFromLine, TrendingUp};

/// Ported from `$S/refs/ui/apps/v4/registry/new-york-v4/charts/chart-radar-icons.tsx`
/// (data, config, copy, card chrome). shadcn's own demo gives each series a
/// `ChartConfig` icon (`ArrowDownFromLine`/`ArrowUpFromLine`), shown next to
/// its legend swatch. Ported here via `ChartSeries.icon`
/// (`Option<ChartIcon>`, added by the refactor lane) IF the tooltip lane's
/// legend-icon rendering has landed by this point -- otherwise this is
/// config-only (the icon is set on the series but not yet drawn in the
/// legend), noted plainly rather than silently dropped. See this lane's own
/// final report for which case applied.
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
    // `ChartConfig::series(key, label, color)` sets `icon: None` internally
    // and has no icon-carrying overload -- set it after the fact via the
    // (public) `ChartSeries.icon` field directly, the only construction
    // this crate's builder exposes for it today.
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
                        width: 300.0,
                        height: 300.0,
                        aria_label: "Total visitors by month, desktop and mobile",
                        radar: RadarOptions { fill_opacity: 0.6, ..Default::default() },
                    }
                    ChartTooltip {}
                    ChartLegend {}
                }
            }
            CardFooter {
                div { "Trending up by 5.2% this month " TrendingUp { size: "16px" } }
                div { "January - June 2024" }
            }
        }
    }
}
