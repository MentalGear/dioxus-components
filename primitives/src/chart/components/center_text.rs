//! The donut/gauge center text (`PieOptions::center_text` /
//! `RadialOptions::center_text`): two `<tspan>`s, a primary value over a
//! secondary caption. One renderer shared by `pie` and `radial`.
//!
//! The geometry is shadcn's own `<Label content={...}>` markup, copied: the
//! `<text>` is anchored `middle` at the chart centre and each line is a
//! `<tspan>` with an absolute `y` -- the primary on the centre (vertically
//! centred) and the caption 24px below it, or, for a half-turn gauge
//! (`chart-radial-stacked`), the primary 16px above the centre and the
//! caption 4px below it (both on their baselines), so the two lines sit in
//! the half-disc the arc encloses. The font sizes (shadcn's `text-3xl` for a
//! donut, `text-4xl` for a radial gauge, `text-2xl` for the half gauge, and
//! the inherited `text-xs` for the caption) live in the themed stylesheet,
//! keyed on `data-layout`.
//!
//! This module also publishes how large the text may *fit* in the hole, as
//! two CSS custom properties the stylesheet clamps with `min()`: a chart
//! narrower than its usual 250px square shrinks the hole with it, and a
//! 5-digit number at its themed size would overflow it.

use dioxus::prelude::*;

use crate::chart::engine::scale::fmt_num;

/// Fraction of the hole's diameter a text line may occupy: the text sits on
/// the horizontal diameter, and a little air is kept on both sides.
const HOLE_FILL: f64 = 0.8;
/// Average advance of a glyph of the (bold) primary line, as a fraction of its
/// font size: digits in a 700-weight sans are ~0.65em, capitals a bit wider.
const PRIMARY_GLYPH_EM: f64 = 0.7;
/// Average advance of a glyph of the (regular) secondary caption.
const SECONDARY_GLYPH_EM: f64 = 0.58;

/// Which of shadcn's center-label layouts to draw -- see the module doc.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum CenterLayout {
    /// A donut (`chart-pie-donut-text`, `chart-pie-interactive`).
    Donut,
    /// A radial gauge (`chart-radial-text`, `chart-radial-shape`).
    Gauge,
    /// A half-turn radial gauge (`chart-radial-stacked`).
    HalfGauge,
}

impl CenterLayout {
    fn as_str(self) -> &'static str {
        match self {
            CenterLayout::Donut => "donut",
            CenterLayout::Gauge => "gauge",
            CenterLayout::HalfGauge => "half-gauge",
        }
    }
}

/// The largest font size (px) at which `text` fits across a hole of radius
/// `inner_radius`, given its average glyph advance in em.
pub(crate) fn max_font_size(inner_radius: f64, text: &str, glyph_em: f64) -> f64 {
    let glyphs = text.chars().count().max(1) as f64;
    (2.0 * inner_radius.max(0.0) * HOLE_FILL) / (glyphs * glyph_em)
}

/// Render the two-line center text for a hole of radius `inner_radius`
/// centred on `(cx, cy)`.
pub(crate) fn render(
    cx: f64,
    cy: f64,
    inner_radius: f64,
    primary: &str,
    secondary: &str,
    layout: CenterLayout,
) -> Element {
    let style = format!(
        "--dx-chart-center-primary-max: {}px; --dx-chart-center-secondary-max: {}px",
        fmt_num(max_font_size(inner_radius, primary, PRIMARY_GLYPH_EM)),
        fmt_num(max_font_size(inner_radius, secondary, SECONDARY_GLYPH_EM)),
    );
    let (baseline, primary_y, secondary_y) = match layout {
        CenterLayout::HalfGauge => (None, cy - 16.0, cy + 4.0),
        CenterLayout::Donut | CenterLayout::Gauge => (Some("middle"), cy, cy + 24.0),
    };
    rsx! {
        text {
            "data-slot": "chart-pie-center-text",
            "data-layout": layout.as_str(),
            x: "{fmt_num(cx)}",
            y: "{fmt_num(cy)}",
            text_anchor: "middle",
            "dominant-baseline": baseline,
            style: "{style}",
            tspan { x: "{fmt_num(cx)}", y: "{fmt_num(primary_y)}", "{primary}" }
            tspan { x: "{fmt_num(cx)}", y: "{fmt_num(secondary_y)}", "{secondary}" }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn longer_text_gets_a_smaller_cap() {
        let short = max_font_size(60.0, "186", PRIMARY_GLYPH_EM);
        let long = max_font_size(60.0, "1,999,999", PRIMARY_GLYPH_EM);
        assert!(long < short, "{long} !< {short}");
        // 3 bold digits across a 120-wide hole at 80% fill: 96 / 2.1.
        assert!((short - 96.0 / 2.1).abs() < 1e-9, "{short}");
    }

    #[test]
    fn cap_scales_with_the_hole_and_is_never_negative_or_infinite() {
        assert!(
            max_font_size(80.0, "186", PRIMARY_GLYPH_EM)
                > max_font_size(40.0, "186", PRIMARY_GLYPH_EM)
        );
        assert_eq!(max_font_size(-5.0, "x", PRIMARY_GLYPH_EM), 0.0);
        assert!(max_font_size(60.0, "", PRIMARY_GLYPH_EM).is_finite());
    }

    #[test]
    fn render_publishes_both_caps_on_the_text_element() {
        let html = dioxus_ssr::render_element(render(
            150.0,
            100.0,
            60.0,
            "186",
            "Visitors",
            CenterLayout::Donut,
        ));
        assert!(
            html.contains(r#"data-slot="chart-pie-center-text""#),
            "{html}"
        );
        assert!(
            html.contains("--dx-chart-center-primary-max: 45.714px"),
            "{html}"
        );
        assert!(html.contains("--dx-chart-center-secondary-max:"), "{html}");
        assert_eq!(html.matches("<tspan").count(), 2, "{html}");
    }

    #[test]
    fn lines_sit_where_shadcns_label_markup_puts_them() {
        // Donut/gauge: primary on the centre (middle baseline), caption +24.
        let html = dioxus_ssr::render_element(render(
            125.0,
            125.0,
            80.0,
            "200",
            "Visitors",
            CenterLayout::Gauge,
        ));
        assert!(html.contains(r#"data-layout="gauge""#), "{html}");
        assert!(html.contains(r#"dominant-baseline="middle""#), "{html}");
        assert!(html.contains(r#"y="125">200<"#), "{html}");
        assert!(html.contains(r#"y="149">Visitors<"#), "{html}");
        // Half gauge: primary 16 above the centre, caption 4 below it.
        let html = dioxus_ssr::render_element(render(
            125.0,
            125.0,
            80.0,
            "1,830",
            "Visitors",
            CenterLayout::HalfGauge,
        ));
        assert!(!html.contains("dominant-baseline"), "{html}");
        assert!(html.contains(r#"y="109">1,830<"#), "{html}");
        assert!(html.contains(r#"y="129">Visitors<"#), "{html}");
    }
}
