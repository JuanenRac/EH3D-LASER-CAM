//! From a drawing to the G-code of the laser: the shapes are placed (in millimetres, the y axis up, the lower left corner at the origin), grouped by layer or colour, and each group is
//! cut (the contours, inside ones first, with the width of the beam taken into account, tabs left uncut and several passes with the focus moving on) or engraved (closed shapes
//! filled with lines at an angle, open ones followed). The G-code has the same shape as the one of a picture (`M4`, `S0` while the head travels, `M5` at the end) and is checked by
//! [`verify_gcode`] before it is given: nothing that could turn the laser on at no speed, above full power or outside the bed leaves this module.

use crate::gcode_out::number as n;
use crate::offset::offset_ring;
use crate::order::{containers, join, sequence, Contour, OrderOptions, Visit};
use crate::vconfig::{Align, KerfSide, LayerMode, LayerParams, Settings};
use crate::vector::{bounds_of, dist, path_len, Drawing, Pt, Shape, Source};

/// A coordinate farther than this from the origin is a mistake, not work.
pub const MAX_COORDINATE_MM: f64 = 100_000.0;

/// The most notes the plan keeps.
const MAX_NOTES: usize = 60;

/// What the plan did, for the status line.
#[derive(Debug, Clone, PartialEq)]
pub struct Stats {
    pub layers: usize,
    pub contours: usize,
    pub cut_mm: f64,
    pub engrave_mm: f64,
    pub travel_mm: f64,
    pub seconds: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub gcode: String,
    pub stats: Stats,
    /// What was skipped or could not be done, and why.
    pub warnings: Vec<String>,
}

/// One layer or colour of a drawing, for the table of the window and the `layers` command.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerInfo {
    pub key: String,
    pub shapes: usize,
    pub closed: usize,
    pub length_mm: f64,
}

/// The layers (or colours) of a drawing in the order they appear.
pub fn layers(drawing: &Drawing, s: &Settings) -> Vec<LayerInfo> {
    let mut out: Vec<LayerInfo> = Vec::new();
    for shape in &drawing.shapes {
        let key = drawing.key_of(shape, s.group_by);
        let at = match out.iter().position(|l| l.key == key) {
            Some(i) => i,
            None => {
                out.push(LayerInfo {
                    key,
                    shapes: 0,
                    closed: 0,
                    length_mm: 0.0,
                });
                out.len() - 1
            }
        };
        out[at].shapes += 1;
        out[at].closed += usize::from(shape.closed);
        out[at].length_mm += path_len(&shape.pts, shape.closed);
    }
    out
}

fn note(notes: &mut Vec<String>, text: String) {
    if notes.len() < MAX_NOTES && !notes.contains(&text) {
        notes.push(text);
    }
}

/// A name that is safe inside a comment of G-code: no line breaks or other control characters, nothing that is not plain ASCII, and not too long.
pub fn comment_text(text: &str) -> String {
    text.chars()
        .take(60)
        .map(|c| {
            if c.is_ascii() && !c.is_ascii_control() {
                c
            } else {
                '?'
            }
        })
        .collect()
}

// ---------------------------------------------------------------- placing the shapes

/// The shapes of the layers that are not skipped, with the y axis up and placed as the settings say.
fn place(drawing: &Drawing, s: &Settings) -> Vec<(String, Contour)> {
    let svg = drawing.source == Source::Svg;
    let flip = |p: Pt| if svg { (p.0, -p.1) } else { p };
    let used: Vec<&Shape> = drawing
        .shapes
        .iter()
        .filter(|sh| s.params_for(&drawing.key_of(sh, s.group_by)).mode != LayerMode::Skip)
        .collect();
    let (dx, dy) = match (s.align, drawing.page_height_mm, svg) {
        (Align::Page, Some(h), true) => (0.0, h),
        (Align::Page, _, false) => (0.0, 0.0),
        _ => match bounds_of(used.iter().flat_map(|sh| sh.pts.iter().map(|&p| flip(p)))) {
            Some((lo, _)) => (-lo.0, -lo.1),
            None => (0.0, 0.0),
        },
    };
    let (ox, oy) = (s.machine.origin.0 + dx, s.machine.origin.1 + dy);
    used.into_iter()
        .map(|sh| {
            let pts = sh
                .pts
                .iter()
                .map(|&p| {
                    let q = flip(p);
                    (q.0 + ox, q.1 + oy)
                })
                .collect();
            (
                drawing.key_of(sh, s.group_by),
                Contour {
                    pts,
                    closed: sh.closed,
                },
            )
        })
        .collect()
}

// ---------------------------------------------------------------- tabs

/// The point at the length `t` along a path (a closed ring also goes back to its first point) and the index of the segment it is on.
fn at_length(pts: &[Pt], cum: &[f64], t: f64) -> (Pt, usize) {
    let last = pts.len() - 1;
    let mut k = 0;
    while k + 1 < last && cum[k + 1] <= t {
        k += 1;
    }
    let seg = cum[k + 1] - cum[k];
    let f = if seg > 0.0 {
        ((t - cum[k]) / seg).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (
        (
            pts[k].0 + (pts[k + 1].0 - pts[k].0) * f,
            pts[k].1 + (pts[k + 1].1 - pts[k].1) * f,
        ),
        k,
    )
}

/// The part of a path between two lengths, with the ends cut exactly.
fn sub_path(pts: &[Pt], cum: &[f64], t0: f64, t1: f64) -> Vec<Pt> {
    let (a, ka) = at_length(pts, cum, t0);
    let (b, kb) = at_length(pts, cum, t1);
    let mut v = vec![a];
    if ka < kb {
        v.extend_from_slice(&pts[ka + 1..=kb]);
    }
    v.push(b);
    v.dedup_by(|p, q| dist(*p, *q) <= 1e-9);
    v
}

/// The pieces of a closed ring that are cut when `count` tabs of `width` mm are left uncut, evenly along the ring from its first point: tab `i` is centred at `(i + 1) / (count + 1)`
/// of the length. The pieces are cut one after the other, the last one running through the first point. None when the ring is too short for the tabs (they would be more than half of it
/// or touch each other).
pub fn split_by_tabs(ring: &[Pt], count: u32, width: f64) -> Option<Vec<Vec<Pt>>> {
    if count == 0 || !width.is_finite() || width <= 0.0 || ring.len() < 3 {
        return None;
    }
    let mut pts = ring.to_vec();
    pts.push(ring[0]);
    let mut cum = vec![0.0];
    for w in pts.windows(2) {
        cum.push(cum[cum.len() - 1] + dist(w[0], w[1]));
    }
    let total = cum[cum.len() - 1];
    let k = f64::from(count);
    if total <= 0.0 || width >= total / (k + 1.0) || width * k > total / 2.0 {
        return None;
    }
    let centre = |i: u32| total * f64::from(i + 1) / (k + 1.0);
    let mut pieces: Vec<Vec<Pt>> = Vec::new();
    for i in 1..count {
        pieces.push(sub_path(
            &pts,
            &cum,
            centre(i - 1) + width / 2.0,
            centre(i) - width / 2.0,
        ));
    }
    // the last piece runs through the first point of the ring
    let mut wrap = sub_path(&pts, &cum, centre(count - 1) + width / 2.0, total);
    let head = sub_path(&pts, &cum, 0.0, centre(0) - width / 2.0);
    wrap.extend(head.into_iter().skip(1));
    pieces.insert(0, wrap);
    Some(pieces)
}

// ---------------------------------------------------------------- hatch

/// The lines that fill closed shapes: lines `spacing` apart at `angle_deg`, the first half a spacing inside the box of the shapes, cut by the even-odd rule so that a shape inside
/// another is a hole. Each line is a list of runs `(from, to)`, the runs of a line in the direction of the angle. Gives an error when there would be more than `max_lines` lines.
pub fn hatch_lines(
    rings: &[&[Pt]],
    angle_deg: f64,
    spacing: f64,
    max_lines: usize,
) -> Result<Vec<Vec<(Pt, Pt)>>, String> {
    if !(spacing.is_finite() && spacing > 0.0 && angle_deg.is_finite()) {
        return Err("the hatch spacing and angle are not valid".to_string());
    }
    let (sin, cos) = angle_deg.to_radians().sin_cos();
    // the shapes turned by -angle: the lines are then horizontal
    let turned: Vec<Vec<Pt>> = rings
        .iter()
        .filter(|r| r.len() >= 3)
        .map(|r| {
            r.iter()
                .map(|&(x, y)| (x * cos + y * sin, -x * sin + y * cos))
                .collect()
        })
        .collect();
    let Some((lo, hi)) = bounds_of(turned.iter().flat_map(|r| r.iter().copied())) else {
        return Ok(Vec::new());
    };
    let count_f = ((hi.1 - lo.1) / spacing).ceil();
    if !count_f.is_finite() || count_f > max_lines as f64 {
        return Err(format!(
            "the hatch would have more than {max_lines} lines (hatch max_lines)"
        ));
    }
    let count = count_f as usize;
    if count == 0 {
        return Ok(Vec::new());
    }
    let y_of = |k: usize| lo.1 + spacing * (k as f64 + 0.5);
    let mut cross: Vec<Vec<f64>> = vec![Vec::new(); count];
    for ring in &turned {
        let m = ring.len();
        for i in 0..m {
            let (a, b) = (ring[i], ring[(i + 1) % m]);
            if a.1 == b.1 {
                continue;
            }
            let (ya, yb, top, bottom) = if a.1 < b.1 {
                (a.1, b.1, b, a)
            } else {
                (b.1, a.1, a, b)
            };
            // the lines y_k with ya <= y_k < yb, decided with the same numbers the lines are made with
            let mut k0 = (((ya - lo.1) / spacing - 0.5).ceil().max(0.0)) as usize;
            while k0 > 0 && y_of(k0 - 1) >= ya {
                k0 -= 1;
            }
            while k0 < count && y_of(k0) < ya {
                k0 += 1;
            }
            let mut k = k0;
            while k < count && y_of(k) < yb {
                let y = y_of(k);
                let f = (y - bottom.1) / (top.1 - bottom.1);
                cross[k].push(bottom.0 + (top.0 - bottom.0) * f);
                k += 1;
            }
        }
    }
    let mut lines = Vec::new();
    for (k, xs) in cross.iter_mut().enumerate() {
        xs.sort_by(f64::total_cmp);
        let y = y_of(k);
        let back = |x: f64| (x * cos - y * sin, x * sin + y * cos);
        let runs: Vec<(Pt, Pt)> = xs
            .as_chunks::<2>()
            .0
            .iter()
            .filter(|c| c[1] - c[0] > 1e-9)
            .map(|c| (back(c[0]), back(c[1])))
            .collect();
        if !runs.is_empty() {
            lines.push(runs);
        }
    }
    Ok(lines)
}

// ---------------------------------------------------------------- the G-code

struct Out<'a> {
    g: String,
    s: &'a Settings,
    head: Pt,
    travel_mm: f64,
    cut_mm: f64,
    engrave_mm: f64,
    seconds: f64,
}

impl Out<'_> {
    fn check(&self, p: Pt) -> Result<(), String> {
        if !(p.0.is_finite() && p.1.is_finite())
            || p.0.abs() > MAX_COORDINATE_MM
            || p.1.abs() > MAX_COORDINATE_MM
        {
            return Err("a point of the work is out of range".to_string());
        }
        if let Some((bx, by)) = self.s.machine.bed_mm {
            let eps = 1e-6;
            if p.0 < -eps || p.1 < -eps || p.0 > bx + eps || p.1 > by + eps {
                return Err(format!(
                    "the work goes outside of the bed ({} x {} mm) at X{} Y{}",
                    bx,
                    by,
                    n(p.0)?,
                    n(p.1)?
                ));
            }
        }
        Ok(())
    }

    fn line(&mut self, text: &str) {
        self.g.push_str(text);
        self.g.push('\n');
    }

    /// A move with the laser off at the speed of the travel.
    fn travel_to(&mut self, p: Pt) -> Result<(), String> {
        self.check(p)?;
        let d = dist(self.head, p);
        self.line(&format!("G0 X{} Y{} S0", n(p.0)?, n(p.1)?));
        self.travel_mm += d;
        self.seconds += d / (self.s.machine.rapid_mm_min / 60.0);
        self.head = p;
        Ok(())
    }

    /// A move at the feed with the laser off (the run-up of a hatch line).
    fn slow_to(&mut self, p: Pt, speed: f64) -> Result<(), String> {
        self.check(p)?;
        let d = dist(self.head, p);
        self.line(&format!("G1 X{} Y{} S0", n(p.0)?, n(p.1)?));
        self.travel_mm += d;
        self.seconds += d / (speed / 60.0);
        self.head = p;
        Ok(())
    }

    /// A move with the laser on. `power` is written on the line only when it is given (the first move of a path).
    fn burn_to(
        &mut self,
        p: Pt,
        power: Option<u32>,
        speed: f64,
        engraving: bool,
    ) -> Result<(), String> {
        self.check(p)?;
        let d = dist(self.head, p);
        match power {
            Some(sv) => self.line(&format!("G1 X{} Y{} S{sv}", n(p.0)?, n(p.1)?)),
            None => self.line(&format!("G1 X{} Y{}", n(p.0)?, n(p.1)?)),
        }
        if engraving {
            self.engrave_mm += d;
        } else {
            self.cut_mm += d;
        }
        self.seconds += d / (speed / 60.0);
        self.head = p;
        Ok(())
    }

    fn focus(&mut self, z: f64) -> Result<(), String> {
        if !z.is_finite() || z.abs() > 1_000.0 {
            return Err("the focus would go out of range".to_string());
        }
        self.line(&format!("G0 Z{}", n(z)?));
        Ok(())
    }
}

/// The spindle value of a power in percent; a power that rounds to nothing would not be a cut.
fn power_value(p: &LayerParams, s_max: u32) -> Result<u32, String> {
    let v = (f64::from(s_max) * p.power_percent / 100.0).round();
    if v < 1.0 {
        return Err(format!(
            "a power of {} % is too low for s_max {s_max}: it rounds to S0",
            p.power_percent
        ));
    }
    Ok((v as u32).min(s_max))
}

fn order_options(s: &Settings) -> OrderOptions {
    OrderOptions {
        two_opt: s.two_opt,
        budget: s.two_opt_budget,
        two_opt_max_items: s.two_opt_max_items,
    }
}

/// The contours in the order they are cut, with the direction and the starting point of each.
fn visits_for(
    items: &[Contour],
    parents: Option<&[Vec<usize>]>,
    head: Pt,
    s: &Settings,
    notes: &mut Vec<String>,
) -> Vec<Visit> {
    let none: Vec<Vec<usize>> = vec![Vec::new(); items.len()];
    let ps = parents.unwrap_or(none.as_slice());
    if items.len() > s.order_max_items {
        note(notes, format!("there are more than {} contours (order max_items): they are cut inside ones first but not in the shortest order", s.order_max_items));
        let mut idx: Vec<usize> = (0..items.len()).collect();
        idx.sort_by_key(|&i| std::cmp::Reverse(ps[i].len()));
        return idx
            .into_iter()
            .map(|index| Visit {
                index,
                reversed: false,
                start: 0,
            })
            .collect();
    }
    sequence(items, ps, head, &order_options(s))
}

struct Layer<'a> {
    key: &'a str,
    params: LayerParams,
    contours: Vec<Contour>,
}

/// Cuts the contours of a layer.
fn cut_layer(out: &mut Out, layer: &Layer, notes: &mut Vec<String>) -> Result<usize, String> {
    let s = out.s;
    let p = &layer.params;
    let power = power_value(p, s.machine.s_max)?;
    let mut items = join(
        layer.contours.clone(),
        s.join_tolerance_mm,
        s.join_max_paths,
    );
    if p.kerf_mm > 0.0 && p.kerf_side != KerfSide::None {
        let parents = containers(&items, s.containment_max_items);
        if parents.is_none() {
            note(notes, format!("layer {}: too many closed contours to tell holes from pieces: the beam goes to one side only", comment_text(layer.key)));
        }
        let half = p.kerf_mm / 2.0;
        let mut moved: Vec<Contour> = Vec::with_capacity(items.len());
        for (i, c) in items.iter().enumerate() {
            if !c.closed {
                note(notes, format!("layer {}: open paths are cut on the line (the width of the beam is not taken into account)", comment_text(layer.key)));
                moved.push(c.clone());
                continue;
            }
            let d = match p.kerf_side {
                KerfSide::Inside => -half,
                KerfSide::Auto if parents.as_ref().is_some_and(|ps| ps[i].len() % 2 == 1) => -half,
                _ => half,
            };
            match offset_ring(&c.pts, d, s.read.tolerance_mm, s.offset_max_points) {
                Ok(rings) if !rings.is_empty() => moved.extend(rings.into_iter().map(|pts| Contour { pts, closed: true })),
                Ok(_) => note(notes, format!("layer {}: a contour is too small or too narrow for the width of the beam and was not cut", comment_text(layer.key))),
                Err(e) => note(notes, format!("layer {}: a contour was not cut: {e}", comment_text(layer.key))),
            }
        }
        items = moved;
    }
    if items.is_empty() {
        return Ok(0);
    }
    let parents = containers(&items, s.containment_max_items);
    if parents.is_none() && items.iter().filter(|c| c.closed).count() > s.containment_max_items {
        note(
            notes,
            format!(
                "layer {}: too many closed contours to cut inside ones first",
                comment_text(layer.key)
            ),
        );
    }
    let visits = visits_for(&items, parents.as_deref(), out.head, s, notes);
    // the pieces of every contour, with the tabs left out
    let mut pieces: Vec<Vec<Pt>> = Vec::new();
    for v in &visits {
        let pts = v.points(&items);
        let c = &items[v.index];
        let hole = parents
            .as_ref()
            .is_some_and(|ps| ps[v.index].len() % 2 == 1);
        if c.closed && p.tabs > 0 && p.tab_width_mm > 0.0 && !hole {
            let ring = &pts[..pts.len() - 1];
            match split_by_tabs(ring, p.tabs, p.tab_width_mm) {
                Some(parts) => {
                    pieces.extend(parts);
                    continue;
                }
                None => note(
                    notes,
                    format!(
                        "layer {}: a contour is too short for {} tabs of {} mm: it is cut whole",
                        comment_text(layer.key),
                        p.tabs,
                        p.tab_width_mm
                    ),
                ),
            }
        }
        pieces.push(pts);
    }
    for pass in 0..p.passes {
        out.line(&format!("; pass {} of {}", pass + 1, p.passes));
        out.line(&format!("F{}", n(p.speed_mm_min)?));
        if p.focus_step_mm != 0.0 {
            out.focus(s.machine.focus_z_mm + p.focus_step_mm * f64::from(pass))?;
        }
        for piece in &pieces {
            if piece.len() < 2 {
                continue;
            }
            out.travel_to(piece[0])?;
            for (k, &pt) in piece.iter().enumerate().skip(1) {
                out.burn_to(pt, (k == 1).then_some(power), p.speed_mm_min, false)?;
            }
            out.line("S0");
        }
    }
    if p.focus_step_mm != 0.0 {
        out.focus(s.machine.focus_z_mm)?;
    }
    Ok(visits.len())
}

/// Engraves a layer: the closed shapes are filled with lines and the open ones are followed.
fn engrave_layer(out: &mut Out, layer: &Layer, notes: &mut Vec<String>) -> Result<usize, String> {
    let s = out.s;
    let p = &layer.params;
    let power = power_value(p, s.machine.s_max)?;
    let items = join(
        layer.contours.clone(),
        s.join_tolerance_mm,
        s.join_max_paths,
    );
    let rings: Vec<&[Pt]> = items
        .iter()
        .filter(|c| c.closed)
        .map(|c| c.pts.as_slice())
        .collect();
    let hatch = hatch_lines(
        &rings,
        p.hatch_angle_deg,
        p.hatch_spacing_mm,
        s.max_hatch_lines,
    )?;
    let lines: Vec<Contour> = items.iter().filter(|c| !c.closed).cloned().collect();
    let visits = visits_for(&lines, None, out.head, s, notes);
    let (sin, cos) = p.hatch_angle_deg.to_radians().sin_cos();
    let over = s.machine.overscan_mm;
    for pass in 0..p.passes {
        out.line(&format!("; pass {} of {}", pass + 1, p.passes));
        out.line(&format!("F{}", n(p.speed_mm_min)?));
        if p.focus_step_mm != 0.0 {
            out.focus(s.machine.focus_z_mm + p.focus_step_mm * f64::from(pass))?;
        }
        for (k, runs) in hatch.iter().enumerate() {
            let backwards = k % 2 == 1;
            let ordered: Vec<(Pt, Pt)> = if backwards {
                runs.iter().rev().map(|&(a, b)| (b, a)).collect()
            } else {
                runs.clone()
            };
            let dir = if backwards { (-cos, -sin) } else { (cos, sin) };
            let first = ordered[0].0;
            let approach = (first.0 - dir.0 * over, first.1 - dir.1 * over);
            out.travel_to(approach)?;
            if over > 0.0 {
                out.slow_to(first, p.speed_mm_min)?;
            }
            let mut here = first;
            for &(from, to) in &ordered {
                if dist(here, from) > 1e-9 {
                    out.slow_to(from, p.speed_mm_min)?;
                }
                out.burn_to(to, Some(power), p.speed_mm_min, true)?;
                here = to;
            }
            if over > 0.0 {
                out.slow_to(
                    (here.0 + dir.0 * over, here.1 + dir.1 * over),
                    p.speed_mm_min,
                )?;
            }
        }
        for v in &visits {
            let pts = v.points(&lines);
            if pts.len() < 2 {
                continue;
            }
            out.travel_to(pts[0])?;
            for (k, &pt) in pts.iter().enumerate().skip(1) {
                out.burn_to(pt, (k == 1).then_some(power), p.speed_mm_min, true)?;
            }
            out.line("S0");
        }
    }
    if p.focus_step_mm != 0.0 {
        out.focus(s.machine.focus_z_mm)?;
    }
    Ok(rings.len() + visits.len())
}

/// Works out the G-code that cuts and engraves a drawing (without the header of the file, like the engraving of a picture). The layers that are engraved come before the ones that are
/// cut, and the ones of each kind by their `order`. Everything the plan could not do is in `warnings`; a value that is not valid, a hatch that is too big or work that goes outside of
/// the bed is an error.
pub fn plan(drawing: &Drawing, s: &Settings) -> Result<Plan, String> {
    s.validate()?;
    let mut notes: Vec<String> = drawing.warnings.clone();
    let placed = place(drawing, s);
    let mut groups: Vec<(String, Vec<Contour>)> = Vec::new();
    for (key, c) in placed {
        match groups.iter_mut().find(|(k, _)| *k == key) {
            Some((_, v)) => v.push(c),
            None => groups.push((key, vec![c])),
        }
    }
    if groups.is_empty() {
        return Err(
            "there is nothing to cut or engrave: every layer is skipped or the drawing is empty"
                .to_string(),
        );
    }
    let mut work: Vec<Layer> = groups
        .iter()
        .map(|(key, contours)| Layer {
            key,
            params: s.params_for(key),
            contours: contours.clone(),
        })
        .collect();
    work.sort_by(|a, b| {
        let rank = |l: &Layer| usize::from(l.params.mode == LayerMode::Cut);
        (rank(a), a.params.order, a.key).cmp(&(rank(b), b.params.order, b.key))
    });
    let mut out = Out {
        g: String::from("G21\nG90\nM4 S0\n"),
        s,
        head: s.machine.origin,
        travel_mm: 0.0,
        cut_mm: 0.0,
        engrave_mm: 0.0,
        seconds: 0.0,
    };
    let mut contours = 0;
    for layer in &work {
        let p = &layer.params;
        out.line(&format!(
            "; layer {}: {}, {} %, {} mm/min, {} pass(es)",
            comment_text(layer.key),
            p.mode.word(),
            n(p.power_percent)?,
            n(p.speed_mm_min)?,
            p.passes
        ));
        contours += match p.mode {
            LayerMode::Cut => cut_layer(&mut out, layer, &mut notes)?,
            LayerMode::Engrave => engrave_layer(&mut out, layer, &mut notes)?,
            LayerMode::Skip => 0,
        };
    }
    out.line("M5");
    out.line("M2");
    verify_gcode(&out.g, s.machine.s_max, s.machine.bed_mm)?;
    Ok(Plan {
        gcode: out.g,
        stats: Stats {
            layers: work.len(),
            contours,
            cut_mm: out.cut_mm,
            engrave_mm: out.engrave_mm,
            travel_mm: out.travel_mm,
            seconds: out.seconds,
        },
        warnings: notes,
    })
}

/// Checks a G-code program of the laser the way the program itself writes it. It is an error when:
/// - a word is not one the program writes (`G0`, `G1`, `G21`, `G90`, `M4`, `M5`, `M2`, `X`, `Y`, `Z`, `F`, `S`), or a number is not finite;
/// - `S` is more than `s_max` or less than 0;
/// - the laser is on (`S` more than 0) during a `G0` move or a move of `Z`;
/// - a `G1` move happens with the laser on and no speed (`F` more than 0) set;
/// - a move goes outside of the bed, when its size is given;
/// - the program does not end with `M5` and `M2`.
pub fn verify_gcode(gcode: &str, s_max: u32, bed: Option<Pt>) -> Result<(), String> {
    let (mut s, mut f) = (0.0_f64, 0.0_f64);
    let mut mode: Option<u8> = None;
    let mut last: Vec<String> = Vec::new();
    for (number, raw) in gcode.lines().enumerate() {
        let line = raw.split(';').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let at = number + 1;
        let (mut x, mut y, mut z) = (None, None, None);
        let mut words: Vec<String> = Vec::new();
        for token in line.split_whitespace() {
            let mut chars = token.chars();
            let letter = chars.next().unwrap_or(' ').to_ascii_uppercase();
            let v: f64 = chars
                .as_str()
                .parse()
                .map_err(|_| format!("line {at}: '{token}' is not a word of the program"))?;
            if !v.is_finite() {
                return Err(format!("line {at}: '{token}' is not a finite number"));
            }
            let code = if v.fract() == 0.0 && v.abs() < 1000.0 {
                v as i64
            } else {
                -1
            };
            match (letter, code) {
                ('G', 0) => mode = Some(0),
                ('G', 1) => mode = Some(1),
                ('G', 21) | ('G', 90) => {}
                ('M', 4) | ('M', 5) | ('M', 2) => words.push(token.to_ascii_uppercase()),
                ('X', _) => x = Some(v),
                ('Y', _) => y = Some(v),
                ('Z', _) => z = Some(v),
                ('F', _) => f = v,
                ('S', _) => {
                    if !(0.0..=f64::from(s_max)).contains(&v) {
                        return Err(format!("line {at}: S{v} is outside of 0 to {s_max}"));
                    }
                    s = v;
                }
                _ => {
                    return Err(format!(
                        "line {at}: '{token}' is not a word the program writes"
                    ))
                }
            }
        }
        if x.is_some() || y.is_some() || z.is_some() {
            match mode {
                Some(0) if s > 0.0 => {
                    return Err(format!("line {at}: a rapid move with the laser on"))
                }
                Some(1) if s > 0.0 && f <= 0.0 => {
                    return Err(format!("line {at}: a cut with no speed set"))
                }
                Some(_) => {}
                None => return Err(format!("line {at}: a move before G0 or G1")),
            }
            if z.is_some() && s > 0.0 {
                return Err(format!("line {at}: the focus moves with the laser on"));
            }
            if let Some((bx, by)) = bed {
                let out =
                    |v: Option<f64>, hi: f64| v.is_some_and(|v| !(-1e-6..=hi + 1e-6).contains(&v));
                if out(x, bx) || out(y, by) {
                    return Err(format!("line {at}: a move outside of the bed"));
                }
            }
        }
        last.extend(words);
    }
    let end: Vec<&str> = last.iter().rev().take(2).map(String::as_str).collect();
    if end != ["M2", "M5"] {
        return Err("the program does not end with M5 and M2".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vconfig::parse;

    fn shape(layer: &str, pts: &[Pt], closed: bool) -> Shape {
        Shape {
            layer: layer.to_string(),
            color: None,
            pts: pts.to_vec(),
            closed,
        }
    }

    fn square(x: f64, y: f64, side: f64) -> Vec<Pt> {
        vec![(x, y), (x + side, y), (x + side, y + side), (x, y + side)]
    }

    fn dxf(shapes: Vec<Shape>) -> Drawing {
        Drawing {
            source: Source::Dxf,
            shapes,
            warnings: Vec::new(),
            page_height_mm: None,
        }
    }

    /// The lines of the program that are not comments.
    fn code(g: &str) -> Vec<&str> {
        g.lines().filter(|l| !l.starts_with(';')).collect()
    }

    fn settings(text: &str) -> Settings {
        parse(text).unwrap()
    }

    /// The x and y of every move of a program.
    fn moves(g: &str) -> Vec<(char, Pt)> {
        let (mut x, mut y) = (0.0, 0.0);
        let mut out = Vec::new();
        let mut mode = '0';
        for l in code(g) {
            let mut moved = false;
            for t in l.split_whitespace() {
                match t.as_bytes()[0] {
                    b'G' if t == "G0" => mode = '0',
                    b'G' if t == "G1" => mode = '1',
                    b'X' => {
                        x = t[1..].parse().unwrap();
                        moved = true;
                    }
                    b'Y' => {
                        y = t[1..].parse().unwrap();
                        moved = true;
                    }
                    _ => {}
                }
            }
            if moved {
                out.push((mode, (x, y)));
            }
        }
        out
    }

    #[test]
    fn a_square_is_cut_in_one_loop_with_the_laser_off_while_it_travels() {
        let d = dxf(vec![shape("l", &square(0.0, 0.0, 10.0), true)]);
        let plan = plan(&d, &settings("[defaults]\npower = 50\nspeed = 600\n")).unwrap();
        assert_eq!(
            code(&plan.gcode),
            vec![
                "G21",
                "G90",
                "M4 S0",
                "F600",
                "G0 X0 Y0 S0",
                "G1 X10 Y0 S500",
                "G1 X10 Y10",
                "G1 X0 Y10",
                "G1 X0 Y0",
                "S0",
                "M5",
                "M2"
            ]
        );
        // 40 mm of cut at 600 mm/min (10 mm/s) is 4 s, and the head was already at the start
        assert_eq!(plan.stats.cut_mm, 40.0);
        assert_eq!(plan.stats.travel_mm, 0.0);
        assert_eq!(plan.stats.seconds, 4.0);
        assert_eq!((plan.stats.layers, plan.stats.contours), (1, 1));
        assert!(plan.warnings.is_empty());
    }

    #[test]
    fn the_drawing_is_moved_to_the_origin_and_the_svg_is_turned_upside_down() {
        // an SVG has y down: the point (5, 3) of the page is above (5, 7) once the page is turned over; the lower left corner of the work is the origin
        let svg = Drawing {
            source: Source::Svg,
            shapes: vec![shape("l", &[(5.0, 3.0), (5.0, 7.0)], false)],
            warnings: Vec::new(),
            page_height_mm: Some(20.0),
        };
        let p = plan(&svg, &settings("")).unwrap();
        // flipped: (5,-3) and (5,-7); the box starts at y = -7, so they are (0,4) and (0,0); the path is cut from the nearer end
        assert_eq!(moves(&p.gcode), vec![('0', (0.0, 0.0)), ('1', (0.0, 4.0))]);
        // with the page kept, the page height is the origin: y = 20 - 3 = 17 and 20 - 7 = 13, x as it is
        let q = plan(
            &svg,
            &settings("[read]\nalign = page\n[machine]\norigin_x = 1\n"),
        )
        .unwrap();
        assert_eq!(
            moves(&q.gcode),
            vec![('0', (6.0, 13.0)), ('1', (6.0, 17.0))]
        );
    }

    #[test]
    fn several_passes_move_the_focus_and_put_it_back() {
        let d = dxf(vec![shape("l", &square(0.0, 0.0, 10.0), true)]);
        let p = plan(
            &d,
            &settings("[machine]\nfocus_z_mm = 2\n[defaults]\npasses = 2\nfocus_step = -0.5\n"),
        )
        .unwrap();
        let c = code(&p.gcode);
        let z: Vec<&&str> = c.iter().filter(|l| l.contains('Z')).collect();
        assert_eq!(z, vec![&"G0 Z2", &"G0 Z1.5", &"G0 Z2"]);
        // the contour is cut twice
        assert_eq!(c.iter().filter(|l| l.starts_with("G1 X10 Y0")).count(), 2);
        assert_eq!(p.stats.cut_mm, 80.0);
        // the Z moves come before the cut of their pass
        let first_z = c.iter().position(|l| *l == "G0 Z2").unwrap();
        let first_cut = c.iter().position(|l| l.starts_with("G1")).unwrap();
        assert!(first_z < first_cut);
    }

    #[test]
    fn a_tab_leaves_the_middle_of_the_outline_uncut() {
        // the ring has 40 mm; one tab of 2 mm is centred 20 mm along: from (10,9) to (9,10), so the cut starts at (9,10) and ends at (10,9)
        let ring = square(0.0, 0.0, 10.0);
        let pieces = split_by_tabs(&ring, 1, 2.0).unwrap();
        assert_eq!(pieces.len(), 1);
        assert_eq!(
            pieces[0],
            vec![
                (9.0, 10.0),
                (0.0, 10.0),
                (0.0, 0.0),
                (10.0, 0.0),
                (10.0, 9.0)
            ]
        );
        let d = dxf(vec![shape("l", &ring, true)]);
        let p = plan(&d, &settings("[defaults]\ntabs = 1\ntab_width = 2\n")).unwrap();
        assert_eq!(
            code(&p.gcode)[4..],
            [
                "G0 X9 Y10 S0",
                "G1 X0 Y10 S500",
                "G1 X0 Y0",
                "G1 X10 Y0",
                "G1 X10 Y9",
                "S0",
                "M5",
                "M2"
            ]
        );
        assert_eq!(p.stats.cut_mm, 38.0);
    }

    #[test]
    fn two_tabs_make_two_pieces_and_a_ring_too_short_for_them_is_cut_whole() {
        let ring = square(0.0, 0.0, 10.0);
        // tabs at 13.33 and 26.67 along the 40 mm, 2 mm wide: the pieces are from 14.33 to 25.67 and from 27.67 round to 12.33
        let pieces = split_by_tabs(&ring, 2, 2.0).unwrap();
        assert_eq!(pieces.len(), 2);
        let len = |p: &Vec<Pt>| path_len(p, false);
        // the piece that runs through the first point: 12.33 + 12.33; the other: 25.67 - 14.33
        assert!(
            (len(&pieces[0]) - 74.0 / 3.0).abs() < 1e-9,
            "{}",
            len(&pieces[0])
        );
        assert!(
            (len(&pieces[1]) - 34.0 / 3.0).abs() < 1e-9,
            "{}",
            len(&pieces[1])
        );
        let total: f64 = pieces.iter().map(len).sum();
        assert!((total - 36.0).abs() < 1e-9, "{total}");
        assert!(
            split_by_tabs(&ring, 1, 30.0).is_none(),
            "the tab is longer than the ring"
        );
        assert!(
            split_by_tabs(&ring, 4, 6.0).is_none(),
            "the tabs would be more than half of the ring"
        );
        assert!(split_by_tabs(&ring, 0, 1.0).is_none());
        assert!(split_by_tabs(&ring, 1, 0.0).is_none());
        let d = dxf(vec![shape("l", &ring, true)]);
        let p = plan(&d, &settings("[defaults]\ntabs = 4\ntab_width = 6\n")).unwrap();
        assert!(p.warnings.iter().any(|w| w.contains("too short")));
        assert_eq!(p.stats.cut_mm, 40.0);
    }

    #[test]
    fn only_the_outline_of_a_piece_has_tabs_not_its_holes() {
        let d = dxf(vec![
            shape("l", &square(0.0, 0.0, 20.0), true),
            shape("l", &square(8.0, 8.0, 4.0), true),
        ]);
        let p = plan(&d, &settings("[defaults]\ntabs = 2\ntab_width = 2\n")).unwrap();
        // the hole is cut whole (16 mm) and the outline loses 2 tabs of 2 mm from its 80
        assert!(
            (p.stats.cut_mm - (16.0 + 76.0)).abs() < 1e-9,
            "{}",
            p.stats.cut_mm
        );
    }

    #[test]
    fn the_width_of_the_beam_goes_outside_a_piece_and_inside_a_hole() {
        let d = dxf(vec![
            shape("l", &square(0.0, 0.0, 10.0), true),
            shape("l", &square(4.0, 4.0, 2.0), true),
        ]);
        let p = plan(&d, &settings("[defaults]\nkerf = 0.2\nkerf_side = auto\n")).unwrap();
        let m = moves(&p.gcode);
        // the hole is cut first, with the beam 0.1 inside the hole: a square from 4.1 to 5.9
        let hole: Vec<Pt> = m
            .iter()
            .map(|x| x.1)
            .take_while(|p| p.0 > 3.0 && p.0 < 7.0)
            .collect();
        assert_eq!(hole.len(), 5);
        assert!(hole
            .iter()
            .all(|p| (p.0 - 4.1).abs() < 1e-9 || (p.0 - 5.9).abs() < 1e-9));
        // the outline is 0.1 outside the piece
        let xs: Vec<f64> = m.iter().map(|x| x.1 .0).skip(hole.len()).collect();
        let lo = xs.iter().cloned().fold(f64::INFINITY, f64::min);
        let hi = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        assert!(
            (lo + 0.1).abs() < 1e-9 && (hi - 10.1).abs() < 1e-9,
            "{lo} {hi}"
        );
        // the same piece with the beam always inside: the outline is at 0.1 and 9.9
        let q = plan(
            &dxf(vec![shape("l", &square(0.0, 0.0, 10.0), true)]),
            &settings("[defaults]\nkerf = 0.2\nkerf_side = inside\n"),
        )
        .unwrap();
        let xs: Vec<f64> = moves(&q.gcode).iter().map(|x| x.1 .0).collect();
        assert!((xs.iter().cloned().fold(f64::INFINITY, f64::min) - 0.1).abs() < 1e-9);
        assert!((xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max) - 9.9).abs() < 1e-9);
    }

    #[test]
    fn a_hole_smaller_than_the_beam_is_left_out_with_a_note() {
        let d = dxf(vec![
            shape("l", &square(0.0, 0.0, 10.0), true),
            shape("l", &square(4.0, 4.0, 0.1), true),
        ]);
        let p = plan(&d, &settings("[defaults]\nkerf = 0.4\nkerf_side = auto\n")).unwrap();
        assert!(p
            .warnings
            .iter()
            .any(|w| w.contains("too small or too narrow")));
        // only the outline is cut
        assert_eq!(p.stats.contours, 1);
    }

    #[test]
    fn open_paths_are_cut_on_the_line_with_a_note_about_the_beam() {
        let d = dxf(vec![shape("l", &[(0.0, 0.0), (10.0, 0.0)], false)]);
        let p = plan(
            &d,
            &settings("[defaults]\nkerf = 0.4\nkerf_side = outside\n"),
        )
        .unwrap();
        assert!(p.warnings.iter().any(|w| w.contains("open paths")));
        assert_eq!(p.stats.cut_mm, 10.0);
    }

    #[test]
    fn the_hole_is_cut_before_the_piece_even_when_the_piece_is_nearer() {
        let d = dxf(vec![
            shape("l", &square(0.0, 0.0, 20.0), true),
            shape("l", &square(8.0, 8.0, 4.0), true),
        ]);
        let p = plan(&d, &settings("")).unwrap();
        let m = moves(&p.gcode);
        // the first travel is to the hole
        assert_eq!(m[0], ('0', (8.0, 8.0)));
        // and the outline starts at its corner nearest to where the hole ended, (8,8): the corner (0,0)
        let second = m.iter().find(|x| x.0 == '0' && x.1 != (8.0, 8.0)).unwrap();
        assert_eq!(second.1, (0.0, 0.0));
    }

    #[test]
    fn layers_are_done_engraving_first_then_cutting_by_their_order() {
        let d = dxf(vec![
            shape("cutA", &square(0.0, 0.0, 5.0), true),
            shape("etch", &square(10.0, 0.0, 5.0), true),
            shape("cutB", &square(20.0, 0.0, 5.0), true),
        ]);
        let s = settings("[layer etch]\nmode = engrave\nhatch_spacing = 2.5\n[layer cutA]\norder = 5\n[layer cutB]\norder = 1\n");
        let p = plan(&d, &s).unwrap();
        let names: Vec<&str> = p
            .gcode
            .lines()
            .filter(|l| l.starts_with("; layer"))
            .map(|l| l.split(':').next().unwrap())
            .collect();
        assert_eq!(names, vec!["; layer etch", "; layer cutb", "; layer cuta"]);
    }

    #[test]
    fn a_skipped_layer_is_not_in_the_program_and_does_not_move_the_origin() {
        let d = dxf(vec![
            shape("guide", &[(-50.0, -50.0), (-40.0, -50.0)], false),
            shape("part", &square(100.0, 100.0, 10.0), true),
        ]);
        let p = plan(&d, &settings("[layer guide]\nmode = skip\n")).unwrap();
        assert_eq!(moves(&p.gcode)[0], ('0', (0.0, 0.0)));
        assert!(!p.gcode.contains("guide"));
        // all layers skipped is an error
        assert!(plan(&d, &settings("[defaults]\nmode = skip\n")).is_err());
        assert!(plan(&dxf(Vec::new()), &settings("")).is_err());
    }

    #[test]
    fn the_hatch_fills_a_square_with_lines_that_go_one_way_and_the_other() {
        let sq = square(0.0, 0.0, 10.0);
        let lines = hatch_lines(&[sq.as_slice()], 0.0, 2.5, 1000).unwrap();
        // 10 / 2.5 = 4 lines, the first half a spacing in
        assert_eq!(lines.len(), 4);
        for (k, l) in lines.iter().enumerate() {
            assert_eq!(l.len(), 1);
            let y = 1.25 + 2.5 * k as f64;
            assert_eq!(l[0], ((0.0, y), (10.0, y)));
        }
        let d = dxf(vec![shape("e", &sq, true)]);
        let p = plan(&d, &settings("[machine]\noverscan_mm = 0\n[defaults]\nmode = engrave\nhatch_spacing = 2.5\npower = 50\n")).unwrap();
        assert_eq!(
            &code(&p.gcode)[3..],
            [
                "F600",
                "G0 X0 Y1.25 S0",
                "G1 X10 Y1.25 S500",
                "G0 X10 Y3.75 S0",
                "G1 X0 Y3.75 S500",
                "G0 X0 Y6.25 S0",
                "G1 X10 Y6.25 S500",
                "G0 X10 Y8.75 S0",
                "G1 X0 Y8.75 S500",
                "M5",
                "M2"
            ]
        );
        assert_eq!(p.stats.engrave_mm, 40.0);
    }

    #[test]
    fn the_overscan_runs_the_head_past_each_end_of_a_hatch_line() {
        let d = dxf(vec![shape("e", &square(0.0, 0.0, 10.0), true)]);
        let p = plan(
            &d,
            &settings(
                "[machine]\noverscan_mm = 2\n[defaults]\nmode = engrave\nhatch_spacing = 5\n",
            ),
        )
        .unwrap();
        assert_eq!(
            &code(&p.gcode)[4..],
            [
                "G0 X-2 Y2.5 S0",
                "G1 X0 Y2.5 S0",
                "G1 X10 Y2.5 S500",
                "G1 X12 Y2.5 S0",
                "G0 X12 Y7.5 S0",
                "G1 X10 Y7.5 S0",
                "G1 X0 Y7.5 S500",
                "G1 X-2 Y7.5 S0",
                "M5",
                "M2"
            ]
        );
    }

    #[test]
    fn a_shape_inside_another_is_a_hole_of_the_hatch() {
        let outer = square(0.0, 0.0, 10.0);
        let inner = square(4.0, 4.0, 2.0);
        let lines = hatch_lines(&[outer.as_slice(), inner.as_slice()], 0.0, 1.0, 1000).unwrap();
        assert_eq!(lines.len(), 10);
        // the lines at y = 0.5 .. 9.5; those at 4.5 and 5.5 are cut by the hole
        assert_eq!(
            lines[4],
            vec![((0.0, 4.5), (4.0, 4.5)), ((6.0, 4.5), (10.0, 4.5))]
        );
        assert_eq!(lines[5].len(), 2);
        assert_eq!(lines[3], vec![((0.0, 3.5), (10.0, 3.5))]);
    }

    #[test]
    fn a_hatch_at_90_degrees_runs_up_the_page() {
        let sq = square(0.0, 0.0, 10.0);
        let lines = hatch_lines(&[sq.as_slice()], 90.0, 2.5, 1000).unwrap();
        assert_eq!(lines.len(), 4);
        // the lines are vertical at x = 8.75, 6.25, 3.75, 1.25 (the first is the one with the lowest y in the turned frame) and each runs from y = 0 up to y = 10
        let xs: Vec<f64> = lines.iter().map(|l| l[0].0 .0).collect();
        for (got, want) in xs.iter().zip([8.75, 6.25, 3.75, 1.25]) {
            assert!((got - want).abs() < 1e-9, "{xs:?}");
        }
        for l in &lines {
            assert!((l[0].0 .1 - 0.0).abs() < 1e-9 && (l[0].1 .1 - 10.0).abs() < 1e-9);
        }
    }

    #[test]
    fn a_hatch_at_45_degrees_has_the_right_total_length() {
        // a square of 10 filled with lines 1 apart at 45 degrees: the area is 100, so the total length is about 100
        let sq = square(0.0, 0.0, 10.0);
        let lines = hatch_lines(&[sq.as_slice()], 45.0, 1.0, 10_000).unwrap();
        let total: f64 = lines.iter().flatten().map(|(a, b)| dist(*a, *b)).sum();
        assert!((total - 100.0).abs() < 1.0, "{total}");
        // the lines are all at 45 degrees
        for (a, b) in lines.iter().flatten() {
            assert!(((b.1 - a.1) - (b.0 - a.0)).abs() < 1e-9);
        }
    }

    #[test]
    fn a_hatch_with_too_many_lines_is_an_error() {
        let sq = square(0.0, 0.0, 10.0);
        assert!(hatch_lines(&[sq.as_slice()], 0.0, 0.01, 100).is_err());
        assert!(hatch_lines(&[sq.as_slice()], 0.0, 0.0, 100).is_err());
        assert!(hatch_lines(&[sq.as_slice()], f64::NAN, 1.0, 100).is_err());
        assert!(hatch_lines(&[], 0.0, 1.0, 100).unwrap().is_empty());
        // open shapes (two points) have no area
        let line = [(0.0, 0.0), (5.0, 5.0)];
        assert!(hatch_lines(&[line.as_slice()], 0.0, 1.0, 100)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn open_paths_of_an_engraved_layer_are_followed() {
        let d = dxf(vec![shape("e", &[(0.0, 0.0), (10.0, 0.0)], false)]);
        let p = plan(&d, &settings("[defaults]\nmode = engrave\n")).unwrap();
        assert_eq!(
            &code(&p.gcode)[3..],
            ["F600", "G0 X0 Y0 S0", "G1 X10 Y0 S500", "S0", "M5", "M2"]
        );
        assert_eq!(p.stats.engrave_mm, 10.0);
    }

    #[test]
    fn loose_lines_that_meet_are_cut_as_one_contour() {
        let d = dxf(vec![
            shape("l", &[(0.0, 0.0), (10.0, 0.0)], false),
            shape("l", &[(10.0, 0.0), (10.0, 10.0)], false),
            shape("l", &[(10.0, 10.0), (0.0, 10.0)], false),
            shape("l", &[(0.0, 10.0), (0.0, 0.0)], false),
        ]);
        let p = plan(&d, &settings("")).unwrap();
        assert_eq!(p.stats.contours, 1);
        // one start, one end of the cut: a single S0 line after the loop
        assert_eq!(
            code(&p.gcode)
                .iter()
                .filter(|l| l.starts_with("G0 X"))
                .count(),
            1
        );
    }

    #[test]
    fn a_power_that_rounds_to_nothing_or_a_bad_value_is_an_error() {
        let d = dxf(vec![shape("l", &square(0.0, 0.0, 10.0), true)]);
        let e = plan(
            &d,
            &settings("[machine]\ns_max = 100\n[defaults]\npower = 0.1\n"),
        )
        .unwrap_err();
        assert!(e.contains("too low"), "{e}");
        let mut s = Settings::default();
        s.defaults.speed_mm_min = 0.0;
        assert!(plan(&d, &s).is_err());
        s.defaults.speed_mm_min = 100.0;
        s.machine.s_max = 0;
        assert!(plan(&d, &s).is_err());
    }

    #[test]
    fn full_power_is_s_max_and_never_more() {
        let d = dxf(vec![shape("l", &square(0.0, 0.0, 10.0), true)]);
        let p = plan(
            &d,
            &settings("[machine]\ns_max = 255\n[defaults]\npower = 100\n"),
        )
        .unwrap();
        assert!(p.gcode.contains("S255"));
        assert!(!p.gcode.contains("S256"));
    }

    #[test]
    fn work_that_goes_outside_of_the_bed_is_an_error() {
        let d = dxf(vec![shape("l", &square(0.0, 0.0, 10.0), true)]);
        assert!(plan(&d, &settings("[machine]\nbed_x_mm = 100\nbed_y_mm = 100\n")).is_ok());
        let e = plan(&d, &settings("[machine]\nbed_x_mm = 9\nbed_y_mm = 100\n")).unwrap_err();
        assert!(e.contains("bed"), "{e}");
        // an origin that pushes it outside
        assert!(plan(
            &d,
            &settings("[machine]\norigin_x = 95\nbed_x_mm = 100\nbed_y_mm = 100\n")
        )
        .is_err());
        // the overscan of the hatch counts
        let e = plan(&d, &settings("[machine]\nbed_x_mm = 10\nbed_y_mm = 10\noverscan_mm = 2\n[defaults]\nmode = engrave\n")).unwrap_err();
        assert!(e.contains("bed"), "{e}");
    }

    #[test]
    fn a_name_cannot_add_a_line_of_g_code() {
        let d = dxf(vec![shape(
            "a\nM3 S1000\r\n(x);é",
            &square(0.0, 0.0, 10.0),
            true,
        )]);
        let p = plan(&d, &settings("")).unwrap();
        for l in p.gcode.lines() {
            assert!(!l.starts_with("M3"), "{l}");
        }
        assert!(p.gcode.contains("; layer a?m3 s1000??(x);?"), "{}", p.gcode);
        assert_eq!(comment_text("é\n"), "??");
        assert_eq!(comment_text(&"x".repeat(100)).len(), 60);
    }

    #[test]
    fn what_the_program_writes_passes_its_own_check() {
        let mut shapes = vec![
            shape("l", &square(0.0, 0.0, 20.0), true),
            shape("l", &square(8.0, 8.0, 4.0), true),
            shape("e", &square(30.0, 0.0, 10.0), true),
        ];
        shapes.push(shape("l", &[(50.0, 0.0), (60.0, 5.0)], false));
        let d = dxf(shapes);
        let p = plan(&d, &settings("[layer e]\nmode = engrave\n[defaults]\ntabs = 2\ntab_width = 1\npasses = 2\nfocus_step = 0.3\nkerf = 0.1\nkerf_side = auto\n")).unwrap();
        verify_gcode(&p.gcode, 1000, None).unwrap();
    }

    #[test]
    fn the_check_refuses_what_could_hurt() {
        let ok = "G21\nG90\nM4 S0\nF600\nG0 X0 Y0 S0\nG1 X10 Y0 S500\nS0\nM5\nM2\n";
        verify_gcode(ok, 1000, None).unwrap();
        let bad = [
            (
                "G21\nM4 S0\nG0 X1 Y1 S0\nG1 X5 Y5 S500\nS0\nM5\nM2\n",
                "no speed",
            ),
            (
                "G21\nM4 S0\nF600\nG1 X1 Y1 S1500\nS0\nM5\nM2\n",
                "outside of 0",
            ),
            (
                "G21\nM4 S0\nF600\nG1 X1 Y1 S-1\nS0\nM5\nM2\n",
                "outside of 0",
            ),
            (
                "G21\nM4 S0\nF600\nG1 X1 Y1 S500\nG0 X5 Y5\nS0\nM5\nM2\n",
                "rapid",
            ),
            (
                "G21\nM4 S0\nF600\nG1 X1 Y1 S500\nG0 Z5\nS0\nM5\nM2\n",
                "rapid",
            ),
            (
                "G21\nM4 S0\nF600\nG1 X1 Y1 S500\nG1 Z5\nS0\nM5\nM2\n",
                "focus",
            ),
            (
                "G21\nM4 S0\nF600\nG1 X1 Y1 S500\nM5\nM2\nM3 S100\n",
                "not a word",
            ),
            ("G21\nM3 S100\nM5\nM2\n", "not a word"),
            ("G21\nM4 S0\nF600\nG1 XNaN Y1 S500\nM5\nM2\n", "finite"),
            ("G21\nM4 S0\nF600\nG1 Xinf Y1 S500\nM5\nM2\n", "finite"),
            ("G21\nM4 S0\nF600\nG1 X1 Y1 S500\nS0\n", "does not end"),
            (
                "G21\nM4 S0\nF600\nG1 X1 Y1 S500\nS0\nM2\nM5\n",
                "does not end",
            ),
            ("G21\nM4 S0\nF600\nG1 X1 Y1 S500\nS0\nM5\n", "does not end"),
            ("X1 Y1\nM5\nM2\n", "before G0 or G1"),
            ("G21\nG2 X1 Y1\nM5\nM2\n", "not a word"),
            ("G21\nM4 S0\nF0\nG1 X1 Y1 S500\nS0\nM5\nM2\n", "no speed"),
        ];
        for (text, needle) in bad {
            let e = verify_gcode(text, 1000, None).expect_err(text);
            assert!(e.contains(needle), "{text:?} gave {e:?}");
        }
        assert!(verify_gcode(ok, 1000, Some((5.0, 5.0)))
            .unwrap_err()
            .contains("bed"));
        verify_gcode(ok, 1000, Some((10.0, 5.0))).unwrap();
        // a comment can hold anything
        verify_gcode("G21\n; M3 S9999 G1 X1\nM4 S0\nM5\nM2\n", 1000, None).unwrap();
    }

    #[test]
    fn a_damaged_drawing_never_panics() {
        let mut seed = 21u64;
        let mut next = move || {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((seed >> 33) % 3000) as f64 / 100.0 - 5.0
        };
        let s = settings("[defaults]\nkerf = 0.3\nkerf_side = auto\ntabs = 2\ntab_width = 0.5\n[layer b]\nmode = engrave\nhatch_spacing = 0.7\nhatch_angle = 33\n");
        for round in 0..60 {
            let mut shapes = Vec::new();
            for k in 0..(1 + round % 6) {
                let n = 2 + (next().abs() as usize) % 7;
                let pts: Vec<Pt> = (0..n).map(|_| (next(), next())).collect();
                shapes.push(shape(if k % 2 == 1 { "b" } else { "a" }, &pts, k % 3 > 0));
            }
            if let Ok(p) = plan(&dxf(shapes), &s) {
                verify_gcode(&p.gcode, 1000, None).unwrap();
            }
        }
    }

    #[test]
    fn the_layers_of_a_drawing_are_listed_with_their_sizes() {
        let d = dxf(vec![
            shape("A", &square(0.0, 0.0, 10.0), true),
            shape("b", &[(0.0, 0.0), (3.0, 4.0)], false),
            shape("a", &square(0.0, 0.0, 1.0), true),
        ]);
        let l = layers(&d, &Settings::default());
        assert_eq!(l.len(), 2);
        assert_eq!((l[0].key.as_str(), l[0].shapes, l[0].closed), ("a", 2, 2));
        assert_eq!(l[0].length_mm, 44.0);
        assert_eq!(
            (l[1].key.as_str(), l[1].shapes, l[1].closed, l[1].length_mm),
            ("b", 1, 0, 5.0)
        );
    }

    #[test]
    fn the_travel_between_contours_is_counted() {
        // two squares 20 apart: the head goes from the end of the first (its start corner (0,0)... (0,0) to the nearest corner of the next, (20,0))
        let d = dxf(vec![
            shape("l", &square(0.0, 0.0, 10.0), true),
            shape("l", &square(20.0, 0.0, 10.0), true),
        ]);
        let p = plan(&d, &settings("")).unwrap();
        assert!(
            (p.stats.travel_mm - 20.0).abs() < 1e-9,
            "{}",
            p.stats.travel_mm
        );
        // 80 mm cut at 10 mm/s = 8 s, 20 mm of travel at 6000 mm/min = 0.2 s
        assert!((p.stats.seconds - 8.2).abs() < 1e-9, "{}", p.stats.seconds);
    }
}
