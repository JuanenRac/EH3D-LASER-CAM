//! The state of the window without a screen: the picture that is open, the settings, the engraving (or the calibration grid) that is worked out, the passes of the laser to draw, the
//! file to write and the status line. The window (`gui`) only draws this and passes on what the person does, so everything that can be decided is decided here, in code that is tested.
//! The engraving is made with the same functions as the command line (`dither`, `engrave`, `grid`) and written with the same header.

use crate::calibrate::{grid, Grid};
use crate::dither::{dither, Method};
use crate::gcode_out::{header, sha256_hex, Header};
use crate::image::{from_bytes, Gray};
use crate::lang::{fill, Lang, Msg};
use crate::raster::{engrave, overscan_for, Params};
use crate::viewport::Viewport;
use std::path::{Path, PathBuf};

/// The largest file the window reads.
pub const MAX_FILE_BYTES: u64 = 256 * 1024 * 1024;

/// The most passes (pieces of the path of the head) that are kept to be drawn.
pub const MAX_SEGMENTS: usize = 300_000;

/// What a file chosen in the window is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Job {
    OpenImage,
    ExportGcode,
}

impl Job {
    pub fn is_export(self) -> bool {
        self == Job::ExportGcode
    }

    /// The extensions the list of files shows.
    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            Job::OpenImage => &["png", "pgm"],
            Job::ExportGcode => &["nc", "ngc", "gcode", "gc", "tap", "txt"],
        }
    }

    pub fn title(self) -> Msg {
        match self {
            Job::OpenImage => Msg::TitleOpenImage,
            Job::ExportGcode => Msg::TitleExport,
        }
    }
}

/// What the program makes: the engraving of a picture or the grid that finds the power and the feed that suit a material.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Engrave,
    Calibrate,
}

/// How a grey picture becomes dots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    FloydSteinberg,
    Atkinson,
    Threshold,
}

impl Kind {
    pub const ALL: [Kind; 3] = [Kind::FloydSteinberg, Kind::Atkinson, Kind::Threshold];

    pub fn label(self) -> Msg {
        match self {
            Kind::FloydSteinberg => Msg::DitherFs,
            Kind::Atkinson => Msg::DitherAtkinson,
            Kind::Threshold => Msg::DitherThreshold,
        }
    }
}

/// A setting that is typed in a box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    WidthMm,
    DotMm,
    Threshold,
    Feed,
    Power,
    SMax,
    Overscan,
    Accel,
    OriginX,
    OriginY,
    Powers,
    Feeds,
}

/// The group of the side panel a setting belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Image,
    Laser,
    Calibration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub key: Key,
    pub label: Msg,
    pub section: Section,
    pub text: String,
}

fn default_fields() -> Vec<Field> {
    let f = |key, label, section, text: &str| Field {
        key,
        label,
        section,
        text: text.to_string(),
    };
    vec![
        f(Key::WidthMm, Msg::WidthMm, Section::Image, "50"),
        f(Key::DotMm, Msg::DotMm, Section::Image, "0.1"),
        f(Key::Threshold, Msg::Threshold, Section::Image, "128"),
        f(Key::Feed, Msg::Feed, Section::Laser, "3000"),
        f(Key::Power, Msg::Power, Section::Laser, "50"),
        f(Key::SMax, Msg::SMax, Section::Laser, "1000"),
        f(Key::Overscan, Msg::Overscan, Section::Laser, "2"),
        f(Key::Accel, Msg::Accel, Section::Laser, "1000"),
        f(Key::OriginX, Msg::OriginX, Section::Laser, "0"),
        f(Key::OriginY, Msg::OriginY, Section::Laser, "0"),
        f(
            Key::Powers,
            Msg::Powers,
            Section::Calibration,
            "20,40,60,80",
        ),
        f(
            Key::Feeds,
            Msg::Feeds,
            Section::Calibration,
            "1000,2000,3000",
        ),
    ]
}

/// A straight pass of the head, seen from above.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Seg {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
    /// The laser is on.
    pub burn: bool,
    /// The power, from 0 to 1 of the full power.
    pub level: f64,
}

/// The passes of a G-code program of the laser, read from its text (the same text that is written), and whether the list was cut short at [`MAX_SEGMENTS`]. A move with `G1` is a
/// pass; it burns when the spindle value `S` is more than 0. `smax` is the value that is full power.
pub fn passes(gcode: &str, smax: u32) -> (Vec<Seg>, bool) {
    let mut segs = Vec::new();
    let (mut x, mut y, mut s) = (0.0_f64, 0.0_f64, 0.0_f64);
    let mut line_mode = false;
    for raw in gcode.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('(') {
            continue;
        }
        let (mut nx, mut ny) = (x, y);
        let mut moved = false;
        for token in line.split_whitespace() {
            let mut chars = token.chars();
            let Some(letter) = chars.next() else {
                continue;
            };
            let Ok(v) = chars.as_str().parse::<f64>() else {
                continue;
            };
            if !v.is_finite() {
                continue;
            }
            match letter.to_ascii_uppercase() {
                'G' if v == 0.0 => line_mode = false,
                'G' if v == 1.0 => line_mode = true,
                'X' => {
                    nx = v;
                    moved = true;
                }
                'Y' => {
                    ny = v;
                    moved = true;
                }
                'S' => s = v,
                _ => {}
            }
        }
        if moved && line_mode && (nx - x).hypot(ny - y) > 0.0 {
            if segs.len() >= MAX_SEGMENTS {
                return (segs, true);
            }
            segs.push(Seg {
                x0: x,
                y0: y,
                x1: nx,
                y1: ny,
                burn: s > 0.0,
                level: (s / f64::from(smax.max(1))).clamp(0.0, 1.0),
            });
        }
        x = nx;
        y = ny;
    }
    (segs, false)
}

/// The colour of a burning pass between the lowest power (0) and the full power (1): dark red, red, orange, yellow.
pub fn fire(t: f64) -> [u8; 3] {
    const STOPS: [[f64; 3]; 4] = [
        [0.45, 0.05, 0.05],
        [0.95, 0.15, 0.10],
        [1.00, 0.60, 0.10],
        [1.00, 0.95, 0.55],
    ];
    let t = if t.is_finite() {
        t.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let pos = t * 3.0;
    let i = (pos.floor() as usize).min(2);
    let f = pos - i as f64;
    std::array::from_fn(|k| {
        let v = STOPS[i][k] + (STOPS[i + 1][k] - STOPS[i][k]) * f;
        (v * 255.0).round() as u8
    })
}

/// The bytes of a file, if it is not too big.
fn read_file(path: &Path) -> Result<Vec<u8>, String> {
    let shown = path.display();
    let meta = std::fs::metadata(path).map_err(|e| format!("{shown}: {e}"))?;
    if meta.len() > MAX_FILE_BYTES {
        return Err(format!("{shown}: the file is larger than the window reads"));
    }
    std::fs::read(path).map_err(|e| format!("{shown}: {e}"))
}

/// Written beside and renamed, so that a file is never left half written.
fn write_atomic(path: &Path, text: &str) -> Result<(), String> {
    let tmp = PathBuf::from(format!("{}.tmp", path.display()));
    std::fs::write(&tmp, text)
        .and_then(|()| std::fs::rename(&tmp, path))
        .map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            format!("{}: {e}", path.display())
        })
}

/// True when two paths name the same file (compared once resolved; by their text when one does not exist yet).
fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

fn name_of(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

/// The picture that is open.
#[derive(Debug, Clone)]
pub struct Picture {
    pub path: PathBuf,
    pub name: String,
    pub bytes: Vec<u8>,
    pub gray: Gray,
}

/// What was worked out, for the status line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Summary {
    Engraved {
        dots_w: usize,
        dots_h: usize,
        width_mm: f64,
        height_mm: f64,
        rows: usize,
        runs: usize,
        burn_mm: f64,
        seconds: f64,
    },
    Calibrated {
        squares: usize,
    },
}

/// The G-code that was worked out and what goes in its header.
#[derive(Debug, Clone)]
pub struct Done {
    pub gcode: String,
    /// The name of the picture and its SHA-256 (none for a calibration grid).
    pub source: Option<(String, String)>,
    pub tools: String,
    pub s_max: u32,
    /// The width of a line of burning, for the picture.
    pub line_mm: f64,
    pub summary: Summary,
}

/// What the status line says.
#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    Ready,
    ImageOpened { name: String, w: usize, h: usize },
    Done(Summary),
    Written(String),
    NeedImage,
    BadValue(Msg),
    SameFile(String),
    Failed(String),
}

impl Status {
    pub fn is_problem(&self) -> bool {
        matches!(
            self,
            Status::NeedImage | Status::BadValue(_) | Status::SameFile(_) | Status::Failed(_)
        )
    }

    pub fn text(&self, lang: Lang) -> String {
        match self {
            Status::Ready => Msg::Ready.tr(lang).to_string(),
            Status::ImageOpened { name, w, h } => fill(
                Msg::ImageOpened.tr(lang),
                &[name, &w.to_string(), &h.to_string()],
            ),
            Status::Done(Summary::Engraved {
                dots_w,
                dots_h,
                width_mm,
                height_mm,
                rows,
                runs,
                burn_mm,
                seconds,
            }) => fill(
                Msg::Engraved.tr(lang),
                &[
                    &dots_w.to_string(),
                    &dots_h.to_string(),
                    &format!("{width_mm:.1}"),
                    &format!("{height_mm:.1}"),
                    &rows.to_string(),
                    &runs.to_string(),
                    &format!("{burn_mm:.0}"),
                    &format!("{seconds:.0}"),
                ],
            ),
            Status::Done(Summary::Calibrated { squares }) => {
                fill(Msg::Calibrated.tr(lang), &[&squares.to_string()])
            }
            Status::Written(name) => fill(Msg::Written.tr(lang), &[name]),
            Status::NeedImage => Msg::NeedImage.tr(lang).to_string(),
            Status::BadValue(label) => fill(Msg::BadValue.tr(lang), &[label.tr(lang)]),
            Status::SameFile(name) => fill(Msg::SameFile.tr(lang), &[name]),
            Status::Failed(e) => fill(Msg::Failed.tr(lang), &[e]),
        }
    }
}

/// The state of the whole window.
#[derive(Debug)]
pub struct Window {
    pub lang: Lang,
    pub fields: Vec<Field>,
    pub mode: Mode,
    pub kind: Kind,
    /// The overscan is worked out from the feed and the acceleration instead of being typed.
    pub overscan_auto: bool,
    pub picture: Option<Picture>,
    pub done: Option<Done>,
    pub segs: Vec<Seg>,
    /// The picture of the passes was cut short at [`MAX_SEGMENTS`].
    pub truncated: bool,
    /// A setting changed after the G-code was worked out.
    pub stale: bool,
    pub status: Status,
    pub viewport: Viewport,
    pub show_travel: bool,
}

impl Window {
    pub fn new(lang: Lang) -> Window {
        Window {
            lang,
            fields: default_fields(),
            mode: Mode::Engrave,
            kind: Kind::FloydSteinberg,
            overscan_auto: false,
            picture: None,
            done: None,
            segs: Vec::new(),
            truncated: false,
            stale: false,
            status: Status::Ready,
            viewport: Viewport::default(),
            show_travel: true,
        }
    }

    pub fn has_work(&self) -> bool {
        self.picture.is_some() || self.done.is_some()
    }

    /// Whether the G-code can be worked out now.
    pub fn can_compute(&self) -> bool {
        self.mode == Mode::Calibrate || self.picture.is_some()
    }

    fn mark_stale(&mut self) {
        if self.done.is_some() {
            self.stale = true;
        }
    }

    /// Writes a setting that the person typed.
    pub fn set_field(&mut self, i: usize, text: String) {
        if let Some(f) = self.fields.get_mut(i) {
            if f.text != text {
                f.text = text;
                self.mark_stale();
            }
        }
    }

    pub fn set_mode(&mut self, mode: Mode) {
        if self.mode != mode {
            self.mode = mode;
            self.mark_stale();
        }
    }

    pub fn set_kind(&mut self, kind: Kind) {
        if self.kind != kind {
            self.kind = kind;
            self.mark_stale();
        }
    }

    pub fn set_overscan_auto(&mut self, on: bool) {
        if self.overscan_auto != on {
            self.overscan_auto = on;
            self.mark_stale();
        }
    }

    fn field(&self, key: Key) -> &Field {
        // every key has its field: `default_fields` makes one for each
        self.fields
            .iter()
            .find(|f| f.key == key)
            .unwrap_or(&self.fields[0])
    }

    fn number(&self, key: Key) -> Result<f64, Status> {
        let f = self.field(key);
        f.text
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
            .ok_or(Status::BadValue(f.label))
    }

    fn whole(&self, key: Key, lo: f64, hi: f64) -> Result<u32, Status> {
        let v = self.number(key)?;
        if v.fract() == 0.0 && (lo..=hi).contains(&v) {
            Ok(v as u32)
        } else {
            Err(Status::BadValue(self.field(key).label))
        }
    }

    /// A list of numbers separated by commas.
    fn list(&self, key: Key) -> Result<Vec<f64>, Status> {
        let f = self.field(key);
        f.text
            .split(',')
            .map(|p| {
                p.trim()
                    .parse::<f64>()
                    .ok()
                    .filter(|v| v.is_finite())
                    .ok_or(Status::BadValue(f.label))
            })
            .collect()
    }

    /// The parameters of the laser from the settings, checked the way the command line checks them.
    pub fn params(&self) -> Result<Params, Status> {
        let feed = self.number(Key::Feed)?;
        let overscan = if self.overscan_auto {
            overscan_for(feed, self.number(Key::Accel)?).map_err(Status::Failed)?
        } else {
            self.number(Key::Overscan)?
        };
        let p = Params {
            pixel_mm: self.number(Key::DotMm)?,
            feed_mm_min: feed,
            s_max: self.whole(Key::SMax, 1.0, 100_000.0)?,
            power_percent: self.number(Key::Power)?,
            overscan_mm: overscan,
            origin: (self.number(Key::OriginX)?, self.number(Key::OriginY)?),
        };
        p.validate().map_err(Status::Failed)?;
        Ok(p)
    }

    /// The way the picture becomes dots.
    pub fn method(&self) -> Result<Method, Status> {
        Ok(match self.kind {
            Kind::FloydSteinberg => Method::FloydSteinberg,
            Kind::Atkinson => Method::Atkinson,
            Kind::Threshold => {
                Method::Threshold(self.number(Key::Threshold)?.clamp(0.0, 255.0) as u8)
            }
        })
    }

    /// The name of the picture that is open.
    pub fn picture_name(&self) -> Option<String> {
        self.picture.as_ref().map(|p| p.name.clone())
    }

    /// The folder of the picture that is open, where the window of files starts.
    pub fn work_folder(&self) -> Option<PathBuf> {
        self.picture
            .as_ref()
            .and_then(|p| p.path.parent().map(Path::to_path_buf))
            .filter(|p| !p.as_os_str().is_empty())
    }

    /// The name offered for the G-code to write.
    pub fn suggested_output_for(&self, job: Job) -> Option<PathBuf> {
        if job != Job::ExportGcode {
            return None;
        }
        match (&self.picture, self.mode) {
            (Some(p), Mode::Engrave) => {
                let stem = p.path.file_stem()?.to_string_lossy().into_owned();
                Some(p.path.with_file_name(format!("{stem}.nc")))
            }
            _ => Some(PathBuf::from("calibration.nc")),
        }
    }

    /// Opens a PNG or PGM picture.
    pub fn open_image(&mut self, path: &Path) -> bool {
        let loaded = read_file(path).and_then(|bytes| {
            let gray = from_bytes(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
            Ok((bytes, gray))
        });
        match loaded {
            Ok((bytes, gray)) => {
                self.status = Status::ImageOpened {
                    name: name_of(path),
                    w: gray.w,
                    h: gray.h,
                };
                self.picture = Some(Picture {
                    path: path.to_path_buf(),
                    name: name_of(path),
                    bytes,
                    gray,
                });
                // what was worked out from the old picture is not valid any more
                if self.mode == Mode::Engrave {
                    self.done = None;
                    self.segs.clear();
                    self.stale = false;
                    self.truncated = false;
                }
                true
            }
            Err(e) => {
                self.status = Status::Failed(e);
                false
            }
        }
    }

    /// Opens a file dropped on the window: the only thing that can be opened is a picture.
    pub fn open_dropped(&mut self, path: &Path) -> bool {
        self.open_image(path)
    }

    /// Works the G-code out for what is chosen (the engraving or the grid), like the command line, and the passes to draw.
    pub fn compute(&mut self) -> bool {
        let made = match self.mode {
            Mode::Engrave => self.engrave_job(),
            Mode::Calibrate => self.calibrate_job(),
        };
        match made {
            Ok(done) => {
                let (segs, cut) = passes(&done.gcode, done.s_max);
                self.segs = segs;
                self.truncated = cut;
                self.status = Status::Done(done.summary);
                self.done = Some(done);
                self.stale = false;
                true
            }
            Err(s) => {
                self.status = s;
                false
            }
        }
    }

    fn engrave_job(&self) -> Result<Done, Status> {
        let pic = self.picture.as_ref().ok_or(Status::NeedImage)?;
        let width = self.number(Key::WidthMm)?;
        if !(width > 0.0 && width <= 2000.0) {
            return Err(Status::BadValue(Msg::WidthMm));
        }
        let p = self.params()?;
        let method = self.method()?;
        let w = (width / p.pixel_mm).round().max(1.0) as usize;
        let h = ((w as f64) * pic.gray.h as f64 / pic.gray.w as f64)
            .round()
            .max(1.0) as usize;
        let dots = dither(&pic.gray.resize(w, h).map_err(Status::Failed)?, method);
        let e = engrave(&dots, &p).map_err(Status::Failed)?;
        Ok(Done {
            gcode: e.gcode,
            source: Some((pic.name.clone(), sha256_hex(&pic.bytes))),
            tools: format!("laser, S 0-{} ({} %)", p.s_max, p.power_percent),
            s_max: p.s_max,
            line_mm: p.pixel_mm,
            summary: Summary::Engraved {
                dots_w: dots.w,
                dots_h: dots.h,
                width_mm: dots.w as f64 * p.pixel_mm,
                height_mm: dots.h as f64 * p.pixel_mm,
                rows: e.rows,
                runs: e.runs,
                burn_mm: e.burn_mm,
                seconds: e.seconds,
            },
        })
    }

    fn calibrate_job(&self) -> Result<Done, Status> {
        let powers = self.list(Key::Powers)?;
        let feeds = self.list(Key::Feeds)?;
        let g = Grid {
            s_max: self.whole(Key::SMax, 1.0, 100_000.0)?,
            ..Grid::default()
        };
        let gcode = grid(&powers, &feeds, &g).map_err(Status::Failed)?;
        Ok(Done {
            gcode,
            source: None,
            tools: format!("laser, S 0-{}, grid of power against feed", g.s_max),
            s_max: g.s_max,
            line_mm: g.line_spacing_mm,
            summary: Summary::Calibrated {
                squares: powers.len() * feeds.len(),
            },
        })
    }

    /// Writes the G-code with the standard header (the G-code is worked out again first when a setting changed). It is never written over the picture.
    pub fn export(&mut self, path: &Path) -> bool {
        if (self.done.is_none() || self.stale) && !self.compute() {
            return false;
        }
        if self
            .picture
            .as_ref()
            .is_some_and(|p| same_file(&p.path, path))
        {
            self.status = Status::SameFile(path.display().to_string());
            return false;
        }
        let Some(done) = &self.done else {
            self.status = Status::NeedImage;
            return false;
        };
        let head = header(&Header {
            generator: "phoenix-laser",
            version: env!("CARGO_PKG_VERSION"),
            source: done.source.as_ref().map(|(n, h)| (n.as_str(), h.clone())),
            tools: &done.tools,
        });
        match write_atomic(path, &format!("{head}{}", done.gcode)) {
            Ok(()) => {
                self.status = Status::Written(name_of(path));
                true
            }
            Err(e) => {
                self.status = Status::Failed(e);
                false
            }
        }
    }

    /// Does what a chosen file is for.
    pub fn run_job(&mut self, job: Job, path: &Path) -> bool {
        match job {
            Job::OpenImage => self.open_image(path),
            Job::ExportGcode => self.export(path),
        }
    }

    /// The area the work covers, from the corner of the origin: the passes, or the size the picture will have.
    pub fn bounds(&self) -> Option<((f64, f64), (f64, f64))> {
        if !self.segs.is_empty() {
            let mut min = (f64::INFINITY, f64::INFINITY);
            let mut max = (f64::NEG_INFINITY, f64::NEG_INFINITY);
            for s in &self.segs {
                for (x, y) in [(s.x0, s.y0), (s.x1, s.y1)] {
                    min = (min.0.min(x), min.1.min(y));
                    max = (max.0.max(x), max.1.max(y));
                }
            }
            return Some((min, max));
        }
        let pic = self.picture.as_ref()?;
        let width = self.number(Key::WidthMm).ok()?;
        let origin = (
            self.number(Key::OriginX).ok()?,
            self.number(Key::OriginY).ok()?,
        );
        let height = width * pic.gray.h as f64 / pic.gray.w as f64;
        Some((origin, (origin.0 + width, origin.1 + height)))
    }

    /// Shows everything in a picture of the size given.
    pub fn fit(&mut self, width_px: f64, height_px: f64) {
        if let Some((min, max)) = self.bounds() {
            self.viewport = Viewport::fit(min, max, width_px, height_px, 24.0);
        }
    }

    /// What the pointer is over.
    pub fn cursor_text(&self, x: f64, y: f64) -> String {
        fill(
            Msg::CursorPlain.tr(self.lang),
            &[&format!("{x:.2}"), &format!("{y:.2}")],
        )
    }

    /// The facts about the picture (what the `info` command prints).
    pub fn info_line(&self) -> Option<String> {
        let g = &self.picture.as_ref()?.gray;
        let mean = g.data.iter().map(|&v| u64::from(v)).sum::<u64>() as f64 / g.data.len() as f64;
        Some(fill(
            Msg::InfoImage.tr(self.lang),
            &[&g.w.to_string(), &g.h.to_string(), &format!("{mean:.1}")],
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Four rows of eight dots: the left half black, the right half white.
    const PICTURE: &str = "P2\n8 4\n255\n0 0 0 0 255 255 255 255\n0 0 0 0 255 255 255 255\n0 0 0 0 255 255 255 255\n0 0 0 0 255 255 255 255\n";

    /// A folder of its own for each test, removed at the end.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(tag: &str) -> Scratch {
            let dir = std::env::temp_dir()
                .join(format!("eh3d-laser-window-{}-{tag}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Scratch(dir)
        }

        fn write(&self, name: &str, text: &str) -> PathBuf {
            let p = self.0.join(name);
            fs::write(&p, text).unwrap();
            p
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn set(w: &mut Window, key: Key, text: &str) {
        let i = w.fields.iter().position(|f| f.key == key).unwrap();
        w.set_field(i, text.to_string());
    }

    /// A window with the picture open, a width of 0.8 mm (eight dots) and the threshold kind of dots.
    fn ready(tag: &str) -> (Scratch, Window) {
        let s = Scratch::new(tag);
        let p = s.write("pic.pgm", PICTURE);
        let mut w = Window::new(Lang::En);
        assert!(w.open_image(&p));
        set(&mut w, Key::WidthMm, "0.8");
        w.set_kind(Kind::Threshold);
        (s, w)
    }

    #[test]
    fn opening_a_picture_keeps_its_bytes_and_says_its_size() {
        let s = Scratch::new("open");
        let p = s.write("pic.pgm", PICTURE);
        let mut w = Window::new(Lang::En);
        assert!(w.open_image(&p));
        assert_eq!(w.picture_name().as_deref(), Some("pic.pgm"));
        assert_eq!(w.picture.as_ref().unwrap().bytes, PICTURE.as_bytes());
        assert!(w.status.text(Lang::En).contains("pic.pgm: 8 x 4 pixels"));
        assert!(w.has_work());
        assert_eq!(
            w.info_line().as_deref(),
            Some("8 x 4 pixels, mean grey 127.5 of 255")
        );
    }

    #[test]
    fn a_file_that_is_not_a_picture_is_refused_with_its_name() {
        let s = Scratch::new("bad");
        let p = s.write("notes.png", "this is not a picture");
        let mut w = Window::new(Lang::En);
        assert!(!w.open_image(&p));
        assert!(w.picture.is_none());
        assert!(w.status.is_problem());
        assert!(w.status.text(Lang::En).contains("notes.png"));
        assert!(!w.open_dropped(Path::new("/no/such/folder/pic.png")));
    }

    #[test]
    fn the_window_engraves_like_the_command_line() {
        let (_s, mut w) = ready("same");
        assert!(w.compute());
        let img = from_bytes(PICTURE.as_bytes()).unwrap();
        let dots = dither(&img.resize(8, 4).unwrap(), Method::Threshold(128));
        let expected = engrave(&dots, &Params::default()).unwrap();
        let done = w.done.as_ref().unwrap();
        assert_eq!(done.gcode, expected.gcode);
        assert_eq!(
            done.summary,
            Summary::Engraved {
                dots_w: 8,
                dots_h: 4,
                width_mm: 0.8,
                height_mm: 0.4,
                rows: expected.rows,
                runs: expected.runs,
                burn_mm: expected.burn_mm,
                seconds: expected.seconds,
            }
        );
        assert_eq!((expected.rows, expected.runs), (4, 4));
        assert!(!w.stale);
    }

    #[test]
    fn the_passes_drawn_are_the_ones_of_the_gcode() {
        let (_s, mut w) = ready("passes");
        assert!(w.compute());
        let burning = w.segs.iter().filter(|s| s.burn).count();
        let travelling = w.segs.iter().filter(|s| !s.burn).count();
        assert_eq!(burning, 4);
        assert!(travelling >= 4);
        // half of the full power
        assert!(w
            .segs
            .iter()
            .filter(|s| s.burn)
            .all(|s| (s.level - 0.5).abs() < 1e-9));
        assert!(!w.truncated);
        let ((x0, _), (x1, _)) = w.bounds().unwrap();
        assert!(x0 < 0.0 && x1 > 0.8, "the overscan runs past the work");
    }

    #[test]
    fn a_pass_burns_only_where_the_power_is_on() {
        let gcode = "G21\nM4 S0\nG0 X0 Y1\nG1 X5 S0\nG1 X8 S500\nG1 X10 S0\n";
        let (segs, cut) = passes(gcode, 1000);
        assert!(!cut);
        assert_eq!(segs.len(), 3);
        assert!(!segs[0].burn && segs[1].burn && !segs[2].burn);
        assert_eq!((segs[1].x0, segs[1].x1, segs[1].y0), (5.0, 8.0, 1.0));
        assert_eq!(segs[1].level, 0.5);
    }

    #[test]
    fn comments_and_words_that_are_not_numbers_are_left_out_of_the_picture() {
        let (segs, _) = passes("; header\n(a note)\nG1 Xabc\nG1 X1 Sxyz\nF3000\n", 1000);
        assert_eq!(segs.len(), 1);
        assert!(!segs[0].burn);
    }

    #[test]
    fn a_huge_gcode_is_cut_short_for_the_picture() {
        let mut text = String::from("G0 X0 Y0\n");
        for k in 1..=(MAX_SEGMENTS + 5) {
            text.push_str(&format!("G1 X{k} S0\n"));
        }
        let (segs, cut) = passes(&text, 1000);
        assert!(cut);
        assert_eq!(segs.len(), MAX_SEGMENTS);
    }

    #[test]
    fn the_colours_go_from_dark_red_to_yellow() {
        let (low, high) = (fire(0.0), fire(1.0));
        assert!(low[0] > low[1] && low[0] > low[2], "the lowest is red");
        assert!(high[1] > low[1], "the highest is lighter");
        assert_eq!(fire(f64::NAN), fire(0.0));
        assert_eq!(fire(-4.0), fire(0.0));
        assert_eq!(fire(9.0), fire(1.0));
    }

    #[test]
    fn a_setting_that_changes_after_the_work_makes_it_stale() {
        let (_s, mut w) = ready("stale");
        assert!(w.compute());
        set(&mut w, Key::Power, "70");
        assert!(w.stale);
        assert!(w.compute());
        assert!(!w.stale);
        w.set_kind(Kind::Atkinson);
        assert!(w.stale);
        assert!(w.compute());
        w.set_overscan_auto(true);
        assert!(w.stale);
        assert!(w.compute());
        w.set_mode(Mode::Calibrate);
        assert!(w.stale);
    }

    #[test]
    fn the_overscan_can_be_worked_out_from_the_acceleration() {
        let (_s, mut w) = ready("auto");
        w.set_overscan_auto(true);
        let p = w.params().unwrap();
        assert_eq!(p.overscan_mm, overscan_for(3000.0, 1000.0).unwrap());
        set(&mut w, Key::Accel, "0");
        assert!(matches!(w.params().unwrap_err(), Status::Failed(_)));
    }

    #[test]
    fn the_settings_are_checked_like_the_command_line_checks_them() {
        let mut w = Window::new(Lang::En);
        assert!(w.params().is_ok());
        set(&mut w, Key::Power, "150");
        assert!(matches!(w.params().unwrap_err(), Status::Failed(_)));
        set(&mut w, Key::Power, "50");
        set(&mut w, Key::SMax, "0");
        assert_eq!(w.params().unwrap_err(), Status::BadValue(Msg::SMax));
        set(&mut w, Key::SMax, "1000.5");
        assert_eq!(w.params().unwrap_err(), Status::BadValue(Msg::SMax));
        set(&mut w, Key::SMax, "1000");
        set(&mut w, Key::Feed, "fast");
        assert_eq!(w.params().unwrap_err(), Status::BadValue(Msg::Feed));
    }

    #[test]
    fn the_threshold_is_held_between_0_and_255() {
        let mut w = Window::new(Lang::En);
        w.set_kind(Kind::Threshold);
        set(&mut w, Key::Threshold, "999");
        assert_eq!(w.method().unwrap(), Method::Threshold(255));
        set(&mut w, Key::Threshold, "-4");
        assert_eq!(w.method().unwrap(), Method::Threshold(0));
        set(&mut w, Key::Threshold, "dark");
        assert_eq!(w.method().unwrap_err(), Status::BadValue(Msg::Threshold));
        w.set_kind(Kind::Atkinson);
        assert_eq!(w.method().unwrap(), Method::Atkinson);
    }

    #[test]
    fn engraving_needs_a_picture_and_a_sensible_width() {
        let mut w = Window::new(Lang::En);
        assert!(!w.compute());
        assert_eq!(w.status, Status::NeedImage);
        let (_s, mut w) = ready("width");
        set(&mut w, Key::WidthMm, "0");
        assert!(!w.compute());
        assert_eq!(w.status, Status::BadValue(Msg::WidthMm));
        set(&mut w, Key::WidthMm, "5000");
        assert!(!w.compute());
        assert_eq!(w.status, Status::BadValue(Msg::WidthMm));
    }

    #[test]
    fn a_picture_that_would_need_too_many_dots_is_refused_by_the_reader() {
        let (_s, mut w) = ready("toomany");
        set(&mut w, Key::WidthMm, "2000");
        set(&mut w, Key::DotMm, "0.01");
        assert!(!w.compute());
        assert!(matches!(w.status, Status::Failed(_)));
    }

    #[test]
    fn the_calibration_grid_is_the_one_of_the_command_line() {
        let mut w = Window::new(Lang::En);
        w.set_mode(Mode::Calibrate);
        assert!(w.can_compute());
        assert!(w.compute());
        let expected = grid(
            &[20.0, 40.0, 60.0, 80.0],
            &[1000.0, 2000.0, 3000.0],
            &Grid::default(),
        )
        .unwrap();
        assert_eq!(w.done.as_ref().unwrap().gcode, expected);
        assert_eq!(
            w.done.as_ref().unwrap().summary,
            Summary::Calibrated { squares: 12 }
        );
        assert!(w.segs.iter().filter(|s| s.burn).count() >= 12 * 80);
        // the squares of higher power are lighter
        let levels: Vec<f64> = w.segs.iter().filter(|s| s.burn).map(|s| s.level).collect();
        assert!(levels.iter().any(|&l| (l - 0.2).abs() < 1e-9));
        assert!(levels.iter().any(|&l| (l - 0.8).abs() < 1e-9));
    }

    #[test]
    fn a_calibration_with_a_list_that_is_not_numbers_says_which_one() {
        let mut w = Window::new(Lang::En);
        w.set_mode(Mode::Calibrate);
        set(&mut w, Key::Feeds, "1000,,3000");
        assert!(!w.compute());
        assert_eq!(w.status, Status::BadValue(Msg::Feeds));
        set(&mut w, Key::Feeds, "1000");
        set(&mut w, Key::Powers, "20,x");
        assert!(!w.compute());
        assert_eq!(w.status, Status::BadValue(Msg::Powers));
        set(&mut w, Key::Powers, "200");
        assert!(!w.compute());
        assert!(matches!(w.status, Status::Failed(_)));
    }

    #[test]
    fn the_exported_file_has_the_header_of_the_command_line() {
        let (s, mut w) = ready("export");
        let out = s.0.join("pic.nc");
        assert!(w.export(&out));
        let text = fs::read_to_string(&out).unwrap();
        assert!(text.starts_with("; generator: phoenix-laser "));
        assert!(text.contains("; units: mm (G21)\n"));
        let hash = sha256_hex(PICTURE.as_bytes());
        assert!(text.contains(&format!("; source: pic.pgm sha256:{hash}\n")));
        assert!(text.contains("; tools: laser, S 0-1000 (50 %)\n"));
        assert!(text.ends_with("M5\nM2\n"));
        assert!(!s.0.join("pic.nc.tmp").exists());
        assert!(w.status.text(Lang::En).contains("pic.nc"));
    }

    #[test]
    fn the_grid_is_written_with_no_source() {
        let s = Scratch::new("gridfile");
        let mut w = Window::new(Lang::En);
        w.set_mode(Mode::Calibrate);
        let out = s.0.join("grid.nc");
        assert!(w.export(&out));
        let text = fs::read_to_string(&out).unwrap();
        assert!(text.contains("; source: none\n"));
        assert!(text.contains("grid of power against feed"));
    }

    #[test]
    fn exporting_works_the_gcode_out_again_when_a_setting_changed() {
        let (s, mut w) = ready("again");
        assert!(w.compute());
        set(&mut w, Key::Power, "100");
        assert!(w.export(&s.0.join("hot.nc")));
        assert!(!w.stale);
        assert!(fs::read_to_string(s.0.join("hot.nc"))
            .unwrap()
            .contains("S1000"));
    }

    #[test]
    fn the_picture_is_never_written_over() {
        let (s, mut w) = ready("overpic");
        let picture = s.0.join("pic.pgm");
        assert!(!w.export(&picture));
        assert!(matches!(w.status, Status::SameFile(_)));
        assert_eq!(fs::read_to_string(&picture).unwrap(), PICTURE);
    }

    #[test]
    fn nothing_is_written_when_there_is_nothing_to_engrave() {
        let s = Scratch::new("nothing");
        let mut w = Window::new(Lang::En);
        assert!(!w.export(&s.0.join("x.nc")));
        assert!(!s.0.join("x.nc").exists());
    }

    #[test]
    fn the_name_offered_follows_the_picture() {
        let (s, mut w) = ready("name");
        assert_eq!(
            w.suggested_output_for(Job::ExportGcode),
            Some(s.0.join("pic.nc"))
        );
        assert_eq!(w.suggested_output_for(Job::OpenImage), None);
        assert_eq!(w.work_folder(), Some(s.0.clone()));
        w.set_mode(Mode::Calibrate);
        assert_eq!(
            w.suggested_output_for(Job::ExportGcode),
            Some(PathBuf::from("calibration.nc"))
        );
    }

    #[test]
    fn the_picture_fits_the_work_before_and_after_it_is_worked_out() {
        let (_s, mut w) = ready("fit");
        // before: the size the picture will have
        assert_eq!(w.bounds(), Some(((0.0, 0.0), (0.8, 0.4))));
        w.fit(800.0, 600.0);
        let (x, y) = w.viewport.to_screen(0.4, 0.2, 800.0, 600.0);
        assert!((x - 400.0).abs() < 1e-6 && (y - 300.0).abs() < 1e-6);
        assert!(w.compute());
        assert!(w.bounds().is_some());
        assert!(Window::new(Lang::En).bounds().is_none());
    }

    #[test]
    fn a_new_picture_drops_the_work_made_from_the_old_one() {
        let (s, mut w) = ready("newpic");
        assert!(w.compute());
        let other = s.write("other.pgm", PICTURE);
        assert!(w.open_image(&other));
        assert!(w.done.is_none() && w.segs.is_empty());
        assert_eq!(w.picture_name().as_deref(), Some("other.pgm"));
    }

    #[test]
    fn the_status_speaks_the_language_chosen() {
        let s = Status::Written("a.nc".into());
        assert_eq!(s.text(Lang::En), "Written: a.nc");
        assert_eq!(s.text(Lang::Es), "Escrito: a.nc");
        assert!(Status::BadValue(Msg::Feed)
            .text(Lang::De)
            .contains("Vorschub"));
        let done = Status::Done(Summary::Calibrated { squares: 6 });
        assert_eq!(done.text(Lang::En), "6 squares of the grid");
        assert_eq!(done.text(Lang::Pt), "6 quadrados da grelha");
    }

    #[test]
    fn each_job_lists_the_files_it_can_open() {
        assert!(Job::OpenImage.extensions().contains(&"png"));
        assert!(Job::OpenImage.extensions().contains(&"pgm"));
        assert!(Job::ExportGcode.is_export() && !Job::OpenImage.is_export());
        assert_eq!(Kind::ALL.len(), 3);
    }
}
