//! `bar_chart` is a docs-only gallery page, not a themed wrapper of its own
//! primitive -- there is no `dioxus_primitives::bar_chart` module, the same
//! situation `data_table`'s own header comment documents for its relationship
//! to `table`. Every demo under `variants/` composes the installable `chart`
//! package's own themed pieces (`Chart`/`ChartContainer`/`ChartTooltip`/
//! `ChartLegend`, all `ChartKind::Bar`) -- ten of shadcn's bar-chart stories
//! (`$S/refs/ui/apps/v4/registry/new-york-v4/charts/chart-bar-*.tsx`), each
//! cited in its own variant file. This file just re-exports those pieces
//! (plus the plain `ChartConfig`/`ChartDatum`/`ChartKind` data
//! types) so every variant can `use super::super::component::*;` exactly like
//! `chart`'s own `variants/bar/mod.rs` does, rather than reaching past this
//! package into `crate::components::chart::*` or (worse, and the one thing
//! `scripts/check-preview-composition.sh` forbids outside a themed wrapper)
//! a raw `dioxus_primitives::chart::*` path.
//!
//! `component.json`'s `componentDependencies: ["chart", "card"]` is what
//! actually installs those two packages for a `dx components add bar_chart`
//! consumer; this re-export is purely a same-package-as-`chart`'s-own-demos
//! convenience for this repo's own gallery code.

use dioxus::prelude::*;
use dioxus_icons::lucide::TrendingUp;

pub use crate::components::card::{
    Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle,
};
pub use crate::components::chart::{
    Chart, ChartConfig, ChartContainer, ChartDatum, ChartKind, ChartLegend, ChartTooltip,
};

/// The "Trending up by 5.2% this month" footer shadcn repeats, byte-for-byte,
/// across nine of its ten `chart-bar-*` demos (`chart-bar-interactive` is the
/// one exception -- its header already shows per-series totals, so it has no
/// footer at all). Factored here once rather than copy-pasted nine times,
/// same as any other shared demo chrome in this repo; every variant that
/// uses it cites the shadcn source it copies the copy from.
#[component]
pub fn BarChartTrendFooter() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/bar_chart/style.css") }
        CardFooter { class: "dx-bar-chart-footer",
            div { class: "dx-bar-chart-footer-trend",
                "Trending up by 5.2% this month"
                TrendingUp { size: "16px" }
            }
            div { class: "dx-bar-chart-footer-caption", "Showing total visitors for the last 6 months" }
        }
    }
}
