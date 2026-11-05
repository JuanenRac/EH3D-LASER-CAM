// EH3D-PHOENIX-LASER - command line.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
#![forbid(unsafe_code)]

mod cli_args;

use cli_args::{finish, flag, num, switch};
use eh3d_phoenix_laser::calibrate::{grid, Grid};
use eh3d_phoenix_laser::dither::{dither, Method};
use eh3d_phoenix_laser::gcode_out::{header, sha256_hex, Header};
use eh3d_phoenix_laser::image::from_bytes;
use eh3d_phoenix_laser::raster::{engrave, overscan_for, Params};
use std::env;
use std::fs;
use std::process::ExitCode;

const HELP: &str = "phoenix-laser: laser engraving from an image

  phoenix-laser engrave IMAGE OUT.nc --width-mm 50 [--dot-mm 0.1] [--feed 3000] [--power 50] [--smax 1000]
                [--dither fs|atkinson|threshold] [--threshold 128] [--overscan MM|auto] [--accel 1000] [--origin X,Y] [--dry-run]
  phoenix-laser calibrate OUT.nc --powers 20,40,60,80 --feeds 1000,2000,3000 [--smax 1000]
  phoenix-laser info IMAGE
  phoenix-laser [gui] [IMAGE]                    the window (a build with --features gui): with no arguments, with an image or with gui it opens; the commands above never open it

IMAGE is a PNG or a PGM file. --dry-run works the engraving out and counts it but writes nothing (OUT.nc may then be left out). --power is a percent of full power; --smax is the spindle value that is full power for the controller (GRBL: $30).
The engraving is made of dots of --dot-mm on a side, so the image is scaled to --width-mm / --dot-mm dots wide.";

fn list(text: &str, what: &str) -> Result<Vec<f64>, String> {
    text.split(',').map(|p| num(p, what)).collect()
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

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
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
