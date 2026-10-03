//! Pointer tracking for [`super::chart::Chart`]: turns a pointer event into
//! (a) the pointer's position relative to the chart box and (b) the datum
//! under it, so the tooltip can follow the pointer the way shadcn/Recharts
//! tooltips do.
//!
//! ## Why client coordinates minus a fresh bounding rect
//!
//! `PointerData::element_coordinates` is the browser's `offsetX/offsetY`,
//! which is relative to whichever child the event happened to target (a
//! hit band, an arc, an axis label ...), not to the chart. The only
//! coordinate every target agrees on is the viewport's, so the position is
//! `client_coordinates() - get_client_rect().origin`, with the rect read
//! when the event fires (never cached: a scroll or layout shift moves the
//! chart without any event on the chart itself). Reading the rect is async
//! in Dioxus, so the position lands a microtask after the event. The datum
//! and the position are published together from that one task, so the
//! tooltip's first visible frame is already at the pointer. That task is
//! guarded by the chart's `PointerGate` (`context.rs`): a close that happens
//! while the rect is being read voids it, so a late result can never reopen
//! a tooltip the user just closed.
//!
//! ## Scaled ancestors
//!
//! The rect is in screen pixels, but the tooltip is placed in the chart
//! box's own layout pixels; under `transform: scale(..)` or CSS `zoom` they
//! differ. [`local_position`] therefore maps the pointer's *fraction* of the
//! rect onto the layout box (whose size comes from the wrapper's
//! `onresize`, the one source of `ChartContext::box_size`), so the hit test,
//! the cursor and the tooltip all live in one frame.
//!
//! ## Every family resolves the datum from coordinates
//!
//! Hover is *not* driven by `pointerenter`/`pointerleave` on the marks, for
//! three reasons that are one class. (1) A hit band per column cannot give
//! the Recharts behaviour (snap to the nearest category, no dead zone in the
//! gaps between bars, hidden outside the plot). (2) A touch pointer is
//! captured by its first target, so it never enters a second mark while
//! dragging. (3) A mark's hit target is its *painted* geometry, and the
//! active-mark affordance transforms it (`scale(1.05)` on an arc): a pointer
//! near an arc's edge leaves the grown arc, un-grows it, re-enters, and the
//! tooltip blinks open/closed in a loop. [`HitTest::resolve`] instead maps
//! the pointer's own coordinates onto the chart's *resting* geometry:
//! the nearest category for Area/Bar/Line, the sector (ring x angle span)
//! under the pointer for Pie/Radial/Radar -- so the answer cannot depend on
//! what the previous answer did to the DOM.

use std::rc::Rc;

use dioxus::prelude::*;

use crate::chart::context::{ChartContext, Cursor};
use crate::chart::BandScale;

/// Which way a category chart's categories run.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Axis {
    /// Columns: categories along x (vertical Area/Bar/Line).
    X,
    /// Rows: categories along y (horizontal bars).
    Y,
}

/// One hit region of a polar chart: a ring segment, in the chart's logical
/// units, around the chart center. Angles are radians, `0` at twelve
/// o'clock and increasing clockwise (the engine's convention, shared with
/// `engine::polar::centroid`); `a1 < a0` is a counter-clockwise sweep.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct Sector {
    /// The datum this region selects.
    pub index: usize,
    pub r0: f64,
    pub r1: f64,
    pub a0: f64,
    pub a1: f64,
}

impl Sector {
    /// Whether the point at radius `r` and bearing `theta` (the same angle
    /// convention, any real number) lies inside this sector. A sweep of a
    /// full turn or more contains every bearing.
    pub(crate) fn contains(&self, r: f64, theta: f64) -> bool {
        use std::f64::consts::TAU;
        if r < self.r0.min(self.r1) || r > self.r0.max(self.r1) {
            return false;
        }
        let sweep = self.a1 - self.a0;
        if sweep.abs() >= TAU {
            return true;
        }
        let along = if sweep >= 0.0 {
            (theta - self.a0).rem_euclid(TAU)
        } else {
            (self.a0 - theta).rem_euclid(TAU)
        };
        along <= sweep.abs()
    }
}

/// How to find the datum under a pointer.
#[derive(Clone, PartialEq, Debug)]
pub(crate) enum HitTest {
    /// Nothing to resolve (no geometry yet).
    None,
    /// Nearest category on `axis` within the plot rectangle.
    Categories {
        axis: Axis,
        /// The category scale, in the chart's logical units.
        band: BandScale,
        /// The plot rectangle `(x0, y0, x1, y1)`, logical units.
        plot: (f64, f64, f64, f64),
        /// The chart's logical size (the viewBox), used to map the
        /// pointer's CSS pixels into logical units.
        view: (f64, f64),
    },
    /// The first [`Sector`] around the viewBox center containing the pointer.
    Sectors {
        view: (f64, f64),
        sectors: Rc<[Sector]>,
    },
}

/// What a pointer position resolved to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Hit {
    /// The category under (nearest to) the pointer.
    Index(usize),
    /// Inside the chart box but outside the plot: nothing is hovered
    /// (shadcn/Recharts hide the tooltip there).
    Outside,
    /// Coordinates say nothing: leave the active index alone.
    Unresolved,
}

impl HitTest {
    /// Resolve a pointer at `(px, py)` CSS px from the chart box's top-left,
    /// in a box of `box_size` CSS px.
    pub(crate) fn resolve(&self, px: f64, py: f64, box_size: (f64, f64)) -> Hit {
        if !(box_size.0 > 0.0 && box_size.1 > 0.0) {
            return Hit::Unresolved;
        }
        match self {
            Self::None => Hit::Unresolved,
            Self::Categories {
                axis,
                band,
                plot,
                view,
            } => {
                let lx = px * view.0 / box_size.0;
                let ly = py * view.1 / box_size.1;
                let (x0, y0, x1, y1) = *plot;
                if lx < x0 || lx > x1 || ly < y0 || ly > y1 {
                    return Hit::Outside;
                }
                let along = match axis {
                    Axis::X => lx,
                    Axis::Y => ly,
                };
                band.nearest_index(along).map_or(Hit::Outside, Hit::Index)
            }
            Self::Sectors { view, sectors } => {
                let dx = px * view.0 / box_size.0 - view.0 / 2.0;
                let dy = py * view.1 / box_size.1 - view.1 / 2.0;
                let r = dx.hypot(dy);
                let theta = dx.atan2(-dy);
                sectors
                    .iter()
                    .find(|s| s.contains(r, theta))
                    .map_or(Hit::Outside, |s| Hit::Index(s.index))
            }
        }
    }
}

/// The pointer's position in the chart box's own (layout) CSS pixels.
///
/// `client` and `origin`/`rect` are in *screen* pixels -- a bounding rect is
/// measured after every ancestor `transform: scale(...)` / CSS `zoom` -- while
/// the tooltip is placed with a `transform: translate(..)` in the box's own
/// layout pixels, inside that same scaled subtree. The two differ by the
/// ancestors' scale `rect / layout`, so the pointer's *fraction* of the rect
/// is carried over to the layout box: `fraction * layout`. With no scaling
/// (`rect == layout`) this is exactly `client - origin`.
pub(crate) fn local_position(
    client: (f64, f64),
    origin: (f64, f64),
    rect: (f64, f64),
    layout: (f64, f64),
) -> (f64, f64) {
    (
        (client.0 - origin.0) / rect.0 * layout.0,
        (client.1 - origin.1) / rect.1 * layout.1,
    )
}

/// Handle one pointer event (`pointermove`/`pointerdown`) on the chart
/// wrapper: measure the wrapper, then publish the pointer position and, for
/// category charts, the datum under it.
///
/// The measurement is async, so the event takes a [`Ticket`] first and the
/// result is applied only if the chart's `PointerGate` still admits it: a
/// close (leave/blur/cancel/keyboard) that happened meanwhile wins.
///
/// The chart box's *layout* size is read from `ChartContext::box_size`,
/// written only by the wrapper's `onresize` (one measure, one box); this
/// handler adopts the rect's size for it only while it is still unmeasured,
/// where scale `1` is the best available guess.
///
/// [`Ticket`]: crate::chart::context::Ticket
pub(crate) fn track_pointer(
    evt: &Event<PointerData>,
    chart: ChartContext,
    mounted: Signal<Option<Rc<MountedData>>>,
    hit: HitTest,
) {
    let client = evt.client_coordinates();
    let Some(element) = mounted.peek().clone() else {
        return;
    };
    let ticket = chart.begin_pointer();
    spawn(async move {
        let Ok(rect) = element.get_client_rect().await else {
            return;
        };
        if !chart.admit_pointer(ticket) {
            return;
        }
        let rect_size = (rect.width(), rect.height());
        if !(rect_size.0 > 0.0 && rect_size.1 > 0.0) {
            return;
        }
        let layout = chart.box_size.peek().unwrap_or_else(|| {
            chart.set_box_size(rect_size);
            rect_size
        });
        let (px, py) = local_position(
            (client.x, client.y),
            (rect.origin.x, rect.origin.y),
            rect_size,
            layout,
        );
        match hit.resolve(px, py, layout) {
            Hit::Outside => chart.clear(),
            Hit::Index(i) => {
                let mut active = chart.active_index;
                if active() != Some(i) {
                    active.set(Some(i));
                }
                set_cursor(chart, px, py);
            }
            Hit::Unresolved => set_cursor(chart, px, py),
        }
    });
}

/// Store the pointer position, rounded to the device-independent half pixel
/// (sub-pixel jitter would otherwise re-render the tooltip for nothing).
fn set_cursor(chart: ChartContext, px: f64, py: f64) {
    let mut cursor = chart.cursor;
    let next = Cursor::At((px * 2.0).round() / 2.0, (py * 2.0).round() / 2.0);
    if cursor() != next {
        cursor.set(next);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn columns() -> HitTest {
        HitTest::Categories {
            axis: Axis::X,
            band: BandScale {
                count: 4,
                range: (8.0, 592.0),
                padding: 0.2,
            },
            plot: (8.0, 8.0, 592.0, 270.0),
            view: (600.0, 300.0),
        }
    }

    #[test]
    fn category_charts_snap_to_the_nearest_band_center() {
        let t = columns();
        // 1:1 box (600x300 css px == viewBox).
        let b = (600.0, 300.0);
        assert_eq!(t.resolve(20.0, 100.0, b), Hit::Index(0));
        assert_eq!(t.resolve(580.0, 100.0, b), Hit::Index(3));
        // The pointer is inside the plot's own padding gap, not on a bar.
        assert!(matches!(t.resolve(300.0, 100.0, b), Hit::Index(_)));
    }

    #[test]
    fn a_scaled_box_maps_pixels_back_to_logical_units() {
        let t = columns();
        // The chart is shown at half size: css (150, 50) is logical
        // (300, 100) -- the same answer as 1:1 at (300, 100).
        let half = (300.0, 150.0);
        assert_eq!(
            t.resolve(150.0, 50.0, half),
            t.resolve(300.0, 100.0, (600.0, 300.0))
        );
    }

    #[test]
    fn outside_the_plot_is_outside() {
        let t = columns();
        let b = (600.0, 300.0);
        assert_eq!(t.resolve(2.0, 100.0, b), Hit::Outside, "left margin");
        assert_eq!(t.resolve(599.0, 100.0, b), Hit::Outside, "right margin");
        assert_eq!(t.resolve(300.0, 285.0, b), Hit::Outside, "axis labels");
        assert_eq!(t.resolve(300.0, 3.0, b), Hit::Outside, "top margin");
    }

    #[test]
    fn rows_resolve_along_y() {
        let t = HitTest::Categories {
            axis: Axis::Y,
            band: BandScale {
                count: 5,
                range: (8.0, 292.0),
                padding: 0.2,
            },
            plot: (8.0, 8.0, 592.0, 292.0),
            view: (600.0, 300.0),
        };
        let b = (600.0, 300.0);
        assert_eq!(t.resolve(300.0, 20.0, b), Hit::Index(0));
        assert_eq!(t.resolve(300.0, 280.0, b), Hit::Index(4));
        // x only matters for the plot test.
        assert_eq!(t.resolve(10.0, 20.0, b), Hit::Index(0));
    }

    #[test]
    fn no_geometry_and_unmeasured_boxes_never_resolve() {
        let b = (600.0, 300.0);
        assert_eq!(HitTest::None.resolve(10.0, 10.0, b), Hit::Unresolved);
        assert_eq!(columns().resolve(10.0, 10.0, (0.0, 0.0)), Hit::Unresolved);
    }

    #[test]
    fn local_position_is_client_minus_origin_without_scaling() {
        let p = local_position((130.0, 90.0), (100.0, 40.0), (300.0, 150.0), (300.0, 150.0));
        assert_eq!(p, (30.0, 50.0));
    }

    #[test]
    fn local_position_undoes_an_ancestor_scale() {
        // `transform: scale(0.6)`: a 300x150 layout box measures 180x90 on
        // screen. A pointer at the screen center of that rect is the layout
        // center -- not (90, 45) layout px.
        let p = local_position((190.0, 85.0), (100.0, 40.0), (180.0, 90.0), (300.0, 150.0));
        assert_eq!(p, (150.0, 75.0));
        // CSS `zoom: 1.5`: 450x225 on screen; the quarter point maps to the
        // layout quarter point, not 1.5x too far.
        let p = local_position(
            (212.5, 96.25),
            (100.0, 40.0),
            (450.0, 225.0),
            (300.0, 150.0),
        );
        assert!(
            (p.0 - 75.0).abs() < 1e-9 && (p.1 - 37.5).abs() < 1e-9,
            "{p:?}"
        );
    }

    #[test]
    fn a_scaled_pointer_resolves_to_the_same_category_as_an_unscaled_one() {
        // The hit test sees layout px either way, so the scale cancels out.
        let t = columns();
        let layout = (600.0, 300.0);
        let unscaled = local_position((300.0, 100.0), (0.0, 0.0), layout, layout);
        let scaled = local_position((150.0, 50.0), (0.0, 0.0), (300.0, 150.0), layout);
        assert_eq!(unscaled, scaled);
        assert_eq!(
            t.resolve(unscaled.0, unscaled.1, layout),
            t.resolve(scaled.0, scaled.1, layout)
        );
    }

    use std::f64::consts::{FRAC_PI_2, PI, TAU};

    fn sector(a0: f64, a1: f64) -> Sector {
        Sector {
            index: 0,
            r0: 10.0,
            r1: 50.0,
            a0,
            a1,
        }
    }

    #[test]
    fn sectors_use_clockwise_from_twelve_oclock() {
        let right_quarter = sector(0.0, FRAC_PI_2);
        assert!(right_quarter.contains(30.0, 0.1), "just clockwise of 12");
        assert!(right_quarter.contains(30.0, FRAC_PI_2 - 0.1));
        assert!(
            !right_quarter.contains(30.0, -0.1),
            "counter-clockwise of 12"
        );
        assert!(!right_quarter.contains(30.0, PI), "opposite side");
    }

    #[test]
    fn sector_radius_is_inclusive_and_bearings_wrap() {
        let s = sector(-FRAC_PI_2, FRAC_PI_2);
        assert!(s.contains(10.0, 0.0) && s.contains(50.0, 0.0));
        assert!(!s.contains(9.9, 0.0) && !s.contains(50.1, 0.0));
        // 3/2 pi is the same bearing as -pi/2 (nine o'clock): the edge.
        assert!(s.contains(30.0, 3.0 * FRAC_PI_2));
        assert!(!s.contains(30.0, PI));
    }

    #[test]
    fn counter_clockwise_and_full_turn_sweeps() {
        let ccw = sector(0.0, -FRAC_PI_2);
        assert!(ccw.contains(30.0, -0.5));
        assert!(!ccw.contains(30.0, 0.5));
        let full = sector(0.0, TAU * 1.05);
        assert!(full.contains(30.0, 2.0) && full.contains(30.0, -2.0));
    }

    #[test]
    fn polar_hit_tests_pick_the_sector_under_the_pointer() {
        // 300x300 chart shown 1:1; center (150,150). Two half-ring sectors.
        let t = HitTest::Sectors {
            view: (300.0, 300.0),
            sectors: Rc::from(vec![
                Sector {
                    index: 0,
                    r0: 20.0,
                    r1: 100.0,
                    a0: 0.0,
                    a1: PI,
                },
                Sector {
                    index: 1,
                    r0: 20.0,
                    r1: 100.0,
                    a0: PI,
                    a1: TAU,
                },
            ]),
        };
        let b = (300.0, 300.0);
        assert_eq!(t.resolve(200.0, 150.0, b), Hit::Index(0), "3 o'clock");
        assert_eq!(t.resolve(100.0, 150.0, b), Hit::Index(1), "9 o'clock");
        assert_eq!(t.resolve(150.0, 155.0, b), Hit::Outside, "in the hole");
        assert_eq!(t.resolve(290.0, 150.0, b), Hit::Outside, "beyond the rim");
        // Half-size box: the same bearings/radii after rescaling.
        assert_eq!(t.resolve(100.0, 75.0, (150.0, 150.0)), Hit::Index(0));
    }
}
