//! Curves turned into straight segments that stay within a tolerance of the true curve: Bezier curves (by the number of pieces that keeps the error under the tolerance), the arcs of an
//! SVG path (first as Bezier curves, which a transform can then move) and the arcs and circles of a DXF file.

use crate::vector::{dist, Pt};
use std::f64::consts::{FRAC_PI_2, TAU};

/// The pieces a quadratic Bezier needs so that no piece is farther than `tol` from the curve (the bound of Wang: `sqrt(d(d-1)M / 8e)` with `d = 2` and `M` the largest second
/// difference of the control points), from 1 to `max`.
pub fn quad_pieces(p0: Pt, p1: Pt, p2: Pt, tol: f64, max: usize) -> usize {
    let m = ((p0.0 - 2.0 * p1.0 + p2.0).hypot(p0.1 - 2.0 * p1.1 + p2.1)).max(0.0);
    pieces(0.25 * m / tol, max)
}

/// The same for a cubic Bezier (`d = 3`).
pub fn cubic_pieces(p0: Pt, p1: Pt, p2: Pt, p3: Pt, tol: f64, max: usize) -> usize {
    let a = (p0.0 - 2.0 * p1.0 + p2.0).hypot(p0.1 - 2.0 * p1.1 + p2.1);
    let b = (p1.0 - 2.0 * p2.0 + p3.0).hypot(p1.1 - 2.0 * p2.1 + p3.1);
    pieces(0.75 * a.max(b) / tol, max)
}

fn pieces(square: f64, max: usize) -> usize {
    if !square.is_finite() {
        return max.max(1);
    }
    (square.max(0.0).sqrt().ceil() as usize).clamp(1, max.max(1))
}

/// The points of a quadratic Bezier after `p0` (the last one is `p2`).
pub fn quad(p0: Pt, p1: Pt, p2: Pt, tol: f64, max: usize, out: &mut Vec<Pt>) {
    let n = quad_pieces(p0, p1, p2, tol, max);
    for k in 1..=n {
        if k == n {
            out.push(p2);
            break;
        }
        let t = k as f64 / n as f64;
        let u = 1.0 - t;
        out.push((
            u * u * p0.0 + 2.0 * u * t * p1.0 + t * t * p2.0,
            u * u * p0.1 + 2.0 * u * t * p1.1 + t * t * p2.1,
        ));
    }
}

/// The points of a cubic Bezier after `p0` (the last one is `p3`).
pub fn cubic(p0: Pt, p1: Pt, p2: Pt, p3: Pt, tol: f64, max: usize, out: &mut Vec<Pt>) {
    let n = cubic_pieces(p0, p1, p2, p3, tol, max);
    for k in 1..=n {
        if k == n {
            out.push(p3);
            break;
        }
        let t = k as f64 / n as f64;
        let u = 1.0 - t;
        let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
        out.push((
            a * p0.0 + b * p1.0 + c * p2.0 + d * p3.0,
            a * p0.1 + b * p1.1 + c * p2.1 + d * p3.1,
        ));
    }
}

/// The pieces an arc of `sweep` radians and `radius` is cut into so that no piece is farther than `tol` from the arc: the angle of a piece is `2 acos(1 - tol / r)`. There are at
/// least four pieces for a whole circle (one for every quarter), and at most `max`.
pub fn arc_pieces(radius: f64, sweep: f64, tol: f64, max: usize) -> usize {
    let sweep = sweep.abs();
    if !(radius.is_finite() && radius > 0.0 && sweep.is_finite()) || sweep == 0.0 {
        return 1;
    }
    let ratio = (1.0 - tol / radius).clamp(-1.0, 1.0);
    let step = 2.0 * ratio.acos();
    let by_tolerance = if step > 1e-9 {
        (sweep / step).ceil()
    } else {
        f64::INFINITY
    };
    let by_quarter = (sweep / FRAC_PI_2).ceil();
    let n = by_tolerance.max(by_quarter);
    if n.is_finite() {
        (n as usize).clamp(1, max.max(1))
    } else {
        max.max(1)
    }
}

/// The points of an arc of a circle after its start (the start is not repeated): `sweep` is in radians and positive turns counterclockwise.
pub fn circle_arc(
    centre: Pt,
    radius: f64,
    start: f64,
    sweep: f64,
    tol: f64,
    max: usize,
) -> Vec<Pt> {
    let n = arc_pieces(radius, sweep, tol, max);
    (1..=n)
        .map(|k| {
            let a = start + sweep * k as f64 / n as f64;
            (centre.0 + radius * a.cos(), centre.1 + radius * a.sin())
        })
        .collect()
}

/// An arc of a path of an SVG (the parameters of its `A` command) as cubic Bezier curves, each of at most a quarter turn: `(p1, p2, p3)` after the start `from`. A radius that is
/// 0 gives a straight line (one curve with its controls on the line); the two ends being the same point gives nothing. The radii are scaled up when they are too small to reach
/// the end, as the SVG rules say.
pub fn svg_arc(
    from: Pt,
    rx: f64,
    ry: f64,
    rotation_deg: f64,
    large: bool,
    sweep: bool,
    to: Pt,
) -> Vec<[Pt; 3]> {
    if dist(from, to) <= 1e-12 {
        return Vec::new();
    }
    let (mut rx, mut ry) = (rx.abs(), ry.abs());
    if rx <= 1e-12 || ry <= 1e-12 || !rx.is_finite() || !ry.is_finite() {
        return vec![[
            (
                from.0 + (to.0 - from.0) / 3.0,
                from.1 + (to.1 - from.1) / 3.0,
            ),
            (
                from.0 + 2.0 * (to.0 - from.0) / 3.0,
                from.1 + 2.0 * (to.1 - from.1) / 3.0,
            ),
            to,
        ]];
    }
    let phi = rotation_deg.to_radians();
    let (sin_phi, cos_phi) = phi.sin_cos();
    let (dx, dy) = ((from.0 - to.0) / 2.0, (from.1 - to.1) / 2.0);
    let x1 = cos_phi * dx + sin_phi * dy;
    let y1 = -sin_phi * dx + cos_phi * dy;
    let lambda = x1 * x1 / (rx * rx) + y1 * y1 / (ry * ry);
    if lambda > 1.0 {
        let s = lambda.sqrt();
        rx *= s;
        ry *= s;
    }
    let num = rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1;
    let den = rx * rx * y1 * y1 + ry * ry * x1 * x1;
    let mut coef = if den > 0.0 {
        (num / den).max(0.0).sqrt()
    } else {
        0.0
    };
    if large == sweep {
        coef = -coef;
    }
    let cxp = coef * rx * y1 / ry;
    let cyp = -coef * ry * x1 / rx;
    let cx = cos_phi * cxp - sin_phi * cyp + (from.0 + to.0) / 2.0;
    let cy = sin_phi * cxp + cos_phi * cyp + (from.1 + to.1) / 2.0;
    let angle = |ux: f64, uy: f64, vx: f64, vy: f64| (ux * vy - uy * vx).atan2(ux * vx + uy * vy);
    let (ux, uy) = ((x1 - cxp) / rx, (y1 - cyp) / ry);
    let (vx, vy) = ((-x1 - cxp) / rx, (-y1 - cyp) / ry);
    let theta = angle(1.0, 0.0, ux, uy);
    let mut delta = angle(ux, uy, vx, vy);
    if !sweep && delta > 0.0 {
        delta -= TAU;
    } else if sweep && delta < 0.0 {
        delta += TAU;
    }
    let n = ((delta.abs() / FRAC_PI_2).ceil() as usize).clamp(1, 8);
    let step = delta / n as f64;
    let k = 4.0 / 3.0 * (step / 4.0).tan();
    let map = |x: f64, y: f64| -> Pt {
        (
            cx + cos_phi * rx * x - sin_phi * ry * y,
            cy + sin_phi * rx * x + cos_phi * ry * y,
        )
    };
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let a0 = theta + step * i as f64;
        let a1 = a0 + step;
        let (s0, c0) = a0.sin_cos();
        let (s1, c1) = a1.sin_cos();
        let p1 = map(c0 - k * s0, s0 + k * c0);
        let p2 = map(c1 + k * s1, s1 - k * c1);
        let p3 = if i + 1 == n { to } else { map(c1, s1) };
        out.push([p1, p2, p3]);
    }
    out
}

/// The points of the arc that a bulge describes between two points (a DXF polyline): the bulge is the tangent of a quarter of the angle the arc turns through, positive when the
/// arc goes counterclockwise from `a` to `b`. The points come after `a` and the last one is `b`.
pub fn bulge_arc(a: Pt, b: Pt, bulge: f64, tol: f64, max: usize) -> Vec<Pt> {
    let chord = dist(a, b);
    if !bulge.is_finite() || bulge == 0.0 || chord <= 1e-12 {
        return vec![b];
    }
    let theta = 4.0 * bulge.atan();
    // the radius from the chord and the angle; the centre lies on the perpendicular bisector, to the left of a-b for a positive bulge
    let radius = chord / (2.0 * (theta / 2.0).sin().abs());
    let half_to_centre = radius * (theta / 2.0).cos() * theta.signum();
    let mid = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
    let (nx, ny) = (-(b.1 - a.1) / chord, (b.0 - a.0) / chord);
    let centre = (mid.0 + nx * half_to_centre, mid.1 + ny * half_to_centre);
    let start = (a.1 - centre.1).atan2(a.0 - centre.0);
    let mut pts = circle_arc(centre, radius, start, theta, tol, max);
    if let Some(last) = pts.last_mut() {
        *last = b;
    }
    pts
}

/// Wraps an angle in degrees to the range from 0 up to 360.
pub fn wrap_degrees(a: f64) -> f64 {
    let r = a.rem_euclid(360.0);
    if r.is_finite() {
        r
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_straight_quadratic_needs_one_piece_and_a_bent_one_more() {
        // the control point is on the line: the second difference is 0
        assert_eq!(
            quad_pieces((0.0, 0.0), (5.0, 0.0), (10.0, 0.0), 0.1, 100),
            1
        );
        // (0,0) (5,5) (10,0): the second difference is (0,-10), M = 10, so sqrt(0.25 * 10 / 0.09) = 5.27 and 6 pieces
        assert_eq!(
            quad_pieces((0.0, 0.0), (5.0, 5.0), (10.0, 0.0), 0.09, 100),
            6
        );
        // the same curve may not be cut into more pieces than the limit
        assert_eq!(quad_pieces((0.0, 0.0), (5.0, 5.0), (10.0, 0.0), 0.09, 4), 4);
    }

    #[test]
    fn the_pieces_of_a_cubic_follow_the_larger_second_difference() {
        // (0,0) (0,10) (10,10) (10,0): the second differences are (10,-10) and (-10,-10), both of length 14.14; sqrt(0.75 * 14.142 / 0.5) = 4.61 and 5 pieces
        let n = cubic_pieces((0.0, 0.0), (0.0, 10.0), (10.0, 10.0), (10.0, 0.0), 0.5, 100);
        assert_eq!(n, 5);
    }

    #[test]
    fn a_quadratic_ends_where_it_should_and_passes_through_its_middle() {
        let mut out = Vec::new();
        quad((0.0, 0.0), (5.0, 5.0), (10.0, 0.0), 0.15, 100, &mut out);
        // sqrt(0.25 * 10 / 0.15) = 4.08 so 5 pieces; the last point is exact
        assert_eq!(out.len(), 5);
        assert_eq!(out[4], (10.0, 0.0));
        // t = 0.2 is the first point: x = 2 * 0.8 * 0.2 * 5 + 0.04 * 10 = 2, y = 2 * 0.8 * 0.2 * 5 = 1.6
        assert!((out[0].0 - 2.0).abs() < 1e-12 && (out[0].1 - 1.6).abs() < 1e-12);
    }

    #[test]
    fn a_cubic_ends_where_it_should() {
        let mut out = Vec::new();
        cubic(
            (0.0, 0.0),
            (0.0, 10.0),
            (10.0, 10.0),
            (10.0, 0.0),
            0.5,
            100,
            &mut out,
        );
        assert_eq!(out.len(), 5);
        assert_eq!(out[4], (10.0, 0.0));
        // t = 0.2: x = 3 * 0.8 * 0.04 * 10 + 0.008 * 10 = 0.96 + 0.08 = 1.04; y = 3 * 0.64 * 0.2 * 10 + 3 * 0.8 * 0.04 * 10 = 3.84 + 0.96 = 4.8
        assert!((out[0].0 - 1.04).abs() < 1e-12 && (out[0].1 - 4.8).abs() < 1e-12);
    }

    #[test]
    fn the_pieces_of_a_circle_follow_the_tolerance_and_never_fall_below_four() {
        // r = 10, tol = 0.1: a piece is 2 acos(0.99) = 0.28308 rad and 2 pi / 0.28308 = 22.2 so 23 pieces
        assert_eq!(arc_pieces(10.0, TAU, 0.1, 10_000), 23);
        // a tolerance as large as the radius still gives the four quarters
        assert_eq!(arc_pieces(1.0, TAU, 5.0, 10_000), 4);
        // a quarter turn of the same circle
        assert_eq!(arc_pieces(10.0, FRAC_PI_2, 0.1, 10_000), 6);
        // the limit and the nonsense
        assert_eq!(arc_pieces(10.0, TAU, 0.1, 8), 8);
        assert_eq!(arc_pieces(0.0, TAU, 0.1, 100), 1);
        assert_eq!(arc_pieces(10.0, 0.0, 0.1, 100), 1);
        assert_eq!(arc_pieces(f64::NAN, 1.0, 0.1, 100), 1);
    }

    #[test]
    fn a_flattened_circle_keeps_every_point_on_the_circle_and_every_piece_within_the_tolerance() {
        let pts = circle_arc((3.0, -2.0), 10.0, 0.0, TAU, 0.1, 10_000);
        assert_eq!(pts.len(), 23);
        for p in &pts {
            assert!((dist(*p, (3.0, -2.0)) - 10.0).abs() < 1e-9);
        }
        // the last point is the first one of the circle again
        assert!(dist(pts[22], (13.0, -2.0)) < 1e-9);
        // the middle of a piece is at most the tolerance from the circle
        let mut prev = (13.0, -2.0);
        for p in &pts {
            let m = ((prev.0 + p.0) / 2.0, (prev.1 + p.1) / 2.0);
            assert!(10.0 - dist(m, (3.0, -2.0)) <= 0.1 + 1e-9);
            prev = *p;
        }
    }

    #[test]
    fn a_half_circle_of_an_svg_path_is_two_curves_that_end_at_the_point() {
        // from (0,0) to (10,0) with radius 5 and the sweep flag 1 (the angle grows): the centre is (5,0) and the arc passes through (5,-5)
        let curves = svg_arc((0.0, 0.0), 5.0, 5.0, 0.0, false, true, (10.0, 0.0));
        assert_eq!(curves.len(), 2);
        assert_eq!(curves[1][2], (10.0, 0.0));
        // the end of the first quarter is the bottom of the circle
        assert!(dist(curves[0][2], (5.0, -5.0)) < 1e-9, "{:?}", curves[0][2]);
    }

    #[test]
    fn the_sweep_flag_picks_the_side_of_the_arc() {
        let grows = svg_arc((0.0, 0.0), 5.0, 5.0, 0.0, false, true, (10.0, 0.0));
        let falls = svg_arc((0.0, 0.0), 5.0, 5.0, 0.0, false, false, (10.0, 0.0));
        assert!(grows[0][2].1 < -4.9);
        assert!(falls[0][2].1 > 4.9);
    }

    #[test]
    fn radii_that_cannot_reach_are_scaled_and_a_zero_radius_is_a_line() {
        // the chord is 10 and the radius 1: it is scaled to 5, a half circle
        let c = svg_arc((0.0, 0.0), 1.0, 1.0, 0.0, false, true, (10.0, 0.0));
        assert_eq!(c.len(), 2);
        assert!(dist(c[0][2], (5.0, -5.0)) < 1e-9);
        let line = svg_arc((0.0, 0.0), 0.0, 5.0, 0.0, false, true, (9.0, 3.0));
        assert_eq!(line.len(), 1);
        assert_eq!(line[0][2], (9.0, 3.0));
        assert!(svg_arc((1.0, 1.0), 5.0, 5.0, 0.0, false, true, (1.0, 1.0)).is_empty());
    }

    #[test]
    fn a_bulge_of_one_is_a_half_circle() {
        // a = (0,0), b = (10,0), bulge 1: a half circle counterclockwise, so it goes below the chord, through (5,-5)
        let pts = bulge_arc((0.0, 0.0), (10.0, 0.0), 1.0, 0.01, 10_000);
        assert_eq!(*pts.last().unwrap(), (10.0, 0.0));
        let low = pts.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
        assert!((low + 5.0).abs() < 0.02, "{low}");
        for p in &pts {
            assert!((dist(*p, (5.0, 0.0)) - 5.0).abs() < 1e-9);
        }
        // a negative bulge goes above
        let up = bulge_arc((0.0, 0.0), (10.0, 0.0), -1.0, 0.01, 10_000);
        let high = up.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
        assert!((high - 5.0).abs() < 0.02);
        // no bulge, or no chord
        assert_eq!(
            bulge_arc((0.0, 0.0), (10.0, 0.0), 0.0, 0.01, 100),
            vec![(10.0, 0.0)]
        );
        assert_eq!(
            bulge_arc((1.0, 1.0), (1.0, 1.0), 1.0, 0.01, 100),
            vec![(1.0, 1.0)]
        );
    }

    #[test]
    fn angles_wrap_into_a_turn() {
        assert_eq!(wrap_degrees(-90.0), 270.0);
        assert_eq!(wrap_degrees(450.0), 90.0);
        assert_eq!(wrap_degrees(f64::NAN), 0.0);
    }
}
