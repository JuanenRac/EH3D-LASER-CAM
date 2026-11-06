// EH3D-PHOENIX-LASER - command line.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
#![forbid(unsafe_code)]

mod cli_args;

use cli_args::{finish, flag, num, switch};
use eh3d_phoenix_laser::calibrate::{grid, Grid};
use eh3d_phoenix_laser::cut::{layers as layer_list, plan};
use eh3d_phoenix_laser::dither::{dither, Method};
use eh3d_phoenix_laser::dxf::read_dxf;
use eh3d_phoenix_laser::gcode_out::{header, sha256_hex, Header};
use eh3d_phoenix_laser::image::from_bytes;
use eh3d_phoenix_laser::raster::{engrave, overscan_for, Params};
use eh3d_phoenix_laser::svg::read_svg;
use eh3d_phoenix_laser::vconfig::{self, KerfSide, LayerMode, LayerParams, Settings};
use eh3d_phoenix_laser::vector::Drawing;
use eh3d_phoenix_laser::window::{same_file, write_atomic};
use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

const HELP: &str = "phoenix-laser: laser engraving from an image

  phoenix-laser engrave IMAGE OUT.nc --width-mm 50 [--dot-mm 0.1] [--feed 3000] [--power 50] [--smax 1000]
                [--dither fs|atkinson|threshold] [--threshold 128] [--overscan MM|auto] [--accel 1000] [--origin X,Y] [--dry-run]
  phoenix-laser calibrate OUT.nc --powers 20,40,60,80 --feeds 1000,2000,3000 [--smax 1000]
  phoenix-laser info IMAGE
  phoenix-laser cut DRAWING OUT.nc [--config FILE] [--power P] [--speed MM/MIN] [--passes N] [--focus-step MM] [--kerf MM] [--kerf-side none|auto|inside|outside]
                [--tabs N] [--tab-width MM] [--smax N] [--overscan MM] [--origin X,Y] [--bed X,Y] [--dpi N] [--tolerance MM] [--dry-run]
  phoenix-laser vector DRAWING OUT.nc [the options of cut] [--angle DEG] [--spacing MM]
  phoenix-laser layers DRAWING [--config FILE] [--dpi N]
  phoenix-laser [gui] [FILE]                     the window (a build with --features gui): with no arguments, with a file or with gui it opens; the commands above never open it
  phoenix-laser gui [FILE] --shot OUT.png [--size WxH] [--theme forge|dark|light] [--wait N] [--lang CODE] [--replace]   a picture of the window (a build with --features gui)

IMAGE is a PNG or a PGM file. --dry-run works the engraving out and counts it but writes nothing (OUT.nc may then be left out). --power is a percent of full power; --smax is the spindle value that is full power for the controller (GRBL: $30).
The engraving is made of dots of --dot-mm on a side, so the image is scaled to --width-mm / --dot-mm dots wide.
DRAWING is an SVG or a DXF file. cut follows the contours (the ones inside first), vector fills the closed shapes with lines and follows the open ones. Every layer (or colour, for an SVG) has its
own settings in the file of --config (see the documentation); an option of the command line changes them for all the layers. Without --config, cut cuts everything and vector engraves everything.
layers lists the layers of a drawing with what would be done with each.";

fn list(text: &str, what: &str) -> Result<Vec<f64>, String> {
    text.split(',').map(|p| num(p, what)).collect()
}

/// A whole number from 0 up to a million.
fn whole(text: &str, what: &str) -> Result<u32, String> {
    let v = num(text, what)?;
    if v.fract() == 0.0 && (0.0..=1_000_000.0).contains(&v) {
        Ok(v as u32)
    } else {
        Err(format!(
            "{what} '{text}' is not a whole number from 0 to 1000000"
        ))
    }
}

/// What the options of a drawing command change in the settings of every layer.
#[derive(Default)]
struct Overrides {
    power: Option<f64>,
    speed: Option<f64>,
    passes: Option<u32>,
    focus_step: Option<f64>,
    kerf: Option<f64>,
    side: Option<KerfSide>,
    tabs: Option<u32>,
    tab_width: Option<f64>,
    angle: Option<f64>,
    spacing: Option<f64>,
}

impl Overrides {
    fn take(args: &mut Vec<String>) -> Result<Overrides, String> {
        let mut o = Overrides::default();
        let mut number = |name: &str| -> Result<Option<f64>, String> {
            flag(args, name)?.map(|v| num(&v, name)).transpose()
        };
        o.power = number("--power")?;
        o.speed = number("--speed")?;
        o.focus_step = number("--focus-step")?;
        o.kerf = number("--kerf")?;
        o.tab_width = number("--tab-width")?;
        o.angle = number("--angle")?;
        o.spacing = number("--spacing")?;
        o.passes = flag(args, "--passes")?
            .map(|v| whole(&v, "--passes"))
            .transpose()?;
        o.tabs = flag(args, "--tabs")?
            .map(|v| whole(&v, "--tabs"))
            .transpose()?;
        o.side = flag(args, "--kerf-side")?
            .map(|v| {
                KerfSide::from_word(&v).ok_or_else(|| {
                    format!("--kerf-side '{v}' is not none, auto, inside or outside")
                })
            })
            .transpose()?;
        Ok(o)
    }

    fn apply(&self, p: &mut LayerParams) {
        p.power_percent = self.power.unwrap_or(p.power_percent);
        p.speed_mm_min = self.speed.unwrap_or(p.speed_mm_min);
        p.passes = self.passes.unwrap_or(p.passes);
        p.focus_step_mm = self.focus_step.unwrap_or(p.focus_step_mm);
        p.kerf_mm = self.kerf.unwrap_or(p.kerf_mm);
        p.kerf_side = self.side.unwrap_or(p.kerf_side);
        p.tabs = self.tabs.unwrap_or(p.tabs);
        p.tab_width_mm = self.tab_width.unwrap_or(p.tab_width_mm);
        p.hatch_angle_deg = self.angle.unwrap_or(p.hatch_angle_deg);
        p.hatch_spacing_mm = self.spacing.unwrap_or(p.hatch_spacing_mm);
    }
}

/// The settings of a drawing command: the file of `--config` (or the defaults, with `default_mode` for every layer) and the options of the command line over it. Gives the file of
/// settings too, so that it is never written over.
fn drawing_settings(
    args: &mut Vec<String>,
    default_mode: LayerMode,
) -> Result<(Settings, Option<String>), String> {
    let config = flag(args, "--config")?;
    let mut s = match &config {
        Some(path) => vconfig::load(Path::new(path))?,
        None => {
            let mut s = Settings::default();
            s.defaults.mode = default_mode;
            s
        }
    };
    if let Some(v) = flag(args, "--dpi")? {
        s.read.dpi = num(&v, "--dpi")?;
    }
    if let Some(v) = flag(args, "--tolerance")? {
        s.read.tolerance_mm = num(&v, "--tolerance")?;
    }
    if let Some(v) = flag(args, "--smax")? {
        s.machine.s_max = whole(&v, "--smax")?;
    }
    if let Some(v) = flag(args, "--overscan")? {
        s.machine.overscan_mm = num(&v, "--overscan")?;
    }
    if let Some(v) = flag(args, "--origin")? {
        let o = list(&v, "--origin")?;
        if o.len() != 2 {
            return Err("--origin needs X,Y".into());
        }
        s.machine.origin = (o[0], o[1]);
    }
    if let Some(v) = flag(args, "--bed")? {
        let b = list(&v, "--bed")?;
        if b.len() != 2 {
            return Err("--bed needs X,Y".into());
        }
        s.machine.bed_mm = Some((b[0], b[1]));
    }
    let over = Overrides::take(args)?;
    over.apply(&mut s.defaults);
    for (_, p) in s.layers.iter_mut() {
        over.apply(p);
    }
    s.validate()?;
    Ok((s, config))
}

/// Reads an SVG or DXF file, with the limit on its size that the settings give.
fn read_drawing(path: &str, s: &Settings) -> Result<(Vec<u8>, Drawing), String> {
    let meta = fs::metadata(path).map_err(|e| format!("{path}: {e}"))?;
    if meta.len() > s.read.limits.max_file_bytes as u64 {
        return Err(format!(
            "{path}: the file is larger than {} bytes (max_file_bytes)",
            s.read.limits.max_file_bytes
        ));
    }
    let bytes = fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    let ext = Path::new(path)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let drawing = match ext.as_str() {
        "svg" => read_svg(&bytes, &s.read),
        "dxf" => read_dxf(&bytes, &s.read),
        _ => Err("it is not an SVG or a DXF file".to_string()),
    }
    .map_err(|e| format!("{path}: {e}"))?;
    Ok((bytes, drawing))
}

/// `cut` and `vector`: a drawing becomes the G-code of the laser.
fn drawing_job(mut args: Vec<String>, default_mode: LayerMode) -> Result<(), String> {
    let dry_run = switch(&mut args, "--dry-run");
    let (settings, config) = drawing_settings(&mut args, default_mode)?;
    let needed = if dry_run { 1 } else { 2 };
    if args.len() < needed {
        return Err(if dry_run {
            "a drawing is needed".into()
        } else {
            "a drawing and an output file are needed".into()
        });
    }
    finish(&args, 2)?;
    let (bytes, drawing) = read_drawing(&args[0], &settings)?;
    let planned = plan(&drawing, &settings)?;
    if dry_run {
        print!("dry run, nothing written: ");
    } else {
        // never over the drawing or the settings that it was made from
        let out = Path::new(&args[1]);
        if same_file(Path::new(&args[0]), out)
            || config
                .as_deref()
                .is_some_and(|c| same_file(Path::new(c), out))
        {
            return Err(format!(
                "{}: the output would be written over a file it is made from",
                args[1]
            ));
        }
        let name = Path::new(&args[0])
            .file_name()
            .map_or(args[0].clone(), |n| n.to_string_lossy().into_owned());
        let tools = format!("laser, S 0-{}, vector layers", settings.machine.s_max);
        let head = header(&Header {
            generator: "phoenix-laser",
            version: env!("CARGO_PKG_VERSION"),
            source: Some((&name, sha256_hex(&bytes))),
            tools: &tools,
        });
        write_atomic(out, &format!("{head}{}", planned.gcode))?;
    }
    for note in &planned.warnings {
        eprintln!("note: {note}");
    }
    let st = &planned.stats;
    println!(
        "{} layers, {} paths, {:.0} mm cut, {:.0} mm engraved, {:.0} mm of travel, about {:.0} s",
        st.layers, st.contours, st.cut_mm, st.engrave_mm, st.travel_mm, st.seconds
    );
    Ok(())
}

/// `layers`: what is in a drawing and what would be done with it.
fn layers_job(mut args: Vec<String>) -> Result<(), String> {
    let (settings, _) = drawing_settings(&mut args, LayerMode::Cut)?;
    finish(&args, 1)?;
    let path = args.first().ok_or("layers needs a drawing")?;
    let (_, drawing) = read_drawing(path, &settings)?;
    for l in layer_list(&drawing, &settings) {
        let p = settings.params_for(&l.key);
        println!(
            "{}: {} shapes ({} closed), {:.1} mm, {} at {} % and {} mm/min, {} pass(es)",
            l.key,
            l.shapes,
            l.closed,
            l.length_mm,
            p.mode.word(),
            p.power_percent,
            p.speed_mm_min,
            p.passes
        );
    }
    for note in &drawing.warnings {
        eprintln!("note: {note}");
    }
    Ok(())
}

fn run(mut args: Vec<String>) -> Result<(), String> {
    if args.is_empty() || matches!(args[0].as_str(), "help" | "--help" | "-h") {
        println!("{HELP}");
        return Ok(());
    }
    let cmd = args.remove(0);
    if cmd == "gui" {
        // a build with the window never gets here (main opens it): this one is only a command line
        return Err("this build has no window: build it with --features gui".into());
    }
    match cmd.as_str() {
        "engrave" => {
            let width = flag(&mut args, "--width-mm")?
                .map(|v| num(&v, "--width-mm"))
                .transpose()?
                .ok_or("--width-mm is needed")?;
            let mut p = Params::default();
            if let Some(v) = flag(&mut args, "--dot-mm")? {
                p.pixel_mm = num(&v, "--dot-mm")?;
            }
            if let Some(v) = flag(&mut args, "--feed")? {
                p.feed_mm_min = num(&v, "--feed")?;
            }
            if let Some(v) = flag(&mut args, "--power")? {
                p.power_percent = num(&v, "--power")?;
            }
            if let Some(v) = flag(&mut args, "--smax")? {
                p.s_max = num(&v, "--smax")? as u32;
            }
            if let Some(v) = flag(&mut args, "--origin")? {
                let o = list(&v, "--origin")?;
                if o.len() != 2 {
                    return Err("--origin needs X,Y".into());
                }
                p.origin = (o[0], o[1]);
            }
            let accel = flag(&mut args, "--accel")?
                .map(|v| num(&v, "--accel"))
                .transpose()?
                .unwrap_or(1000.0);
            match flag(&mut args, "--overscan")?.as_deref() {
                Some("auto") => p.overscan_mm = overscan_for(p.feed_mm_min, accel)?,
                Some(v) => p.overscan_mm = num(v, "--overscan")?,
                None => {}
            }
            let threshold = flag(&mut args, "--threshold")?
                .map(|v| num(&v, "--threshold"))
                .transpose()?
                .unwrap_or(128.0)
                .clamp(0.0, 255.0) as u8;
            let method = match flag(&mut args, "--dither")?.as_deref().unwrap_or("fs") {
                "fs" => Method::FloydSteinberg,
                "atkinson" => Method::Atkinson,
                "threshold" => Method::Threshold(threshold),
                other => {
                    return Err(format!(
                        "--dither '{other}' is not fs, atkinson or threshold"
                    ))
                }
            };
            let dry_run = switch(&mut args, "--dry-run");
            let needed = if dry_run { 1 } else { 2 };
            if args.len() < needed {
                return Err(if dry_run {
                    "engrave needs an image".into()
                } else {
                    "engrave needs an image and an output file".into()
                });
            }
            finish(&args, 2)?;
            let bytes = fs::read(&args[0]).map_err(|e| format!("{}: {e}", args[0]))?;
            let img = from_bytes(&bytes)?;
            if !(width.is_finite() && width > 0.0 && width <= 2000.0) {
                return Err("--width-mm must be from 0 to 2000".into());
            }
            p.validate()?;
            let w = (width / p.pixel_mm).round().max(1.0) as usize;
            let h = ((w as f64) * img.h as f64 / img.w as f64).round().max(1.0) as usize;
            let dots = dither(&img.resize(w, h)?, method);
            let e = engrave(&dots, &p)?;
            // a dry run works the engraving out and counts it, and writes nothing
            if dry_run {
                print!("dry run, nothing written: ");
            } else {
                // the standard header: the generator, the units, the picture it was made from with its hash, and the laser
                let name = std::path::Path::new(&args[0])
                    .file_name()
                    .map_or(args[0].clone(), |n| n.to_string_lossy().into_owned());
                let tools = format!("laser, S 0-{} ({} %)", p.s_max, p.power_percent);
                let head = header(&Header {
                    generator: "phoenix-laser",
                    version: env!("CARGO_PKG_VERSION"),
                    source: Some((&name, sha256_hex(&bytes))),
                    tools: &tools,
                });
                fs::write(&args[1], format!("{head}{}", e.gcode))
                    .map_err(|er| format!("{}: {er}", args[1]))?;
            }
            println!("{} x {} dots ({:.1} x {:.1} mm), {} lines, {} runs, {:.0} mm burned, about {:.0} s", dots.w, dots.h, dots.w as f64 * p.pixel_mm, dots.h as f64 * p.pixel_mm, e.rows, e.runs, e.burn_mm, e.seconds);
            Ok(())
        }
        "calibrate" => {
            let powers = list(
                &flag(&mut args, "--powers")?.ok_or("--powers is needed")?,
                "--powers",
            )?;
            let feeds = list(
                &flag(&mut args, "--feeds")?.ok_or("--feeds is needed")?,
                "--feeds",
            )?;
            let mut g = Grid::default();
            if let Some(v) = flag(&mut args, "--smax")? {
                g.s_max = num(&v, "--smax")? as u32;
            }
            finish(&args, 1)?;
            let out = args.first().ok_or("calibrate needs an output file")?;
            let head = header(&Header {
                generator: "phoenix-laser",
                version: env!("CARGO_PKG_VERSION"),
                source: None,
                tools: &format!("laser, S 0-{}, grid of power against feed", g.s_max),
            });
            fs::write(out, format!("{head}{}", grid(&powers, &feeds, &g)?))
                .map_err(|e| format!("{out}: {e}"))?;
            println!("{} squares written to {out}", powers.len() * feeds.len());
            Ok(())
        }
        "cut" => drawing_job(args, LayerMode::Cut),
        "vector" => drawing_job(args, LayerMode::Engrave),
        "layers" => layers_job(args),
        "info" => {
            finish(&args, 1)?;
            let path = args.first().ok_or("info needs an image")?;
            let img = from_bytes(&fs::read(path).map_err(|e| format!("{path}: {e}"))?)?;
            let mean =
                img.data.iter().map(|&v| u64::from(v)).sum::<u64>() as f64 / img.data.len() as f64;
            println!("{} x {} pixels, mean grey {:.1} of 255", img.w, img.h, mean);
            Ok(())
        }
        other => Err(format!("{other}: unknown command (see help)")),
    }
}

/// Whether the program opens its window instead of running a command, and the picture to open in it: no arguments, `gui` (with a picture or without), or a single picture given by the
/// system to open (a name with an extension that is a file; the commands have none).
#[cfg(feature = "gui")]
fn window_request(args: &[String]) -> Option<Option<String>> {
    match args {
        [] => Some(None),
        [a] if a == "gui" => Some(None),
        [a, f] if a == "gui" => Some(Some(f.clone())),
        [f] if !f.starts_with('-')
            && std::path::Path::new(f).extension().is_some()
            && std::path::Path::new(f).is_file() =>
        {
            Some(Some(f.clone()))
        }
        _ => None,
    }
}

#[cfg(feature = "gui")]
fn open_window(picture: Option<String>) -> ExitCode {
    let lang = eh3d_phoenix_laser::lang::Lang::from_env();
    match eh3d_phoenix_laser::gui::run(picture.map(std::path::PathBuf::from), lang) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: the window could not be opened: {e}");
            ExitCode::from(1)
        }
    }
}

/// The picture mode: `--shot FILE.png` opens the window, saves what it draws as a PNG file and closes. The exit code is 0 when the file was written, 1 when it was not and 2 when the
/// arguments are wrong.
#[cfg(feature = "gui")]
fn take_picture(args: &[String]) -> ExitCode {
    let parsed = match eh3d_phoenix_laser::shot::parse_window_args(args, 1) {
        Ok(parsed) => parsed,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    let Some(request) = parsed.shot else {
        eprintln!("error: --shot needs a file to write");
        return ExitCode::from(2);
    };
    let lang = parsed
        .lang
        .unwrap_or_else(eh3d_phoenix_laser::lang::Lang::from_env);
    match eh3d_phoenix_laser::gui::run_shot(parsed.files, lang, request) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: the picture could not be taken: {e}");
            ExitCode::from(1)
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if eh3d_phoenix_laser::shot::wants_picture(&args) {
        #[cfg(feature = "gui")]
        return take_picture(&args);
        #[cfg(not(feature = "gui"))]
        {
            eprintln!("error: this build has no window: build it with --features gui");
            return ExitCode::from(1);
        }
    }
    #[cfg(feature = "gui")]
    if let Some(picture) = window_request(&args) {
        return open_window(picture);
    }
    match run(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}
