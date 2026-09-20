//! `ChartTooltip`'s gallery -- shadcn/ui's nine `chart-tooltip-*.tsx` demos
//! (`$S/refs/ui/apps/v4/registry/new-york-v4/charts/chart-tooltip-*.tsx`,
//! `dev-docs/research/chart-2026-09-19.md` §1.2's "Tooltip variants: 9"
//! row), each showing one tooltip/legend option this crate's `chart`
//! package exposes: the indicator shape (dot/line/dashed/none), hiding the
//! label and/or indicator, a custom label, a label formatter, a fully
//! custom value formatter, per-series icons, and a fully custom row
//! renderer with a computed total.
//!
//! Like `data_table` (row 69) or `table` (row 68), there is no
//! `dioxus_primitives::chart_tooltip` primitive of its own: every demo here
//! composes the SAME themed `Chart`/`ChartContainer`/`ChartTooltip`
//! pieces the `chart` package ships (`componentDependencies: ["chart"]` in
//! this folder's own `component.json`) -- this file re-exports them
//! (`pub use crate::components::chart::component::*;`) purely so every
//! `variants/<name>/mod.rs` can `use super::super::component::*;`
//! uniformly, matching `chart`'s own variant files' exact import shape.
//! One level deeper than `chart::*` on purpose (`$S/stage2-lanes.md`'s
//! cross-lane alert from s2-area): `preview/src/components/mod.rs`'s
//! `examples!` macro gives every registered component, this one included,
//! its own crate-visible `mod component;`, so a blanket `chart::*` would
//! re-export `chart`'s `component` SUBMODULE NAME too and collide with
//! this file's own macro-declared `component` module --
//! `error[E0659]: `component` is ambiguous`. Importing from
//! `chart::component::*` specifically re-exports that module's public
//! items without also naming the module itself, so the collision can't
//! arise.
//!
//! `TooltipIndicator`/`TooltipRow`/`ChartSeries`/`ChartIcon` are re-exported
//! directly off `dioxus_primitives::chart` here rather than through
//! `chart`'s own `component.rs` (which this lane does not own -- see
//! `ChartTooltipFull`'s own doc below for why this file also can't just ask
//! that file to forward every prop): they are plain, render-nothing
//! value/data types with no theme of their own to attach (the same
//! rationale `chart/component.rs`'s own header comment gives for
//! re-exporting `ChartConfig`/`ChartDatum`/`ChartKind`/`LegendAlign` the
//! identical way), so pulling them in raw here is the wrapper layer doing
//! its job, not a `check-preview-composition.sh` violation (every
//! `component.rs` is exempt from that scan by construction -- see that
//! script's own header).

use dioxus::prelude::*;
use dioxus_primitives::dioxus_attributes::attributes;
use dioxus_primitives::merge_attributes;

pub use crate::components::chart::component::*;
pub use dioxus_primitives::chart::{ChartIcon, ChartSeries, TooltipIndicator, TooltipRow};

/// Forwards every [`dioxus_primitives::chart::ChartTooltipProps`] field via
/// `..props`, unlike `crate::components::chart::ChartTooltip` (`chart`'s
/// OWN themed wrapper), which today still hand-lists only `label_format`/
/// `value_format`/`hide_label`/`hide_indicator`/`children` and would
/// silently drop `indicator`/`label_key`/`name_key`/`formatter` -- a caller
/// setting any of those through that wrapper sees no compile error and no
/// effect at all (the exact class of bug this lane's own `$S/
/// stage2-lanes.md` entry documents, with the fix `chart::Chart`'s own
/// wrapper already applies: a struct-update spread). `chart/component.rs`
/// is stage-1/integration-owned, not this lane's to edit -- the exact fix
/// is filed there as a ledger request instead
/// (`$S/stage2-lanes.md`'s "requests for the refactor owner"). Until that
/// lands, every demo below that needs one of the new fields renders
/// through this pass-through instead of the themed `ChartTooltip`, so it
/// demonstrates the real, working behavior rather than a silently no-op'd
/// prop -- every demo that only needs the OLDER fields keeps using the
/// normal themed `ChartTooltip` unchanged. Renders byte-identical markup to
/// what the themed wrapper would (same `.dx-chart-tooltip` class, same
/// stylesheet link): this is not a new primitive, just this gap's
/// temporary, gallery-local fix.
#[component]
pub fn ChartTooltipFull(props: dioxus_primitives::chart::ChartTooltipProps) -> Element {
    let base = attributes!(div {
        class: "dx-chart-tooltip",
    });
    // `.clone()`, not a move: `..props` below needs `props.attributes`
    // (among every other field) still intact -- a `..base` spread cannot
    // follow an earlier statement that already partially moved out of
    // `base` (`error[E0382]`, even for a field the spread's own explicit
    // `attributes: merged` immediately overrides), the one combination
    // (merge a base class in via a prior statement, THEN spread the rest)
    // `chart::Chart`'s own `..props` (no merging at all) and
    // `chart::ChartContainer`'s own hand-listed forward (merging, no
    // spread) each individually avoid.
    let merged = merge_attributes(vec![base, props.attributes.clone()]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/chart/style.css") }
        dioxus_primitives::chart::ChartTooltip { attributes: merged, ..props }
    }
}

/// Wraps every demo in this gallery in one `.dx-chart-tooltip-gallery` root
/// -- this component's own `dx-<name>` class
/// (`scripts/check-dx-class-prefix.sh`), and the scoping hook `style.css`'s
/// indicator/icon rules use so they apply only within this page, never
/// `chart`'s own gallery elsewhere on the site (those rules are a
/// temporary copy of a request filed to `$S/stage2-lanes.md`'s "requests
/// for the refactor owner" -- see `style.css`'s own header comment).
#[component]
pub fn Gallery(children: Element) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/chart_tooltip/style.css") }
        div { class: "dx-chart-tooltip-gallery", {children} }
    }
}

/// Parses a `"YYYY-MM-DD"` date string and returns its short English
/// weekday name (`"Mon"`..`"Sun"`) via Sakamoto's algorithm (public domain
/// -- Wikipedia, "Determination of the day of the week"), used instead of
/// a date-handling dependency per this workspace's no-new-Cargo-deps rule
/// for the chart round (`$S/chart-common.md`). Falls back to `iso_date`
/// unchanged when it isn't exactly `"YYYY-MM-DD"` with a month in `1..=12`
/// -- this is demo-only formatting, not primitive code, so a malformed
/// input degrades to "show the raw string" rather than panicking.
///
/// Every one of this gallery's nine demos ports shadcn's own per-file
/// `tickFormatter={(value) => new Date(value).toLocaleDateString("en-US",
/// { weekday: "short" })}` (`$S/refs/ui/.../charts/chart-tooltip-*.tsx`,
/// repeated identically in each of the nine source files); this is the one
/// shared Rust equivalent so the nine variant files don't each duplicate a
/// date algorithm the way the source duplicates the one-liner.
pub(crate) fn short_weekday(iso_date: &str) -> String {
    let parts: Vec<&str> = iso_date.split('-').collect();
    if let [y, m, d] = parts.as_slice() {
        if let (Ok(year), Ok(month), Ok(day)) =
            (y.parse::<i64>(), m.parse::<i64>(), d.parse::<i64>())
        {
            if (1..=12).contains(&month) && day >= 1 {
                const T: [i64; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
                let y = if month < 3 { year - 1 } else { year };
                let dow =
                    (y + y / 4 - y / 100 + y / 400 + T[(month - 1) as usize] + day).rem_euclid(7);
                const NAMES: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
                return NAMES[dow as usize].to_string();
            }
        }
    }
    iso_date.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_weekday_matches_the_fixed_six_day_demo_fixture() {
        // Ground truth: `python3 -c "import datetime; ..."`, cross-checked
        // against every demo's shared `chartData` dates.
        assert_eq!(short_weekday("2024-07-15"), "Mon");
        assert_eq!(short_weekday("2024-07-16"), "Tue");
        assert_eq!(short_weekday("2024-07-17"), "Wed");
        assert_eq!(short_weekday("2024-07-18"), "Thu");
        assert_eq!(short_weekday("2024-07-19"), "Fri");
        assert_eq!(short_weekday("2024-07-20"), "Sat");
    }

    #[test]
    fn short_weekday_handles_leap_days_and_century_years() {
        assert_eq!(short_weekday("2000-01-01"), "Sat");
        assert_eq!(short_weekday("2024-01-01"), "Mon");
        assert_eq!(short_weekday("1999-12-31"), "Fri");
        assert_eq!(short_weekday("2024-02-29"), "Thu");
        assert_eq!(short_weekday("2024-03-01"), "Fri");
    }

    #[test]
    fn short_weekday_falls_back_to_the_raw_string_when_malformed() {
        assert_eq!(short_weekday("not-a-date"), "not-a-date");
        assert_eq!(short_weekday("2024-13-01"), "2024-13-01");
        assert_eq!(short_weekday("2024-07"), "2024-07");
        assert_eq!(short_weekday(""), "");
    }
}
