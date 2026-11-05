//! The state of the window without a screen: the picture that is open, the settings, the engraving (or the calibration grid) that is worked out, the passes of the laser to draw, the
//! file to write and the status line. The window (`gui`) only draws this and passes on what the person does, so everything that can be decided is decided here, in code that is tested.
//! The engraving is made with the same functions as the command line (`dither`, `engrave`, `grid`) and written with the same header.

use crate::calibrate::{grid, Grid};
use crate::cut::{layers as layer_list, plan as plan_cut};
use crate::dither::{dither, Method};
use crate::dxf::read_dxf;
use crate::gcode_out::{header, sha256_hex, Header};
use crate::image::{from_bytes, Gray};
use crate::lang::{fill, Lang, Msg};
use crate::raster::{engrave, overscan_for, Params};
use crate::svg::read_svg;
use crate::vconfig::{self, KerfSide, LayerMode, LayerParams, Settings};
use crate::vector::{Drawing, ReadOptions};
use crate::viewport::Viewport;
use std::path::{Path, PathBuf};

/// The largest file the window reads.
pub const MAX_FILE_BYTES: u64 = 256 * 1024 * 1024;

/// The most passes (pieces of the path of the head) that are kept to be drawn.
pub const MAX_SEGMENTS: usize = 300_000;

/// What a file chosen in the window is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Job {
    /// A picture (PNG, PGM) or a drawing (SVG, DXF).
    OpenImage,
    /// A settings file for the vector work.
    OpenConfig,
    ExportGcode,
}

impl Job {
    pub fn is_export(self) -> bool {
        self == Job::ExportGcode
    }

    /// The extensions the list of files shows.
    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            Job::OpenImage => &["png", "pgm", "svg", "dxf"],
            Job::OpenConfig => &["cfg"],
            Job::ExportGcode => &["nc", "ngc", "gcode", "gc", "tap", "txt"],
        }
    }

    pub fn title(self) -> Msg {
        match self {
            Job::OpenImage => Msg::TitleOpenImage,
            Job::OpenConfig => Msg::TitleOpenSettings,
            Job::ExportGcode => Msg::TitleExport,
        }
    }
}

/// What the program makes: the engraving of a picture, the grid that finds the power and the feed that suit a material, or the cutting and engraving of a vector drawing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Engrave,
    Calibrate,
    Cut,
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
    Dpi,
    Tolerance,
}

/// The group of the side panel a setting belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Image,
    Laser,
    Calibration,
    Vector,
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
        f(Key::Dpi, Msg::Dpi, Section::Vector, "96"),
        f(Key::Tolerance, Msg::Tolerance, Section::Vector, "0.01"),
    ]
}

/// A setting of a layer that is typed in a box of the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerField {
    Power,
    Speed,
    Passes,
    Focus,
    Kerf,
    Tabs,
    TabWidth,
    HatchAngle,
    HatchSpacing,
}

impl LayerField {
    pub const ALL: [LayerField; 9] = [
        LayerField::Power,
        LayerField::Speed,
        LayerField::Passes,
        LayerField::Focus,
        LayerField::Kerf,
        LayerField::Tabs,
        LayerField::TabWidth,
        LayerField::HatchAngle,
        LayerField::HatchSpacing,
    ];

    pub fn label(self) -> Msg {
        match self {
            LayerField::Power => Msg::LayerPower,
            LayerField::Speed => Msg::LayerSpeed,
            LayerField::Passes => Msg::Passes,
            LayerField::Focus => Msg::FocusStep,
            LayerField::Kerf => Msg::Kerf,
            LayerField::Tabs => Msg::Tabs,
            LayerField::TabWidth => Msg::TabWidth,
            LayerField::HatchAngle => Msg::HatchAngle,
            LayerField::HatchSpacing => Msg::HatchSpacing,
        }
    }
}

/// A row of the table of layers: a layer or a colour of the drawing with what is done with it.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerRow {
    pub key: String,
    pub shapes: usize,
    pub mode: LayerMode,
    pub kerf_side: KerfSide,
    /// The text of each box, in the order of [`LayerField::ALL`].
    pub fields: [String; 9],
}

impl LayerRow {
    fn new(key: &str, shapes: usize, p: &LayerParams) -> LayerRow {
        let t = |v: f64| format!("{v}");
        LayerRow {
            key: key.to_string(),
            shapes,
            mode: p.mode,
            kerf_side: p.kerf_side,
            fields: [
                t(p.power_percent),
                t(p.speed_mm_min),
                p.passes.to_string(),
                t(p.focus_step_mm),
                t(p.kerf_mm),
                p.tabs.to_string(),
                t(p.tab_width_mm),
                t(p.hatch_angle_deg),
                t(p.hatch_spacing_mm),
            ],
        }
    }
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
    passes_of(gcode, smax, false)
}

/// The same, and with `rapid` also the moves with `G0` (the travel between the contours of a drawing) as passes with the laser off.
pub fn passes_of(gcode: &str, smax: u32, rapid: bool) -> (Vec<Seg>, bool) {
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
        if moved && (line_mode || rapid) && (nx - x).hypot(ny - y) > 0.0 {
            if segs.len() >= MAX_SEGMENTS {
                return (segs, true);
            }
            segs.push(Seg {
                x0: x,
                y0: y,
                x1: nx,
                y1: ny,
                burn: line_mode && s > 0.0,
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
pub fn write_atomic(path: &Path, text: &str) -> Result<(), String> {
    let tmp = PathBuf::from(format!("{}.tmp", path.display()));
    std::fs::write(&tmp, text)
        .and_then(|()| std::fs::rename(&tmp, path))
        .map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            format!("{}: {e}", path.display())
        })
}

/// True when two paths name the same file (compared once resolved; by their text when one does not exist yet).
pub fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

/// The extension of a file in lower case ("" when it has none).
fn extension_of(path: &Path) -> String {
    path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// The bytes of a file, if it is not larger than `max` bytes.
fn read_file_up_to(path: &Path, max: u64) -> Result<Vec<u8>, String> {
    let shown = path.display();
    let meta = std::fs::metadata(path).map_err(|e| format!("{shown}: {e}"))?;
    if meta.len() > max {
        return Err(format!(
            "{shown}: the file is larger than {max} bytes (max_file_bytes)"
        ));
    }
    std::fs::read(path).map_err(|e| format!("{shown}: {e}"))
}

/// Reads a drawing according to the extension of its file.
fn parse_drawing(bytes: &[u8], ext: &str, opts: &ReadOptions) -> Result<Drawing, String> {
    match ext {
        "svg" => read_svg(bytes, opts),
        "dxf" => read_dxf(bytes, opts),
        _ => Err("not a drawing this program reads (SVG, DXF)".to_string()),
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

/// The drawing that is open: the file as it was read (its bytes, for the hash of the header and to read it again when the dpi or the tolerance changes) and what it holds.
#[derive(Debug, Clone)]
pub struct VectorFile {
    pub path: PathBuf,
    pub name: String,
    /// `svg` or `dxf`.
    pub ext: String,
    pub bytes: Vec<u8>,
    pub drawing: Drawing,
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
    Cut {
        layers: usize,
        paths: usize,
        cut_mm: f64,
        engrave_mm: f64,
        travel_mm: f64,
        seconds: f64,
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
    /// What was skipped or could not be done, for the list of notes.
    pub notes: Vec<String>,
}

/// What the status line says.
#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    Ready,
    ImageOpened {
        name: String,
        w: usize,
        h: usize,
    },
    VectorOpened {
        name: String,
        shapes: usize,
        layers: usize,
    },
    ConfigLoaded(String),
    Done(Summary),
    Written(String),
    NeedImage,
    NeedVector,
    BadValue(Msg),
    SameFile(String),
    Failed(String),
}

impl Status {
    pub fn is_problem(&self) -> bool {
        matches!(
            self,
            Status::NeedImage
                | Status::NeedVector
                | Status::BadValue(_)
                | Status::SameFile(_)
                | Status::Failed(_)
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
            Status::VectorOpened {
                name,
                shapes,
                layers,
            } => fill(
                Msg::VectorOpened.tr(lang),
                &[name, &shapes.to_string(), &layers.to_string()],
            ),
            Status::ConfigLoaded(name) => fill(Msg::ConfigLoaded.tr(lang), &[name]),
            Status::Done(Summary::Calibrated { squares }) => {
                fill(Msg::Calibrated.tr(lang), &[&squares.to_string()])
            }
            Status::Done(Summary::Cut {
                layers,
                paths,
                cut_mm,
                engrave_mm,
                travel_mm,
                seconds,
            }) => fill(
                Msg::Cutting.tr(lang),
                &[
                    &layers.to_string(),
                    &paths.to_string(),
                    &format!("{cut_mm:.0}"),
                    &format!("{engrave_mm:.0}"),
                    &format!("{travel_mm:.0}"),
                    &format!("{seconds:.0}"),
                ],
            ),
            Status::Written(name) => fill(Msg::Written.tr(lang), &[name]),
            Status::NeedImage => Msg::NeedImage.tr(lang).to_string(),
            Status::NeedVector => Msg::NeedVector.tr(lang).to_string(),
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
    /// The drawing that is open and the table of its layers.
    pub vector: Option<VectorFile>,
    pub rows: Vec<LayerRow>,
    /// The settings read from a file (the limits, the machine, the defaults of a layer...), under what is typed in the window.
    pub settings: Settings,
    pub config_path: Option<PathBuf>,
    /// What was skipped when the drawing was read and planned.
    pub warnings: Vec<String>,
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
            vector: None,
            rows: Vec::new(),
            settings: Settings::default(),
            config_path: None,
            warnings: Vec::new(),
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
        self.picture.is_some() || self.vector.is_some() || self.done.is_some()
    }

    /// Whether the G-code can be worked out now.
    pub fn can_compute(&self) -> bool {
        match self.mode {
            Mode::Calibrate => true,
            Mode::Engrave => self.picture.is_some(),
            Mode::Cut => self.vector.is_some(),
        }
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

    /// The name of the file the work is made from: the drawing when the program is cutting, the picture otherwise.
    pub fn current_name(&self) -> Option<String> {
        match self.mode {
            Mode::Cut => self.vector.as_ref().map(|v| v.name.clone()),
            _ => self.picture_name(),
        }
    }

    /// The folder of the file that is open, where the window of files starts.
    pub fn work_folder(&self) -> Option<PathBuf> {
        let path = match self.mode {
            Mode::Cut => self.vector.as_ref().map(|v| &v.path),
            _ => self.picture.as_ref().map(|p| &p.path),
        };
        path.and_then(|p| p.parent().map(Path::to_path_buf))
            .filter(|p| !p.as_os_str().is_empty())
    }

    /// The name offered for the G-code to write.
    pub fn suggested_output_for(&self, job: Job) -> Option<PathBuf> {
        if job != Job::ExportGcode {
            return None;
        }
        match (&self.picture, self.mode) {
            (_, Mode::Cut) if self.vector.is_some() => {
                let v = self.vector.as_ref()?;
                let stem = v.path.file_stem()?.to_string_lossy().into_owned();
                Some(v.path.with_file_name(format!("{stem}.nc")))
            }
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
                if matches!(self.mode, Mode::Engrave | Mode::Cut) {
                    self.mode = Mode::Engrave;
                    self.warnings.clear();
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

    /// Opens a file chosen or dropped on the window: a drawing (SVG, DXF) or a picture (anything else, which the reader of pictures refuses when it is not one).
    pub fn open_dropped(&mut self, path: &Path) -> bool {
        match extension_of(path).as_str() {
            "svg" | "dxf" => self.open_vector(path),
            "png" | "pgm" => self.open_image(path),
            _ => {
                self.status = Status::Failed(format!(
                    "{}: not a file this program reads (PNG, PGM, SVG, DXF)",
                    path.display()
                ));
                false
            }
        }
    }

    /// How the drawing is read, from the boxes of the window.
    fn read_options(&self) -> Result<ReadOptions, Status> {
        let mut o = self.settings.read;
        o.dpi = self.number(Key::Dpi)?;
        o.tolerance_mm = self.number(Key::Tolerance)?;
        o.validate().map_err(Status::Failed)?;
        Ok(o)
    }

    /// The settings of a layer from its row: what the settings file says for it with what is typed in the table over it.
    fn layer_params(&self, row: &LayerRow) -> Result<LayerParams, Status> {
        let num = |f: LayerField| {
            row.fields[f as usize]
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|v| v.is_finite())
                .ok_or(Status::BadValue(f.label()))
        };
        let whole = |f: LayerField| {
            let v = num(f)?;
            if v.fract() == 0.0 && (0.0..=1_000_000.0).contains(&v) {
                Ok(v as u32)
            } else {
                Err(Status::BadValue(f.label()))
            }
        };
        let mut p = self.settings.params_for(&row.key);
        p.mode = row.mode;
        p.kerf_side = row.kerf_side;
        p.power_percent = num(LayerField::Power)?;
        p.speed_mm_min = num(LayerField::Speed)?;
        p.passes = whole(LayerField::Passes)?;
        p.focus_step_mm = num(LayerField::Focus)?;
        p.kerf_mm = num(LayerField::Kerf)?;
        p.tabs = whole(LayerField::Tabs)?;
        p.tab_width_mm = num(LayerField::TabWidth)?;
        p.hatch_angle_deg = num(LayerField::HatchAngle)?;
        p.hatch_spacing_mm = num(LayerField::HatchSpacing)?;
        Ok(p)
    }

    /// All the settings of the vector work: the file of settings under the boxes of the window and the table of layers.
    pub fn vector_settings(&self) -> Result<Settings, Status> {
        let mut s = self.settings.clone();
        s.read = self.read_options()?;
        s.machine.s_max = self.whole(Key::SMax, 1.0, 100_000.0)?;
        s.machine.overscan_mm = self.number(Key::Overscan)?;
        s.machine.origin = (self.number(Key::OriginX)?, self.number(Key::OriginY)?);
        s.layers = self
            .rows
            .iter()
            .map(|r| Ok((r.key.clone(), self.layer_params(r)?)))
            .collect::<Result<_, Status>>()?;
        s.validate().map_err(Status::Failed)?;
        Ok(s)
    }

    /// Opens an SVG or DXF drawing and builds the table of its layers; the program then cuts instead of engraving.
    pub fn open_vector(&mut self, path: &Path) -> bool {
        let opened = self.read_options().and_then(|opts| {
            let ext = extension_of(path);
            let bytes =
                read_file_up_to(path, opts.limits.max_file_bytes as u64).map_err(Status::Failed)?;
            let drawing = parse_drawing(&bytes, &ext, &opts)
                .map_err(|e| Status::Failed(format!("{}: {e}", path.display())))?;
            Ok((ext, bytes, drawing))
        });
        match opened {
            Ok((ext, bytes, drawing)) => {
                self.warnings = drawing.warnings.clone();
                self.vector = Some(VectorFile {
                    path: path.to_path_buf(),
                    name: name_of(path),
                    ext,
                    bytes,
                    drawing,
                });
                self.rebuild_rows();
                self.mode = Mode::Cut;
                self.done = None;
                self.segs.clear();
                self.stale = false;
                self.truncated = false;
                let shapes = self.vector.as_ref().map_or(0, |v| v.drawing.shapes.len());
                self.status = Status::VectorOpened {
                    name: name_of(path),
                    shapes,
                    layers: self.rows.len(),
                };
                true
            }
            Err(s) => {
                self.status = s;
                false
            }
        }
    }

    /// The table of layers of the drawing that is open, with what the settings say for each.
    fn rebuild_rows(&mut self) {
        let Some(v) = &self.vector else {
            self.rows.clear();
            return;
        };
        self.rows = layer_list(&v.drawing, &self.settings)
            .iter()
            .map(|l| LayerRow::new(&l.key, l.shapes, &self.settings.params_for(&l.key)))
            .collect();
    }

    fn put(&mut self, key: Key, text: String) {
        if let Some(f) = self.fields.iter_mut().find(|f| f.key == key) {
            f.text = text;
        }
    }

    /// Reads a file of settings: the boxes of the window and the table of layers take what it says.
    pub fn load_config(&mut self, path: &Path) -> bool {
        match vconfig::load(path) {
            Ok(s) => {
                let t = |v: f64| format!("{v}");
                self.put(Key::Dpi, t(s.read.dpi));
                self.put(Key::Tolerance, t(s.read.tolerance_mm));
                self.put(Key::SMax, s.machine.s_max.to_string());
                self.put(Key::Overscan, t(s.machine.overscan_mm));
                self.put(Key::OriginX, t(s.machine.origin.0));
                self.put(Key::OriginY, t(s.machine.origin.1));
                self.settings = s;
                self.config_path = Some(path.to_path_buf());
                self.rebuild_rows();
                self.mark_stale();
                self.status = Status::ConfigLoaded(name_of(path));
                true
            }
            Err(e) => {
                self.status = Status::Failed(e);
                false
            }
        }
    }

    /// Reads the settings file that is named by `PHOENIX_LASER_CONFIG`, or `phoenix-laser.cfg` in the folder the program runs in, when there is one.
    pub fn load_default_config(&mut self) {
        let path = std::env::var_os("PHOENIX_LASER_CONFIG")
            .map(PathBuf::from)
            .or_else(|| Some(PathBuf::from("phoenix-laser.cfg")).filter(|p| p.is_file()));
        if let Some(p) = path {
            self.load_config(&p);
        }
    }

    pub fn set_layer_field(&mut self, row: usize, field: LayerField, text: String) {
        if let Some(r) = self.rows.get_mut(row) {
            if r.fields[field as usize] != text {
                r.fields[field as usize] = text;
                self.mark_stale();
            }
        }
    }

    pub fn set_layer_mode(&mut self, row: usize, mode: LayerMode) {
        if let Some(r) = self.rows.get_mut(row) {
            if r.mode != mode {
                r.mode = mode;
                self.mark_stale();
            }
        }
    }

    pub fn set_kerf_side(&mut self, row: usize, side: KerfSide) {
        if let Some(r) = self.rows.get_mut(row) {
            if r.kerf_side != side {
                r.kerf_side = side;
                self.mark_stale();
            }
        }
    }

    /// Works the G-code out for what is chosen (the engraving or the grid), like the command line, and the passes to draw.
    pub fn compute(&mut self) -> bool {
        let made = match self.mode {
            Mode::Engrave => self.engrave_job(),
            Mode::Calibrate => self.calibrate_job(),
            Mode::Cut => self.cut_job(),
        };
        match made {
            Ok(done) => {
                self.warnings = done.notes.clone();
                let (segs, cut) = passes_of(&done.gcode, done.s_max, self.mode == Mode::Cut);
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
            notes: Vec::new(),
        })
    }

    /// The G-code that cuts and engraves the drawing that is open, with the settings of the boxes and of the table of layers.
    fn cut_job(&self) -> Result<Done, Status> {
        let file = self.vector.as_ref().ok_or(Status::NeedVector)?;
        let opts = self.read_options()?;
        let drawing = parse_drawing(&file.bytes, &file.ext, &opts).map_err(Status::Failed)?;
        let settings = self.vector_settings()?;
        let plan = plan_cut(&drawing, &settings).map_err(Status::Failed)?;
        Ok(Done {
            gcode: plan.gcode,
            source: Some((file.name.clone(), sha256_hex(&file.bytes))),
            tools: format!("laser, S 0-{}, vector layers", settings.machine.s_max),
            s_max: settings.machine.s_max,
            line_mm: 0.1,
            summary: Summary::Cut {
                layers: plan.stats.layers,
                paths: plan.stats.contours,
                cut_mm: plan.stats.cut_mm,
                engrave_mm: plan.stats.engrave_mm,
                travel_mm: plan.stats.travel_mm,
                seconds: plan.stats.seconds,
            },
            notes: plan.warnings,
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
            notes: Vec::new(),
        })
    }

    /// Writes the G-code with the standard header (the G-code is worked out again first when a setting changed). It is never written over the picture.
    pub fn export(&mut self, path: &Path) -> bool {
        if (self.done.is_none() || self.stale) && !self.compute() {
            return false;
        }
        let inputs = [
            self.picture.as_ref().map(|p| &p.path),
            self.vector.as_ref().map(|v| &v.path),
            self.config_path.as_ref(),
        ];
        if inputs.into_iter().flatten().any(|p| same_file(p, path)) {
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
            Job::OpenImage => self.open_dropped(path),
            Job::OpenConfig => self.load_config(path),
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
        if self.mode == Mode::Cut {
            let v = self.vector.as_ref()?;
            let ((x0, y0), (x1, y1)) = v.drawing.bounds()?;
            return Some(fill(
                Msg::InfoVector.tr(self.lang),
                &[
                    &v.drawing.shapes.len().to_string(),
                    &self.rows.len().to_string(),
                    &format!("{:.1}", x1 - x0),
                    &format!("{:.1}", y1 - y0),
                ],
            ));
        }
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

    /// A square of 20 mm drawn in red and a circle of radius 3 drawn in blue (one user unit is one mm).
    const DRAWING: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="20mm" height="20mm" viewBox="0 0 20 20"><rect x="0" y="0" width="20" height="20" stroke="#ff0000" fill="none"/><circle cx="10" cy="10" r="3" stroke="#0000ff" fill="none"/></svg>"##;

    /// A window with the drawing open.
    fn drawn(tag: &str) -> (Scratch, Window, PathBuf) {
        let s = Scratch::new(tag);
        let p = s.write("part.svg", DRAWING);
        let mut w = Window::new(Lang::En);
        assert!(w.open_vector(&p));
        (s, w, p)
    }

    fn cut_summary(w: &Window) -> (usize, usize, f64, f64) {
        match w.done.as_ref().unwrap().summary {
            Summary::Cut {
                layers,
                paths,
                cut_mm,
                engrave_mm,
                ..
            } => (layers, paths, cut_mm, engrave_mm),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn opening_a_drawing_builds_the_table_of_layers_and_switches_to_cutting() {
        let (_s, w, _p) = drawn("vec-open");
        assert_eq!(w.mode, Mode::Cut);
        assert_eq!(
            w.status,
            Status::VectorOpened {
                name: "part.svg".into(),
                shapes: 2,
                layers: 2
            }
        );
        let keys: Vec<&str> = w.rows.iter().map(|r| r.key.as_str()).collect();
        assert_eq!(keys, vec!["#ff0000", "#0000ff"]);
        // the defaults of a layer
        assert_eq!(w.rows[0].mode, LayerMode::Cut);
        assert_eq!(w.rows[0].fields[LayerField::Power as usize], "50");
        assert_eq!(w.rows[0].fields[LayerField::Speed as usize], "600");
        assert_eq!(w.rows[0].fields[LayerField::Passes as usize], "1");
        assert!(w.can_compute() && w.has_work());
        assert_eq!(w.current_name().as_deref(), Some("part.svg"));
        // 2 shapes in 2 layers, 20 x 20 mm
        assert_eq!(w.info_line().unwrap(), "2 shapes, 2 layers, 20.0 x 20.0 mm");
    }

    #[test]
    fn the_drawing_is_cut_with_the_settings_of_its_table() {
        let (_s, mut w, _p) = drawn("vec-cut");
        assert!(w.compute());
        let (layers, paths, cut_mm, engrave_mm) = cut_summary(&w);
        assert_eq!((layers, paths), (2, 2));
        assert_eq!(engrave_mm, 0.0);
        // the square is 80 mm; the circle has a perimeter of 18.85 mm and its flattened form a little less
        assert!(cut_mm > 98.7 && cut_mm < 98.86, "{cut_mm}");
        // both layers are cut (the blue one first: same order, lower name), the passes of the picture are there
        let g = &w.done.as_ref().unwrap().gcode;
        let first_layer = g.lines().find(|l| l.starts_with("; layer")).unwrap();
        assert!(
            first_layer.starts_with("; layer #0000ff: cut, 50 %, 600 mm/min"),
            "{first_layer}"
        );
        assert!(w.segs.iter().any(|s| s.burn) && w.segs.iter().any(|s| !s.burn));
        assert!(!w.stale);
        // a layer that is skipped is not cut: only the square is left, 80 mm
        w.set_layer_mode(1, LayerMode::Skip);
        assert!(w.stale);
        assert!(w.compute());
        assert_eq!(cut_summary(&w), (1, 1, 80.0, 0.0));
    }

    #[test]
    fn the_beam_width_of_a_layer_moves_the_cut_outwards() {
        let (_s, mut w, _p) = drawn("vec-kerf");
        w.set_layer_mode(1, LayerMode::Skip);
        w.set_layer_field(0, LayerField::Kerf, "0.2".into());
        w.set_kerf_side(0, KerfSide::Outside);
        assert!(w.compute());
        let ((x0, y0), (x1, y1)) = w.bounds().unwrap();
        assert!(
            (x0 + 0.1).abs() < 1e-9 && (y0 + 0.1).abs() < 1e-9,
            "{x0} {y0}"
        );
        assert!(
            (x1 - 20.1).abs() < 1e-9 && (y1 - 20.1).abs() < 1e-9,
            "{x1} {y1}"
        );
    }

    #[test]
    fn a_box_of_the_table_that_is_not_valid_says_which() {
        let (_s, mut w, _p) = drawn("vec-bad");
        let try_value = |w: &mut Window, f: LayerField, text: &str| {
            w.set_layer_field(0, f, text.into());
            let ok = w.compute();
            let status = w.status.clone();
            w.set_layer_field(0, f, "1".into());
            (ok, status)
        };
        assert_eq!(
            try_value(&mut w, LayerField::Power, "0"),
            (
                false,
                Status::Failed("layer #ff0000: power is 0: it must be from 0.001 to 100".into())
            )
        );
        assert_eq!(
            try_value(&mut w, LayerField::Speed, "abc"),
            (false, Status::BadValue(Msg::LayerSpeed))
        );
        assert_eq!(
            try_value(&mut w, LayerField::Passes, "1.5"),
            (false, Status::BadValue(Msg::Passes))
        );
        assert_eq!(
            try_value(&mut w, LayerField::Tabs, "-1"),
            (false, Status::BadValue(Msg::Tabs))
        );
        assert_eq!(
            try_value(&mut w, LayerField::HatchAngle, "x"),
            (false, Status::BadValue(Msg::HatchAngle))
        );
        // a box of the window that is not a number
        set(&mut w, Key::Dpi, "abc");
        assert!(!w.compute());
        assert_eq!(w.status, Status::BadValue(Msg::Dpi));
        set(&mut w, Key::Dpi, "96");
        set(&mut w, Key::SMax, "0");
        assert!(!w.compute());
        assert_eq!(w.status, Status::BadValue(Msg::SMax));
        // all the layers skipped: there is nothing to cut
        set(&mut w, Key::SMax, "1000");
        w.set_layer_mode(0, LayerMode::Skip);
        w.set_layer_mode(1, LayerMode::Skip);
        assert!(!w.compute());
        assert!(w.status.is_problem());
    }

    #[test]
    fn the_drawing_is_exported_with_the_header_of_the_picture_and_never_over_the_input() {
        let (s, mut w, p) = drawn("vec-export");
        let out = s.0.join("part.nc");
        assert_eq!(w.suggested_output_for(Job::ExportGcode), Some(out.clone()));
        assert!(w.export(&out));
        let text = fs::read_to_string(&out).unwrap();
        assert!(text.starts_with("; generator: phoenix-laser "));
        assert!(text.contains(&format!(
            "; source: part.svg sha256:{}\n",
            sha256_hex(DRAWING.as_bytes())
        )));
        assert!(text.ends_with("M5\nM2\n"));
        crate::cut::verify_gcode(&text, 1000, None).unwrap();
        // never over the drawing
        assert!(!w.export(&p));
        assert!(matches!(w.status, Status::SameFile(_)));
        assert_eq!(fs::read_to_string(&p).unwrap(), DRAWING);
        // nor over the settings file that was read
        let cfg = s.write("a.cfg", "[machine]\ns_max = 1000\n");
        assert!(w.load_config(&cfg));
        assert!(!w.export(&cfg));
        assert!(matches!(w.status, Status::SameFile(_)));
        assert_eq!(
            fs::read_to_string(&cfg).unwrap(),
            "[machine]\ns_max = 1000\n"
        );
    }

    #[test]
    fn a_file_of_settings_fills_the_boxes_and_the_table() {
        let (s, mut w, _p) = drawn("vec-config");
        let cfg = s.write(
            "a.cfg",
            "[machine]\ns_max = 255\noverscan_mm = 1.5\n[read]\ndpi = 72\n[layer #ff0000]\npower = 80\nmode = engrave\n",
        );
        assert!(w.load_config(&cfg));
        assert_eq!(w.status, Status::ConfigLoaded("a.cfg".into()));
        let field = |w: &Window, k: Key| w.fields.iter().find(|f| f.key == k).unwrap().text.clone();
        assert_eq!(field(&w, Key::SMax), "255");
        assert_eq!(field(&w, Key::Overscan), "1.5");
        assert_eq!(field(&w, Key::Dpi), "72");
        assert_eq!(w.rows[0].mode, LayerMode::Engrave);
        assert_eq!(w.rows[0].fields[LayerField::Power as usize], "80");
        assert_eq!(w.rows[1].mode, LayerMode::Cut);
        assert_eq!(w.rows[1].fields[LayerField::Power as usize], "50");
        // the engraving of the square: lines 0.1 apart across 20 mm are 200 lines of 20 mm; the circle is cut
        assert!(w.compute());
        let (layers, _, cut_mm, engrave_mm) = cut_summary(&w);
        assert_eq!(layers, 2);
        assert!((engrave_mm - 4000.0).abs() < 1e-6, "{engrave_mm}");
        assert!(cut_mm > 18.7 && cut_mm < 18.86, "{cut_mm}");
        // 80 % of 255 is 204 and 50 % is 127.5, which rounds to 128
        let g = &w.done.as_ref().unwrap().gcode;
        assert!(g.contains(" S204\n") && g.contains(" S128\n"));
        assert!(!g.contains("S1000"));
    }

    #[test]
    fn a_file_of_settings_with_a_mistake_changes_nothing() {
        let (s, mut w, _p) = drawn("vec-config-bad");
        let cfg = s.write("b.cfg", "[machine]\ns_max = 255\n[defaults]\npower = 500\n");
        assert!(!w.load_config(&cfg));
        assert!(
            matches!(&w.status, Status::Failed(e) if e.contains("power") && e.contains("b.cfg"))
        );
        let smax = w
            .fields
            .iter()
            .find(|f| f.key == Key::SMax)
            .unwrap()
            .text
            .clone();
        assert_eq!(smax, "1000");
        assert!(w.config_path.is_none());
        assert!(!w.load_config(&s.0.join("missing.cfg")));
    }

    #[test]
    fn a_dropped_file_is_opened_by_what_it_is() {
        let s = Scratch::new("vec-drop");
        let svg = s.write("part.svg", DRAWING);
        let pgm = s.write("pic.pgm", PICTURE);
        let txt = s.write("notes.txt", "hello");
        let mut w = Window::new(Lang::En);
        assert!(w.open_dropped(&svg));
        assert_eq!(w.mode, Mode::Cut);
        // a file that is not one of the four kinds leaves everything as it is
        assert!(!w.open_dropped(&txt));
        assert!(matches!(&w.status, Status::Failed(e) if e.contains("notes.txt")));
        assert_eq!(w.mode, Mode::Cut);
        assert_eq!(w.rows.len(), 2);
        // a picture takes the program back to engraving
        assert!(w.open_dropped(&pgm));
        assert_eq!(w.mode, Mode::Engrave);
        assert!(
            w.vector.is_some(),
            "the drawing stays open for when the person comes back to cutting"
        );
        assert!(w.can_compute());
        w.set_mode(Mode::Cut);
        assert_eq!(w.current_name().as_deref(), Some("part.svg"));
        // the extension is read in any case
        let upper = s.write("PART2.SVG", DRAWING);
        assert!(w.open_dropped(&upper));
    }

    #[test]
    fn a_drawing_that_cannot_be_read_is_refused_and_the_one_that_was_open_stays() {
        let (s, mut w, _p) = drawn("vec-refuse");
        let bad = s.write("bad.svg", "<svg");
        assert!(!w.open_vector(&bad));
        assert!(matches!(&w.status, Status::Failed(e) if e.contains("bad.svg")));
        assert_eq!(w.rows.len(), 2);
        assert_eq!(w.current_name().as_deref(), Some("part.svg"));
        let entity = s.write("entity.svg", r#"<!DOCTYPE svg [<!ENTITY a "b">]><svg/>"#);
        assert!(!w.open_vector(&entity));
        assert!(matches!(&w.status, Status::Failed(e) if e.contains("entities")));
        // larger than the limit of the settings
        w.settings.read.limits.max_file_bytes = 10;
        let again = s.write("again.svg", DRAWING);
        assert!(!w.open_vector(&again));
        assert!(matches!(&w.status, Status::Failed(e) if e.contains("larger")));
        assert!(!w.open_vector(&s.0.join("missing.svg")));
    }

    #[test]
    fn cutting_needs_a_drawing() {
        let mut w = Window::new(Lang::En);
        w.set_mode(Mode::Cut);
        assert!(!w.can_compute());
        assert!(!w.compute());
        assert_eq!(w.status, Status::NeedVector);
        assert!(w.status.is_problem());
        assert_eq!(
            w.suggested_output_for(Job::ExportGcode),
            Some(PathBuf::from("calibration.nc"))
        );
    }

    #[test]
    fn a_dxf_drawing_is_cut_by_layer() {
        let s = Scratch::new("vec-dxf");
        let dxf = "0\nSECTION\n2\nENTITIES\n0\nCIRCLE\n8\nHoles\n10\n10\n20\n10\n40\n2\n0\nLINE\n8\nScore\n10\n0\n20\n0\n11\n20\n21\n0\n0\nENDSEC\n0\nEOF\n";
        let p = s.write("plate.dxf", dxf);
        let mut w = Window::new(Lang::En);
        assert!(w.open_vector(&p));
        let keys: Vec<&str> = w.rows.iter().map(|r| r.key.as_str()).collect();
        assert_eq!(keys, vec!["holes", "score"]);
        w.set_layer_mode(1, LayerMode::Engrave);
        assert!(w.compute());
        let (layers, paths, cut_mm, engrave_mm) = cut_summary(&w);
        assert_eq!((layers, paths), (2, 2));
        // the line is engraved at its length, the hole is a circle of radius 2
        assert_eq!(engrave_mm, 20.0);
        assert!(cut_mm > 12.4 && cut_mm < 12.57, "{cut_mm}");
        assert_eq!(
            w.suggested_output_for(Job::ExportGcode),
            Some(s.0.join("plate.nc"))
        );
    }

    #[test]
    fn the_notes_of_the_reading_are_kept_for_the_window() {
        let s = Scratch::new("vec-notes");
        let p = s.write(
            "n.svg",
            r##"<svg><text x="0" y="0">hello</text><path d="M0 0 L10 0" stroke="#f00"/></svg>"##,
        );
        let mut w = Window::new(Lang::En);
        assert!(w.open_vector(&p));
        assert_eq!(w.warnings.len(), 1);
        assert!(w.compute());
        assert_eq!(
            w.warnings.len(),
            1,
            "the note of the reading is still there after the plan"
        );
        assert!(w.warnings[0].contains("text"));
    }

    #[test]
    fn the_status_of_the_vector_work_speaks_every_language() {
        let opened = Status::VectorOpened {
            name: "a.svg".into(),
            shapes: 3,
            layers: 2,
        };
        assert_eq!(opened.text(Lang::En), "a.svg: 3 shapes in 2 layers");
        assert_eq!(opened.text(Lang::Es), "a.svg: 3 formas en 2 capas");
        assert_eq!(opened.text(Lang::Pt), "a.svg: 3 formas em 2 camadas");
        let done = Status::Done(Summary::Cut {
            layers: 2,
            paths: 3,
            cut_mm: 98.8,
            engrave_mm: 0.0,
            travel_mm: 12.4,
            seconds: 9.6,
        });
        assert_eq!(
            done.text(Lang::En),
            "2 layers, 3 paths, 99 mm cut, 0 mm engraved, 12 mm of travel, about 10 s"
        );
        assert_eq!(
            Status::ConfigLoaded("x.cfg".into()).text(Lang::En),
            "Settings read from x.cfg"
        );
        assert!(Status::NeedVector.text(Lang::De).contains("SVG"));
        for l in Lang::ALL {
            assert!(!done.text(l).contains('{'), "{}", l.code());
            assert!(!opened.text(l).contains('{'), "{}", l.code());
        }
    }

    #[test]
    fn each_job_lists_the_files_it_can_open() {
        assert!(Job::OpenImage.extensions().contains(&"png"));
        assert!(Job::OpenImage.extensions().contains(&"pgm"));
        assert!(Job::OpenImage.extensions().contains(&"svg"));
        assert!(Job::OpenImage.extensions().contains(&"dxf"));
        assert_eq!(Job::OpenConfig.extensions(), &["cfg"]);
        assert!(
            Job::ExportGcode.is_export()
                && !Job::OpenImage.is_export()
                && !Job::OpenConfig.is_export()
        );
        assert_eq!(Kind::ALL.len(), 3);
    }
}
