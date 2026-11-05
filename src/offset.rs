//! Moving a closed contour sideways by a distance (the width of the beam, the kerf): outwards or inwards, with round corners where the offset line has to go around a corner and a
//! point where two offset lines cross. The raw offset line is cleaned of the loops it makes when a feature is narrower than the offset, and then every edge of the result is
//! checked to be as far from the original as it should be: a contour that cannot be offset correctly (a neck narrower than the beam) gives nothing instead of a wrong line.

use crate::vector::{
    dist, dist_to_ring, dist_to_segment, point_in_polygon, segment_crossing, signed_area, Pt,
    SAME_POINT_MM,
};

/// Two points closer than this are one when a line is cut into loops.
const JOIN: f64 = 1e-12;

/// The most loops that are cut out of one offset line before it is given up as too complex.
const MAX_SPLITS: usize = 10_000;

/// The ring without repeated points and without the closing point, going counterclockwise; none when it has no area.
pub fn normalised(ring: &[Pt]) -> Option<Vec<Pt>> {
    let mut v: Vec<Pt> = Vec::with_capacity(ring.len());
    for &p in ring {
        if v.last().is_none_or(|&l| dist(l, p) > SAME_POINT_MM) {
            v.push(p);
        }
    }
    while v.len() > 1 && dist(v[0], v[v.len() - 1]) <= SAME_POINT_MM {
        v.pop();
    }
    if v.len() < 3 {
        return None;
    }
    let area = signed_area(&v);
    if area.abs() <= 1e-12 {
        return None;
    }
    if area < 0.0 {
        v.reverse();
    }
    Some(v)
}

/// The unit vector from `a` to `b`.
fn direction(a: Pt, b: Pt) -> Pt {
    let len = dist(a, b);
    ((b.0 - a.0) / len, (b.1 - a.1) / len)
}

/// The offset line before it is cleaned: the lines of the edges moved by `d` to their right (`d` positive is outwards for a counterclockwise ring), joined by an arc at a corner the
/// offset line goes around and by the crossing point of the two lines at a corner it cuts.
fn raw_offset(ring: &[Pt], d: f64, tol: f64, max_points: usize) -> Result<Vec<Pt>, String> {
    let n = ring.len();
    let dirs: Vec<Pt> = (0..n)
        .map(|i| direction(ring[i], ring[(i + 1) % n]))
        .collect();
    let step = {
        let ratio = (1.0 - tol / d.abs()).clamp(-1.0, 1.0);
        (2.0 * ratio.acos()).max(1e-3)
    };
    let mut out: Vec<Pt> = Vec::with_capacity(n * 2);
    for i in 0..n {
        let p = ring[i];
        let e0 = dirs[(i + n - 1) % n];
        let e1 = dirs[i];
        // the normals to the right of the edges
        let n0 = (e0.1, -e0.0);
        let n1 = (e1.1, -e1.0);
        let cross = e0.0 * e1.1 - e0.1 * e1.0;
        let dot = e0.0 * e1.0 + e0.1 * e1.1;
        let at = |v: Pt| (p.0 + v.0 * d, p.1 + v.1 * d);
        if cross.abs() < 1e-12 && dot > 0.0 {
            out.push(at(n0));
        } else if cross.abs() < 1e-12 {
            // the contour turns back on itself
            out.push(at(n0));
            out.push(at(n1));
        } else if d * cross > 0.0 {
            // the offset line goes round this corner
            let turn = cross.atan2(dot);
            let pieces = ((turn.abs() / step).ceil() as usize).clamp(1, 100_000);
            for s in 0..=pieces {
                let a = turn * s as f64 / pieces as f64;
                let (sin, cos) = a.sin_cos();
                out.push(at((n0.0 * cos - n0.1 * sin, n0.0 * sin + n0.1 * cos)));
            }
        } else {
            // the two offset lines cross here
            let k = 1.0 + (n0.0 * n1.0 + n0.1 * n1.1);
            if k > 1e-4 {
                out.push((p.0 + (n0.0 + n1.0) * d / k, p.1 + (n0.1 + n1.1) * d / k));
            } else {
                out.push(at(n0));
                out.push(at(n1));
            }
        }
        if out.len() > max_points {
            return Err(format!(
                "the offset of the contour has more than {max_points} points (offset max_points)"
            ));
        }
    }
    Ok(out)
}

/// Where two segments meet, ends included (the symmetric parts of a drawing make offset lines that touch exactly at a vertex), when they are not parallel.
fn meet(a: Pt, b: Pt, c: Pt, d: Pt) -> Option<Pt> {
    let (r, s) = ((b.0 - a.0, b.1 - a.1), (d.0 - c.0, d.1 - c.1));
    let den = r.0 * s.1 - r.1 * s.0;
    if den.abs() <= 1e-18 {
        return None;
    }
    let (qp0, qp1) = (c.0 - a.0, c.1 - a.1);
    let t = (qp0 * s.1 - qp1 * s.0) / den;
    let u = (qp0 * r.1 - qp1 * r.0) / den;
    let range = -JOIN..=1.0 + JOIN;
    if range.contains(&t) && range.contains(&u) {
        Some((a.0 + t.clamp(0.0, 1.0) * r.0, a.1 + t.clamp(0.0, 1.0) * r.1))
    } else {
        None
    }
}

/// A closed line without the points that repeat the one before them.
fn without_repeats(v: Vec<Pt>) -> Vec<Pt> {
    let mut out: Vec<Pt> = Vec::with_capacity(v.len());
    for p in v {
        if out.last().is_none_or(|&l| dist(l, p) > JOIN) {
            out.push(p);
        }
    }
    while out.len() > 1 && dist(out[0], out[out.len() - 1]) <= JOIN {
        out.pop();
    }
    out
}

/// The first place where two edges of a closed line meet: the edges and the point.
fn first_crossing(r: &[Pt]) -> Option<(usize, usize, Pt)> {
    let n = r.len();
    for i in 0..n {
        let (a, b) = (r[i], r[(i + 1) % n]);
        let (lo_x, hi_x) = (a.0.min(b.0), a.0.max(b.0));
        let (lo_y, hi_y) = (a.1.min(b.1), a.1.max(b.1));
        for j in (i + 2)..n {
            if i == 0 && j == n - 1 {
                continue;
            }
            let (c, d) = (r[j], r[(j + 1) % n]);
            if c.0.max(d.0) < lo_x
                || c.0.min(d.0) > hi_x
                || c.1.max(d.1) < lo_y
                || c.1.min(d.1) > hi_y
            {
                continue;
            }
            if let Some(x) = meet(a, b, c, d) {
                return Some((i, j, x));
            }
        }
    }
    None
}

/// Cuts the loops out of a closed line that crosses itself: a loop that goes clockwise is left out, a loop that goes counterclockwise is a part of its own (the neck between two
/// lobes has vanished) unless it lies inside another one. Gives the counterclockwise rings that are left.
fn without_loops(ring: Vec<Pt>) -> Result<Vec<Vec<Pt>>, String> {
    let mut work = vec![ring];
    let mut done: Vec<Vec<Pt>> = Vec::new();
    let mut splits = 0;
    while let Some(r) = work.pop() {
        if r.len() < 3 {
            continue;
        }
        match first_crossing(&r) {
            None => {
                if signed_area(&r) > 1e-12 {
                    done.push(r);
                }
            }
            Some((i, j, x)) => {
                splits += 1;
                if splits > MAX_SPLITS {
                    return Err(
                        "the offset of the contour crosses itself too many times".to_string()
                    );
                }
                let mut inner = vec![x];
                inner.extend_from_slice(&r[i + 1..=j]);
                let mut outer: Vec<Pt> = r[..=i].to_vec();
                outer.push(x);
                outer.extend_from_slice(&r[j + 1..]);
                work.push(without_repeats(outer));
                work.push(without_repeats(inner));
            }
        }
    }
    // a ring inside another one adds nothing
    let keep: Vec<bool> = (0..done.len())
        .map(|a| {
            let probe = done[a].iter().find(|p| {
                done.iter()
                    .enumerate()
                    .all(|(b, other)| b == a || dist_to_ring(**p, other) > 1e-9)
            });
            let Some(&probe) = probe else {
                return true;
            };
            !(0..done.len()).any(|b| b != a && point_in_polygon(&done[b], probe))
        })
        .collect();
    Ok(done
        .into_iter()
        .zip(keep)
        .filter(|(_, k)| *k)
        .map(|(r, _)| r)
        .collect())
}

/// The distance between two segments.
pub fn dist_seg_seg(a: Pt, b: Pt, c: Pt, d: Pt) -> f64 {
    if segment_crossing(a, b, c, d).is_some() {
        return 0.0;
    }
    dist_to_segment(a, c, d)
        .min(dist_to_segment(b, c, d))
        .min(dist_to_segment(c, a, b))
        .min(dist_to_segment(d, a, b))
}

/// Whether every edge of `result` is at least `min` from every edge of `original`.
fn far_enough(result: &[Pt], original: &[Pt], min: f64) -> bool {
    if min <= 0.0 {
        return true;
    }
    let (n, m) = (result.len(), original.len());
    for i in 0..n {
        let (a, b) = (result[i], result[(i + 1) % n]);
        let (lo_x, hi_x) = (a.0.min(b.0) - min, a.0.max(b.0) + min);
        let (lo_y, hi_y) = (a.1.min(b.1) - min, a.1.max(b.1) + min);
        for j in 0..m {
            let (c, d) = (original[j], original[(j + 1) % m]);
            if c.0.max(d.0) < lo_x
                || c.0.min(d.0) > hi_x
                || c.1.max(d.1) < lo_y
                || c.1.min(d.1) > hi_y
            {
                continue;
            }
            if dist_seg_seg(a, b, c, d) < min {
                return false;
            }
        }
    }
    true
}

/// Whether the rings hold every part of the true offset line that can be seen from the original: a point of the line that is `d` from the middle of an edge (and a quarter of the
/// way along it) and not nearer than that to any other edge must be on one of the rings. A ring that was left out because it was not valid would show here.
fn complete(rings: &[Vec<Pt>], base: &[Pt], d: f64, tol: f64) -> bool {
    let n = base.len();
    let need = d.abs() - 1e-6;
    for i in 0..n {
        let (a, b) = (base[i], base[(i + 1) % n]);
        let e = direction(a, b);
        let normal = (e.1, -e.0);
        for t in [0.25, 0.5, 0.75] {
            let m = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
            let q = (m.0 + normal.0 * d, m.1 + normal.1 * d);
            if dist_to_ring(q, base) < need {
                continue;
            }
            let near = rings
                .iter()
                .map(|r| dist_to_ring(q, r))
                .fold(f64::INFINITY, f64::min);
            if near > tol + 1e-6 {
                return false;
            }
        }
    }
    true
}

/// Moves a closed contour by `d` mm: outwards (away from its inside) when `d` is positive and inwards when it is negative. `tol` is how far a round corner may be from the true arc.
/// The answer is the rings of the moved line (more than one when the contour splits, none when it vanishes or when it is narrower than the offset, where no correct line exists).
pub fn offset_ring(
    ring: &[Pt],
    d: f64,
    tol: f64,
    max_points: usize,
) -> Result<Vec<Vec<Pt>>, String> {
    if !d.is_finite() || !tol.is_finite() || tol <= 0.0 {
        return Err("the offset is not a number".to_string());
    }
    let Some(base) = normalised(ring) else {
        return Ok(Vec::new());
    };
    if base.len() > max_points {
        return Err(format!(
            "the contour has more than {max_points} points (offset max_points)"
        ));
    }
    if d == 0.0 {
        return Ok(vec![base]);
    }
    let raw = raw_offset(&base, d, tol, max_points.saturating_mul(2))?;
    let rings = without_loops(raw)?;
    let min = d.abs() - tol - 1e-7;
    let valid: Vec<Vec<Pt>> = rings
        .into_iter()
        .filter(|r| far_enough(r, &base, min))
        .collect();
    // a line that lost a part (a lobe joined by a neck narrower than the offset) would cut part of the work and not the rest: nothing is better than that
    if complete(&valid, &base, d, tol) {
        Ok(valid)
    } else {
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    const SQUARE: [Pt; 4] = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
    const L_SHAPE: [Pt; 6] = [
        (0.0, 0.0),
        (10.0, 0.0),
        (10.0, 5.0),
        (5.0, 5.0),
        (5.0, 10.0),
        (0.0, 10.0),
    ];

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn a_ring_is_cleaned_and_turned_counterclockwise() {
        let mut cw = SQUARE.to_vec();
        cw.reverse();
        cw.push(cw[0]);
        cw.insert(1, cw[0]);
        let r = normalised(&cw).unwrap();
        assert_eq!(r.len(), 4);
        assert!(signed_area(&r) > 0.0);
        assert!(normalised(&SQUARE[..2]).is_none());
        // three points in a line have no area
        assert!(normalised(&[(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)]).is_none());
    }

    #[test]
    fn a_square_moved_in_by_one_is_a_square_8_wide() {
        let r = offset_ring(&SQUARE, -1.0, 0.01, 1000).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0], vec![(1.0, 1.0), (9.0, 1.0), (9.0, 9.0), (1.0, 9.0)]);
    }

    #[test]
    fn a_square_moved_out_by_one_has_round_corners() {
        let r = offset_ring(&SQUARE, 1.0, 0.01, 1000).unwrap();
        assert_eq!(r.len(), 1);
        // a quarter turn at radius 1 and 0.01 mm: 2 acos(0.99) = 0.2831 rad, (pi/2) / 0.2831 = 5.55, so 6 pieces and 7 points at each corner
        assert_eq!(r[0].len(), 28);
        assert_eq!(r[0][0], (-1.0, 0.0));
        // the area is the square, four strips of 10 x 1 and four fans of 6 triangles with an angle of pi/12 each (the corners are polygons, not circles)
        let expected = 100.0 + 40.0 + 12.0 * (PI / 12.0).sin();
        assert!(
            near(signed_area(&r[0]), expected),
            "{} against {expected}",
            signed_area(&r[0])
        );
        // every point of the arcs is at 1 from the corner
        for p in &r[0][..7] {
            assert!(near(dist(*p, (0.0, 0.0)), 1.0));
        }
    }

    #[test]
    fn the_side_does_not_depend_on_the_direction_the_ring_is_written() {
        let mut cw = SQUARE.to_vec();
        cw.reverse();
        let r = offset_ring(&cw, -1.0, 0.01, 1000).unwrap();
        assert!(near(signed_area(&r[0]), 64.0));
    }

    #[test]
    fn an_offset_of_nothing_is_the_ring_itself() {
        let r = offset_ring(&SQUARE, 0.0, 0.01, 1000).unwrap();
        assert_eq!(r, vec![SQUARE.to_vec()]);
    }

    #[test]
    fn a_square_moved_in_by_half_its_width_or_more_vanishes() {
        // at 5 the four corners meet in the middle (no area); at 6 the line has turned inside out and is not a valid offset
        assert!(offset_ring(&SQUARE, -5.0, 0.01, 1000).unwrap().is_empty());
        assert!(offset_ring(&SQUARE, -6.0, 0.01, 1000).unwrap().is_empty());
    }

    #[test]
    fn an_l_shape_moved_out_goes_round_the_convex_corners_and_crosses_at_the_inner_one() {
        let r = offset_ring(&L_SHAPE, 1.0, 0.01, 1000).unwrap();
        assert_eq!(r.len(), 1);
        // the inner corner (5,5) is replaced by the crossing of the two lines, (6,6)
        assert!(r[0].iter().any(|p| near(p.0, 6.0) && near(p.1, 6.0)));
        // area: the L (75) + strips along 40 mm of edge + the five fans of 3 sin(pi/12) - the square of 1 x 1 that two strips share at the inner corner
        let expected = 75.0 + 40.0 + 15.0 * (PI / 12.0).sin() - 1.0;
        assert!(
            near(signed_area(&r[0]), expected),
            "{} against {expected}",
            signed_area(&r[0])
        );
    }

    #[test]
    fn an_l_shape_moved_in_goes_round_the_inner_corner() {
        let r = offset_ring(&L_SHAPE, -1.0, 0.01, 1000).unwrap();
        assert_eq!(r.len(), 1);
        // the eroded L is two rectangles that overlap (39 mm2) plus what is left of the unit square at the inner corner once the quarter disc of radius 1 about (5,5) is taken out
        // (the disc is a fan of 6 triangles here)
        let expected = 39.0 + 1.0 - 3.0 * (PI / 12.0).sin();
        assert!(
            near(signed_area(&r[0]), expected),
            "{} against {expected}",
            signed_area(&r[0])
        );
        // no point is closer than 1 to the inner corner or to the sides
        for p in &r[0] {
            assert!(
                dist(*p, (5.0, 5.0)) >= 1.0 - 1e-9 || p.0 < 4.0 || p.1 < 4.0,
                "{p:?}"
            );
        }
    }

    #[test]
    fn a_slot_narrower_than_the_offset_is_closed_up() {
        // a block 10 x 10 with a slot 1 wide and 5 deep from the top: moved out by 1 the slot is filled and only a small dip is left at its mouth
        let block = [
            (0.0, 0.0),
            (10.0, 0.0),
            (10.0, 10.0),
            (5.0, 10.0),
            (5.0, 5.0),
            (4.0, 5.0),
            (4.0, 10.0),
            (0.0, 10.0),
        ];
        let r = offset_ring(&block, 1.0, 0.01, 1000).unwrap();
        assert_eq!(r.len(), 1, "{r:?}");
        assert!(point_in_polygon(&r[0], (4.5, 7.0)), "the slot is filled");
        assert!(
            point_in_polygon(&r[0], (4.5, 10.5)),
            "the mouth is almost level"
        );
        assert!(!point_in_polygon(&r[0], (4.5, 11.5)));
        let xs: Vec<f64> = r[0].iter().map(|p| p.0).collect();
        let ys: Vec<f64> = r[0].iter().map(|p| p.1).collect();
        assert!(near(xs.iter().cloned().fold(f64::INFINITY, f64::min), -1.0));
        assert!(near(
            xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            11.0
        ));
        assert!(near(ys.iter().cloned().fold(f64::INFINITY, f64::min), -1.0));
        assert!(near(
            ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            11.0
        ));
    }

    #[test]
    fn a_neck_narrower_than_the_offset_splits_the_line_in_two() {
        // two squares joined by a neck 2 wide, moved in by 1.5: the neck vanishes and each square is left with a small bulge towards the neck
        let dumbbell = [
            (0.0, 0.0),
            (10.0, 0.0),
            (10.0, 4.0),
            (20.0, 4.0),
            (20.0, 0.0),
            (30.0, 0.0),
            (30.0, 10.0),
            (20.0, 10.0),
            (20.0, 6.0),
            (10.0, 6.0),
            (10.0, 10.0),
            (0.0, 10.0),
        ];
        let r = offset_ring(&dumbbell, -1.5, 0.01, 1000).unwrap();
        assert_eq!(r.len(), 2);
        // the square 7 x 7 is 49; the bulge is 2 x the integral of (1.5 - sqrt(2.25 - u^2)) over 0..1, which is 0.2401 (the arcs are drawn with chords, a little more)
        for ring in &r {
            let a = signed_area(ring);
            assert!(a > 49.24 && a < 49.27, "{a}");
        }
        let (left, right) = if r[0][0].0 < 15.0 {
            (&r[0], &r[1])
        } else {
            (&r[1], &r[0])
        };
        assert!(left.iter().all(|p| p.0 < 10.0));
        assert!(right.iter().all(|p| p.0 > 20.0));
        // moved in by 0.5 the neck is still there and the line is one piece
        let ok = offset_ring(&dumbbell, -0.5, 0.01, 1000).unwrap();
        assert_eq!(ok.len(), 1);
    }

    #[test]
    fn a_line_that_lost_a_part_is_not_given() {
        // the rings of a valid offset with one of them left out are not complete: nothing is better than a line that cuts only a part
        let dumbbell = [
            (0.0, 0.0),
            (10.0, 0.0),
            (10.0, 4.0),
            (20.0, 4.0),
            (20.0, 0.0),
            (30.0, 0.0),
            (30.0, 10.0),
            (20.0, 10.0),
            (20.0, 6.0),
            (10.0, 6.0),
            (10.0, 10.0),
            (0.0, 10.0),
        ];
        let base = normalised(&dumbbell).unwrap();
        let both = offset_ring(&dumbbell, -1.5, 0.01, 1000).unwrap();
        assert!(complete(&both, &base, -1.5, 0.01));
        assert!(!complete(&both[..1], &base, -1.5, 0.01));
        assert!(!complete(&[], &base, -1.5, 0.01));
    }

    #[test]
    fn a_circle_moved_by_its_own_size_stays_a_circle_and_a_big_inward_move_vanishes() {
        let circle: Vec<Pt> = (0..64)
            .map(|k| {
                let a = 2.0 * PI * f64::from(k) / 64.0;
                (10.0 * a.cos(), 10.0 * a.sin())
            })
            .collect();
        let out = offset_ring(&circle, 2.0, 0.01, 10_000).unwrap();
        assert_eq!(out.len(), 1);
        // the points are about 12 from the centre (the corners of the 64-gon are round, the sides are 2 away from the sides)
        for p in &out[0] {
            let r = dist(*p, (0.0, 0.0));
            assert!(r > 11.9 && r < 12.1, "{r}");
        }
        assert!(offset_ring(&circle, -11.0, 0.01, 10_000)
            .unwrap()
            .is_empty());
        let inner = offset_ring(&circle, -2.0, 0.01, 10_000).unwrap();
        assert_eq!(inner.len(), 1);
        for p in &inner[0] {
            assert!(dist(*p, (0.0, 0.0)) < 8.1);
        }
    }

    #[test]
    fn too_many_points_or_a_value_that_is_not_a_number_is_an_error() {
        assert!(offset_ring(&SQUARE, 1.0, 0.01, 3).is_err());
        assert!(offset_ring(&SQUARE, f64::NAN, 0.01, 1000).is_err());
        assert!(offset_ring(&SQUARE, 1.0, 0.0, 1000).is_err());
        // few points allowed for the round corners
        assert!(offset_ring(&SQUARE, 1.0, 0.01, 8).is_err());
    }

    #[test]
    fn a_ring_with_no_area_gives_nothing() {
        assert!(offset_ring(&[], 1.0, 0.01, 100).unwrap().is_empty());
        assert!(
            offset_ring(&[(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)], 1.0, 0.01, 100)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn the_distance_between_segments() {
        assert_eq!(
            dist_seg_seg((0.0, 0.0), (10.0, 0.0), (0.0, 3.0), (10.0, 3.0)),
            3.0
        );
        assert_eq!(
            dist_seg_seg((0.0, 0.0), (10.0, 10.0), (0.0, 10.0), (10.0, 0.0)),
            0.0
        );
        // 3-4-5 between the ends
        assert_eq!(
            dist_seg_seg((0.0, 0.0), (1.0, 0.0), (4.0, 4.0), (5.0, 4.0)),
            5.0
        );
    }

    #[test]
    fn random_rings_never_panic_and_what_is_given_is_a_counterclockwise_ring() {
        let mut seed = 7u64;
        let mut next = move || {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((seed >> 33) % 2000) as f64 / 100.0
        };
        for _ in 0..200 {
            let n = 3 + (next() as usize) % 9;
            let ring: Vec<Pt> = (0..n).map(|_| (next(), next())).collect();
            for d in [-1.0, -0.2, 0.3, 1.5] {
                if let Ok(rings) = offset_ring(&ring, d, 0.01, 10_000) {
                    for r in &rings {
                        assert!(signed_area(r) > 0.0);
                    }
                }
            }
        }
    }
}
