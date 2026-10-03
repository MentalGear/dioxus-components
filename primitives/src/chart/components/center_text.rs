//! The donut/gauge center text (`PieOptions::center_text` /
//! `RadialOptions::center_text`): two `<tspan>`s, a primary value over a
//! secondary caption, centered in the hole. One renderer shared by `pie` and
//! `radial` (they used to carry identical copies).
//!
//! Sizing is split between the themed stylesheet and this module. The
//! stylesheet picks the *preferred* size (`--dx-text-2xl`/`--dx-text-sm` times
//! the chart's text scale, never below 1 -- the centre number is part of the
//! gauge graphic and stays proportional to the donut when the chart is shown
//! larger than its viewBox). This module only publishes how large the text
//! may *fit* in the hole, as two CSS custom properties the stylesheet clamps
//! with `min()`: on a narrow container the text scale grows (up to 2.5x) but
//! the hole does not, so without the clamp a 3-digit number overflows it.

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

/// The largest font size (logical units) at which `text` fits across a hole
/// of radius `inner_radius`, given its average glyph advance in em.
pub(crate) fn max_font_size(inner_radius: f64, text: &str, glyph_em: f64) -> f64 {
    let glyphs = text.chars().count().max(1) as f64;
    (2.0 * inner_radius.max(0.0) * HOLE_FILL) / (glyphs * glyph_em)
}

/// Render the centered two-line text for a hole of radius `inner_radius`.
pub(crate) fn render(
    cx: f64,
    cy: f64,
    inner_radius: f64,
    primary: &str,
    secondary: &str,
) -> Element {
    let style = format!(
        "--dx-chart-center-primary-max: {}px; --dx-chart-center-secondary-max: {}px",
        fmt_num(max_font_size(inner_radius, primary, PRIMARY_GLYPH_EM)),
        fmt_num(max_font_size(inner_radius, secondary, SECONDARY_GLYPH_EM)),
    );
    rsx! {
        text {
            "data-slot": "chart-pie-center-text",
            x: "{fmt_num(cx)}",
            y: "{fmt_num(cy)}",
            text_anchor: "middle",
            style: "{style}",
            tspan { x: "{fmt_num(cx)}", dy: "-0.1em", "{primary}" }
            tspan { x: "{fmt_num(cx)}", dy: "1.4em", "{secondary}" }
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
        let html = dioxus_ssr::render_element(render(150.0, 100.0, 60.0, "186", "Visitors"));
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
}
