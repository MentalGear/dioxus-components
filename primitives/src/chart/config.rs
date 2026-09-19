//! [`ChartConfig`]/[`ChartSeries`]/[`ChartIcon`] -- the series identity/
//! label/color/icon model a caller builds once and passes to
//! `ChartContainer`.
//!
//! ## Why this lives here, not under [`super::engine`]
//!
//! Every other plain-data chart type ([`super::engine::ChartDatum`],
//! [`super::engine::ChartKind`]) lives under `engine`, which
//! [`super::engine`]'s own module doc holds to a hard rule: no `dioxus`
//! type, ever, anywhere in that tree (it's meant to be liftable into a
//! standalone upstream crate unchanged). [`ChartSeries::icon`] is a
//! `Callback<(), Element>`-backed [`ChartIcon`] -- a real `dioxus_core`
//! type, added deliberately (stage-2 chart round, unused until a later
//! lane wires it into `ChartLegend`/`ChartTooltip`) so a caller can supply
//! a custom per-series icon. The moment [`ChartSeries`] carries that field,
//! it can no longer honestly claim to be dioxus-free -- so it (and
//! [`ChartConfig`], which only exists to hold an ordered `Vec<ChartSeries>`)
//! moved out of `engine/data.rs` into this file instead, mirroring exactly
//! why [`super::context`] itself is "the one place in `chart/` that is
//! deliberately not part of the `engine` seam" (that module's own doc).
//! [`super::engine::ChartDatum`]/[`super::engine::ChartKind`] needed no such
//! move: [`super::engine::ChartDatum::color`] (added the same round) is a
//! plain `Option<String>`, so `ChartDatum` stays exactly where it was.
//!
//! Nothing under `engine/` ever names [`ChartConfig`]/[`ChartSeries`] (this
//! was true before this move and remains true after it -- checked directly
//! against every file in that tree), so relocating them here changes no
//! internal dependency edge, only which file the definition lives in; every
//! public item here is still re-exported flatly from [`crate::chart`], so
//! this is invisible to this crate's own consumers.

use dioxus::prelude::*;

/// The set of series a chart draws: colors, labels, and their draw order
/// (also the stacking order for a stacked [`super::engine::ChartKind::Area`]/
/// [`super::engine::ChartKind::Bar`]) -- built once, generally as a
/// `use_signal`-held constant, and passed to `ChartContainer`.
///
/// Every series and every [`super::engine::ChartDatum::values`] entry lines
/// up with this list *positionally*: the series at `series[i]` reads
/// `datum.values[i]` from every datum. There is no by-key lookup at draw
/// time -- order is the only thing that matters, which is also why this
/// type carries a `Vec`, not a map.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct ChartConfig {
    /// The series, in draw/stacking order.
    pub series: Vec<ChartSeries>,
}

impl ChartConfig {
    /// An empty config -- call [`Self::series`] to add each series, in the
    /// order they should draw.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append one series and return `self`, for chained construction:
    ///
    /// ```
    /// use dioxus_primitives::chart::ChartConfig;
    ///
    /// let config = ChartConfig::new()
    ///     .series("desktop", "Desktop", "var(--dx-chart-1)")
    ///     .series("mobile", "Mobile", "var(--dx-chart-2)");
    /// assert_eq!(config.series.len(), 2);
    /// assert_eq!(config.series[0].key, "desktop");
    /// ```
    pub fn series(
        mut self,
        key: impl Into<String>,
        label: impl Into<String>,
        color: impl Into<String>,
    ) -> Self {
        self.series.push(ChartSeries {
            key: key.into(),
            label: label.into(),
            color: color.into(),
            icon: None,
        });
        self
    }
}

/// One series' identity, label, color, and (optional) icon.
#[derive(Clone, PartialEq, Debug)]
pub struct ChartSeries {
    /// A stable identifier for this series (e.g. `"desktop"`). Rendered
    /// (sanitized to `[a-z0-9_-]`, see its own `slot()` method) as the
    /// `--color-<key>` CSS custom property name and as the `data-series`
    /// attribute value everywhere a mark for this series renders.
    pub key: String,
    /// The human-readable label -- used in the legend, the tooltip, the
    /// hidden data table's column header, and as each swatch's accessible
    /// name.
    pub label: String,
    /// Any valid CSS color: a literal (`"#2a78d6"`) or, typically, a
    /// `var(--dx-chart-N)` reference into this crate's categorical palette.
    pub color: String,
    /// An optional custom icon for this series, rendered next to its label
    /// wherever one is shown (legend, tooltip). **Unused for now** -- added
    /// (stage-2 chart round) as an extension point for a later lane to wire
    /// up; every series currently renders with no icon regardless of this
    /// field. `None` via [`ChartConfig::series`]; set it with a struct
    /// update (`ChartSeries { icon: Some(ChartIcon(my_callback)), ..series }`)
    /// once a lane gives this a builder method of its own.
    pub icon: Option<ChartIcon>,
}

impl ChartSeries {
    /// This series' [`sanitize_key`]-d key -- the exact token used for both
    /// the `--color-<key>` CSS custom property suffix (`ChartContainer`)
    /// and the `data-series` attribute value (`Chart`/`ChartTooltip`/
    /// `ChartLegend`, all three), so a caller's raw `key` (which is not
    /// attribute-safe -- arbitrary Unicode, spaces, quotes) never has to
    /// appear in markup directly.
    pub(crate) fn slot(&self) -> String {
        sanitize_key(&self.key)
    }
}

/// Sanitize a series key into a safe CSS custom-property-name / HTML
/// attribute-value token: ASCII-lowercased, with every character outside
/// `[a-z0-9_-]` replaced (not stripped -- so two keys that differ only in a
/// punctuation character don't collide into the same token) with `-`.
fn sanitize_key(key: &str) -> String {
    key.chars()
        .map(|c| {
            let c = c.to_ascii_lowercase();
            if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

/// A [`ChartSeries::icon`]-shaped render callback: `Clone`/`Copy`/
/// `PartialEq`/`Debug` exactly the way [`Callback`] itself already is
/// (`Callback` hand-implements `Copy`/`Clone` as a plain copy of its
/// internal `GenerationalBox` handle, `PartialEq` by comparing that
/// handle's identity rather than the closure's behavior, and `Debug` by
/// printing that same handle -- wrapping it in a one-field tuple struct and
/// deriving all four inherits exactly that semantics, the same "compare by
/// identity" contract this crate's other callback-holding props already
/// rely on: e.g. `preview::MasonryCard`'s own `component: Callback<(),
/// Element>` field needs no manual `PartialEq` impl for the same reason).
/// `Debug` is required here, not merely inherited for free: [`ChartSeries`]
/// itself derives `Debug`, so every field type it holds -- this one
/// included -- must implement it too, or that derive fails to compile.
/// **Unused for now**, alongside [`ChartSeries::icon`] itself.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ChartIcon(pub Callback<(), Element>);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chart_config_builder_preserves_order() {
        let config = ChartConfig::new()
            .series("desktop", "Desktop", "var(--dx-chart-1)")
            .series("mobile", "Mobile", "var(--dx-chart-2)");
        assert_eq!(config.series.len(), 2);
        assert_eq!(config.series[0].key, "desktop");
        assert_eq!(config.series[0].label, "Desktop");
        assert_eq!(config.series[0].color, "var(--dx-chart-1)");
        assert_eq!(config.series[1].key, "mobile");
    }

    #[test]
    fn chart_config_default_is_empty() {
        assert_eq!(ChartConfig::default(), ChartConfig { series: Vec::new() });
        assert_eq!(ChartConfig::new().series.len(), 0);
    }

    #[test]
    fn series_built_via_the_config_builder_has_no_icon() {
        let config = ChartConfig::new().series("desktop", "Desktop", "#000");
        assert_eq!(config.series[0].icon, None);
    }

    #[test]
    fn series_slot_sanitizes_the_key() {
        let series = ChartSeries {
            key: "Desktop Users!".to_string(),
            label: "Desktop".to_string(),
            color: "#000".to_string(),
            icon: None,
        };
        assert_eq!(series.slot(), "desktop-users-");
        assert_eq!(
            ChartSeries {
                key: "already-safe_1".to_string(),
                label: String::new(),
                color: String::new(),
                icon: None,
            }
            .slot(),
            "already-safe_1"
        );
    }
}
