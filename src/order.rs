//! The order in which the contours of a layer are cut: open paths whose ends meet are joined, the contours that lie inside another one are cut before it (so that a piece does not
//! move or fall before its holes are cut), and among the orders that respect that the head travels little with the laser off (the nearest next contour, then a limited 2-opt
//! improvement that never breaks the inside-first rule).

use crate::vector::{dist, dist_to_ring, point_in_polygon, Pt};

/// The most sweeps of the 2-opt improvement.
const MAX_SWEEPS: usize = 50;

/// A path to cut: closed ones come back to their first point by themselves.
#[derive(Debug, Clone, PartialEq)]
pub struct Contour {
    pub pts: Vec<Pt>,
    pub closed: bool,
}

/// How a contour is visited: a closed one from the vertex `start` (the rest in the order it is written), an open one from its first point or, when `reversed`, from its last.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Visit {
    pub index: usize,
    pub reversed: bool,
    pub start: usize,
}

impl Visit {
    fn ends(&self, items: &[Contour]) -> (Pt, Pt) {
        let c = &items[self.index];
        if c.closed {
            let p = c.pts[self.start];
            (p, p)
        } else if self.reversed {
            (c.pts[c.pts.len() - 1], c.pts[0])
        } else {
            (c.pts[0], c.pts[c.pts.len() - 1])
        }
    }

    /// The points of the contour in the order they are cut; a closed contour ends at its start again.
    pub fn points(&self, items: &[Contour]) -> Vec<Pt> {
        let c = &items[self.index];
        if c.closed {
            let n = c.pts.len();
            let mut v: Vec<Pt> = (0..n).map(|k| c.pts[(self.start + k) % n]).collect();
            v.push(c.pts[self.start]);
            v
        } else if self.reversed {
            c.pts.iter().rev().copied().collect()
        } else {
            c.pts.clone()
        }
    }
}

/// Joins the open paths whose ends are closer than `tol` (the end of one to the start or the end of another), so that a drawing made of loose lines is cut as one path; a path that
/// comes back to its start becomes a closed contour. Nothing is joined when there are more than `max_paths` open paths (the search is quadratic) or `tol` is 0.
pub fn join(paths: Vec<Contour>, tol: f64, max_paths: usize) -> Vec<Contour> {
    let open_count = paths.iter().filter(|c| !c.closed).count();
    if tol <= 0.0 || open_count == 0 || open_count > max_paths {
        return paths;
    }
    let mut out: Vec<Contour> = Vec::with_capacity(paths.len());
    let mut pool: Vec<Option<Vec<Pt>>> = Vec::new();
    for c in paths {
        if c.closed {
            out.push(c);
        } else {
            pool.push(Some(c.pts));
        }
    }
    for seed in 0..pool.len() {
        let Some(mut cur) = pool[seed].take() else {
            continue;
        };
        loop {
            let last = cur[cur.len() - 1];
            let first = cur[0];
            let mut found = false;
            for other in pool.iter_mut().filter(|o| o.is_some()) {
                let o = other.as_ref().map_or(&[][..], |v| v.as_slice());
                if o.len() < 2 {
                    continue;
                }
                let (of, ol) = (o[0], o[o.len() - 1]);
                if dist(last, of) <= tol {
                    cur.extend_from_slice(&o[1..]);
                } else if dist(last, ol) <= tol {
                    cur.extend(o[..o.len() - 1].iter().rev());
                } else if dist(first, ol) <= tol {
                    let mut v = o.to_vec();
                    v.extend_from_slice(&cur[1..]);
                    cur = v;
                } else if dist(first, of) <= tol {
                    let mut v: Vec<Pt> = o[1..].iter().rev().copied().collect();
                    v.extend_from_slice(&cur);
                    cur = v;
                } else {
                    continue;
                }
                *other = None;
                found = true;
                break;
            }
            if !found {
                break;
            }
        }
        let closed = cur.len() >= 4 && dist(cur[0], cur[cur.len() - 1]) <= tol;
        if closed {
            cur.pop();
        }
        out.push(Contour { pts: cur, closed });
    }
    out
}

fn bbox(pts: &[Pt]) -> (Pt, Pt) {
    let mut lo = (f64::INFINITY, f64::INFINITY);
    let mut hi = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for &(x, y) in pts {
        lo = (lo.0.min(x), lo.1.min(y));
        hi = (hi.0.max(x), hi.1.max(y));
    }
    (lo, hi)
}

/// For every contour, the closed contours it lies inside (decided by the first of its first eight points that is not on the edge of the other one). `None` when there are more than
/// `max_closed` closed contours, because the search is quadratic.
pub fn containers(items: &[Contour], max_closed: usize) -> Option<Vec<Vec<usize>>> {
    let closed: Vec<usize> = (0..items.len()).filter(|&i| items[i].closed).collect();
    if closed.len() > max_closed {
        return None;
    }
    let boxes: Vec<(Pt, Pt)> = closed.iter().map(|&j| bbox(&items[j].pts)).collect();
    let mut out: Vec<Vec<usize>> = vec![Vec::new(); items.len()];
    for (i, item) in items.iter().enumerate() {
        for (k, &j) in closed.iter().enumerate() {
            if i == j {
                continue;
            }
            let (lo, hi) = boxes[k];
            let ring = &items[j].pts;
            let probe = item.pts.iter().take(8).find(|p| {
                (lo.0..=hi.0).contains(&p.0)
                    && (lo.1..=hi.1).contains(&p.1)
                    && dist_to_ring(**p, ring) > 1e-6
            });
            if let Some(&p) = probe {
                if point_in_polygon(ring, p) {
                    out[i].push(j);
                }
            }
        }
    }
    Some(out)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrderOptions {
    pub two_opt: bool,
    /// The most moves the 2-opt improvement tries.
    pub budget: u64,
    /// The 2-opt improvement is not tried for more contours than this.
    pub two_opt_max_items: usize,
}

fn nearest_vertex(pts: &[Pt], p: Pt) -> usize {
    let mut best = (0, f64::INFINITY);
    for (k, &q) in pts.iter().enumerate() {
        let d = dist(p, q);
        if d < best.1 {
            best = (k, d);
        }
    }
    best.0
}

/// The order and the direction in which to cut the contours, starting from `from`. `parents[i]` are the closed contours that item `i` lies inside; every contour is cut before them.
/// With nothing in `parents` the order is only the short one.
pub fn sequence(
    items: &[Contour],
    parents: &[Vec<usize>],
    from: Pt,
    opts: &OrderOptions,
) -> Vec<Visit> {
    let n = items.len();
    let mut pending = vec![0usize; n];
    for ps in parents {
        for &j in ps {
            pending[j] += 1;
        }
    }
    let mut done = vec![false; n];
    let mut visits: Vec<Visit> = Vec::with_capacity(n);
    let mut head = from;
    for _ in 0..n {
        let mut best: Option<(f64, usize, bool)> = None;
        for (i, item) in items.iter().enumerate() {
            if done[i] || pending[i] > 0 {
                continue;
            }
            let first = dist(head, item.pts[0]);
            let (d, rev) = if item.closed {
                (first, false)
            } else {
                let last = dist(head, item.pts[item.pts.len() - 1]);
                if last < first {
                    (last, true)
                } else {
                    (first, false)
                }
            };
            if best.is_none_or(|b| d < b.0) {
                best = Some((d, i, rev));
            }
        }
        // a cycle in the containment (it cannot happen with real shapes) must not lose a contour: the first one left goes next
        let (_, i, rev) = best.unwrap_or_else(|| {
            let i = (0..n).find(|&i| !done[i]).unwrap_or(0);
            (0.0, i, false)
        });
        let start = if items[i].closed {
            nearest_vertex(&items[i].pts, head)
        } else {
            0
        };
        let v = Visit {
            index: i,
            reversed: rev,
            start,
        };
        head = v.ends(items).1;
        done[i] = true;
        if let Some(ps) = parents.get(i) {
            for &j in ps {
                pending[j] = pending[j].saturating_sub(1);
            }
        }
        visits.push(v);
    }
    if opts.two_opt && n <= opts.two_opt_max_items && n >= 3 {
        improve(items, parents, from, &mut visits, opts.budget);
    }
    visits
}

/// The 2-opt improvement: reverses a run of contours when that shortens the travel with the laser off and keeps every contour before the ones it lies inside.
fn improve(items: &[Contour], parents: &[Vec<usize>], from: Pt, visits: &mut [Visit], budget: u64) {
    let m = visits.len();
    let mut ends: Vec<(Pt, Pt)> = visits.iter().map(|v| v.ends(items)).collect();
    let mut pos = vec![0usize; items.len()];
    for (k, v) in visits.iter().enumerate() {
        pos[v.index] = k;
    }
    let mut evals = 0u64;
    'sweeps: for _ in 0..MAX_SWEEPS {
        let mut improved = false;
        for i in 0..m {
            for j in (i + 1)..m {
                evals += 1;
                if evals > budget {
                    break 'sweeps;
                }
                let prev = if i == 0 { from } else { ends[i - 1].1 };
                let open_i = !items[visits[i].index].closed;
                let open_j = !items[visits[j].index].closed;
                // reversed, the run begins with contour j taken backwards and ends with contour i taken backwards
                let new_start = if open_j { ends[j].1 } else { ends[j].0 };
                let new_end = if open_i { ends[i].0 } else { ends[i].1 };
                let (old_out, new_out) = if j + 1 < m {
                    let next = ends[j + 1].0;
                    (dist(ends[j].1, next), dist(new_end, next))
                } else {
                    (0.0, 0.0)
                };
                let gain = dist(prev, ends[i].0) + old_out - dist(prev, new_start) - new_out;
                if gain <= 1e-9 {
                    continue;
                }
                // reversing puts the contours of the run in the opposite order: it is not allowed when one of them must come before another of the run
                let breaks = (i..=j).any(|p| {
                    parents
                        .get(visits[p].index)
                        .is_some_and(|ps| ps.iter().any(|&q| (i..=j).contains(&pos[q])))
                });
                if breaks {
                    continue;
                }
                visits[i..=j].reverse();
                ends[i..=j].reverse();
                for p in i..=j {
                    if !items[visits[p].index].closed {
                        ends[p] = (ends[p].1, ends[p].0);
                        visits[p].reversed = !visits[p].reversed;
                    }
                    pos[visits[p].index] = p;
                }
                improved = true;
            }
        }
        if !improved {
            break;
        }
    }
}

/// The length of the moves with the laser off, from `from` through all the visits.
pub fn travel_mm(items: &[Contour], visits: &[Visit], from: Pt) -> f64 {
    let mut head = from;
    let mut sum = 0.0;
    for v in visits {
        let (s, e) = v.ends(items);
        sum += dist(head, s);
        head = e;
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open(pts: &[Pt]) -> Contour {
        Contour {
            pts: pts.to_vec(),
            closed: false,
        }
    }

    fn square(x: f64, y: f64, side: f64) -> Contour {
        Contour {
            pts: vec![(x, y), (x + side, y), (x + side, y + side), (x, y + side)],
            closed: true,
        }
    }

    const OPTS: OrderOptions = OrderOptions {
        two_opt: true,
        budget: 1_000_000,
        two_opt_max_items: 1000,
    };

    #[test]
    fn four_loose_lines_that_meet_become_one_closed_contour() {
        let joined = join(
            vec![
                open(&[(0.0, 0.0), (1.0, 0.0)]),
                open(&[(1.0, 0.0), (1.0, 1.0)]),
                open(&[(1.0, 1.005), (0.0, 1.005)]),
                open(&[(0.0, 1.002), (0.0, 0.0)]),
            ],
            0.01,
            100,
        );
        assert_eq!(joined.len(), 1);
        assert!(joined[0].closed);
        assert_eq!(
            joined[0].pts,
            vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.005)]
        );
    }

    #[test]
    fn lines_written_the_other_way_round_are_joined_too() {
        // the second line goes from its end to the end of the first, the third from the start of the first
        let joined = join(
            vec![
                open(&[(1.0, 0.0), (2.0, 0.0)]),
                open(&[(3.0, 0.0), (2.0, 0.0)]),
                open(&[(0.0, 0.0), (1.0, 0.0)]),
            ],
            0.001,
            100,
        );
        assert_eq!(joined.len(), 1);
        assert!(!joined[0].closed);
        assert_eq!(
            joined[0].pts,
            vec![(0.0, 0.0), (1.0, 0.0), (2.0, 0.0), (3.0, 0.0)]
        );
    }

    #[test]
    fn lines_that_are_apart_stay_apart_and_closed_ones_are_not_touched() {
        let sq = square(0.0, 0.0, 1.0);
        let input = vec![
            sq.clone(),
            open(&[(5.0, 5.0), (6.0, 5.0)]),
            open(&[(8.0, 5.0), (9.0, 5.0)]),
        ];
        let joined = join(input.clone(), 0.01, 100);
        assert_eq!(joined.len(), 3);
        assert_eq!(joined[0], sq);
        // too many open paths, or no tolerance: left as they are
        assert_eq!(join(input.clone(), 0.01, 1), input);
        assert_eq!(join(input.clone(), 0.0, 100), input);
    }

    #[test]
    fn a_single_open_line_that_comes_back_to_its_start_is_closed() {
        let joined = join(
            vec![open(&[(0.0, 0.0), (4.0, 0.0), (4.0, 3.0), (0.0, 0.0)])],
            0.01,
            10,
        );
        assert_eq!(joined.len(), 1);
        assert!(joined[0].closed);
        assert_eq!(joined[0].pts, vec![(0.0, 0.0), (4.0, 0.0), (4.0, 3.0)]);
    }

    #[test]
    fn what_is_inside_what_is_found() {
        let items = [
            square(0.0, 0.0, 10.0),            // 0: the outer one
            square(4.0, 4.0, 2.0),             // 1: inside 0
            square(4.5, 4.5, 1.0),             // 2: inside 0 and 1
            square(20.0, 0.0, 2.0),            // 3: apart
            open(&[(1.0, 1.0), (2.0, 1.0)]),   // 4: an open line inside 0
            open(&[(-5.0, 5.0), (15.0, 5.0)]), // 5: an open line that starts and ends outside
        ];
        let c = containers(&items, 100).unwrap();
        assert!(c[0].is_empty());
        assert_eq!(c[1], vec![0]);
        assert_eq!(c[2], vec![0, 1]);
        assert!(c[3].is_empty());
        assert_eq!(c[4], vec![0]);
        assert!(c[5].is_empty());
        // four closed contours are too many for a limit of three
        assert!(containers(&items, 3).is_none());
    }

    #[test]
    fn two_equal_squares_are_not_inside_each_other() {
        // all the points of one are on the edge of the other
        let items = [square(0.0, 0.0, 10.0), square(0.0, 0.0, 10.0)];
        let c = containers(&items, 100).unwrap();
        assert!(c[0].is_empty() && c[1].is_empty());
    }

    #[test]
    fn a_contour_is_cut_before_the_one_it_lies_inside_even_when_that_one_is_nearer() {
        // from (0,0) the outer square (a corner at (0,0)) is at distance 0 and the inner one at 5.66, but the inner one goes first
        let items = [square(0.0, 0.0, 10.0), square(4.0, 4.0, 2.0)];
        let parents = containers(&items, 10).unwrap();
        let v = sequence(&items, &parents, (0.0, 0.0), &OPTS);
        assert_eq!(v.iter().map(|x| x.index).collect::<Vec<_>>(), vec![1, 0]);
        // the inner square is entered at its corner nearest to the head, (4,4); the outer one at its corner nearest to (4,4), (0,0)
        assert_eq!(v[0].start, 0);
        assert_eq!(v[1].start, 0);
    }

    #[test]
    fn a_closed_contour_starts_at_the_vertex_nearest_to_the_head() {
        // a square written from (0,0); the head is at (11,11), nearest to (10,10), the third vertex
        let items = [square(0.0, 0.0, 10.0)];
        let v = sequence(&items, &[Vec::new()], (11.0, 11.0), &OPTS);
        assert_eq!(v[0].start, 2);
        let pts = v[0].points(&items);
        assert_eq!(
            pts,
            vec![
                (10.0, 10.0),
                (0.0, 10.0),
                (0.0, 0.0),
                (10.0, 0.0),
                (10.0, 10.0)
            ]
        );
    }

    #[test]
    fn the_nearest_open_path_is_next_and_is_taken_from_its_nearer_end() {
        // from (0,0): A (written from its far end) is entered at (0,0), then C at 2 (1 away), then B at 10 (7 away): 0 + 1 + 7 = 8
        let items = [
            open(&[(1.0, 0.0), (0.0, 0.0)]),   // A
            open(&[(10.0, 0.0), (11.0, 0.0)]), // B
            open(&[(2.0, 0.0), (3.0, 0.0)]),   // C
        ];
        let none = vec![Vec::new(); 3];
        let off = OrderOptions {
            two_opt: false,
            ..OPTS
        };
        let v = sequence(&items, &none, (0.0, 0.0), &off);
        assert_eq!(
            v.iter().map(|x| (x.index, x.reversed)).collect::<Vec<_>>(),
            vec![(0, true), (2, false), (1, false)]
        );
        assert_eq!(travel_mm(&items, &v, (0.0, 0.0)), 8.0);
        assert_eq!(v[0].points(&items), vec![(0.0, 0.0), (1.0, 0.0)]);
    }

    /// Small closed squares stand for points: they are entered and left at the same place.
    fn dot(x: f64, y: f64) -> Contour {
        square(x, y, 0.001)
    }

    #[test]
    fn two_opt_shortens_the_trip_the_nearest_neighbour_makes() {
        // points (2,3) (4,5) (5,1) (9,9) (0,4) seen from (0,0) (each is a square of 0.001, so the numbers are exact to 0.02).
        // Nearest neighbour: (2,3) 3.606, (0,4) 2.236, (4,5) 4.123, (5,1) 4.123, (9,9) 8.944 = 23.03.
        // 2-opt reverses the run of the first two: (0,4) 4, (2,3) 2.236, (5,1) 3.606, (4,5) 4.123, (9,9) 6.403 = 20.37.
        let items = [
            dot(2.0, 3.0),
            dot(4.0, 5.0),
            dot(5.0, 1.0),
            dot(9.0, 9.0),
            dot(0.0, 4.0),
        ];
        let none = vec![Vec::new(); 5];
        let off = OrderOptions {
            two_opt: false,
            ..OPTS
        };
        let greedy = sequence(&items, &none, (0.0, 0.0), &off);
        let better = sequence(&items, &none, (0.0, 0.0), &OPTS);
        assert_eq!(
            greedy.iter().map(|v| v.index).collect::<Vec<_>>(),
            vec![0, 4, 1, 2, 3]
        );
        assert_eq!(
            better.iter().map(|v| v.index).collect::<Vec<_>>(),
            vec![4, 0, 2, 1, 3]
        );
        assert!((travel_mm(&items, &greedy, (0.0, 0.0)) - 23.032).abs() < 0.02);
        assert!((travel_mm(&items, &better, (0.0, 0.0)) - 20.368).abs() < 0.02);
    }

    #[test]
    fn two_opt_with_open_paths_visits_each_once_and_never_makes_the_trip_longer() {
        let mut seed = 3u64;
        let mut next = move || {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((seed >> 33) % 400) as f64 / 10.0
        };
        for round in 0..40 {
            let items: Vec<Contour> = (0..(6 + round % 7))
                .map(|k| {
                    let (x, y) = (next(), next());
                    if k % 2 == 1 {
                        open(&[(x, y), (x + 1.0 + next() / 10.0, y + 0.5)])
                    } else {
                        square(x, y, 0.5)
                    }
                })
                .collect();
            let none = vec![Vec::new(); items.len()];
            let off = OrderOptions {
                two_opt: false,
                ..OPTS
            };
            let g = sequence(&items, &none, (0.0, 0.0), &off);
            let b = sequence(&items, &none, (0.0, 0.0), &OPTS);
            let mut seen: Vec<usize> = b.iter().map(|v| v.index).collect();
            seen.sort_unstable();
            assert_eq!(seen, (0..items.len()).collect::<Vec<_>>());
            assert!(travel_mm(&items, &b, (0.0, 0.0)) <= travel_mm(&items, &g, (0.0, 0.0)) + 1e-9);
        }
    }

    #[test]
    fn two_opt_never_breaks_the_inside_first_rule() {
        // many nested pairs spread out: whatever the order, the inner square of each pair comes before its outer one
        let mut items = Vec::new();
        for k in 0..12 {
            let x = f64::from(k % 4) * 30.0;
            let y = f64::from(k / 4) * 30.0 + f64::from(k % 3);
            items.push(square(x, y, 10.0));
            items.push(square(x + 4.0, y + 4.0, 2.0));
        }
        let parents = containers(&items, 100).unwrap();
        let v = sequence(&items, &parents, (50.0, 50.0), &OPTS);
        assert_eq!(v.len(), items.len());
        let at: Vec<usize> = {
            let mut a = vec![0; items.len()];
            for (k, x) in v.iter().enumerate() {
                a[x.index] = k;
            }
            a
        };
        for (i, ps) in parents.iter().enumerate() {
            for &j in ps {
                assert!(at[i] < at[j], "{i} must come before {j}");
            }
        }
    }

    #[test]
    fn the_budget_and_the_size_limit_stop_the_improvement_but_not_the_order() {
        let items: Vec<Contour> = (0..30)
            .map(|k| dot(f64::from((k * 7) % 30), f64::from((k * 11) % 17)))
            .collect();
        let none = vec![Vec::new(); 30];
        for opts in [
            OrderOptions { budget: 0, ..OPTS },
            OrderOptions {
                two_opt_max_items: 5,
                ..OPTS
            },
            OrderOptions {
                two_opt: false,
                ..OPTS
            },
        ] {
            let v = sequence(&items, &none, (0.0, 0.0), &opts);
            let mut seen: Vec<usize> = v.iter().map(|x| x.index).collect();
            seen.sort_unstable();
            assert_eq!(seen, (0..30).collect::<Vec<_>>());
        }
    }

    #[test]
    fn nothing_is_lost_when_the_containment_has_a_cycle() {
        // two contours each said to be inside the other: they are both cut
        let items = [square(0.0, 0.0, 1.0), square(5.0, 5.0, 1.0)];
        let v = sequence(&items, &[vec![1], vec![0]], (0.0, 0.0), &OPTS);
        assert_eq!(v.len(), 2);
    }

    #[test]
    fn no_contours_give_no_visits() {
        assert!(sequence(&[], &[], (0.0, 0.0), &OPTS).is_empty());
        assert_eq!(travel_mm(&[], &[], (0.0, 0.0)), 0.0);
    }
}
