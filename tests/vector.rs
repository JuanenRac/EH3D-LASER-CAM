//! Cutting and engraving from a drawing: the commands `cut`, `vector` and `layers` on SVG and DXF files written here by hand, with the program checked line by line, the protections of
//! the export, hostile files and the safety check of the G-code.

use eh3d_phoenix_laser::cut::{plan, verify_gcode};
use eh3d_phoenix_laser::dxf::read_dxf;
use eh3d_phoenix_laser::gcode_out::sha256_hex;
use eh3d_phoenix_laser::svg::read_svg;
use eh3d_phoenix_laser::vconfig::{parse, Settings};
use eh3d_phoenix_laser::vector::ReadOptions;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_phoenix-laser");

/// A square of 10 mm as a light polyline in a DXF file.
const SQUARE_DXF: &str = "0\nSECTION\n2\nENTITIES\n0\nLWPOLYLINE\n8\nl\n90\n4\n70\n1\n10\n0\n20\n0\n10\n10\n20\n0\n10\n10\n20\n10\n10\n0\n20\n10\n0\nENDSEC\n0\nEOF\n";

/// A plate of 28 x 18 mm with a hole of radius 2, drawn in red.
const PLATE_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="30mm" height="20mm" viewBox="0 0 30 20">
  <rect x="1" y="1" width="28" height="18" fill="none" stroke="#ff0000"/>
  <circle cx="15" cy="10" r="2" fill="none" stroke="#ff0000"/>
</svg>"##;

/// A folder of its own for each test, removed at the end.
struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Scratch {
        let dir =
            std::env::temp_dir().join(format!("eh3d-laser-vector-{}-{tag}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }

    fn write(&self, name: &str, text: &str) -> String {
        let p = self.0.join(name);
        fs::write(&p, text).unwrap();
        p.to_string_lossy().into_owned()
    }

    fn path(&self, name: &str) -> String {
        self.0.join(name).to_string_lossy().into_owned()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run(args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(BIN)
        .args(args)
        .output()
        .expect("the program starts");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// The lines of a program that are not comments.
fn code(g: &str) -> Vec<&str> {
    g.lines().filter(|l| !l.starts_with(';')).collect()
}

/// The places the head goes to, with the word of the move.
fn moves(g: &str) -> Vec<(bool, f64, f64)> {
    let (mut x, mut y, mut rapid) = (0.0, 0.0, true);
    let mut out = Vec::new();
    for l in code(g) {
        let mut moved = false;
        for t in l.split_whitespace() {
            if t == "G0" {
                rapid = true;
            } else if t == "G1" {
                rapid = false;
            } else if let Some(v) = t.strip_prefix('X') {
                x = v.parse().unwrap();
                moved = true;
            } else if let Some(v) = t.strip_prefix('Y') {
                y = v.parse().unwrap();
                moved = true;
            }
        }
        if moved {
            out.push((rapid, x, y));
        }
    }
    out
}

#[test]
fn cut_writes_the_program_of_a_square_and_says_what_it_did() {
    let s = Scratch::new("square");
    let input = s.write("sq.dxf", SQUARE_DXF);
    let out = s.path("sq.nc");
    let (code_, stdout, stderr) = run(&["cut", &input, &out]);
    assert_eq!(code_, Some(0), "{stderr}");
    assert_eq!(
        stdout.trim(),
        "1 layers, 1 paths, 40 mm cut, 0 mm engraved, 0 mm of travel, about 4 s"
    );
    let text = fs::read_to_string(&out).unwrap();
    let head: Vec<&str> = text.lines().take(4).collect();
    assert!(head[0].starts_with("; generator: phoenix-laser "));
    assert_eq!(head[1], "; units: mm (G21)");
    assert_eq!(
        head[2],
        format!(
            "; source: sq.dxf sha256:{}",
            sha256_hex(SQUARE_DXF.as_bytes())
        )
    );
    assert_eq!(head[3], "; tools: laser, S 0-1000, vector layers");
    assert_eq!(
        code(&text),
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
    verify_gcode(&text, 1000, None).unwrap();
}

#[test]
fn options_over_the_defaults_change_the_power_the_speed_and_the_full_power_value() {
    let s = Scratch::new("options");
    let input = s.write("sq.dxf", SQUARE_DXF);
    let out = s.path("o.nc");
    let (c, _, e) = run(&[
        "cut", &input, &out, "--power", "25", "--speed", "120", "--passes", "2", "--smax", "255",
    ]);
    assert_eq!(c, Some(0), "{e}");
    let text = fs::read_to_string(&out).unwrap();
    // 25 % of 255 is 63.75, which rounds to 64
    let c = code(&text);
    assert_eq!(c.iter().filter(|l| **l == "F120").count(), 2);
    assert_eq!(c.iter().filter(|l| l.ends_with(" S64")).count(), 2);
    assert!(!text.contains("S500"));
    let (c, _, e) = run(&["cut", &input, &out, "--power", "100", "--smax", "255"]);
    assert_eq!(c, Some(0), "{e}");
    assert!(fs::read_to_string(&out).unwrap().contains("G1 X10 Y0 S255"));
}

#[test]
fn vector_fills_the_square_with_lines_that_go_one_way_and_the_other() {
    let s = Scratch::new("hatch");
    let input = s.write("sq.dxf", SQUARE_DXF);
    let out = s.path("h.nc");
    let (c, stdout, e) = run(&[
        "vector",
        &input,
        &out,
        "--spacing",
        "2.5",
        "--overscan",
        "0",
    ]);
    assert_eq!(c, Some(0), "{e}");
    // 4 lines of 10 mm; the head travels 1.25 up to the first line and 2.5 between the others: 8.75 mm, 0.0875 s
    assert_eq!(
        stdout.trim(),
        "1 layers, 1 paths, 0 mm cut, 40 mm engraved, 9 mm of travel, about 4 s"
    );
    let text = fs::read_to_string(&out).unwrap();
    assert_eq!(
        &code(&text)[3..],
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
}

#[test]
fn a_dry_run_works_the_job_out_and_writes_nothing() {
    let s = Scratch::new("dry");
    let input = s.write("sq.dxf", SQUARE_DXF);
    let out = s.path("never.nc");
    let (c, stdout, _) = run(&["cut", &input, &out, "--dry-run"]);
    assert_eq!(c, Some(0));
    assert!(
        stdout.starts_with("dry run, nothing written: 1 layers"),
        "{stdout}"
    );
    assert!(!PathBuf::from(&out).exists());
    // the output may be left out
    let (c, _, e) = run(&["cut", &input, "--dry-run"]);
    assert_eq!(c, Some(0), "{e}");
    let (c, _, e) = run(&["cut", &input]);
    assert_eq!(c, Some(1));
    assert!(e.contains("output file"), "{e}");
}

#[test]
fn the_output_is_never_written_over_the_drawing_or_the_settings_it_is_made_from() {
    let s = Scratch::new("never");
    let input = s.write("sq.dxf", SQUARE_DXF);
    let cfg = s.write("a.cfg", "[defaults]\npower = 40\n");
    let (c, _, e) = run(&["cut", &input, &input]);
    assert_eq!(c, Some(1));
    assert!(e.contains("written over"), "{e}");
    assert_eq!(fs::read_to_string(&input).unwrap(), SQUARE_DXF);
    let (c, _, e) = run(&["cut", &input, &cfg, "--config", &cfg]);
    assert_eq!(c, Some(1));
    assert!(e.contains("written over"), "{e}");
    assert_eq!(
        fs::read_to_string(&cfg).unwrap(),
        "[defaults]\npower = 40\n"
    );
    // a path that spells the same file another way
    let other = format!("{}/./sq.dxf", s.0.display());
    let (c, _, _) = run(&["cut", &input, &other]);
    assert_eq!(c, Some(1));
    assert_eq!(fs::read_to_string(&input).unwrap(), SQUARE_DXF);
}

#[test]
fn layers_lists_what_the_drawing_holds_and_what_would_be_done_with_it() {
    let s = Scratch::new("layers");
    let dxf = s.write("sq.dxf", SQUARE_DXF);
    let (c, out, _) = run(&["layers", &dxf]);
    assert_eq!(c, Some(0));
    assert_eq!(
        out,
        "l: 1 shapes (1 closed), 40.0 mm, cut at 50 % and 600 mm/min, 1 pass(es)\n"
    );
    // an SVG is grouped by colour; the plate has a rectangle of 92 mm and a circle of about 12.6 mm, both red
    let svg = s.write("plate.svg", PLATE_SVG);
    let (c, out, _) = run(&["layers", &svg]);
    assert_eq!(c, Some(0));
    assert!(out.starts_with("#ff0000: 2 shapes (2 closed), 10"), "{out}");
    assert!(
        out.ends_with(" mm, cut at 50 % and 600 mm/min, 1 pass(es)\n"),
        "{out}"
    );
    let cfg = s.write(
        "l.cfg",
        "[layer #ff0000]\nmode = engrave\npower = 70\nspeed = 900\npasses = 3\n",
    );
    let (c, out, _) = run(&["layers", &svg, "--config", &cfg]);
    assert_eq!(c, Some(0));
    assert!(
        out.contains(" mm, engrave at 70 % and 900 mm/min, 3 pass(es)\n"),
        "{out}"
    );
}

#[test]
fn layers_tell_the_energy_of_a_pass_only_when_the_power_of_the_laser_is_known() {
    let s = Scratch::new("energy");
    let dxf = s.write("sq.dxf", SQUARE_DXF);
    // 10 W at 50 % over 600 mm/min (10 mm/s) is 0.5 J for each millimetre of the cut
    let cfg = s.write("w.cfg", "[machine]\nlaser_watts = 10\n");
    let (c, out, _) = run(&["layers", &dxf, "--config", &cfg]);
    assert_eq!(c, Some(0));
    assert_eq!(
        out,
        "l: 1 shapes (1 closed), 40.0 mm, cut at 50 % and 600 mm/min, 1 pass(es), 0.500 J/mm a pass\n"
    );
    // the same 0.5 J/mm over hatch lines 0.1 mm apart is 5 J for each square millimetre
    let svg = s.write("plate.svg", PLATE_SVG);
    let hatch = s.write(
        "h.cfg",
        "[machine]\nlaser_watts = 10\n[layer #ff0000]\nmode = engrave\nhatch_spacing = 0.1\n",
    );
    let (c, out, _) = run(&["layers", &svg, "--config", &hatch]);
    assert_eq!(c, Some(0));
    assert!(
        out.ends_with(" mm, engrave at 50 % and 600 mm/min, 1 pass(es), 5.000 J/mm2 a pass\n"),
        "{out}"
    );
    // a power that is not a power of a laser is refused with its name
    let bad = s.write("b.cfg", "[machine]\nlaser_watts = 5000\n");
    let (c, _, e) = run(&["layers", &dxf, "--config", &bad]);
    assert_ne!(c, Some(0));
    assert!(e.contains("laser_watts"), "{e}");
}

#[test]
fn a_settings_file_sets_each_layer_and_the_command_line_changes_them_all() {
    let s = Scratch::new("config");
    let input = s.write("sq.dxf", SQUARE_DXF);
    let cfg = s.write(
        "c.cfg",
        "[machine]\ns_max = 255\n[layer l]\npower = 100\nspeed = 300\n",
    );
    let out = s.path("c.nc");
    let (c, _, e) = run(&["cut", &input, &out, "--config", &cfg]);
    assert_eq!(c, Some(0), "{e}");
    let text = fs::read_to_string(&out).unwrap();
    assert!(code(&text).contains(&"F300") && text.contains("G1 X10 Y0 S255"));
    // --speed over the file
    let (c, _, e) = run(&["cut", &input, &out, "--config", &cfg, "--speed", "450"]);
    assert_eq!(c, Some(0), "{e}");
    assert!(code(&fs::read_to_string(&out).unwrap()).contains(&"F450"));
    // a mistake of the file names its line and the file, and nothing is written
    let bad = s.write("bad.cfg", "[machine]\ns_maxx = 1\n");
    let none = s.path("none.nc");
    let (c, _, e) = run(&["cut", &input, &none, "--config", &bad]);
    assert_eq!(c, Some(1));
    assert!(e.contains("bad.cfg") && e.contains("line 2"), "{e}");
    assert!(!PathBuf::from(&none).exists());
}

#[test]
fn the_hole_of_a_plate_is_cut_first_and_the_beam_goes_outside_the_plate_and_inside_the_hole() {
    let s = Scratch::new("plate");
    let svg = s.write("plate.svg", PLATE_SVG);
    let out = s.path("plate.nc");
    let (c, _, e) = run(&[
        "cut",
        &svg,
        &out,
        "--kerf",
        "0.2",
        "--kerf-side",
        "auto",
        "--tabs",
        "2",
        "--tab-width",
        "2",
    ]);
    assert_eq!(c, Some(0), "{e}");
    let text = fs::read_to_string(&out).unwrap();
    verify_gcode(&text, 1000, None).unwrap();
    let m = moves(&text);
    // the plate is placed with its lower left corner at the origin: the hole is centred at (14, 9) and the first travel is to a point of the hole 1.9 from the centre
    let first = m[0];
    assert!(first.0);
    assert!(
        ((first.1 - 14.0).hypot(first.2 - 9.0) - 1.9).abs() < 0.01,
        "{first:?}"
    );
    // the outline goes 0.1 outside of the plate: from -0.1 to 28.1 and from -0.1 to 18.1
    let xs: Vec<f64> = m.iter().map(|p| p.1).collect();
    let ys: Vec<f64> = m.iter().map(|p| p.2).collect();
    let (min_x, max_x) = (
        xs.iter().cloned().fold(f64::INFINITY, f64::min),
        xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
    );
    let (min_y, max_y) = (
        ys.iter().cloned().fold(f64::INFINITY, f64::min),
        ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
    );
    assert!(
        (min_x + 0.1).abs() < 1e-9 && (max_x - 28.1).abs() < 1e-9,
        "{min_x} {max_x}"
    );
    assert!(
        (min_y + 0.1).abs() < 1e-9 && (max_y - 18.1).abs() < 1e-9,
        "{min_y} {max_y}"
    );
    // the outline is cut in two pieces (two tabs): three travels in all, one for the hole
    assert_eq!(
        code(&text).iter().filter(|l| l.starts_with("G0 X")).count(),
        3
    );
}

#[test]
fn work_that_does_not_fit_the_bed_is_refused_and_nothing_is_written() {
    let s = Scratch::new("bed");
    let input = s.write("sq.dxf", SQUARE_DXF);
    let out = s.path("bed.nc");
    let (c, _, e) = run(&["cut", &input, &out, "--bed", "5,5"]);
    assert_eq!(c, Some(1));
    assert!(e.contains("bed"), "{e}");
    assert!(!PathBuf::from(&out).exists());
    let (c, _, e) = run(&["cut", &input, &out, "--bed", "10,10"]);
    assert_eq!(c, Some(0), "{e}");
    let (c, _, e) = run(&["cut", &input, &out, "--bed", "10"]);
    assert_eq!(c, Some(1));
    assert!(e.contains("--bed needs X,Y"), "{e}");
}

#[test]
fn the_options_and_the_files_that_are_wrong_say_so() {
    let s = Scratch::new("wrong");
    let input = s.write("sq.dxf", SQUARE_DXF);
    let out = s.path("w.nc");
    let text = s.write("notes.txt", "hello");
    for (args, needle) in [
        (vec!["cut", &input, &out, "--powr", "5"], "--powr"),
        (vec!["cut", &input, &out, "--power", "abc"], "--power"),
        (vec!["cut", &input, &out, "--power", "500"], "power"),
        (vec!["cut", &input, &out, "--power", "0"], "power"),
        (vec!["cut", &input, &out, "--speed", "0"], "speed"),
        (vec!["cut", &input, &out, "--passes", "1.5"], "--passes"),
        (vec!["cut", &input, &out, "--passes", "0"], "passes"),
        (
            vec!["cut", &input, &out, "--kerf-side", "left"],
            "--kerf-side",
        ),
        (
            vec!["cut", &input, &out, "--origin", "1"],
            "--origin needs X,Y",
        ),
        (vec!["cut", &input, &out, "--smax", "0"], "s_max"),
        (vec!["cut", &input, &out, "--dpi", "0"], "dpi"),
        (vec!["cut", &input, &out, "--tolerance", "5"], "tolerance"),
        (vec!["cut", &input, &out, "--speed"], "needs a value"),
        (vec!["cut", &input, &out, "extra"], "unexpected argument"),
        (vec!["cut", &text, &out], "not an SVG or a DXF"),
        (vec!["cut", "missing.svg", &out], "missing.svg"),
        (vec!["layers"], "needs a drawing"),
        (vec!["layers", &input, "extra"], "unexpected argument"),
    ] {
        let (c, _, e) = run(&args);
        assert_eq!(c, Some(1), "{args:?}: {e}");
        assert!(e.contains(needle), "{args:?} gave {e:?}");
        assert!(!PathBuf::from(&out).exists(), "{args:?} wrote a file");
    }
}

#[test]
fn hostile_files_are_refused_without_a_panic() {
    let s = Scratch::new("hostile");
    let out = s.path("h.nc");
    let deep = format!("<svg>{}{}</svg>", "<g>".repeat(5000), "</g>".repeat(5000));
    let far = r##"<svg><path d="M0 0 L1e308 1e308" stroke="#f00"/><path d="M0 0 L-1e308 5" stroke="#f00"/></svg>"##;
    let bomb = r#"<?xml version="1.0"?><!DOCTYPE s [<!ENTITY a "aaaaaaaaaa"><!ENTITY b "&a;&a;&a;&a;&a;&a;&a;&a;">]><svg><path d="M0 0 L&b; 1" stroke="red"/></svg>"#;
    let script = r##"<svg><script>alert(1)</script><image href="file:///etc/passwd"/><use href="http://example.org/x.svg#a"/><path d="M0 0 L5 0" stroke="#f00"/></svg>"##;
    for (name, body) in [
        ("empty.svg", String::new()),
        ("garbage.svg", "\u{0}\u{1}\u{2}<<<>>>&&&".to_string()),
        ("deep.svg", deep),
        ("far.svg", far.to_string()),
        ("bomb.svg", bomb.to_string()),
        ("cut.dxf", SQUARE_DXF[..40].to_string()),
        ("text.dxf", "hello\nworld\n".to_string()),
        ("binary.dxf", "AutoCAD Binary DXF\r\n\u{1a}\0".to_string()),
    ] {
        let input = s.write(name, &body);
        let (c, _, e) = run(&["cut", &input, &out]);
        assert_ne!(c, Some(101), "{name} panicked: {e}");
        assert_eq!(c, Some(1), "{name}: {e}");
        assert!(!e.contains("panicked"), "{name}: {e}");
        assert!(!PathBuf::from(&out).exists(), "{name} wrote a file");
    }
    // a script, an image and a link are not run or fetched: they are skipped with a note and the rest is cut
    let input = s.write("script.svg", script);
    // one unit of the drawing is one mm at 25.4 pixels to the inch
    let (c, stdout, e) = run(&["cut", &input, &out, "--dpi", "25.4"]);
    assert_eq!(c, Some(0), "{e}");
    assert!(
        e.contains("script") && e.contains("image") && e.contains("use"),
        "{e}"
    );
    assert!(stdout.contains("1 paths, 5 mm cut"), "{stdout}");
}

#[test]
fn a_damaged_copy_of_a_valid_file_is_never_a_panic() {
    let svg = PLATE_SVG.as_bytes();
    let dxf = SQUARE_DXF.as_bytes();
    let opts = ReadOptions::default();
    for end in 0..svg.len() {
        let _ = read_svg(&svg[..end], &opts);
    }
    for end in 0..dxf.len() {
        let _ = read_dxf(&dxf[..end], &opts);
    }
    // bytes changed at every place
    for at in 0..svg.len() {
        let mut b = svg.to_vec();
        b[at] = b'<';
        let _ = read_svg(&b, &opts);
        b[at] = 0xff;
        let _ = read_svg(&b, &opts);
    }
    for at in 0..dxf.len() {
        let mut b = dxf.to_vec();
        b[at] = b'9';
        let _ = read_dxf(&b, &opts);
        b[at] = b'\n';
        let _ = read_dxf(&b, &opts);
    }
}

#[test]
fn whatever_the_settings_the_program_passes_its_own_safety_check_or_is_not_given() {
    let drawing = read_svg(PLATE_SVG.as_bytes(), &ReadOptions::default()).unwrap();
    let mut made = 0;
    for mode in ["cut", "engrave"] {
        for power in ["0.001", "0.5", "50", "100"] {
            for s_max in ["100", "255", "1000", "100000"] {
                for kerf in ["0", "0.3"] {
                    for tabs in ["0", "3"] {
                        for passes in ["1", "3"] {
                            let text = format!(
                                "[machine]\ns_max = {s_max}\noverscan_mm = 1\n[defaults]\nmode = {mode}\npower = {power}\nkerf = {kerf}\nkerf_side = auto\ntabs = {tabs}\ntab_width = 1.5\npasses = {passes}\nfocus_step = -0.2\nhatch_spacing = 0.5\nhatch_angle = 30\n"
                            );
                            let settings = parse(&text).unwrap();
                            if let Ok(p) = plan(&drawing, &settings) {
                                made += 1;
                                verify_gcode(&p.gcode, settings.machine.s_max, None).unwrap();
                                assert!(p.gcode.ends_with("M5\nM2\n"));
                                // the full power value is the most S ever written
                                let top = code(&p.gcode)
                                    .iter()
                                    .flat_map(|l| l.split_whitespace())
                                    .filter_map(|w| w.strip_prefix('S'))
                                    .filter_map(|n| n.parse::<u32>().ok())
                                    .max()
                                    .unwrap();
                                assert!(top <= settings.machine.s_max);
                            }
                        }
                    }
                }
            }
        }
    }
    // 2 modes x 4 powers x 4 full power values x 2 beam widths x 2 tab counts x 2 pass counts are 256 jobs. A power of 0.001 % of a full power value of 100, 255 or 1000 is less than half
    // of S1 and rounds to nothing, so it is refused: 2 modes x 3 values x 2 x 2 x 2 = 48 jobs. 0.5 % of 100 is S0.5, which rounds up to S1, and every other job is made.
    assert_eq!(made, 256 - 48);
}

#[test]
fn the_same_job_twice_gives_the_same_program() {
    let drawing = read_svg(PLATE_SVG.as_bytes(), &ReadOptions::default()).unwrap();
    let settings =
        parse("[defaults]\nkerf = 0.2\nkerf_side = auto\ntabs = 2\npasses = 2\n").unwrap();
    let a = plan(&drawing, &settings).unwrap();
    let b = plan(&drawing, &settings).unwrap();
    assert_eq!(a, b);
    assert_eq!(
        plan(&drawing, &Settings::default()).unwrap().stats.contours,
        2
    );
}

/// The number in front of a word in the line the commands print ("246 mm cut").
fn number_before(text: &str, word: &str) -> f64 {
    let at = text
        .find(word)
        .unwrap_or_else(|| panic!("{word} is not in {text:?}"));
    text[..at]
        .trim_end()
        .rsplit(' ')
        .next()
        .unwrap()
        .parse()
        .unwrap()
}

#[test]
fn the_examples_are_cut_and_engraved_with_their_settings_file() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cfg = root
        .join("examples/plate.cfg")
        .to_string_lossy()
        .into_owned();
    let s = Scratch::new("examples");
    // (file, the layer that is engraved, the layer that is cut, the length cut in mm)
    // The outline (38 x 23 mm, 122 mm; with round corners of 3 mm 116.85 mm) grows by 2 pi 0.075 mm at its corners and loses its two tabs of 1.5 mm; the hole (radius 1.5 mm) shrinks to 1.425 mm and is
    // 8.95 mm long; both are cut twice at 300 mm/min. The panel of 20 x 9 mm is engraved with lines 0.2 mm apart: 180 / 0.2 = 900 mm.
    for (name, engraved, cut, low, high) in [
        ("plate.svg", "#0000ff", "#ff0000", 245.5, 247.0),
        ("plate.dxf", "mark", "cut", 256.0, 257.5),
    ] {
        let input = root
            .join("examples")
            .join(name)
            .to_string_lossy()
            .into_owned();
        let out = s.path(&format!("{name}.nc"));
        let (c, stdout, e) = run(&["cut", &input, &out, "--config", &cfg]);
        assert_eq!(c, Some(0), "{name}: {e}");
        assert!(stdout.starts_with("2 layers, 3 paths, "), "{stdout}");
        let mm_cut = number_before(&stdout, " mm cut");
        assert!(mm_cut > low && mm_cut < high, "{name}: {mm_cut}");
        assert!(
            (number_before(&stdout, " mm engraved") - 900.0).abs() < 1.0,
            "{stdout}"
        );
        let text = fs::read_to_string(&out).unwrap();
        verify_gcode(&text, 1000, None).unwrap();
        let layers: Vec<&str> = text.lines().filter(|l| l.starts_with("; layer ")).collect();
        assert!(
            layers[0].starts_with(&format!("; layer {engraved}: engrave, 25 %, 1500 mm/min")),
            "{layers:?}"
        );
        assert!(
            layers[1].starts_with(&format!("; layer {cut}: cut, 90 %, 300 mm/min, 2 pass(es)")),
            "{layers:?}"
        );
        let c = code(&text);
        // 25 % and 90 % of 1000, the speeds of the two layers, and the focus going down 0.1 mm for the second pass and coming back
        for want in ["F1500", "F300", "G0 Z0", "G0 Z-0.1"] {
            assert!(c.contains(&want), "{name}: {want}");
        }
        assert!(c.iter().any(|l| l.ends_with(" S250")) && c.iter().any(|l| l.ends_with(" S900")));
        // the layers listing agrees
        let (c, listing, _) = run(&["layers", &input, "--config", &cfg]);
        assert_eq!(c, Some(0));
        assert!(
            listing.contains("engrave at 25 % and 1500 mm/min, 1 pass(es)")
                && listing.contains("cut at 90 % and 300 mm/min, 2 pass(es)"),
            "{listing}"
        );
    }
}

#[test]
fn every_setting_the_file_reads_is_documented_and_every_documented_setting_is_read() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(root.join("src/vconfig.rs")).unwrap();
    let doc = fs::read_to_string(root.join("docs/vector-settings.md")).unwrap();
    let layer = source.find("fn set_layer").unwrap();
    let global = source.find("fn set_global").unwrap();
    let end = source.find("pub fn parse").unwrap();
    let mut keys: Vec<String> = Vec::new();
    // the keys of a layer are the arms `"key" => ...` of set_layer
    for line in source[layer..global].lines() {
        if let Some((key, after)) = line
            .trim()
            .strip_prefix('"')
            .and_then(|r| r.split_once('"'))
        {
            if after.trim_start().starts_with("=>") {
                keys.push(key.to_string());
            }
        }
    }
    let layer_keys = keys.len();
    // the others are the arms `("section", "key") => ...` of set_global
    for line in source[global..end].lines() {
        if let Some(rest) = line.trim().strip_prefix("(\"") {
            // `limits", "max_depth") => ...` splits into the section, ", ", the key and the rest
            let parts: Vec<&str> = rest.split('"').collect();
            if parts.len() > 3 && parts[1] == ", " {
                keys.push(parts[2].to_string());
            }
        }
    }
    assert_eq!(layer_keys, 12, "{keys:?}");
    assert_eq!(keys.len(), 12 + 31, "{keys:?}");
    for k in &keys {
        assert!(
            doc.contains(&format!("`{k}`")),
            "the setting {k} is not in docs/vector-settings.md"
        );
    }
    // the first cell of a row of the tables of keys holds names in code format, and each one is a key that is read
    for line in doc.lines().filter(|l| l.starts_with("| `")) {
        let cell = line.trim_start_matches('|').split('|').next().unwrap();
        for word in cell.split('`').skip(1).step_by(2) {
            if word.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
                assert!(
                    keys.iter().any(|k| k == word),
                    "{word} is documented but not read"
                );
            }
        }
    }
}
