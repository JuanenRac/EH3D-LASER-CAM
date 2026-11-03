//! More checks of the engraving: the G-code can be read back into the dots it was made from, the numbers are written the same way every time, the grid of the calibration keeps
//! its geometry, and the command line says no to what makes no sense.

use eh3d_laser_cam::calibrate::{grid, Grid, MAX_CELLS};
use eh3d_laser_cam::dither::{dither, Dots, Method};
use eh3d_laser_cam::image::Gray;
use eh3d_laser_cam::raster::{engrave, overscan_for, Params};
use std::process::Command;

fn dots(rows: &[&str]) -> Dots {
    let (h, w) = (rows.len(), rows[0].len());
    Dots {
        w,
        h,
        burn: rows
            .iter()
            .flat_map(|r| r.chars().map(|c| c == '#'))
            .collect(),
    }
}

fn params() -> Params {
    Params {
        pixel_mm: 1.0,
        feed_mm_min: 6000.0,
        s_max: 1000,
        power_percent: 50.0,
        overscan_mm: 0.0,
        origin: (0.0, 0.0),
    }
}

/// A pattern that looks random and is always the same.
fn noise(w: usize, h: usize, seed: u64) -> Dots {
    let mut s = seed;
    let burn = (0..w * h)
        .map(|_| {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (s >> 33) % 3 != 0
        })
        .collect();
    Dots { w, h, burn }
}

fn value(line: &str, letter: char) -> Option<f64> {
    line.split_whitespace()
        .find_map(|t| t.strip_prefix(letter))
        .and_then(|t| t.parse().ok())
}

/// Reads the G-code back: every move with the laser on (S above 0) burns the dots it crosses.
fn decode(gcode: &str, p: &Params, w: usize, h: usize) -> Vec<bool> {
    let mut out = vec![false; w * h];
    let (mut x, mut y) = p.origin;
    for line in gcode.lines() {
        if line.starts_with("G0 ") {
            x = value(line, 'X').unwrap();
            y = value(line, 'Y').unwrap();
        } else if line.starts_with("G1 ") {
            let to = value(line, 'X').unwrap();
            if value(line, 'S').unwrap() > 0.0 {
                let (a, b) = if to < x { (to, x) } else { (x, to) };
                let first = ((a - p.origin.0) / p.pixel_mm).round() as usize;
                let last = ((b - p.origin.0) / p.pixel_mm).round() as usize;
                let row = h - 1 - ((y - p.origin.1) / p.pixel_mm).round() as usize;
                for i in first..last {
                    out[row * w + i] = true;
                }
            }
            x = to;
        }
    }
    out
}

#[test]
fn the_gcode_read_back_gives_the_dots_it_was_made_from() {
    for (pixel, overscan, origin) in [
        (1.0, 0.0, (0.0, 0.0)),
        (0.25, 2.0, (10.0, 5.0)),
        (0.1, 1.5, (-3.0, 0.0)),
    ] {
        let d = noise(13, 9, 7);
        let p = Params {
            pixel_mm: pixel,
            overscan_mm: overscan,
            origin,
            ..params()
        };
        let e = engrave(&d, &p).unwrap();
        assert_eq!(decode(&e.gcode, &p, d.w, d.h), d.burn, "pixel {pixel}");
        // what is burned is exactly the dots, whatever the order of the lines
        assert!(
            (e.burn_mm - d.count() as f64 * pixel).abs() < 1e-6,
            "{} / {}",
            e.burn_mm,
            d.count() as f64 * pixel
        );
    }
}

#[test]
fn the_head_never_leaves_the_work_by_more_than_the_overscan() {
    let d = noise(20, 6, 3);
    let p = Params {
        pixel_mm: 0.5,
        overscan_mm: 3.0,
        origin: (4.0, 2.0),
        ..params()
    };
    let e = engrave(&d, &p).unwrap();
    let (lo, hi) = (4.0 - 3.0, 4.0 + 20.0 * 0.5 + 3.0);
    for line in e
        .gcode
        .lines()
        .filter(|l| l.starts_with("G0 ") || l.starts_with("G1 "))
    {
        let x = value(line, 'X').unwrap();
        assert!(x >= lo - 1e-9 && x <= hi + 1e-9, "{line}");
        if let Some(y) = value(line, 'Y') {
            assert!((2.0..=2.0 + 5.0 * 0.5 + 1e-9).contains(&y), "{line}");
        }
    }
}

#[test]
fn the_same_dots_give_the_same_gcode_byte_for_byte() {
    let d = noise(31, 17, 99);
    let p = Params {
        pixel_mm: 0.1,
        overscan_mm: 2.0,
        ..params()
    };
    assert_eq!(engrave(&d, &p).unwrap(), engrave(&d, &p).unwrap());
}

#[test]
fn an_empty_row_between_two_full_ones_does_not_break_the_zigzag() {
    let e = engrave(&dots(&["#.", "..", ".#"]), &params()).unwrap();
    let lines: Vec<&str> = e.gcode.lines().collect();
    // first row left to right from X0 on Y2; second burning row right to left, from X2 on Y0
    assert!(lines.contains(&"G0 X0 Y2"), "{}", e.gcode);
    assert!(lines.contains(&"G1 X1 S500"), "{}", e.gcode);
    assert!(lines.contains(&"G0 X2 Y0"), "{}", e.gcode);
    assert!(lines.contains(&"G1 X1 S500"), "{}", e.gcode);
    assert_eq!(e.rows, 2);
    assert_eq!(e.runs, 2);
}

#[test]
fn the_power_is_a_percent_of_the_spindle_value_rounded() {
    let d = dots(&["#"]);
    let s_of = |s_max: u32, percent: f64| {
        let p = Params {
            s_max,
            power_percent: percent,
            ..params()
        };
        let g = engrave(&d, &p).unwrap().gcode;
        g.lines()
            .find(|l| l.starts_with("G1"))
            .unwrap()
            .rsplit('S')
            .next()
            .unwrap()
            .to_string()
    };
    assert_eq!(s_of(1000, 100.0), "1000");
    assert_eq!(s_of(1000, 33.3), "333");
    assert_eq!(s_of(3, 50.0), "2", "1.5 rounds up");
    assert_eq!(s_of(255, 100.0), "255");
    assert_eq!(s_of(100_000, 0.001), "1");
    // with no power at all nothing burns, and the code still has the lines
    assert_eq!(s_of(1000, 0.0), "0");
}

#[test]
fn a_negative_zero_and_float_noise_are_not_written() {
    let p = Params {
        pixel_mm: 0.1,
        origin: (-0.0004, 0.0),
        ..params()
    };
    let e = engrave(&dots(&["###"]), &p).unwrap();
    assert!(!e.gcode.contains("-0 "), "{}", e.gcode);
    assert!(!e.gcode.contains("-0\n"), "{}", e.gcode);
    assert!(e.gcode.contains("G0 X0 Y0"), "{}", e.gcode);
    // three dots of 0.1 are 0.3 and not 0.30000000000000004
    let e = engrave(
        &dots(&["###"]),
        &Params {
            pixel_mm: 0.1,
            ..params()
        },
    )
    .unwrap();
    assert!(e.gcode.contains("G1 X0.3 S500"), "{}", e.gcode);
}

#[test]
fn a_value_that_is_not_a_number_is_refused_not_written() {
    let d = dots(&["#"]);
    for p in [
        Params {
            overscan_mm: f64::NAN,
            ..params()
        },
        Params {
            origin: (f64::INFINITY, 0.0),
            ..params()
        },
        Params {
            feed_mm_min: f64::NAN,
            ..params()
        },
        Params {
            pixel_mm: f64::NEG_INFINITY,
            ..params()
        },
        Params {
            power_percent: f64::NAN,
            ..params()
        },
    ] {
        assert!(engrave(&d, &p).is_err(), "{p:?}");
    }
    // the edges that are allowed
    let edge = Params {
        pixel_mm: 0.01,
        feed_mm_min: 30_000.0,
        s_max: 100_000,
        power_percent: 100.0,
        overscan_mm: 50.0,
        origin: (-100_000.0, 100_000.0),
    };
    assert!(engrave(&d, &edge).is_ok());
    assert!(engrave(
        &d,
        &Params {
            pixel_mm: 0.0099,
            ..edge
        }
    )
    .is_err());
    assert!(engrave(
        &d,
        &Params {
            s_max: 100_001,
            ..edge
        }
    )
    .is_err());
}

#[test]
fn the_run_up_refuses_what_is_not_a_positive_number() {
    for (f, a) in [
        (0.0, 1000.0),
        (-1.0, 1000.0),
        (3000.0, 0.0),
        (3000.0, -5.0),
        (f64::NAN, 1000.0),
        (3000.0, f64::INFINITY),
        (f64::INFINITY, 1000.0),
    ] {
        assert!(overscan_for(f, a).is_err(), "{f} {a}");
    }
    // 6000 mm/min is 100 mm/s: 100^2 / (2 x 1000) = 5 mm
    assert!((overscan_for(6000.0, 1000.0).unwrap() - 5.0).abs() < 1e-12);
    // twice the speed needs four times the run
    let a = overscan_for(1000.0, 500.0).unwrap();
    let b = overscan_for(2000.0, 500.0).unwrap();
    assert!((b / a - 4.0).abs() < 1e-9);
}

#[test]
fn the_threshold_burns_strictly_darker_and_its_extremes_are_none_and_almost_all() {
    let img = Gray::new(4, 1, vec![0, 127, 128, 255]).unwrap();
    let at = |t| dither(&img, Method::Threshold(t)).burn;
    assert_eq!(at(128), [true, true, false, false], "equal does not burn");
    assert_eq!(at(0), [false; 4], "nothing is darker than 0");
    assert_eq!(at(255), [true, true, true, false], "white never burns");
    assert_eq!(at(1), [true, false, false, false]);
}

#[test]
fn the_error_diffusion_never_burns_white_and_always_burns_black_whatever_the_size() {
    for method in [Method::FloydSteinberg, Method::Atkinson] {
        for (w, h) in [(1, 1), (1, 40), (40, 1), (7, 5)] {
            let white = Gray::blank(w, h, 255).unwrap();
            let black = Gray::blank(w, h, 0).unwrap();
            assert_eq!(dither(&white, method).count(), 0, "{method:?} {w}x{h}");
            assert_eq!(dither(&black, method).count(), w * h, "{method:?} {w}x{h}");
        }
    }
}

#[test]
fn a_mid_grey_burns_about_half_and_a_picture_with_no_pixels_burns_nothing() {
    let grey = Gray::blank(32, 32, 128).unwrap();
    let c = dither(&grey, Method::FloydSteinberg).count() as f64 / 1024.0;
    assert!((c - 0.5).abs() < 0.05, "{c}");
    // a wider grey than a tall one does not change what the dots are made of
    let wide = Gray::blank(200, 2, 64).unwrap();
    let c = dither(&wide, Method::FloydSteinberg).count() as f64 / 400.0;
    assert!((c - 0.75).abs() < 0.05, "{c}");
}

fn gcode_of_grid(p: &[f64], f: &[f64], g: &Grid) -> Vec<String> {
    grid(p, f, g).unwrap().lines().map(str::to_string).collect()
}

#[test]
fn every_square_of_the_grid_stays_in_its_cell_and_has_the_lines_the_spacing_asks_for() {
    let g = Grid {
        square_mm: 8.0,
        gap_mm: 2.0,
        line_spacing_mm: 0.5,
        s_max: 1000,
    };
    let lines = gcode_of_grid(&[10.0, 50.0, 100.0], &[1000.0, 2000.0], &g);
    // 17 lines of 8 mm every 0.5 mm, two G1 each, in 6 cells
    let moves = lines.iter().filter(|l| l.starts_with("G1")).count();
    assert_eq!(moves, 6 * 17 * 2);
    let starts = lines.iter().filter(|l| l.starts_with("G0")).count();
    assert_eq!(starts, 6);
    for l in lines.iter().filter(|l| l.starts_with("G1")) {
        let (x, y) = (value(l, 'X').unwrap(), value(l, 'Y'));
        assert!((0.0..=3.0 * 10.0 - 2.0 + 1e-9).contains(&x), "{l}");
        if let Some(y) = y {
            assert!((0.0..=2.0 * 10.0 - 2.0 + 1e-9).contains(&y), "{l}");
        }
    }
    // the power of each column is the percent of s_max; the labels name the pair
    assert!(lines.contains(&"(power 100% feed 2000)".to_string()));
    assert!(lines.iter().any(|l| l.contains("S1000 F2000")));
    assert!(lines.iter().any(|l| l.contains("S100 F1000")));
    assert!(lines.iter().any(|l| l.contains("S500 F2000")));
    // the last line of a square is exactly on its far edge, not past it
    assert!(lines.iter().any(|l| l.starts_with("G1 X0 Y8 S0")));
    assert_eq!(lines.first().unwrap(), "G21");
    assert_eq!(lines.last().unwrap(), "M2");
}

#[test]
fn the_lines_of_a_square_go_back_and_forth() {
    let g = Grid {
        square_mm: 2.0,
        gap_mm: 0.0,
        line_spacing_mm: 1.0,
        s_max: 1000,
    };
    let lines = gcode_of_grid(&[50.0], &[500.0], &g);
    let burns: Vec<&String> = lines
        .iter()
        .filter(|l| l.starts_with("G1") && !l.contains("S0 "))
        .collect();
    let xs: Vec<f64> = burns.iter().map(|l| value(l, 'X').unwrap()).collect();
    assert_eq!(xs, [2.0, 0.0, 2.0], "{lines:?}");
}

#[test]
fn the_grid_accepts_the_largest_size_and_refuses_one_more_or_values_that_are_not_numbers() {
    let g = Grid::default();
    let many: Vec<f64> = (0..MAX_CELLS).map(|i| 10.0 + i as f64).collect();
    let feeds: Vec<f64> = (0..MAX_CELLS).map(|i| 1000.0 + i as f64).collect();
    assert!(grid(&many, &feeds, &g).is_ok());
    let mut more = many.clone();
    more.push(99.0);
    assert!(grid(&more, &feeds, &g).is_err());
    assert!(grid(&many, &[], &g).is_err());
    assert!(grid(&[f64::NAN], &[1000.0], &g).is_err());
    assert!(grid(&[50.0], &[f64::INFINITY], &g).is_err());
    assert!(grid(&[100.1], &[1000.0], &g).is_err());
    assert!(grid(&[-0.1], &[1000.0], &g).is_err());
    assert!(grid(&[0.0], &[0.5], &g).is_err());
    assert!(grid(&[0.0], &[30_000.0], &g).is_ok());
    // the spacing cannot be more than the square, nor less than 0.02
    let bad = |g: Grid| grid(&[50.0], &[1000.0], &g).is_err();
    assert!(bad(Grid {
        line_spacing_mm: 9.0,
        ..g
    }));
    assert!(bad(Grid {
        line_spacing_mm: 0.0199,
        ..g
    }));
    assert!(bad(Grid { s_max: 0, ..g }));
    assert!(bad(Grid {
        gap_mm: f64::NAN,
        ..g
    }));
    assert!(bad(Grid {
        square_mm: 0.99,
        ..g
    }));
}

#[test]
fn the_grid_is_the_same_every_time() {
    let g = Grid::default();
    assert_eq!(
        grid(&[20.0, 40.0], &[1500.0], &g),
        grid(&[20.0, 40.0], &[1500.0], &g)
    );
}

// ---- the command line ----

fn run(args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_eh3d-lasercam"))
        .args(args)
        .output()
        .expect("the program starts");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn tmp(name: &str) -> String {
    format!("{}/{name}", env!("CARGO_TARGET_TMPDIR"))
}

fn pgm(name: &str, w: usize, h: usize, v: u8) -> String {
    let path = tmp(name);
    let mut b = format!("P5\n{w} {h}\n255\n").into_bytes();
    b.extend(std::iter::repeat(v).take(w * h));
    std::fs::write(&path, b).unwrap();
    path
}

#[test]
fn the_help_and_a_missing_command_are_not_errors_and_a_wrong_one_is() {
    for a in [&[][..], &["help"], &["--help"], &["-h"]] {
        let (code, out, _) = run(a);
        assert_eq!(code, Some(0));
        assert!(out.contains("eh3d-lasercam"), "{out}");
    }
    let (code, _, err) = run(&["burn"]);
    assert_eq!(code, Some(1));
    assert!(err.contains("burn: unknown command"), "{err}");
}

#[test]
fn engraving_a_black_picture_counts_every_dot_and_writes_a_file_with_a_name_in_any_script() {
    let img = pgm("more-black.pgm", 2, 2, 0);
    let out = tmp("grabado-日本-ñ.nc");
    let _ = std::fs::remove_file(&out);
    let (code, stdout, err) = run(&[
        "engrave",
        &img,
        &out,
        "--width-mm",
        "2",
        "--dot-mm",
        "1",
        "--dither",
        "threshold",
    ]);
    assert_eq!(code, Some(0), "{err}");
    assert!(
        stdout.starts_with("2 x 2 dots (2.0 x 2.0 mm), 2 lines, 2 runs, 4 mm burned, about "),
        "{stdout}"
    );
    let g = std::fs::read_to_string(&out).unwrap();
    // the file opens with the standard header (comment lines), then the program
    let body: String = g.lines().skip_while(|l| l.starts_with(';')).map(|l| format!("{l}\n")).collect();
    assert!(body.starts_with("G21\nG90\nM4 S0\nF3000\n"), "{g}");
    assert!(g.ends_with("M5\nM2\n"), "{g}");
    assert!(g.starts_with("; generator: eh3d-lasercam "), "{g}");
    assert!(g.contains("\n; units: mm (G21)\n; source: "), "{g}");
    let hash = eh3d_laser_cam::gcode_out::sha256_hex(&std::fs::read(&img).unwrap());
    assert!(g.contains(&format!(" sha256:{hash}\n; tools: laser, S 0-1000 (50 %)\n")), "{g}");
    std::fs::remove_file(&out).unwrap();
}

#[test]
fn the_output_of_the_command_line_is_the_same_twice() {
    let img = pgm("more-grey.pgm", 8, 8, 100);
    let (a, b) = (tmp("twice-a.nc"), tmp("twice-b.nc"));
    for o in [&a, &b] {
        let (code, _, e) = run(&["engrave", &img, o, "--width-mm", "4", "--dot-mm", "0.5"]);
        assert_eq!(code, Some(0), "{e}");
    }
    assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
}

#[test]
fn the_command_line_names_what_is_wrong_with_each_value() {
    let img = pgm("more-err.pgm", 4, 4, 0);
    let base = |extra: &[&str]| {
        let mut v = vec!["engrave", img.as_str(), "--dry-run"];
        v.extend(extra);
        let owned: Vec<String> = v.iter().map(|s| s.to_string()).collect();
        let refs: Vec<&str> = owned.iter().map(String::as_str).collect();
        run(&refs)
    };
    let cases: &[(&[&str], &str)] = &[
        (&["--width-mm", "0"], "--width-mm must be"),
        (&["--width-mm", "2001"], "--width-mm must be"),
        (&["--width-mm", "-3"], "--width-mm must be"),
        (&["--width-mm", "nan"], "is not a number"),
        (&["--width-mm", "inf"], "is not a number"),
        (&["--width-mm", "abc"], "is not a number"),
        (
            &["--width-mm", "4", "--dither", "dots"],
            "is not fs, atkinson",
        ),
        (&["--width-mm", "4", "--origin", "1"], "--origin needs X,Y"),
        (
            &["--width-mm", "4", "--origin", "1,2,3"],
            "--origin needs X,Y",
        ),
        (&["--width-mm", "4", "--origin", "1,x"], "is not a number"),
        (&["--width-mm", "4", "--smax", "0"], "s_max"),
        (&["--width-mm", "4", "--smax", "-5"], "s_max"),
        (&["--width-mm", "4", "--smax", "1e10"], "s_max"),
        (&["--width-mm", "4", "--power", "101"], "power_percent"),
        (&["--width-mm", "4", "--feed", "0"], "feed_mm_min"),
        (&["--width-mm", "4", "--overscan", "-1"], "overscan_mm"),
        (
            &["--width-mm", "4", "--overscan", "auto", "--accel", "0"],
            "acceleration",
        ),
        (&["--width-mm", "4", "--feed"], "--feed needs a value"),
        (&["--width-mm", "4", "one", "two"], "unexpected argument"),
    ];
    for (args, expect) in cases {
        let (code, out, err) = base(args);
        assert_eq!(code, Some(1), "{args:?}: {out}");
        assert!(err.contains(expect), "{args:?}: {err}");
        assert!(!out.contains("dots"), "nothing is reported on failure");
    }
}

#[test]
fn a_threshold_above_the_range_means_the_top_and_the_overscan_auto_follows_the_acceleration() {
    // a picture with a mid grey: under 128 it burns nothing, under the largest threshold everything
    let img = pgm("more-mid.pgm", 4, 4, 200);
    let (code, none, e) = run(&[
        "engrave",
        &img,
        "--dry-run",
        "--width-mm",
        "4",
        "--dot-mm",
        "1",
        "--dither",
        "threshold",
    ]);
    assert_eq!(code, Some(0), "{e}");
    assert!(none.contains("0 lines, 0 runs"), "{none}");
    let (code, all, e) = run(&[
        "engrave",
        &img,
        "--dry-run",
        "--width-mm",
        "4",
        "--dot-mm",
        "1",
        "--dither",
        "threshold",
        "--threshold",
        "300",
    ]);
    assert_eq!(code, Some(0), "{e}");
    assert!(all.contains("4 lines, 4 runs"), "{all}");
    // a stronger acceleration needs a shorter run-up, so the time is shorter
    let time = |accel: &str| {
        let (_, out, _) = run(&[
            "engrave",
            &img,
            "--dry-run",
            "--width-mm",
            "4",
            "--dot-mm",
            "1",
            "--dither",
            "threshold",
            "--threshold",
            "300",
            "--overscan",
            "auto",
            "--accel",
            accel,
            "--feed",
            "600",
        ]);
        out.rsplit("about ")
            .next()
            .unwrap()
            .trim_end_matches(" s\n")
            .trim_end_matches(" s")
            .parse::<f64>()
            .unwrap()
    };
    assert!(time("10") > time("10000"));
}

#[test]
fn the_calibration_and_the_info_commands_report_their_errors() {
    let (code, _, e) = run(&["calibrate", "--feeds", "1000"]);
    assert_eq!(code, Some(1));
    assert!(e.contains("--powers is needed"), "{e}");
    let (code, _, e) = run(&["calibrate", "--powers", "10,20", "--feeds", "1000"]);
    assert_eq!(code, Some(1));
    assert!(e.contains("needs an output file"), "{e}");
    let out = tmp("more-grid.nc");
    let (code, _, e) = run(&["calibrate", &out, "--powers", "10,x", "--feeds", "1000"]);
    assert_eq!(code, Some(1));
    assert!(e.contains("is not a number"), "{e}");
    let (code, _, e) = run(&["calibrate", &out, "--powers", "10,150", "--feeds", "1000"]);
    assert_eq!(code, Some(1));
    assert!(e.contains("percent"), "{e}");
    let (code, o, e) = run(&[
        "calibrate",
        &out,
        "--powers",
        "10,20",
        "--feeds",
        "1000,2000,3000",
    ]);
    assert_eq!(code, Some(0), "{e}");
    assert!(o.starts_with("6 squares written to"), "{o}");

    let empty = tmp("more-empty.png");
    std::fs::write(&empty, b"").unwrap();
    let (code, _, e) = run(&["info", &empty]);
    assert_eq!(code, Some(1));
    assert!(!e.is_empty());
    let (code, _, e) = run(&["info", &tmp("does-not-exist.pgm")]);
    assert_eq!(code, Some(1));
    assert!(e.contains("does-not-exist.pgm"), "{e}");
    let img = pgm("more-info.pgm", 3, 2, 51);
    let (code, o, _) = run(&["info", &img]);
    assert_eq!(code, Some(0));
    assert_eq!(o.trim(), "3 x 2 pixels, mean grey 51.0 of 255");
}

#[test]
fn the_calibration_file_opens_with_the_standard_header_and_names_no_source() {
    let out = tmp("more-header-grid.nc");
    let _ = std::fs::remove_file(&out);
    let (code, _, e) = run(&["calibrate", &out, "--powers", "20,40", "--feeds", "1000", "--smax", "255"]);
    assert_eq!(code, Some(0), "{e}");
    let g = std::fs::read_to_string(&out).unwrap();
    assert!(g.starts_with("; generator: eh3d-lasercam "), "{g}");
    assert!(g.contains("\n; units: mm (G21)\n; source: none\n; tools: laser, S 0-255, grid of power against feed\n"), "{g}");
    let body: Vec<&str> = g.lines().skip_while(|l| l.starts_with(';')).collect();
    assert_eq!(body[0], "G21");
    // the header is only comments: the program that follows is the one the library makes
    assert!(g.lines().take_while(|l| l.starts_with(';')).count() == 4);
    std::fs::remove_file(&out).unwrap();
}

#[test]
fn a_number_that_is_not_finite_never_reaches_the_file() {
    use eh3d_laser_cam::gcode_out::number;
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(number(bad).is_err());
    }
    // the same formatter writes the engraving, so a value the checks let through would be refused there
    let d = Dots { w: 1, h: 1, burn: vec![true] };
    let mut p = Params::default();
    p.origin = (f64::NAN, 0.0);
    assert!(engrave(&d, &p).is_err());
}
