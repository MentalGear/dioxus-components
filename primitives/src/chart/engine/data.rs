//! The chart's plain data model -- [`ChartDatum`]/[`ChartKind`] -- pure
//! data, no `dioxus` types (see the `engine` module doc for why).
//! [`super::super::ChartContext`] (the Dioxus-facing layer) holds these
//! behind reactive signals; this module itself never touches reactivity at
//! all.
//!
//! Per `dev-docs/research/chart-2026-09-19.md` §1.4, this crate's chart API
//! is data-driven (`ChartConfig`/`ChartDatum` as plain props), never
//! children-as-configuration -- Dioxus's `children: Element` gives a parent
//! no equivalent of React's `Children.map`/`cloneElement`, so a Recharts-
//! style `Chart { XAxis {...} Bar { data_key: "desktop" } }` that
//! reconfigures itself from what got nested inside it cannot be built here
//! (or in any other Rust GUI framework with the same opaque-children
//! property). `ChartConfig`'s ordered `series` list (`super::super::config`
//! -- not this module; see that module's own doc for why it moved out from
//! here), not the nesting of any child component, is what determines both
//! draw order and stacking order everywhere in this module.

/// One x-axis category (e.g. a month or a day) and its value for every
/// configured series, in `ChartConfig::series` (`super::super::config`)
/// order.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct ChartDatum {
    /// The x-axis category label, e.g. `"January"` or `"2024-06-01"`.
    pub label: String,
    /// One value per configured series, positionally aligned to
    /// `ChartConfig::series` -- `values[i]` is series `i`'s value for
    /// this datum. `None` renders as a gap (a line/area breaks; a bar is
    /// omitted) and as `"—"` in the hidden data table, distinct from
    /// `Some(0.0)`.
    pub values: Vec<Option<f64>>,
    /// An optional per-datum color override. **Unused for now** -- added
    /// (stage-2 chart round) alongside `ChartSeries::icon`
    /// (`super::super::config`) as a reserved extension point; every mark
    /// still colors itself from its series' `--color-<key>` regardless of
    /// this field. Intended future use: a single-series chart (Pie/
    /// RadialBar, most plausibly) coloring each category's own slice/arc
    /// independently instead of every slice sharing one series color.
    pub color: Option<String>,
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
    /// A single ring of proportional wedges, one per datum. **Stub**:
    /// `Chart` renders only a placeholder group and a reduced data table
    /// for this kind today -- see `components::series::pie` and
    /// `engine::polar` (owned by a later stage-2 lane, `s2-polar`).
    Pie,
    /// A closed polygon per series across N radial (per-datum) axes.
    /// **Stub** -- see `components::series::radar` and `engine::radar`
    /// (`s2-radar`).
    Radar,
    /// Concentric proportional arcs, one ring per series. **Stub** -- see
    /// `components::series::radial` and `engine::polar` (`s2-polar`).
    RadialBar,
}

impl ChartKind {
    /// The `data-kind` attribute value, matching this crate's existing
    /// `data-orientation`/`data-direction`-style lowercase tokens.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Area => "area",
            Self::Bar => "bar",
            Self::Line => "line",
            Self::Pie => "pie",
            Self::Radar => "radar",
            Self::RadialBar => "radial-bar",
        }
    }

    /// Whether this kind draws on the Cartesian (band x scale + linear y
    /// scale) plot `components::layout::build` computes -- grid, axes, the
    /// cursor, and the hit-bands all only make sense for these three;
    /// `Chart` renders the polar/radial kinds' own, much simpler, stub
    /// markup instead (no grid/axes/cursor/hit-bands, a single placeholder
    /// mark group, a reduced data table) until a later lane replaces it.
    ///
    /// Not the same split as `ChartProps::stacked`'s own area+bar allow-
    /// list: `Line` is Cartesian too, it simply has no stacking
    /// construction of its own (a modeling choice, not a Cartesian/polar
    /// one) -- see that prop's own doc.
    pub(crate) fn is_cartesian(self) -> bool {
        matches!(self, Self::Area | Self::Bar | Self::Line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chart_datum_default_has_no_color() {
        assert_eq!(ChartDatum::default().color, None);
        assert_eq!(ChartDatum::default().values, Vec::<Option<f64>>::new());
    }

    #[test]
    fn chart_kind_as_str_is_lowercase() {
        assert_eq!(ChartKind::Area.as_str(), "area");
        assert_eq!(ChartKind::Bar.as_str(), "bar");
        assert_eq!(ChartKind::Line.as_str(), "line");
        assert_eq!(ChartKind::Pie.as_str(), "pie");
        assert_eq!(ChartKind::Radar.as_str(), "radar");
        assert_eq!(ChartKind::RadialBar.as_str(), "radial-bar");
    }

    #[test]
    fn is_cartesian_is_true_only_for_area_bar_line() {
        assert!(ChartKind::Area.is_cartesian());
        assert!(ChartKind::Bar.is_cartesian());
        assert!(ChartKind::Line.is_cartesian());
        assert!(!ChartKind::Pie.is_cartesian());
        assert!(!ChartKind::Radar.is_cartesian());
        assert!(!ChartKind::RadialBar.is_cartesian());
    }
}
