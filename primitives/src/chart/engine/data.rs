//! The chart's plain data model -- [`ChartConfig`]/[`ChartSeries`]/
//! [`ChartDatum`]/[`ChartKind`] -- pure data, no `dioxus` types (see the
//! `engine` module doc for why). [`super::super::ChartContext`] (the
//! Dioxus-facing layer) holds these behind reactive signals; this module
//! itself never touches reactivity at all.
//!
//! Per `dev-docs/research/chart-2026-09-19.md` §1.4, this crate's chart API
//! is data-driven (`ChartConfig`/`ChartDatum` as plain props), never
//! children-as-configuration -- Dioxus's `children: Element` gives a parent
//! no equivalent of React's `Children.map`/`cloneElement`, so a Recharts-
//! style `Chart { XAxis {...} Bar { data_key: "desktop" } }` that
//! reconfigures itself from what got nested inside it cannot be built here
//! (or in any other Rust GUI framework with the same opaque-children
//! property). [`ChartConfig`]'s ordered `series` list, not the nesting of
//! any child component, is what determines both draw order and stacking
//! order everywhere in this module.

/// The set of series a chart draws: colors, labels, and their draw order
/// (also the stacking order for a stacked [`ChartKind::Area`]/
/// [`ChartKind::Bar`]) -- built once, generally as a `use_signal`-held
/// constant, and passed to `ChartContainer`.
///
/// Every series and every [`ChartDatum::values`] entry lines up with this
/// list *positionally*: the series at `series[i]` reads `datum.values[i]`
/// from every datum. There is no by-key lookup at draw time -- order is
/// the only thing that matters, which is also why this type carries a
/// `Vec`, not a map.
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
        });
        self
    }
}

/// One series' identity, label, and color.
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

/// One x-axis category (e.g. a month or a day) and its value for every
/// configured series, in [`ChartConfig::series`] order.
#[derive(Clone, PartialEq, Debug)]
pub struct ChartDatum {
    /// The x-axis category label, e.g. `"January"` or `"2024-06-01"`.
    pub label: String,
    /// One value per configured series, positionally aligned to
    /// [`ChartConfig::series`] -- `values[i]` is series `i`'s value for
    /// this datum. `None` renders as a gap (a line/area breaks; a bar is
    /// omitted) and as `"—"` in the hidden data table, distinct from
    /// `Some(0.0)`.
    pub values: Vec<Option<f64>>,
}

/// Which mark family a `Chart` draws. Owned by `ChartContainer` (not
/// `Chart` itself) because the same value also selects the container's own
/// `data-kind` attribute, for a themed wrapper to style by.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ChartKind {
    /// A filled area under a (possibly curved) line, optionally stacked.
    Area,
    /// Discrete bars, grouped side-by-side per series unless stacked.
    Bar,
    /// A (possibly curved) line with no fill.
    Line,
}

impl ChartKind {
    /// The `data-kind` attribute value, matching this crate's existing
    /// `data-orientation`/`data-direction`-style lowercase tokens.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Area => "area",
            Self::Bar => "bar",
            Self::Line => "line",
        }
    }
}

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
    fn series_slot_sanitizes_the_key() {
        let series = ChartSeries {
            key: "Desktop Users!".to_string(),
            label: "Desktop".to_string(),
            color: "#000".to_string(),
        };
        assert_eq!(series.slot(), "desktop-users-");
        assert_eq!(
            ChartSeries {
                key: "already-safe_1".to_string(),
                label: String::new(),
                color: String::new(),
            }
            .slot(),
            "already-safe_1"
        );
    }

    #[test]
    fn chart_kind_as_str_is_lowercase() {
        assert_eq!(ChartKind::Area.as_str(), "area");
        assert_eq!(ChartKind::Bar.as_str(), "bar");
        assert_eq!(ChartKind::Line.as_str(), "line");
    }
}
