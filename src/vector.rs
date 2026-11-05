//! What a drawing is once it is read, whatever file it came from: shapes (lists of points in millimetres) with the layer and the colour they were drawn with, the limits that keep a
//! hostile file from asking for gigabytes, and the small geometry the other parts share (distance, area, point in a polygon, where two segments cross).

/// A point of the drawing, in millimetres.
pub type Pt = (f64, f64);

/// The most notes (things skipped or not understood) a drawing keeps; more of them add nothing for the person.
pub const MAX_WARNINGS: usize = 40;

/// Two points closer than this (mm) are the same point.
pub const SAME_POINT_MM: f64 = 1e-9;

/// The kind of file a drawing came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Svg,
    Dxf,
}

/// One stroke of the drawing: the points are joined by straight lines (curves are already flattened), and a closed shape also goes from the last point back to the first (which is not
/// repeated).
#[derive(Debug, Clone, PartialEq)]
pub struct Shape {
    pub layer: String,
    /// The colour as `#rrggbb` (or the name the file gave it when it is not one this reader knows).
    pub color: Option<String>,
    pub pts: Vec<Pt>,
    pub closed: bool,
}

/// What a file says: its shapes and what was left out.
#[derive(Debug, Clone, PartialEq)]
pub struct Drawing {
    pub source: Source,
    pub shapes: Vec<Shape>,
    pub warnings: Vec<String>,
    /// The height of the page in mm when the file says it (an SVG with a size).
    pub page_height_mm: Option<f64>,
}

/// What makes two shapes belong to the same group, with the same settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupBy {
    /// The colour for an SVG (the layer when a shape has none) and the layer for a DXF.
    Auto,
    Layer,
    Color,
}

impl GroupBy {
    pub fn from_word(word: &str) -> Option<GroupBy> {
        match word.trim().to_ascii_lowercase().as_str() {
            "auto" => Some(GroupBy::Auto),
            "layer" => Some(GroupBy::Layer),
            "color" | "colour" => Some(GroupBy::Color),
            _ => None,
        }
    }
}

impl Drawing {
    /// The name of the group a shape belongs to (lower case, so that `Red` and `red` are one).
    pub fn key_of(&self, s: &Shape, by: GroupBy) -> String {
        let key = match by {
            GroupBy::Layer => s.layer.clone(),
            GroupBy::Color => s.color.clone().unwrap_or_else(|| "none".to_string()),
            GroupBy::Auto => match self.source {
                Source::Svg => s.color.clone().unwrap_or_else(|| s.layer.clone()),
                Source::Dxf => s.layer.clone(),
            },
        };
        key.trim().to_lowercase()
    }

    /// The groups of the drawing in the order they first appear.
    pub fn keys(&self, by: GroupBy) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for s in &self.shapes {
            let k = self.key_of(s, by);
            if !out.contains(&k) {
                out.push(k);
            }
        }
        out
    }

    /// The box that holds every point: the lower left and the upper right corner.
    pub fn bounds(&self) -> Option<(Pt, Pt)> {
        bounds_of(self.shapes.iter().flat_map(|s| s.pts.iter().copied()))
    }
}

/// The box that holds some points, or none when there are none.
pub fn bounds_of(points: impl Iterator<Item = Pt>) -> Option<(Pt, Pt)> {
    let mut min = (f64::INFINITY, f64::INFINITY);
    let mut max = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    let mut any = false;
    for (x, y) in points {
        any = true;
        min = (min.0.min(x), min.1.min(y));
        max = (max.0.max(x), max.1.max(y));
    }
    any.then_some((min, max))
}

/// The limits that keep a hostile or broken file from asking for too much. Every one of them is a setting (`[limits]` of the settings file).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Limits {
    /// The largest file that is read.
    pub max_file_bytes: usize,
    /// How deep the elements of an SVG may be nested.
    pub max_depth: usize,
    /// How many elements (SVG) or entities (DXF) are read.
    pub max_nodes: usize,
    pub max_shapes: usize,
    /// All the points of all the shapes.
    pub max_points: usize,
    /// The most pieces one curve is cut into.
    pub max_curve_segments: usize,
    /// A point farther than this from the origin (mm) is a mistake of the file, not a drawing.
    pub max_coordinate_mm: f64,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_file_bytes: 32 * 1024 * 1024,
            max_depth: 64,
            max_nodes: 200_000,
            max_shapes: 100_000,
            max_points: 2_000_000,
            max_curve_segments: 2_000,
            max_coordinate_mm: 100_000.0,
        }
    }
}

impl Limits {
    pub fn validate(&self) -> Result<(), String> {
        let ok = (1..=1 << 30).contains(&self.max_file_bytes)
            && (1..=1_000).contains(&self.max_depth)
            && (1..=50_000_000).contains(&self.max_nodes)
            && (1..=10_000_000).contains(&self.max_shapes)
            && (1..=100_000_000).contains(&self.max_points)
            && (1..=100_000).contains(&self.max_curve_segments)
            && self.max_coordinate_mm.is_finite()
            && self.max_coordinate_mm > 0.0
            && self.max_coordinate_mm <= 1.0e7;
        if ok {
            Ok(())
        } else {
            Err("a limit of [limits] is out of range".to_string())
        }
    }
}

/// How a file is read.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReadOptions {
    /// Pixels in an inch, for an SVG whose lengths are pixels (96 is what browsers use).
    pub dpi: f64,
    /// How far a flattened curve may be from the true curve, in mm.
    pub tolerance_mm: f64,
    pub limits: Limits,
    /// The millimetres of a unit of a DXF file that does not say its unit.
    pub dxf_unit_mm: f64,
}

impl Default for ReadOptions {
    fn default() -> Self {
        ReadOptions {
            dpi: 96.0,
            tolerance_mm: 0.01,
            limits: Limits::default(),
            dxf_unit_mm: 1.0,
        }
    }
}

impl ReadOptions {
    pub fn validate(&self) -> Result<(), String> {
        self.limits.validate()?;
        if !(self.dpi.is_finite() && (1.0..=2400.0).contains(&self.dpi)) {
            return Err(format!("dpi is {}: it must be from 1 to 2400", self.dpi));
        }
        if !(self.tolerance_mm.is_finite() && (0.001..=1.0).contains(&self.tolerance_mm)) {
            return Err(format!(
                "tolerance is {}: it must be from 0.001 to 1 mm",
                self.tolerance_mm
            ));
        }
        if !(self.dxf_unit_mm.is_finite() && (1e-3..=1e4).contains(&self.dxf_unit_mm)) {
            return Err("dxf_unit_mm must be from 0.001 to 10000".to_string());
        }
        Ok(())
    }
}

/// Gathers the shapes a reader finds, applying the limits and cleaning each shape (no repeated points, a closed shape needs three).
pub struct Collector {
    drawing: Drawing,
    limits: Limits,
    points: usize,
}

impl Collector {
    pub fn new(source: Source, limits: Limits) -> Collector {
        Collector {
            drawing: Drawing {
                source,
                shapes: Vec::new(),
                warnings: Vec::new(),
                page_height_mm: None,
            },
            limits,
            points: 0,
        }
    }

    /// Notes something that was skipped. The same note is kept once, and only [`MAX_WARNINGS`] of them are kept.
    pub fn warn(&mut self, msg: impl Into<String>) {
        let msg = msg.into();
        if self.drawing.warnings.len() < MAX_WARNINGS && !self.drawing.warnings.contains(&msg) {
            self.drawing.warnings.push(msg);
        }
    }

    pub fn set_page_height(&mut self, h: f64) {
        self.drawing.page_height_mm = Some(h);
    }

    /// How many points more the limits allow.
    pub fn room(&self) -> usize {
        self.limits.max_points.saturating_sub(self.points)
    }

    /// Adds a shape. A shape with a point that is not a number or is out of range is left out with a note; too many shapes or points is an error.
    pub fn push(
        &mut self,
        layer: &str,
        color: Option<&str>,
        pts: Vec<Pt>,
        closed: bool,
    ) -> Result<(), String> {
        let max = self.limits.max_coordinate_mm;
        let mut v: Vec<Pt> = Vec::with_capacity(pts.len());
        for p in pts {
            if !(p.0.is_finite() && p.1.is_finite()) || p.0.abs() > max || p.1.abs() > max {
                self.warn(
                    "a shape with a point that is not a number or is too far away was skipped",
                );
                return Ok(());
            }
            if v.last().is_none_or(|&l| dist(l, p) > SAME_POINT_MM) {
                v.push(p);
            }
        }
        if closed && v.len() > 1 && dist(v[0], v[v.len() - 1]) <= SAME_POINT_MM {
            v.pop();
        }
        let closed = closed && v.len() >= 3;
        if v.len() < 2 {
            return Ok(());
        }
        if self.drawing.shapes.len() >= self.limits.max_shapes {
            return Err(format!(
                "the drawing has more than {} shapes (max_shapes)",
                self.limits.max_shapes
            ));
        }
        if v.len() > self.room() {
            return Err(format!(
                "the drawing has more than {} points (max_points)",
                self.limits.max_points
            ));
        }
        self.points += v.len();
        self.drawing.shapes.push(Shape {
            layer: layer.to_string(),
            color: color.map(str::to_string),
            pts: v,
            closed,
        });
        Ok(())
    }

    pub fn finish(self) -> Drawing {
        self.drawing
    }
}

/// The distance between two points.
pub fn dist(a: Pt, b: Pt) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

/// The length of a path; a closed one also counts the way back from the last point to the first.
pub fn path_len(pts: &[Pt], closed: bool) -> f64 {
    let mut sum: f64 = pts.windows(2).map(|w| dist(w[0], w[1])).sum();
    if closed && pts.len() > 2 {
        sum += dist(pts[pts.len() - 1], pts[0]);
    }
    sum
}

/// The signed area of a ring (positive when it goes counterclockwise); the ring is closed by itself.
pub fn signed_area(ring: &[Pt]) -> f64 {
    if ring.len() < 3 {
        return 0.0;
    }
    let mut sum = 0.0;
    let mut prev = ring[ring.len() - 1];
    for &p in ring {
        sum += prev.0 * p.1 - p.0 * prev.1;
        prev = p;
    }
    sum / 2.0
}

/// Whether a point is inside a polygon (even-odd rule; the polygon is closed by itself).
pub fn point_in_polygon(poly: &[Pt], p: Pt) -> bool {
    if poly.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut prev = poly[poly.len() - 1];
    for &cur in poly {
        let (xi, yi) = cur;
        let (xj, yj) = prev;
        if (yi > p.1) != (yj > p.1) && p.0 < (xj - xi) * (p.1 - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        prev = cur;
    }
    inside
}

/// The distance from a point to a segment.
pub fn dist_to_segment(p: Pt, a: Pt, b: Pt) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    if len2 <= 0.0 {
        return dist(p, a);
    }
    let t = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0);
    dist(p, (a.0 + t * dx, a.1 + t * dy))
}

/// The distance from a point to the edge of a ring (closed by itself).
pub fn dist_to_ring(p: Pt, ring: &[Pt]) -> f64 {
    if ring.is_empty() {
        return f64::INFINITY;
    }
    let mut best = f64::INFINITY;
    let mut prev = ring[ring.len() - 1];
    for &cur in ring {
        best = best.min(dist_to_segment(p, prev, cur));
        prev = cur;
    }
    best
}

/// Where the segments `ab` and `cd` cross, when they cross strictly inside both (not at an end): the place along each (0 to 1) and the point.
pub fn segment_crossing(a: Pt, b: Pt, c: Pt, d: Pt) -> Option<(f64, f64, Pt)> {
    let (r, s) = ((b.0 - a.0, b.1 - a.1), (d.0 - c.0, d.1 - c.1));
    let den = r.0 * s.1 - r.1 * s.0;
    if den.abs() <= 1e-18 {
        return None;
    }
    let (qp0, qp1) = (c.0 - a.0, c.1 - a.1);
    let t = (qp0 * s.1 - qp1 * s.0) / den;
    let u = (qp0 * r.1 - qp1 * r.0) / den;
    const EDGE: f64 = 1e-9;
    if t > EDGE && t < 1.0 - EDGE && u > EDGE && u < 1.0 - EDGE {
        Some((t, u, (a.0 + t * r.0, a.1 + t * r.1)))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SQUARE: [Pt; 4] = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];

    #[test]
    fn the_area_of_a_square_is_100_and_its_sign_follows_the_turn() {
        assert_eq!(signed_area(&SQUARE), 100.0);
        let mut rev = SQUARE;
        rev.reverse();
        assert_eq!(signed_area(&rev), -100.0);
        assert_eq!(signed_area(&SQUARE[..2]), 0.0);
    }

    #[test]
    fn a_point_is_inside_or_outside_a_polygon() {
        assert!(point_in_polygon(&SQUARE, (5.0, 5.0)));
        assert!(!point_in_polygon(&SQUARE, (15.0, 5.0)));
        assert!(!point_in_polygon(&SQUARE, (5.0, -0.1)));
        assert!(!point_in_polygon(&SQUARE[..2], (5.0, 5.0)));
    }

    #[test]
    fn the_distance_to_a_segment_and_to_a_ring() {
        assert_eq!(dist_to_segment((5.0, 3.0), (0.0, 0.0), (10.0, 0.0)), 3.0);
        // beyond the end the nearest point is the end: 3-4-5
        assert_eq!(dist_to_segment((13.0, 4.0), (0.0, 0.0), (10.0, 0.0)), 5.0);
        // a segment of no length
        assert_eq!(dist_to_segment((3.0, 4.0), (0.0, 0.0), (0.0, 0.0)), 5.0);
        assert_eq!(dist_to_ring((5.0, 5.0), &SQUARE), 5.0);
        assert_eq!(dist_to_ring((12.0, 5.0), &SQUARE), 2.0);
        assert!(dist_to_ring((1.0, 1.0), &[]).is_infinite());
    }

    #[test]
    fn two_segments_cross_strictly_inside_or_not_at_all() {
        let x = segment_crossing((0.0, 0.0), (10.0, 10.0), (0.0, 10.0), (10.0, 0.0)).unwrap();
        assert!((x.0 - 0.5).abs() < 1e-12 && (x.1 - 0.5).abs() < 1e-12);
        assert!((x.2 .0 - 5.0).abs() < 1e-12 && (x.2 .1 - 5.0).abs() < 1e-12);
        // parallel
        assert!(segment_crossing((0.0, 0.0), (10.0, 0.0), (0.0, 1.0), (10.0, 1.0)).is_none());
        // they meet at an end only
        assert!(segment_crossing((0.0, 0.0), (10.0, 0.0), (10.0, 0.0), (10.0, 5.0)).is_none());
        // they do not reach each other
        assert!(segment_crossing((0.0, 0.0), (1.0, 0.0), (5.0, -1.0), (5.0, 1.0)).is_none());
    }

    #[test]
    fn the_length_of_a_path_counts_the_way_back_when_closed() {
        assert_eq!(path_len(&SQUARE, true), 40.0);
        assert_eq!(path_len(&SQUARE, false), 30.0);
        assert_eq!(path_len(&[], false), 0.0);
    }

    #[test]
    fn the_collector_cleans_a_shape() {
        let mut c = Collector::new(Source::Dxf, Limits::default());
        // repeated points go, and the last point that repeats the first goes from a closed shape
        c.push(
            "a",
            None,
            vec![(0.0, 0.0), (0.0, 0.0), (4.0, 0.0), (4.0, 3.0), (0.0, 0.0)],
            true,
        )
        .unwrap();
        // a closed shape with two points is an open line
        c.push("a", None, vec![(0.0, 0.0), (1.0, 0.0)], true)
            .unwrap();
        // one point is nothing
        c.push("a", None, vec![(0.0, 0.0), (0.0, 0.0)], false)
            .unwrap();
        let d = c.finish();
        assert_eq!(d.shapes.len(), 2);
        assert_eq!(d.shapes[0].pts, vec![(0.0, 0.0), (4.0, 0.0), (4.0, 3.0)]);
        assert!(d.shapes[0].closed);
        assert!(!d.shapes[1].closed);
    }

    #[test]
    fn the_collector_skips_a_shape_that_is_not_a_number_and_refuses_too_much() {
        let mut c = Collector::new(Source::Svg, Limits::default());
        c.push("a", None, vec![(0.0, 0.0), (f64::NAN, 1.0)], false)
            .unwrap();
        c.push("a", None, vec![(0.0, 0.0), (1.0e9, 1.0)], false)
            .unwrap();
        assert_eq!(c.room(), Limits::default().max_points);
        let d = c.finish();
        assert!(d.shapes.is_empty());
        assert_eq!(d.warnings.len(), 1, "the same note is kept once");

        let lim = Limits {
            max_shapes: 1,
            ..Limits::default()
        };
        let mut c = Collector::new(Source::Svg, lim);
        c.push("a", None, vec![(0.0, 0.0), (1.0, 0.0)], false)
            .unwrap();
        assert!(c
            .push("a", None, vec![(0.0, 0.0), (2.0, 0.0)], false)
            .is_err());
        let lim = Limits {
            max_points: 3,
            ..Limits::default()
        };
        let mut c = Collector::new(Source::Svg, lim);
        assert!(c
            .push(
                "a",
                None,
                vec![(0.0, 0.0), (1.0, 0.0), (2.0, 0.0), (3.0, 0.0)],
                false
            )
            .is_err());
    }

    #[test]
    fn shapes_are_grouped_by_colour_for_an_svg_and_by_layer_for_a_dxf() {
        let shape = |layer: &str, color: Option<&str>| Shape {
            layer: layer.to_string(),
            color: color.map(str::to_string),
            pts: vec![(0.0, 0.0), (1.0, 1.0)],
            closed: false,
        };
        let shapes = vec![
            shape("Top", Some("#FF0000")),
            shape("Top", Some("#00ff00")),
            shape("Base", None),
            shape("Top", Some("#ff0000")),
        ];
        let svg = Drawing {
            source: Source::Svg,
            shapes: shapes.clone(),
            warnings: vec![],
            page_height_mm: None,
        };
        assert_eq!(svg.keys(GroupBy::Auto), vec!["#ff0000", "#00ff00", "base"]);
        assert_eq!(svg.keys(GroupBy::Layer), vec!["top", "base"]);
        let dxf = Drawing {
            source: Source::Dxf,
            ..svg
        };
        assert_eq!(dxf.keys(GroupBy::Auto), vec!["top", "base"]);
        assert_eq!(dxf.keys(GroupBy::Color), vec!["#ff0000", "#00ff00", "none"]);
    }

    #[test]
    fn the_bounds_hold_every_point() {
        assert_eq!(bounds_of(std::iter::empty()), None);
        assert_eq!(
            bounds_of([(1.0, 5.0), (-2.0, 3.0), (4.0, -1.0)].into_iter()),
            Some(((-2.0, -1.0), (4.0, 5.0)))
        );
    }

    #[test]
    fn the_limits_and_options_are_checked() {
        assert!(Limits::default().validate().is_ok());
        let bad = Limits {
            max_depth: 0,
            ..Limits::default()
        };
        assert!(bad.validate().is_err());
        assert!(ReadOptions::default().validate().is_ok());
        for o in [
            ReadOptions {
                dpi: 0.0,
                ..ReadOptions::default()
            },
            ReadOptions {
                tolerance_mm: 0.0,
                ..ReadOptions::default()
            },
            ReadOptions {
                dxf_unit_mm: f64::NAN,
                ..ReadOptions::default()
            },
        ] {
            assert!(o.validate().is_err());
        }
        assert_eq!(GroupBy::from_word(" Colour "), Some(GroupBy::Color));
        assert_eq!(GroupBy::from_word("x"), None);
    }
}
