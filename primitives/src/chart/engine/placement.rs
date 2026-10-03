//! Tooltip placement: where a floating tooltip box goes relative to the
//! point it describes, flipped and clamped so it stays inside the chart.
//!
//! Pure arithmetic, no DOM: the caller supplies the anchor (the point the
//! tooltip hangs off, in the chart box's own CSS pixels), the tooltip's own
//! measured size, and the chart box's size.
//!
//! The rule is Recharts' `getTooltipTranslate` (what shadcn/ui's charts use,
//! measured on ui.shadcn.com): the box sits [`TOOLTIP_OFFSET`] px right of
//! and below the anchor; when that would overflow the container's right
//! (bottom) edge it **flips** to the left of (above) the anchor instead,
//! still [`TOOLTIP_OFFSET`] away; and the result is finally **clamped** into
//! the container, so a tooltip wider than the room on either side never
//! leaves the box (it covers the anchor rather than escaping).

/// The gap, in CSS px, between the anchor and the tooltip box on both axes
/// (shadcn/Recharts: exactly 10).
pub const TOOLTIP_OFFSET: f64 = 10.0;

/// Where to put a tooltip: its top-left corner in the chart box's CSS
/// pixels, plus which side of the anchor each axis ended up on (so a
/// stylesheet or test can tell a flipped placement from a plain one).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct TooltipPlacement {
    /// Left edge, CSS px from the chart box's left.
    pub x: f64,
    /// Top edge, CSS px from the chart box's top.
    pub y: f64,
    /// `true` when the tooltip sits to the LEFT of the anchor (flipped).
    pub flipped_x: bool,
    /// `true` when the tooltip sits ABOVE the anchor (flipped).
    pub flipped_y: bool,
}

/// Place one axis. Preferred: `anchor + offset` (after the anchor). If that
/// overflows `bound`, **flip** to `anchor - size - offset` (before it). If
/// that overflows too (a container narrower than the tooltip plus its
/// anchor's distance from both edges), take the side with more free room
/// and **clamp** into `[0, bound - size]` (or `0` when the tooltip is larger
/// than the container) -- so the box stays beside the anchor instead of
/// being clamped over it whenever one side has any room at all. Returns the
/// start coordinate and whether it ended up before the anchor.
fn place_axis(anchor: f64, size: f64, bound: f64, offset: f64) -> (f64, bool) {
    let after = anchor + offset;
    let before = anchor - size - offset;
    let max = (bound - size).max(0.0);
    if after + size <= bound {
        (after.max(0.0), false)
    } else if before >= 0.0 {
        (before.min(max), true)
    } else {
        let room_after = bound - after;
        let room_before = anchor - offset;
        if room_before > room_after {
            (0.0, true)
        } else {
            (max, false)
        }
    }
}

/// Place a tooltip of `tip` = `(width, height)` hanging off `anchor` =
/// `(x, y)` inside a container of `bounds` = `(width, height)` -- all in the
/// same CSS-pixel space, origin at the container's top-left. Non-finite
/// inputs fall back to the origin rather than propagating `NaN` into a
/// `style` string.
///
/// ```
/// use dioxus_primitives::chart::engine::placement::place_tooltip;
///
/// // Room on the right and below: +10px on both axes.
/// let p = place_tooltip((100.0, 40.0), (120.0, 50.0), (400.0, 200.0));
/// assert_eq!((p.x, p.y), (110.0, 50.0));
/// // Near the right edge it flips to the left of the anchor.
/// let p = place_tooltip((350.0, 40.0), (120.0, 50.0), (400.0, 200.0));
/// assert_eq!(p.x, 350.0 - 120.0 - 10.0);
/// assert!(p.flipped_x);
/// ```
pub fn place_tooltip(anchor: (f64, f64), tip: (f64, f64), bounds: (f64, f64)) -> TooltipPlacement {
    let finite = |v: f64| if v.is_finite() { v } else { 0.0 };
    let (ax, ay) = (finite(anchor.0), finite(anchor.1));
    let (tw, th) = (finite(tip.0).max(0.0), finite(tip.1).max(0.0));
    let (bw, bh) = (finite(bounds.0).max(0.0), finite(bounds.1).max(0.0));
    let (x, flipped_x) = place_axis(ax, tw, bw, TOOLTIP_OFFSET);
    let (y, flipped_y) = place_axis(ay, th, bh, TOOLTIP_OFFSET);
    TooltipPlacement {
        x,
        y,
        flipped_x,
        flipped_y,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOUNDS: (f64, f64) = (369.0, 208.0);
    const TIP: (f64, f64) = (128.0, 66.0);

    #[test]
    fn plain_placement_is_offset_right_and_below() {
        let p = place_tooltip((81.0, 40.0), TIP, BOUNDS);
        assert_eq!((p.x, p.y), (91.0, 50.0));
        assert!(!p.flipped_x && !p.flipped_y);
    }

    /// shadcn's measured area chart (419px card): points at x = 12, 81,
    /// 150, 219 sit right of the pointer; 288 and 357 flip left by
    /// `size + 10`.
    #[test]
    fn matches_the_shadcn_area_trace() {
        for (point, want) in [(12.0, 22.0), (81.0, 91.0), (150.0, 160.0), (219.0, 229.0)] {
            let p = place_tooltip((point, 100.0), TIP, BOUNDS);
            assert_eq!(p.x, want, "point {point}");
            assert!(!p.flipped_x);
        }
        for (point, want) in [(288.0, 150.0), (357.0, 219.0)] {
            let p = place_tooltip((point, 100.0), TIP, BOUNDS);
            assert_eq!(p.x, want, "point {point}");
            assert!(p.flipped_x);
        }
    }

    /// shadcn: a 66px tall tooltip at pointer y=100 sits below (ty=110),
    /// at y=130 above (ty=54) -- in a 178px plot.
    #[test]
    fn flips_above_when_it_would_overflow_the_bottom() {
        let below = place_tooltip((50.0, 100.0), TIP, (369.0, 178.0));
        assert_eq!((below.y, below.flipped_y), (110.0, false));
        let above = place_tooltip((50.0, 130.0), TIP, (369.0, 178.0));
        assert_eq!((above.y, above.flipped_y), (54.0, true));
    }

    #[test]
    fn the_flip_boundary_is_inclusive_of_an_exact_fit() {
        // 200 + 10 + 128 == 338 == bound: fits, so no flip.
        let p = place_tooltip((200.0, 0.0), (128.0, 10.0), (338.0, 100.0));
        assert!(!p.flipped_x);
        let p = place_tooltip((200.5, 0.0), (128.0, 10.0), (338.0, 100.0));
        assert!(p.flipped_x);
        assert_eq!(p.x, 200.5 - 128.0 - 10.0);
    }

    #[test]
    fn when_neither_side_fits_it_takes_the_roomier_side_clamped_beside_the_anchor() {
        // Container narrower than the tooltip + the anchor's distance to
        // both edges. Anchor 60 of 150: 80px free after it, 50 before it.
        let p = place_tooltip((60.0, 10.0), (128.0, 30.0), (150.0, 100.0));
        assert!(!p.flipped_x);
        assert_eq!(p.x, 22.0, "pinned to the right edge, right of the anchor");
        assert!(
            p.x > 60.0 - 128.0,
            "and not clamped over the anchor's far side"
        );
        // Anchor near the right: more room before it -> left edge.
        let p = place_tooltip((100.0, 10.0), (128.0, 30.0), (150.0, 100.0));
        assert!(p.flipped_x);
        assert_eq!(p.x, 0.0);
        // Same on the vertical axis.
        let p = place_tooltip((10.0, 20.0), (50.0, 90.0), (300.0, 100.0));
        assert!(!p.flipped_y);
        assert_eq!(p.y, 10.0);
        let p = place_tooltip((10.0, 80.0), (50.0, 90.0), (300.0, 100.0));
        assert!(p.flipped_y);
        assert_eq!(p.y, 0.0);
    }

    #[test]
    fn a_tooltip_larger_than_the_container_pins_to_the_origin() {
        let p = place_tooltip((50.0, 50.0), (500.0, 400.0), (100.0, 100.0));
        assert_eq!((p.x, p.y), (0.0, 0.0));
    }

    #[test]
    fn the_result_always_stays_inside_the_container() {
        for ax in (-50..=450).step_by(25) {
            for ay in (-50..=260).step_by(26) {
                let p = place_tooltip((ax as f64, ay as f64), TIP, BOUNDS);
                assert!(p.x >= 0.0 && p.x + TIP.0 <= BOUNDS.0, "{ax},{ay}: {p:?}");
                assert!(p.y >= 0.0 && p.y + TIP.1 <= BOUNDS.1, "{ax},{ay}: {p:?}");
            }
        }
    }

    #[test]
    fn non_finite_input_never_produces_nan() {
        let p = place_tooltip((f64::NAN, f64::INFINITY), TIP, BOUNDS);
        assert!(p.x.is_finite() && p.y.is_finite());
    }
}
