//! The reader of SVG drawings: a safe subset that is enough to cut and engrave. It understands `path` (`M L H V C S Q T A Z`, absolute and relative), `rect`, `circle`, `ellipse`,
//! `line`, `polyline` and `polygon`, the `transform` attribute (`translate`, `scale`, `rotate`, `skewX`, `skewY` and `matrix`), groups (the layers of Inkscape give the name of a
//! layer), the stroke and fill colours, and the units `mm`, `cm`, `in`, `pt`, `pc` and `px` (with a `dpi` that is a setting).
//!
//! It never runs or fetches anything: scripts, images, `use`, text, styles with selectors, filters and every other element it does not know are skipped (with a note), an external
//! entity or a declared entity is an error, and the size of the file, the depth of the elements, their number and the points are all limited by [`ReadOptions`].

use crate::flatten;
use crate::vector::{Collector, Drawing, Pt, ReadOptions, Source};

/// The elements whose whole content is skipped.
const SKIPPED: [&str; 24] = [
    "defs",
    "clipPath",
    "mask",
    "symbol",
    "pattern",
    "marker",
    "metadata",
    "title",
    "desc",
    "style",
    "script",
    "foreignObject",
    "filter",
    "linearGradient",
    "radialGradient",
    "image",
    "use",
    "text",
    "textPath",
    "tspan",
    "animate",
    "set",
    "namedview",
    "view",
];

/// The attributes one tag may have.
const MAX_ATTRS: usize = 512;

/// Names of the colours of CSS that a drawing program writes.
const NAMES: [(&str, &str); 20] = [
    ("black", "#000000"),
    ("white", "#ffffff"),
    ("red", "#ff0000"),
    ("lime", "#00ff00"),
    ("green", "#008000"),
    ("blue", "#0000ff"),
    ("yellow", "#ffff00"),
    ("cyan", "#00ffff"),
    ("aqua", "#00ffff"),
    ("magenta", "#ff00ff"),
    ("fuchsia", "#ff00ff"),
    ("gray", "#808080"),
    ("grey", "#808080"),
    ("silver", "#c0c0c0"),
    ("maroon", "#800000"),
    ("olive", "#808000"),
    ("navy", "#000080"),
    ("purple", "#800080"),
    ("teal", "#008080"),
    ("orange", "#ffa500"),
];

/// A 2D transform `[a, b, c, d, e, f]`: `x' = a x + c y + e` and `y' = b x + d y + f`.
type Mat = [f64; 6];
const IDENTITY: Mat = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

/// `p` applied after `q`.
fn mul(p: &Mat, q: &Mat) -> Mat {
    [
        p[0] * q[0] + p[2] * q[1],
        p[1] * q[0] + p[3] * q[1],
        p[0] * q[2] + p[2] * q[3],
        p[1] * q[2] + p[3] * q[3],
        p[0] * q[4] + p[2] * q[5] + p[4],
        p[1] * q[4] + p[3] * q[5] + p[5],
    ]
}

fn apply(m: &Mat, p: Pt) -> Pt {
    (
        m[0] * p.0 + m[2] * p.1 + m[4],
        m[1] * p.0 + m[3] * p.1 + m[5],
    )
}

/// What went wrong with a piece of the file: bad data skips that piece, a limit stops the whole reading.
enum Fail {
    Bad(String),
    Limit(String),
}

impl Fail {
    fn bad(msg: &str) -> Fail {
        Fail::Bad(msg.to_string())
    }
}

// ---------------------------------------------------------------- the text of the numbers

struct Scanner<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Scanner<'a> {
    fn new(text: &'a str) -> Scanner<'a> {
        Scanner {
            b: text.as_bytes(),
            i: 0,
        }
    }

    fn skip_sep(&mut self) {
        while self.i < self.b.len()
            && (self.b[self.i].is_ascii_whitespace() || self.b[self.i] == b',')
        {
            self.i += 1;
        }
    }

    fn eof(&mut self) -> bool {
        self.skip_sep();
        self.i >= self.b.len()
    }

    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }

    /// A number: a sign, digits, a point, digits and an exponent; `1.5.5` is two numbers and `-1-2` too.
    fn number(&mut self) -> Option<f64> {
        self.skip_sep();
        let b = self.b;
        let start = self.i;
        let mut j = start;
        let digit = |k: usize| b.get(k).is_some_and(u8::is_ascii_digit);
        if matches!(b.get(j), Some(b'+' | b'-')) {
            j += 1;
        }
        let mut digits = 0;
        while digit(j) {
            j += 1;
            digits += 1;
        }
        if b.get(j) == Some(&b'.') {
            j += 1;
            while digit(j) {
                j += 1;
                digits += 1;
            }
        }
        if digits == 0 {
            return None;
        }
        if matches!(b.get(j), Some(b'e' | b'E')) {
            let mut k = j + 1;
            if matches!(b.get(k), Some(b'+' | b'-')) {
                k += 1;
            }
            let first = k;
            while digit(k) {
                k += 1;
            }
            if k > first {
                j = k;
            }
        }
        let v = std::str::from_utf8(&b[start..j])
            .ok()?
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())?;
        self.i = j;
        Some(v)
    }

    /// An arc flag: a single `0` or `1`, which may be written with nothing after it.
    fn flag(&mut self) -> Option<bool> {
        self.skip_sep();
        match self.peek() {
            Some(b'0') => {
                self.i += 1;
                Some(false)
            }
            Some(b'1') => {
                self.i += 1;
                Some(true)
            }
            _ => None,
        }
    }
}

/// All the numbers of a text (the points of a polyline); anything else in it is an error.
fn numbers(text: &str) -> Result<Vec<f64>, Fail> {
    let mut sc = Scanner::new(text);
    let mut out = Vec::new();
    while !sc.eof() {
        out.push(
            sc.number()
                .ok_or_else(|| Fail::bad("a list of numbers has something else in it"))?,
        );
    }
    Ok(out)
}

// ---------------------------------------------------------------- units and colours

/// The millimetres of a unit of length, or none for a unit that cannot be turned into a length (a percent, `em`).
fn unit_mm(unit: &str, px_mm: f64) -> Option<f64> {
    match unit.trim().to_ascii_lowercase().as_str() {
        "" | "px" => Some(px_mm),
        "mm" => Some(1.0),
        "cm" => Some(10.0),
        "in" => Some(25.4),
        "pt" => Some(25.4 / 72.0),
        "pc" => Some(25.4 / 6.0),
        _ => None,
    }
}

/// A length with its unit split: the number and the rest.
fn split_length(text: &str) -> Option<(f64, &str)> {
    let t = text.trim();
    let end = t
        .char_indices()
        .find(|&(i, c)| {
            c.is_ascii_alphabetic()
                && !(matches!(c, 'e' | 'E')
                    && t[i + 1..].starts_with(|n: char| n.is_ascii_digit() || n == '-' || n == '+'))
                || c == '%'
        })
        .map_or(t.len(), |(i, _)| i);
    let v = t[..end]
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())?;
    Some((v, t[end..].trim()))
}

/// A length of the page in mm.
fn page_mm(text: &str, px_mm: f64) -> Option<f64> {
    let (v, unit) = split_length(text)?;
    let mm = v * unit_mm(unit, px_mm)?;
    (mm.is_finite() && mm > 0.0).then_some(mm)
}

#[derive(Clone, PartialEq)]
enum Paint {
    None,
    Color(String),
}

/// What a value of `stroke` or `fill` says: a new paint, or `None` when it changes nothing (`inherit`, `currentColor`).
fn paint_of(value: &str, warn: &mut bool) -> Option<Paint> {
    let v = value.trim().to_ascii_lowercase();
    if v == "none" || v == "transparent" {
        return Some(Paint::None);
    }
    if v.is_empty() || v == "inherit" || v == "currentcolor" {
        return None;
    }
    if let Some(hex) = v.strip_prefix('#') {
        if hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            if hex.len() == 6 {
                return Some(Paint::Color(format!("#{hex}")));
            }
            if hex.len() == 3 {
                let d: String = hex.chars().flat_map(|c| [c, c]).collect();
                return Some(Paint::Color(format!("#{d}")));
            }
        }
        return Some(Paint::Color(v));
    }
    if let Some(args) = v.strip_prefix("rgb(").and_then(|r| r.strip_suffix(')')) {
        let parts: Vec<&str> = args.split(',').map(str::trim).collect();
        if parts.len() == 3 {
            let channel = |p: &str| -> Option<u8> {
                if let Some(pc) = p.strip_suffix('%') {
                    let f = pc.trim().parse::<f64>().ok().filter(|f| f.is_finite())?;
                    Some((f.clamp(0.0, 100.0) * 2.55).round() as u8)
                } else {
                    let f = p.parse::<f64>().ok().filter(|f| f.is_finite())?;
                    Some(f.clamp(0.0, 255.0).round() as u8)
                }
            };
            if let (Some(r), Some(g), Some(b)) =
                (channel(parts[0]), channel(parts[1]), channel(parts[2]))
            {
                return Some(Paint::Color(format!("#{r:02x}{g:02x}{b:02x}")));
            }
        }
        return Some(Paint::Color(v));
    }
    if v.starts_with("url(") {
        *warn = true;
        return Some(Paint::Color("#000000".to_string()));
    }
    if let Some((_, hex)) = NAMES.iter().find(|(n, _)| *n == v) {
        return Some(Paint::Color((*hex).to_string()));
    }
    Some(Paint::Color(v))
}

// ---------------------------------------------------------------- transforms

/// The matrix of a `transform` attribute.
fn parse_transform(text: &str) -> Result<Mat, Fail> {
    let mut m = IDENTITY;
    let mut rest = text.trim();
    let mut count = 0;
    while !rest.is_empty() {
        count += 1;
        if count > 64 {
            return Err(Fail::bad("a transform has too many parts"));
        }
        let open = rest
            .find('(')
            .ok_or_else(|| Fail::bad("a transform has no ("))?;
        let close = rest
            .find(')')
            .ok_or_else(|| Fail::bad("a transform has no )"))?;
        if close < open {
            return Err(Fail::bad("a transform is malformed"));
        }
        let name = rest[..open].trim().trim_start_matches(',').trim();
        let args = numbers(&rest[open + 1..close])?;
        let t: Mat = match (name, args.as_slice()) {
            ("translate", [x]) => [1.0, 0.0, 0.0, 1.0, *x, 0.0],
            ("translate", [x, y]) => [1.0, 0.0, 0.0, 1.0, *x, *y],
            ("scale", [s]) => [*s, 0.0, 0.0, *s, 0.0, 0.0],
            ("scale", [x, y]) => [*x, 0.0, 0.0, *y, 0.0, 0.0],
            ("rotate", [a]) => rotation(*a),
            ("rotate", [a, cx, cy]) => {
                let back = [1.0, 0.0, 0.0, 1.0, *cx, *cy];
                let there = [1.0, 0.0, 0.0, 1.0, -*cx, -*cy];
                mul(&back, &mul(&rotation(*a), &there))
            }
            ("skewX", [a]) => [1.0, 0.0, a.to_radians().tan(), 1.0, 0.0, 0.0],
            ("skewY", [a]) => [1.0, a.to_radians().tan(), 0.0, 1.0, 0.0, 0.0],
            ("matrix", [a, b, c, d, e, f]) => [*a, *b, *c, *d, *e, *f],
            _ => return Err(Fail::bad("a transform is not one this reader knows")),
        };
        if t.iter().any(|v| !v.is_finite() || v.abs() > 1.0e9) {
            return Err(Fail::bad("a transform has a value that is too large"));
        }
        m = mul(&m, &t);
        rest = rest[close + 1..].trim_start_matches([' ', ',', '\t', '\r', '\n']);
    }
    if m.iter().any(|v| !v.is_finite()) {
        return Err(Fail::bad("a transform is not a number"));
    }
    Ok(m)
}

fn rotation(deg: f64) -> Mat {
    let (s, c) = deg.to_radians().sin_cos();
    [c, s, -s, c, 0.0, 0.0]
}

// ---------------------------------------------------------------- the path of a shape

/// Builds the strokes of a shape: every point is moved by the transform at once, so the curves are cut into pieces in millimetres.
struct Builder<'a> {
    m: Mat,
    tol: f64,
    max_seg: usize,
    done: Vec<(Vec<Pt>, bool)>,
    cur: Vec<Pt>,
    pos: Pt,
    start: Pt,
    room: &'a mut usize,
}

impl<'a> Builder<'a> {
    fn new(m: Mat, opts: &ReadOptions, room: &'a mut usize) -> Builder<'a> {
        Builder {
            m,
            tol: opts.tolerance_mm,
            max_seg: opts.limits.max_curve_segments,
            done: Vec::new(),
            cur: Vec::new(),
            pos: (0.0, 0.0),
            start: (0.0, 0.0),
            room,
        }
    }

    fn spend(&mut self, n: usize) -> Result<(), Fail> {
        if n > *self.room {
            return Err(Fail::Limit(
                "the drawing has more points than max_points".to_string(),
            ));
        }
        *self.room -= n;
        Ok(())
    }

    fn flush_open(&mut self) {
        if self.cur.len() >= 2 {
            self.done.push((std::mem::take(&mut self.cur), false));
        } else {
            self.cur.clear();
        }
    }

    fn begin(&mut self, p: Pt) -> Result<(), Fail> {
        self.flush_open();
        self.spend(1)?;
        self.cur.push(apply(&self.m, p));
        self.pos = p;
        self.start = p;
        Ok(())
    }

    fn started(&mut self) -> Result<(), Fail> {
        if self.cur.is_empty() {
            self.begin(self.pos)?;
        }
        Ok(())
    }

    fn line_to(&mut self, p: Pt) -> Result<(), Fail> {
        self.started()?;
        self.spend(1)?;
        self.cur.push(apply(&self.m, p));
        self.pos = p;
        Ok(())
    }

    fn here(&self) -> Pt {
        self.cur
            .last()
            .copied()
            .unwrap_or_else(|| apply(&self.m, self.pos))
    }

    fn cubic_to(&mut self, c1: Pt, c2: Pt, p: Pt) -> Result<(), Fail> {
        self.started()?;
        let from = self.here();
        let (a, b, c) = (apply(&self.m, c1), apply(&self.m, c2), apply(&self.m, p));
        let mut pts = Vec::new();
        flatten::cubic(from, a, b, c, self.tol, self.max_seg, &mut pts);
        self.spend(pts.len())?;
        self.cur.extend(pts);
        self.pos = p;
        Ok(())
    }

    fn quad_to(&mut self, c1: Pt, p: Pt) -> Result<(), Fail> {
        self.started()?;
        let from = self.here();
        let (a, c) = (apply(&self.m, c1), apply(&self.m, p));
        let mut pts = Vec::new();
        flatten::quad(from, a, c, self.tol, self.max_seg, &mut pts);
        self.spend(pts.len())?;
        self.cur.extend(pts);
        self.pos = p;
        Ok(())
    }

    fn arc_to(
        &mut self,
        rx: f64,
        ry: f64,
        rot: f64,
        large: bool,
        sweep: bool,
        p: Pt,
    ) -> Result<(), Fail> {
        self.started()?;
        for [c1, c2, c3] in flatten::svg_arc(self.pos, rx, ry, rot, large, sweep, p) {
            self.cubic_to(c1, c2, c3)?;
        }
        self.pos = p;
        Ok(())
    }

    fn close(&mut self) {
        if self.cur.len() >= 2 {
            self.done.push((std::mem::take(&mut self.cur), true));
        } else {
            self.cur.clear();
        }
        self.pos = self.start;
    }

    fn finish(mut self) -> Vec<(Vec<Pt>, bool)> {
        self.flush_open();
        self.done
    }
}

/// Reads the `d` attribute of a path into the builder.
fn parse_path(d: &str, b: &mut Builder) -> Result<(), Fail> {
    let mut sc = Scanner::new(d);
    let mut cmd: Option<u8> = None;
    let mut last_c2: Option<Pt> = None;
    let mut last_q: Option<Pt> = None;
    let need =
        |v: Option<f64>| v.ok_or_else(|| Fail::bad("a path has a command with a missing number"));
    loop {
        if sc.eof() {
            break;
        }
        let c = sc.peek().unwrap_or(b' ');
        if c.is_ascii_alphabetic() {
            sc.i += 1;
            cmd = Some(c);
            if c == b'z' || c == b'Z' {
                b.close();
                last_c2 = None;
                last_q = None;
                continue;
            }
        } else if cmd.is_none() {
            return Err(Fail::bad("a path does not start with a command"));
        } else if matches!(cmd, Some(b'z' | b'Z')) {
            return Err(Fail::bad("a path has numbers after Z"));
        }
        let command = cmd.unwrap_or(b'M');
        if cmd.is_none()
            || (b.cur.is_empty() && b.done.is_empty() && !matches!(command, b'M' | b'm'))
        {
            return Err(Fail::bad("a path must start with a move"));
        }
        let rel = command.is_ascii_lowercase();
        let (ox, oy) = if rel { b.pos } else { (0.0, 0.0) };
        match command.to_ascii_uppercase() {
            b'M' => {
                let p = (ox + need(sc.number())?, oy + need(sc.number())?);
                b.begin(p)?;
                cmd = Some(if rel { b'l' } else { b'L' });
                last_c2 = None;
                last_q = None;
            }
            b'L' => {
                let p = (ox + need(sc.number())?, oy + need(sc.number())?);
                b.line_to(p)?;
                last_c2 = None;
                last_q = None;
            }
            b'H' => {
                let x = ox + need(sc.number())?;
                let y = b.pos.1;
                b.line_to((x, y))?;
                last_c2 = None;
                last_q = None;
            }
            b'V' => {
                let y = oy + need(sc.number())?;
                let x = b.pos.0;
                b.line_to((x, y))?;
                last_c2 = None;
                last_q = None;
            }
            b'C' => {
                let c1 = (ox + need(sc.number())?, oy + need(sc.number())?);
                let c2 = (ox + need(sc.number())?, oy + need(sc.number())?);
                let p = (ox + need(sc.number())?, oy + need(sc.number())?);
                b.cubic_to(c1, c2, p)?;
                last_c2 = Some(c2);
                last_q = None;
            }
            b'S' => {
                let c1 = match last_c2 {
                    Some(q) => (2.0 * b.pos.0 - q.0, 2.0 * b.pos.1 - q.1),
                    None => b.pos,
                };
                let c2 = (ox + need(sc.number())?, oy + need(sc.number())?);
                let p = (ox + need(sc.number())?, oy + need(sc.number())?);
                b.cubic_to(c1, c2, p)?;
                last_c2 = Some(c2);
                last_q = None;
            }
            b'Q' => {
                let c1 = (ox + need(sc.number())?, oy + need(sc.number())?);
                let p = (ox + need(sc.number())?, oy + need(sc.number())?);
                b.quad_to(c1, p)?;
                last_q = Some(c1);
                last_c2 = None;
            }
            b'T' => {
                let c1 = match last_q {
                    Some(q) => (2.0 * b.pos.0 - q.0, 2.0 * b.pos.1 - q.1),
                    None => b.pos,
                };
                let p = (ox + need(sc.number())?, oy + need(sc.number())?);
                b.quad_to(c1, p)?;
                last_q = Some(c1);
                last_c2 = None;
            }
            b'A' => {
                let rx = need(sc.number())?;
                let ry = need(sc.number())?;
                let rot = need(sc.number())?;
                let large = sc
                    .flag()
                    .ok_or_else(|| Fail::bad("an arc has a flag that is not 0 or 1"))?;
                let sweep = sc
                    .flag()
                    .ok_or_else(|| Fail::bad("an arc has a flag that is not 0 or 1"))?;
                let p = (ox + need(sc.number())?, oy + need(sc.number())?);
                b.arc_to(rx, ry, rot, large, sweep, p)?;
                last_c2 = None;
                last_q = None;
            }
            _ => return Err(Fail::bad("a path has a command this reader does not know")),
        }
    }
    Ok(())
}

// ---------------------------------------------------------------- the XML

enum Tok {
    Start {
        name: String,
        attrs: Vec<(String, String)>,
        empty: bool,
    },
    End(String),
}

struct Lexer<'a> {
    s: &'a str,
    i: usize,
}

/// The text of an attribute with the five predefined entities and the numbers `&#10;` or `&#xA;` turned into their letters. Any other entity stays as it is written: a declared
/// entity is never expanded.
fn decode(raw: &str) -> String {
    if !raw.contains('&') {
        return raw.to_string();
    }
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let Some(semi) = rest.find(';').filter(|&s| s <= 10) else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let name = &rest[1..semi];
        let ch = match name {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ => name
                .strip_prefix("#x")
                .or_else(|| name.strip_prefix("#X"))
                .and_then(|h| u32::from_str_radix(h, 16).ok())
                .or_else(|| name.strip_prefix('#').and_then(|d| d.parse::<u32>().ok()))
                .and_then(char::from_u32)
                .filter(|c| !c.is_control() || c.is_whitespace()),
        };
        match ch {
            Some(c) => {
                out.push(c);
                rest = &rest[semi + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn local(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

impl Lexer<'_> {
    fn token(&mut self) -> Result<Option<Tok>, String> {
        loop {
            let Some(off) = self.s[self.i..].find('<') else {
                return Ok(None);
            };
            self.i += off;
            let rest = &self.s[self.i..];
            if let Some(inside) = rest.strip_prefix("<!--") {
                let end = inside.find("-->").ok_or("a comment is not closed")?;
                self.i += 4 + end + 3;
                continue;
            }
            if rest.starts_with("<![CDATA[") {
                let end = rest.find("]]>").ok_or("a CDATA section is not closed")?;
                self.i += end + 3;
                continue;
            }
            if rest.starts_with("<?") {
                let end = rest
                    .find("?>")
                    .ok_or("a processing instruction is not closed")?;
                self.i += end + 2;
                continue;
            }
            if rest.starts_with("<!") {
                let mut depth = 0usize;
                let mut end = None;
                for (k, c) in rest.char_indices() {
                    match c {
                        '[' => depth += 1,
                        ']' => depth = depth.saturating_sub(1),
                        '>' if depth == 0 => {
                            end = Some(k);
                            break;
                        }
                        _ => {}
                    }
                }
                let end = end.ok_or("a declaration is not closed")?;
                if rest[..end].to_ascii_uppercase().contains("<!ENTITY") {
                    return Err(
                        "the file declares entities, which this reader does not expand".to_string(),
                    );
                }
                self.i += end + 1;
                continue;
            }
            if let Some(body) = rest.strip_prefix("</") {
                let end = body.find('>').ok_or("an end tag is not closed")?;
                let name = local(body[..end].trim()).to_string();
                self.i += 2 + end + 1;
                return Ok(Some(Tok::End(name)));
            }
            let tok = self.start_tag()?;
            if let Tok::Start {
                name, empty: false, ..
            } = &tok
            {
                if name == "script" || name == "style" {
                    self.skip_raw(name)?;
                }
            }
            return Ok(Some(tok));
        }
    }

    /// Moves to the end tag of an element whose content is not markup (a script or a style), without reading what is inside.
    fn skip_raw(&mut self, name: &str) -> Result<(), String> {
        let mut at = self.i;
        while let Some(off) = self.s[at..].find("</") {
            let start = at + off;
            let tag = &self.s[start + 2..];
            if tag.len() >= name.len()
                && tag.is_char_boundary(name.len())
                && tag[..name.len()].eq_ignore_ascii_case(name)
            {
                self.i = start;
                return Ok(());
            }
            at = start + 2;
        }
        Err(format!("the {name} element is not closed"))
    }

    fn start_tag(&mut self) -> Result<Tok, String> {
        let bytes = self.s.as_bytes();
        let mut j = self.i + 1;
        let name_start = j;
        while j < bytes.len()
            && !bytes[j].is_ascii_whitespace()
            && bytes[j] != b'/'
            && bytes[j] != b'>'
        {
            j += 1;
        }
        let name = local(&self.s[name_start..j]).to_string();
        if name.is_empty() {
            return Err("a tag has no name".to_string());
        }
        let mut attrs: Vec<(String, String)> = Vec::new();
        loop {
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            match bytes.get(j) {
                None => return Err("a tag is not closed".to_string()),
                Some(b'>') => {
                    self.i = j + 1;
                    return Ok(Tok::Start {
                        name,
                        attrs,
                        empty: false,
                    });
                }
                Some(b'/') => {
                    if bytes.get(j + 1) == Some(&b'>') {
                        self.i = j + 2;
                        return Ok(Tok::Start {
                            name,
                            attrs,
                            empty: true,
                        });
                    }
                    return Err("a tag has a / in the wrong place".to_string());
                }
                _ => {}
            }
            let key_start = j;
            while j < bytes.len()
                && !bytes[j].is_ascii_whitespace()
                && !matches!(bytes[j], b'=' | b'/' | b'>')
            {
                j += 1;
            }
            let key = self.s[key_start..j].to_string();
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if bytes.get(j) != Some(&b'=') {
                return Err("an attribute has no value".to_string());
            }
            j += 1;
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            let quote = match bytes.get(j) {
                Some(&q) if q == b'"' || q == b'\'' => q,
                _ => return Err("an attribute value is not quoted".to_string()),
            };
            j += 1;
            let close = self.s[j..]
                .find(quote as char)
                .ok_or("an attribute value is not closed")?;
            if attrs.len() >= MAX_ATTRS {
                return Err("a tag has too many attributes".to_string());
            }
            attrs.push((key, decode(&self.s[j..j + close])));
            j += close + 1;
        }
    }
}

// ---------------------------------------------------------------- the elements

fn get<'a>(attrs: &'a [(String, String)], name: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.as_str())
}

/// A property of the presentation: the `style` attribute wins over the attribute of the same name.
fn prop(attrs: &[(String, String)], name: &str) -> Option<String> {
    if let Some(style) = get(attrs, "style") {
        for decl in style.split(';') {
            if let Some((k, v)) = decl.split_once(':') {
                if k.trim() == name {
                    return Some(v.trim().to_string());
                }
            }
        }
    }
    get(attrs, name).map(|v| v.trim().to_string())
}

#[derive(Clone)]
struct Ctx {
    m: Mat,
    stroke: Paint,
    fill: Paint,
    layer: String,
    hidden: bool,
    /// The millimetres of a unit of the drawing.
    user_mm: f64,
}

impl Ctx {
    fn derive(&self, attrs: &[(String, String)], col: &mut Collector) -> Ctx {
        let mut c = self.clone();
        if let Some(t) = get(attrs, "transform") {
            match parse_transform(t) {
                Ok(m) => c.m = mul(&c.m, &m),
                Err(_) => {
                    col.warn("an element with a transform that cannot be read was skipped");
                    c.hidden = true;
                }
            }
        }
        if prop(attrs, "display").as_deref() == Some("none") {
            c.hidden = true;
        }
        let mut warn = false;
        if let Some(p) = prop(attrs, "stroke").and_then(|v| paint_of(&v, &mut warn)) {
            c.stroke = p;
        }
        if let Some(p) = prop(attrs, "fill").and_then(|v| paint_of(&v, &mut warn)) {
            c.fill = p;
        }
        if warn {
            col.warn("a paint server (a gradient or a pattern) is not supported: black is used");
        }
        if get(attrs, "inkscape:groupmode") == Some("layer") {
            if let Some(name) = get(attrs, "inkscape:label").or_else(|| get(attrs, "id")) {
                c.layer = name.trim().to_string();
            }
        }
        c
    }

    /// The colour a shape is drawn with: the stroke, or the fill when there is no stroke.
    fn color(&self) -> Option<String> {
        match (&self.stroke, &self.fill) {
            (Paint::Color(c), _) | (Paint::None, Paint::Color(c)) => Some(c.clone()),
            _ => None,
        }
    }
}

/// A length of a shape in the units of the drawing (the unit of the number decides).
fn user_len(attrs: &[(String, String)], name: &str, user_mm: f64) -> Option<f64> {
    let (v, unit) = split_length(get(attrs, name)?)?;
    let r = if unit.is_empty() || unit.eq_ignore_ascii_case("px") {
        v
    } else {
        v * unit_mm(unit, user_mm)? / user_mm
    };
    r.is_finite().then_some(r)
}

fn quarter_ellipse(b: &mut Builder, cx: f64, cy: f64, rx: f64, ry: f64) -> Result<(), Fail> {
    b.begin((cx + rx, cy))?;
    b.arc_to(rx, ry, 0.0, false, true, (cx, cy + ry))?;
    b.arc_to(rx, ry, 0.0, false, true, (cx - rx, cy))?;
    b.arc_to(rx, ry, 0.0, false, true, (cx, cy - ry))?;
    b.arc_to(rx, ry, 0.0, false, true, (cx + rx, cy))?;
    b.close();
    Ok(())
}

/// The strokes of a shape element, or none when the element draws nothing.
fn shape(
    name: &str,
    attrs: &[(String, String)],
    ctx: &Ctx,
    opts: &ReadOptions,
    room: &mut usize,
) -> Result<Vec<(Vec<Pt>, bool)>, Fail> {
    let um = ctx.user_mm;
    let len = |n: &str| user_len(attrs, n, um);
    let mut b = Builder::new(ctx.m, opts, room);
    match name {
        "path" => {
            let d = get(attrs, "d").ok_or_else(|| Fail::bad("a path has no d"))?;
            parse_path(d, &mut b)?;
        }
        "rect" => {
            let (x, y) = (len("x").unwrap_or(0.0), len("y").unwrap_or(0.0));
            let (w, h) = (len("width").unwrap_or(0.0), len("height").unwrap_or(0.0));
            if w <= 0.0 || h <= 0.0 {
                return Ok(Vec::new());
            }
            let (rx, ry) = match (len("rx"), len("ry")) {
                (Some(a), Some(c)) => (a, c),
                (Some(a), None) => (a, a),
                (None, Some(c)) => (c, c),
                (None, None) => (0.0, 0.0),
            };
            let (rx, ry) = (rx.clamp(0.0, w / 2.0), ry.clamp(0.0, h / 2.0));
            if rx > 0.0 && ry > 0.0 {
                b.begin((x + rx, y))?;
                b.line_to((x + w - rx, y))?;
                b.arc_to(rx, ry, 0.0, false, true, (x + w, y + ry))?;
                b.line_to((x + w, y + h - ry))?;
                b.arc_to(rx, ry, 0.0, false, true, (x + w - rx, y + h))?;
                b.line_to((x + rx, y + h))?;
                b.arc_to(rx, ry, 0.0, false, true, (x, y + h - ry))?;
                b.line_to((x, y + ry))?;
                b.arc_to(rx, ry, 0.0, false, true, (x + rx, y))?;
            } else {
                b.begin((x, y))?;
                b.line_to((x + w, y))?;
                b.line_to((x + w, y + h))?;
                b.line_to((x, y + h))?;
            }
            b.close();
        }
        "circle" => {
            let r = len("r").unwrap_or(0.0);
            if r <= 0.0 {
                return Ok(Vec::new());
            }
            quarter_ellipse(
                &mut b,
                len("cx").unwrap_or(0.0),
                len("cy").unwrap_or(0.0),
                r,
                r,
            )?;
        }
        "ellipse" => {
            let (rx, ry) = (len("rx").unwrap_or(0.0), len("ry").unwrap_or(0.0));
            if rx <= 0.0 || ry <= 0.0 {
                return Ok(Vec::new());
            }
            quarter_ellipse(
                &mut b,
                len("cx").unwrap_or(0.0),
                len("cy").unwrap_or(0.0),
                rx,
                ry,
            )?;
        }
        "line" => {
            b.begin((len("x1").unwrap_or(0.0), len("y1").unwrap_or(0.0)))?;
            b.line_to((len("x2").unwrap_or(0.0), len("y2").unwrap_or(0.0)))?;
        }
        "polyline" | "polygon" => {
            let v = numbers(get(attrs, "points").unwrap_or(""))?;
            let mut pairs = v.chunks_exact(2);
            for (k, p) in pairs.by_ref().enumerate() {
                if k == 0 {
                    b.begin((p[0], p[1]))?;
                } else {
                    b.line_to((p[0], p[1]))?;
                }
            }
            if name == "polygon" {
                b.close();
            }
        }
        _ => return Ok(Vec::new()),
    }
    Ok(b.finish())
}

const SHAPES: [&str; 7] = [
    "path", "rect", "circle", "ellipse", "line", "polyline", "polygon",
];

/// Reads an SVG file.
pub fn read_svg(bytes: &[u8], opts: &ReadOptions) -> Result<Drawing, String> {
    opts.validate()?;
    let limits = opts.limits;
    if bytes.len() > limits.max_file_bytes {
        return Err(format!(
            "the file is larger than {} bytes (max_file_bytes)",
            limits.max_file_bytes
        ));
    }
    let text = String::from_utf8_lossy(bytes);
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let px_mm = 25.4 / opts.dpi;
    let mut col = Collector::new(Source::Svg, limits);
    let mut lexer = Lexer { s: text, i: 0 };
    let base = Ctx {
        m: IDENTITY,
        stroke: Paint::None,
        fill: Paint::Color("#000000".to_string()),
        layer: "default".to_string(),
        hidden: false,
        user_mm: px_mm,
    };
    let mut stack: Vec<(String, Ctx)> = Vec::new();
    let mut skip = 0usize;
    let mut root_seen = false;
    let mut nodes = 0usize;
    let mut room = limits.max_points;
    while let Some(tok) = lexer.token()? {
        match tok {
            Tok::Start { name, attrs, empty } => {
                nodes += 1;
                if nodes > limits.max_nodes {
                    return Err(format!(
                        "the file has more than {} elements (max_nodes)",
                        limits.max_nodes
                    ));
                }
                if skip > 0 {
                    if !empty {
                        skip += 1;
                    }
                    continue;
                }
                if stack.len() >= limits.max_depth {
                    return Err(format!(
                        "the elements are nested deeper than {} (max_depth)",
                        limits.max_depth
                    ));
                }
                let parent = stack
                    .last()
                    .map_or_else(|| base.clone(), |(_, c)| c.clone());
                if name == "svg" || name == "g" || name == "a" || name == "switch" {
                    let mut ctx = parent.derive(&attrs, &mut col);
                    if name == "svg" && !root_seen {
                        root_seen = true;
                        root_frame(&attrs, &mut ctx, px_mm, &mut col);
                    } else if name == "svg" {
                        col.warn("an svg inside another svg is read as a group: its size and view box are ignored");
                    }
                    if ctx.hidden && !empty {
                        skip = 1;
                    } else if !empty {
                        stack.push((name, ctx));
                    }
                    continue;
                }
                if SHAPES.contains(&name.as_str()) {
                    if !empty {
                        skip = 1;
                    }
                    if !root_seen {
                        continue;
                    }
                    let ctx = parent.derive(&attrs, &mut col);
                    if ctx.hidden {
                        continue;
                    }
                    let Some(color) = ctx.color() else {
                        continue;
                    };
                    match shape(&name, &attrs, &ctx, opts, &mut room) {
                        Ok(strokes) => {
                            for (pts, closed) in strokes {
                                col.push(&ctx.layer, Some(color.as_str()), pts, closed)?;
                            }
                        }
                        Err(Fail::Bad(why)) => col.warn(format!("a {name} was skipped: {why}")),
                        Err(Fail::Limit(why)) => return Err(why),
                    }
                    continue;
                }
                if !empty {
                    skip = 1;
                }
                if SKIPPED.contains(&name.as_str()) {
                    if matches!(
                        name.as_str(),
                        "script" | "image" | "use" | "foreignObject" | "text"
                    ) {
                        col.warn(format!("{name} is not supported and was skipped"));
                    }
                } else {
                    col.warn(format!(
                        "the element {name} is not supported and was skipped"
                    ));
                }
            }
            Tok::End(name) => {
                if skip > 0 {
                    skip -= 1;
                    continue;
                }
                match stack.pop() {
                    Some((open, _)) if open == name => {}
                    _ => return Err("the end of an element does not match its start".to_string()),
                }
            }
        }
    }
    if !stack.is_empty() || skip > 0 {
        return Err("the file ends before an element is closed".to_string());
    }
    if !root_seen {
        return Err("this is not an SVG file: there is no svg element".to_string());
    }
    Ok(col.finish())
}

/// The size of the page and the view box of the first `svg`: the matrix that takes the units of the drawing to millimetres.
fn root_frame(attrs: &[(String, String)], ctx: &mut Ctx, px_mm: f64, col: &mut Collector) {
    let width = get(attrs, "width").and_then(|v| page_mm(v, px_mm));
    let height = get(attrs, "height").and_then(|v| page_mm(v, px_mm));
    let view = get(attrs, "viewBox")
        .or_else(|| get(attrs, "viewbox"))
        .and_then(|v| numbers(v).ok())
        .filter(|v| v.len() == 4 && v[2] > 0.0 && v[3] > 0.0);
    let (scale, offset, page_h) = match view {
        Some(v) => {
            let by_width = width.map(|w| w / v[2]);
            let by_height = height.map(|h| h / v[3]);
            let s = match (by_width, by_height) {
                (Some(a), Some(c)) => a.min(c),
                (Some(a), None) => a,
                (None, Some(c)) => c,
                (None, None) => px_mm,
            };
            (s, (-v[0] * s, -v[1] * s), Some(v[3] * s))
        }
        None => (px_mm, (0.0, 0.0), height),
    };
    ctx.m = mul(&ctx.m, &[scale, 0.0, 0.0, scale, offset.0, offset.1]);
    ctx.user_mm = scale;
    if let Some(h) = page_h {
        col.set_page_height(h);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One user unit is one millimetre: the arithmetic of the tests is then plain.
    fn mm() -> ReadOptions {
        ReadOptions {
            dpi: 25.4,
            ..ReadOptions::default()
        }
    }

    fn read(svg: &str) -> Drawing {
        read_svg(svg.as_bytes(), &mm()).unwrap()
    }

    fn close(a: Pt, b: Pt) -> bool {
        (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9
    }

    #[test]
    fn a_closed_path_with_a_size_and_a_view_box_is_read_in_millimetres() {
        // 100 mm over 100 units: one unit is one mm
        let d = read(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="50mm" viewBox="0 0 100 50"><path d="M0 0 L10 0 L10 10 Z" stroke="#f00" fill="none"/></svg>"##,
        );
        assert_eq!(d.shapes.len(), 1);
        let s = &d.shapes[0];
        assert_eq!(s.pts, vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]);
        assert!(s.closed);
        assert_eq!(s.color.as_deref(), Some("#ff0000"));
        assert_eq!(d.page_height_mm, Some(50.0));
    }

    #[test]
    fn the_view_box_scales_and_moves_the_drawing() {
        // 50 mm wide over 100 units: a unit is 0.5 mm; the box starts at (10, 20) so that point is the origin
        let d = read(
            r#"<svg width="50mm" height="50mm" viewBox="10 20 100 100"><line x1="10" y1="20" x2="110" y2="20" stroke="black"/></svg>"#,
        );
        assert_eq!(d.shapes[0].pts, vec![(0.0, 0.0), (50.0, 0.0)]);
        assert_eq!(d.page_height_mm, Some(50.0));
        // the smaller of the two scales is used when the shape of the page differs from the view box
        let d = read(
            r#"<svg width="100mm" height="20mm" viewBox="0 0 100 100"><line x1="0" y1="0" x2="100" y2="0" stroke="black"/></svg>"#,
        );
        assert!(close(d.shapes[0].pts[1], (20.0, 0.0)));
    }

    #[test]
    fn pixels_are_millimetres_at_the_dpi_of_the_settings() {
        let svg = r#"<svg><line x1="0" y1="0" x2="96" y2="0" stroke="red"/></svg>"#;
        let at96 = read_svg(svg.as_bytes(), &ReadOptions::default()).unwrap();
        assert!((at96.shapes[0].pts[1].0 - 25.4).abs() < 1e-9);
        let at72 = read_svg(
            svg.as_bytes(),
            &ReadOptions {
                dpi: 72.0,
                ..ReadOptions::default()
            },
        )
        .unwrap();
        assert!((at72.shapes[0].pts[1].0 - 96.0 * 25.4 / 72.0).abs() < 1e-9);
    }

    #[test]
    fn lengths_with_units_are_millimetres_whatever_the_dpi() {
        let svg = r#"<svg><line x1="0" y1="0" x2="1in" y2="2cm" stroke="red"/><rect x="5mm" y="0" width="10mm" height="1pc" stroke="red"/></svg>"#;
        let d = read_svg(
            svg.as_bytes(),
            &ReadOptions {
                dpi: 72.0,
                ..ReadOptions::default()
            },
        )
        .unwrap();
        assert!(close(d.shapes[0].pts[1], (25.4, 20.0)));
        // the rect: from x = 5 to 15, and a pica is 25.4 / 6 mm
        let r = &d.shapes[1].pts;
        assert!(close(r[0], (5.0, 0.0)) && close(r[1], (15.0, 0.0)));
        assert!((r[2].1 - 25.4 / 6.0).abs() < 1e-9);
    }

    #[test]
    fn translate_then_scale_moves_the_points_the_way_a_browser_does() {
        // scale first (it is the one nearest the point): (1,1) -> (2,2) -> (12,7), and (2,1) -> (4,2) -> (14,7)
        let d = read(
            r##"<svg><path d="M1 1 L2 1" stroke="#000" transform="translate(10 5) scale(2)"/></svg>"##,
        );
        assert_eq!(d.shapes[0].pts, vec![(12.0, 7.0), (14.0, 7.0)]);
    }

    #[test]
    fn a_rotation_turns_a_point_a_quarter_and_the_matrix_form_agrees() {
        let d = read(r#"<svg><path d="M0 0 L1 0" stroke="red" transform="rotate(90)"/></svg>"#);
        assert!(close(d.shapes[0].pts[1], (0.0, 1.0)));
        // rotate(90 10 10) takes (11, 10) to (10, 11)
        let d = read(
            r#"<svg><path d="M10 10 L11 10" stroke="red" transform="rotate(90 10 10)"/></svg>"#,
        );
        assert!(close(d.shapes[0].pts[0], (10.0, 10.0)) && close(d.shapes[0].pts[1], (10.0, 11.0)));
        let m = read(
            r#"<svg><path d="M1 2 L3 4" stroke="red" transform="matrix(0 1 -1 0 5 6)"/></svg>"#,
        );
        // x' = 0 x - y + 5 and y' = x + 0 y + 6: (1,2) -> (3,7) and (3,4) -> (1,9)
        assert_eq!(m.shapes[0].pts, vec![(3.0, 7.0), (1.0, 9.0)]);
    }

    #[test]
    fn the_transforms_of_nested_groups_are_multiplied() {
        let d = read(
            r#"<svg><g transform="translate(100 0)"><g transform="scale(2)"><path d="M1 1 L2 1" stroke="red"/></g></g></svg>"#,
        );
        assert_eq!(d.shapes[0].pts, vec![(102.0, 2.0), (104.0, 2.0)]);
    }

    #[test]
    fn relative_commands_and_the_implied_repeats() {
        // m 10 10 then the pairs 5 0 and 0 5 are two relative lines
        let d = read(r#"<svg><path d="m 10 10 l 5 0 0 5 z" stroke="red"/></svg>"#);
        assert_eq!(
            d.shapes[0].pts,
            vec![(10.0, 10.0), (15.0, 10.0), (15.0, 15.0)]
        );
        assert!(d.shapes[0].closed);
        // a pair after M is a line to
        let d = read(r#"<svg><path d="M 1 1 2 2 3 3" stroke="red"/></svg>"#);
        assert_eq!(d.shapes[0].pts.len(), 3);
        assert!(!d.shapes[0].closed);
    }

    #[test]
    fn horizontal_and_vertical_lines_keep_the_other_coordinate() {
        let d = read(r#"<svg><path d="M 0 0 H 10 V 10 H 0 Z" stroke="red"/></svg>"#);
        assert_eq!(
            d.shapes[0].pts,
            vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]
        );
        let rel = read(r#"<svg><path d="M 1 1 h 2 v 3 h -2 z" stroke="red"/></svg>"#);
        assert_eq!(
            rel.shapes[0].pts,
            vec![(1.0, 1.0), (3.0, 1.0), (3.0, 4.0), (1.0, 4.0)]
        );
    }

    #[test]
    fn numbers_may_be_written_together() {
        // 1.5.5 is 1.5 and .5; -1-2 is -1 and -2
        let d = read(r#"<svg><path d="M0,0L1.5.5-1-2" stroke="red"/></svg>"#);
        assert_eq!(d.shapes[0].pts, vec![(0.0, 0.0), (1.5, 0.5), (-1.0, -2.0)]);
        let e = read(r#"<svg><path d="M1e1 2E0 l5e-1 0" stroke="red"/></svg>"#);
        assert_eq!(e.shapes[0].pts, vec![(10.0, 2.0), (10.5, 2.0)]);
    }

    #[test]
    fn a_path_moves_again_after_z_from_the_start_of_the_shape() {
        let d = read(r#"<svg><path d="M 0 0 L 4 0 L 4 4 Z L 0 -4" stroke="red"/></svg>"#);
        assert_eq!(d.shapes.len(), 2);
        assert!(d.shapes[0].closed);
        assert_eq!(d.shapes[1].pts, vec![(0.0, 0.0), (0.0, -4.0)]);
    }

    #[test]
    fn arc_flags_may_be_written_without_spaces() {
        // the flags 1 and 0, then x = 10 and y = 0: a half circle of radius 5
        let d = read(r#"<svg><path d="M0 0 a5 5 0 1010 0" stroke="red"/></svg>"#);
        let pts = &d.shapes[0].pts;
        assert!(close(*pts.last().unwrap(), (10.0, 0.0)));
        for p in pts {
            assert!(
                (crate::vector::dist(*p, (5.0, 0.0)) - 5.0).abs() < 0.01 + 1e-9,
                "{p:?}"
            );
        }
    }

    #[test]
    fn a_smooth_cubic_reflects_the_last_control_point() {
        // the second curve starts at (10,0) with the control (10,-10) (the reflection of (10,10)) and ends at (20,0); at its middle x = 15 and y = -7.5
        let d = read_svg(
            br#"<svg><path d="M0 0 C 0 10 10 10 10 0 S 20 -10 20 0" stroke="red"/></svg>"#,
            &ReadOptions {
                tolerance_mm: 0.375,
                ..mm()
            },
        )
        .unwrap();
        assert!(
            d.shapes[0].pts.iter().any(|p| close(*p, (15.0, -7.5))),
            "{:?}",
            d.shapes[0].pts
        );
    }

    #[test]
    fn a_smooth_quadratic_reflects_the_last_control_point() {
        // the second curve has the control (15,-10); at its middle x = 15 and y = -5
        let d = read_svg(
            br#"<svg><path d="M0 0 Q 5 10 10 0 T 20 0" stroke="red"/></svg>"#,
            &ReadOptions {
                tolerance_mm: 0.1,
                ..mm()
            },
        )
        .unwrap();
        assert!(
            d.shapes[0].pts.iter().any(|p| close(*p, (15.0, -5.0))),
            "{:?}",
            d.shapes[0].pts
        );
    }

    #[test]
    fn the_basic_shapes() {
        let d = read(
            r#"<svg>
              <rect x="1" y="2" width="10" height="5" stroke="red"/>
              <line x1="0" y1="0" x2="3" y2="4" stroke="red"/>
              <polyline points="0,0 1,0 1,1" stroke="red"/>
              <polygon points="0,0 2,0 2,2 0,2" stroke="red"/>
            </svg>"#,
        );
        assert_eq!(d.shapes.len(), 4);
        assert_eq!(
            d.shapes[0].pts,
            vec![(1.0, 2.0), (11.0, 2.0), (11.0, 7.0), (1.0, 7.0)]
        );
        assert!(d.shapes[0].closed);
        assert_eq!(d.shapes[1].pts, vec![(0.0, 0.0), (3.0, 4.0)]);
        assert!(!d.shapes[2].closed);
        assert!(d.shapes[3].closed);
        assert_eq!(d.shapes[3].pts.len(), 4);
    }

    #[test]
    fn a_circle_and_an_ellipse_stay_within_the_tolerance() {
        let d = read(
            r#"<svg><circle cx="10" cy="10" r="5" stroke="blue"/><ellipse cx="0" cy="0" rx="8" ry="2" stroke="blue"/></svg>"#,
        );
        assert_eq!(d.shapes.len(), 2);
        let c = &d.shapes[0];
        assert!(c.closed);
        for p in &c.pts {
            assert!(
                (crate::vector::dist(*p, (10.0, 10.0)) - 5.0).abs() < 0.01 + 1e-9,
                "{p:?}"
            );
        }
        // the first point is the right end
        assert!(close(c.pts[0], (15.0, 10.0)));
        for p in &d.shapes[1].pts {
            let e = (p.0 / 8.0).powi(2) + (p.1 / 2.0).powi(2);
            assert!((e - 1.0).abs() < 0.01, "{p:?} {e}");
        }
    }

    #[test]
    fn a_rounded_rectangle_has_its_corners_cut_off() {
        let d =
            read(r#"<svg><rect x="0" y="0" width="20" height="10" rx="2" stroke="red"/></svg>"#);
        let pts = &d.shapes[0].pts;
        assert!(pts
            .iter()
            .all(|p| (-1e-9..=20.0 + 1e-9).contains(&p.0) && (-1e-9..=10.0 + 1e-9).contains(&p.1)));
        // the corner (0,0) is not a point of the shape, and (2,0) is
        assert!(!pts.iter().any(|p| close(*p, (0.0, 0.0))));
        assert!(pts.iter().any(|p| close(*p, (2.0, 0.0))));
    }

    #[test]
    fn colours_come_from_the_attributes_the_style_and_the_names() {
        let d = read(
            r##"<svg>
              <path d="M0 0 L1 0" stroke="red"/>
              <path d="M0 0 L2 0" style="stroke:#0F0;fill:none"/>
              <path d="M0 0 L3 0" stroke="rgb(0, 0, 255)"/>
              <path d="M0 0 L4 0" stroke="none" fill="#abc"/>
              <path d="M0 0 L5 0" stroke="none" fill="none"/>
              <path d="M0 0 L6 0" stroke="red" style="stroke:blue"/>
            </svg>"##,
        );
        let colors: Vec<_> = d.shapes.iter().map(|s| s.color.clone().unwrap()).collect();
        assert_eq!(
            colors,
            vec!["#ff0000", "#00ff00", "#0000ff", "#aabbcc", "#0000ff"]
        );
    }

    #[test]
    fn an_unstyled_shape_is_black_and_the_colour_is_inherited_from_the_group() {
        let d = read(
            r##"<svg><g stroke="#123456"><path d="M0 0 L1 0"/></g><path d="M0 0 L2 0"/></svg>"##,
        );
        assert_eq!(d.shapes[0].color.as_deref(), Some("#123456"));
        assert_eq!(d.shapes[1].color.as_deref(), Some("#000000"));
    }

    #[test]
    fn the_layers_of_inkscape_name_their_shapes() {
        let d = read(
            r#"<svg xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape">
              <g inkscape:groupmode="layer" inkscape:label="Holes"><path d="M0 0 L1 0" stroke="red"/></g>
              <g inkscape:groupmode="layer" id="outline"><g><path d="M0 0 L2 0" stroke="red"/></g></g>
              <path d="M0 0 L3 0" stroke="red"/>
            </svg>"#,
        );
        let layers: Vec<_> = d.shapes.iter().map(|s| s.layer.as_str()).collect();
        assert_eq!(layers, vec!["Holes", "outline", "default"]);
    }

    #[test]
    fn what_is_hidden_or_not_supported_is_skipped_with_a_note() {
        let d = read(
            r#"<svg>
              <defs><path d="M0 0 L9 0" stroke="red"/></defs>
              <g display="none"><path d="M0 0 L8 0" stroke="red"/></g>
              <path d="M0 0 L7 0" stroke="red" style="display:none"/>
              <script>if (a < b) { fetch("http://example.org/x") }</script>
              <image href="http://example.org/a.png" width="10" height="10"/>
              <use href="a"/>
              <text x="0" y="0">hello</text>
              <path d="M0 0 L1 0" stroke="red"/>
            </svg>"#,
        );
        assert_eq!(d.shapes.len(), 1);
        assert_eq!(d.shapes[0].pts[1], (1.0, 0.0));
        assert!(d.warnings.iter().any(|w| w.contains("script")));
        assert!(d.warnings.iter().any(|w| w.contains("image")));
        assert!(d.warnings.iter().any(|w| w.contains("use")));
    }

    #[test]
    fn a_bad_path_is_skipped_and_the_rest_is_read() {
        let d = read(
            r#"<svg>
              <path d="M 0 0 L 5" stroke="red"/>
              <path d="L 1 1" stroke="red"/>
              <path d="M 0 0 X 1 1" stroke="red"/>
              <path d="M 0 0 L 1 0" stroke="red" transform="bogus(1)"/>
              <path d="M 0 0 L 2 0" stroke="red"/>
            </svg>"#,
        );
        assert_eq!(d.shapes.len(), 1);
        assert_eq!(d.shapes[0].pts[1], (2.0, 0.0));
        assert!(d.warnings.len() >= 4, "{:?}", d.warnings);
    }

    #[test]
    fn entities_are_never_expanded_and_declaring_one_is_an_error() {
        let bomb = r#"<?xml version="1.0"?><!DOCTYPE svg [<!ENTITY a "aaaa"><!ENTITY b "&a;&a;&a;">]><svg><path d="M0 0 L1 0" stroke="red"/></svg>"#;
        assert!(read_svg(bomb.as_bytes(), &mm())
            .unwrap_err()
            .contains("entities"));
        let external = r#"<!DOCTYPE svg [<!ENTITY x SYSTEM "file:///etc/passwd">]><svg/>"#;
        assert!(read_svg(external.as_bytes(), &mm()).is_err());
        // a doctype without entities is skipped, and the predefined ones in an attribute are decoded
        let d = read(
            r#"<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN" "http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd"><svg><path d="M0 0 L1 0" stroke="red" id="a&amp;b&#x41;&other;"/></svg>"#,
        );
        assert_eq!(d.shapes.len(), 1);
        assert_eq!(decode("a&amp;b&#x41;&#66;&other;&lt;"), "a&bAB&other;<");
        assert_eq!(decode("a & b &;"), "a & b &;");
    }

    #[test]
    fn the_limits_stop_the_reading() {
        let deep = format!("<svg>{}{}</svg>", "<g>".repeat(10), "</g>".repeat(10));
        let lim = |f: fn(&mut ReadOptions)| {
            let mut o = mm();
            f(&mut o);
            o
        };
        assert!(read_svg(deep.as_bytes(), &lim(|o| o.limits.max_depth = 5))
            .unwrap_err()
            .contains("max_depth"));
        assert!(read_svg(deep.as_bytes(), &mm()).is_ok());
        assert!(read_svg(deep.as_bytes(), &lim(|o| o.limits.max_nodes = 5))
            .unwrap_err()
            .contains("max_nodes"));
        assert!(
            read_svg(deep.as_bytes(), &lim(|o| o.limits.max_file_bytes = 10))
                .unwrap_err()
                .contains("max_file_bytes")
        );
        let many = r#"<svg><path d="M0 0 L1 0 L2 0 L3 0 L4 0 L5 0" stroke="red"/></svg>"#;
        assert!(read_svg(many.as_bytes(), &lim(|o| o.limits.max_points = 4))
            .unwrap_err()
            .contains("max_points"));
        let shapes =
            r#"<svg><path d="M0 0 L1 0" stroke="red"/><path d="M0 0 L2 0" stroke="red"/></svg>"#;
        assert!(
            read_svg(shapes.as_bytes(), &lim(|o| o.limits.max_shapes = 1))
                .unwrap_err()
                .contains("max_shapes")
        );
    }

    #[test]
    fn a_long_curve_is_cut_at_the_limit_of_pieces() {
        let o = ReadOptions {
            tolerance_mm: 0.001,
            limits: crate::vector::Limits {
                max_curve_segments: 7,
                ..crate::vector::Limits::default()
            },
            ..mm()
        };
        let d = read_svg(
            br#"<svg><path d="M0 0 C 0 100 100 100 100 0" stroke="red"/></svg>"#,
            &o,
        )
        .unwrap();
        // the first point and 7 pieces
        assert_eq!(d.shapes[0].pts.len(), 8);
    }

    #[test]
    fn things_that_are_not_svg_or_are_cut_short_are_errors() {
        for bad in [
            "",
            "hello",
            "<svg",
            "<svg><g></svg>",
            "<svg></g></svg>",
            "<svg><path d=\"M0 0\" stroke=red/></svg>",
            "<svg><!-- never closed",
            "<g><path d=\"M0 0 L1 1\" stroke=\"red\"/></g>",
            "<svg a=\"1\" a",
        ] {
            assert!(read_svg(bad.as_bytes(), &mm()).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn a_file_cut_at_any_place_or_made_of_garbage_never_panics() {
        let svg = r##"<?xml version="1.0"?><svg xmlns="http://www.w3.org/2000/svg" width="10mm" height="10mm" viewBox="0 0 10 10"><g transform="translate(1 1)"><path d="M0 0 C 1 2 3 4 5 6 S 1 1 2 2 Q 1 1 2 2 T 3 3 A 1 2 3 1 0 4 4 Z" stroke="#f00" fill="none"/><circle cx="5" cy="5" r="2"/><rect x="1" y="1" width="3" height="3" rx="1"/><polygon points="0,0 1,1 2,0"/></g></svg>"##;
        for end in 0..=svg.len() {
            let _ = read_svg(&svg.as_bytes()[..end], &mm());
        }
        // random bytes and random damage to the valid file
        let mut seed = 12345u64;
        let mut next = move || {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (seed >> 33) as usize
        };
        for _ in 0..300 {
            let mut bytes = svg.as_bytes().to_vec();
            for _ in 0..3 {
                let at = next() % bytes.len();
                bytes[at] = (next() % 256) as u8;
            }
            let _ = read_svg(&bytes, &mm());
        }
        for _ in 0..100 {
            let junk: Vec<u8> = (0..200).map(|_| (next() % 256) as u8).collect();
            let _ = read_svg(&junk, &mm());
        }
    }
}
