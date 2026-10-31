use eh3d_laser_cam::calibrate::{grid, Grid};
use eh3d_laser_cam::dither::{dither, Dots, Method};
use eh3d_laser_cam::image::{from_bytes, from_pgm, from_png, Gray, MAX_PIXELS};
use eh3d_laser_cam::raster::{engrave, overscan_for, Params};

fn gray(w: usize, h: usize, v: u8) -> Gray {
    Gray::new(w, h, vec![v; w * h]).unwrap()
}

fn png_of(w: u32, h: u32, color: png::ColorType, depth: png::BitDepth, data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut enc = png::Encoder::new(&mut out, w, h);
    enc.set_color(color);
    enc.set_depth(depth);
    enc.write_header().unwrap().write_image_data(data).unwrap();
    out
}

#[test]
fn a_pgm_is_read_in_both_forms_with_comments() {
    let text = from_pgm(b"P2\n# a comment\n3 2\n255\n0 128 255\n255 128 0\n").unwrap();
    assert_eq!(
        (text.w, text.h, text.data.clone()),
        (3, 2, vec![0, 128, 255, 255, 128, 0])
    );
    let mut bin = b"P5\n2 2\n255\n".to_vec();
    bin.extend([10, 20, 30, 40]);
    assert_eq!(from_pgm(&bin).unwrap().data, vec![10, 20, 30, 40]);
    assert_eq!(
        from_pgm(b"P2\n2 1\n15\n0 15\n").unwrap().data,
        vec![0, 255],
        "a maximum value of 15 is scaled to 255"
    );
}

#[test]
fn a_damaged_pgm_is_an_error() {
    for bad in [
        &b""[..],
        b"P6\n1 1\n255\n\0\0\0",
        b"P5\n2 2\n255\n\x01",
        b"P2\n2 2\n255\n1 2 3",
        b"P2\n0 5\n255\n",
        b"P2\n3 3\n65535\n",
        b"P2\n99999 99999\n255\n",
        b"P2\nx y\n255\n",
        b"P5",
    ] {
        assert!(from_pgm(bad).is_err(), "{:?}", String::from_utf8_lossy(bad));
    }
    assert!(Gray::new(MAX_PIXELS, 2, vec![]).is_err());
    assert!(Gray::new(2, 2, vec![0; 3]).is_err());
}

#[test]
fn a_png_in_grey_colour_and_transparency_becomes_grey_levels() {
    let g = from_png(&png_of(
        2,
        1,
        png::ColorType::Grayscale,
        png::BitDepth::Eight,
        &[0, 200],
    ))
    .unwrap();
    assert_eq!(g.data, vec![0, 200]);
    let c = from_png(&png_of(
        2,
        1,
        png::ColorType::Rgb,
        png::BitDepth::Eight,
        &[255, 0, 0, 255, 255, 255],
    ))
    .unwrap();
    assert_eq!(c.data, vec![76, 255], "red is 0.299 of white");
    let a = from_png(&png_of(
        2,
        1,
        png::ColorType::Rgba,
        png::BitDepth::Eight,
        &[0, 0, 0, 0, 0, 0, 0, 255],
    ))
    .unwrap();
    assert_eq!(
        a.data,
        vec![255, 0],
        "transparent is white and opaque black is black"
    );
    let sixteen = from_png(&png_of(
        1,
        1,
        png::ColorType::Grayscale,
        png::BitDepth::Sixteen,
        &[0x80, 0x00],
    ))
    .unwrap();
    assert_eq!(sixteen.data, vec![128]);
    assert_eq!(
        from_bytes(&png_of(
            1,
            1,
            png::ColorType::Grayscale,
            png::BitDepth::Eight,
            &[7]
        ))
        .unwrap()
        .data,
        vec![7]
    );
    assert!(from_bytes(b"P5\n1 1\n255\n\x09").is_ok());
}

#[test]
fn a_damaged_png_is_an_error() {
    let good = png_of(
        4,
        4,
        png::ColorType::Grayscale,
        png::BitDepth::Eight,
        &[9; 16],
    );
    assert!(from_png(&good[..good.len() / 2]).is_err());
    assert!(from_png(&good[..8]).is_err());
    assert!(from_png(b"not a png").is_err());
}

#[test]
fn resizing_averages_the_pixels() {
    let g = Gray::new(4, 2, vec![0, 0, 255, 255, 0, 0, 255, 255]).unwrap();
    assert_eq!(g.resize(2, 1).unwrap().data, vec![0, 255]);
    let up = g.resize(8, 4).unwrap();
    assert_eq!((up.w, up.h, up.at(0, 0), up.at(7, 3)), (8, 4, 0, 255));
    assert_eq!(g.resize(1, 1).unwrap().data, vec![127]);
    assert!(g.resize(MAX_PIXELS, 2).is_err());
}

fn ratio(d: &Dots) -> f64 {
    d.count() as f64 / (d.w * d.h) as f64
}

#[test]
fn a_threshold_burns_what_is_darker() {
    let g = Gray::new(4, 1, vec![0, 100, 128, 255]).unwrap();
    assert_eq!(
        dither(&g, Method::Threshold(128)).burn,
        vec![true, true, false, false]
    );
    assert_eq!(dither(&g, Method::Threshold(0)).count(), 0);
    assert_eq!(dither(&g, Method::Threshold(255)).count(), 3);
}

#[test]
fn error_diffusion_keeps_the_darkness_of_the_picture() {
    for method in [Method::FloydSteinberg, Method::Atkinson] {
        assert_eq!(
            dither(&gray(40, 40, 255), method).count(),
            0,
            "white burns nothing"
        );
        assert_eq!(
            dither(&gray(40, 40, 0), method).count(),
            40 * 40,
            "black burns everything"
        );
        for (level, expected) in [(64u8, 0.75), (128, 0.5), (192, 0.25)] {
            let r = ratio(&dither(&gray(64, 64, level), method));
            let tolerance = if method == Method::Atkinson {
                0.15
            } else {
                0.03
            };
            assert!(
                (r - expected).abs() < tolerance,
                "{method:?} grey {level}: {r}"
            );
        }
    }
}

#[test]
fn dithering_is_deterministic_and_handles_any_shape() {
    let g = Gray::new(7, 5, (0..35).map(|i| (i * 7) as u8).collect()).unwrap();
    assert_eq!(
        dither(&g, Method::FloydSteinberg),
        dither(&g, Method::FloydSteinberg)
    );
    for (w, h) in [(1, 1), (1, 9), (9, 1)] {
        let d = dither(&gray(w, h, 100), Method::Atkinson);
        assert_eq!(d.burn.len(), w * h);
    }
}

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

#[test]
fn a_row_of_dots_becomes_runs_with_the_power_on_only_where_it_burns() {
    let e = engrave(&dots(&["##..##"]), &params()).unwrap();
    assert_eq!(
        e.gcode,
        "G21\nG90\nM4 S0\nF6000\nG0 X0 Y0\nG1 X2 S500\nG1 X4 S0\nG1 X6 S500\nM5\nM2\n"
    );
    assert_eq!((e.rows, e.runs), (1, 2));
    assert!((e.burn_mm - 4.0).abs() < 1e-9);
}

#[test]
fn the_rows_go_back_and_forth_and_the_top_of_the_image_is_the_highest_y() {
    let e = engrave(&dots(&["#.", "#."]), &params()).unwrap();
    let lines: Vec<&str> = e.gcode.lines().collect();
    assert!(
        lines.contains(&"G0 X0 Y1"),
        "the first row of the image is the top: {lines:?}"
    );
    assert!(
        lines.contains(&"G0 X1 Y0"),
        "the second row starts where the first one ended: {lines:?}"
    );
    assert!(
        lines.contains(&"G1 X0 S500"),
        "and goes right to left: {lines:?}"
    );
}

#[test]
fn an_empty_row_is_skipped_and_an_empty_picture_burns_nothing() {
    let e = engrave(&dots(&["..", "#.", ".."]), &params()).unwrap();
    assert_eq!((e.rows, e.runs), (1, 1));
    let none = engrave(&dots(&["...", "..."]), &params()).unwrap();
    assert_eq!((none.rows, none.runs, none.burn_mm), (0, 0, 0.0));
    assert!(!none.gcode.contains("S500"));
}

#[test]
fn the_overscan_runs_the_head_past_the_work_with_the_laser_off() {
    let p = Params {
        overscan_mm: 3.0,
        ..params()
    };
    let e = engrave(&dots(&["..##.."]), &p).unwrap();
    let lines: Vec<&str> = e.gcode.lines().collect();
    assert!(lines.contains(&"G0 X-1 Y0"), "{lines:?}");
    assert!(
        lines.contains(&"G1 X2 S0"),
        "the run-up at the feed with the laser off: {lines:?}"
    );
    assert_eq!(
        &lines[lines.len() - 4..],
        ["G1 X4 S500", "G1 X7 S0", "M5", "M2"]
    );
}

#[test]
fn the_run_up_is_what_the_acceleration_needs() {
    assert!(
        (overscan_for(6000.0, 1000.0).unwrap() - 5.0).abs() < 1e-9,
        "100 mm/s with 1000 mm/s^2 needs 5 mm"
    );
    assert!(overscan_for(0.0, 1000.0).is_err() && overscan_for(100.0, f64::NAN).is_err());
}

#[test]
fn the_parameters_are_checked() {
    let d = dots(&["#"]);
    for bad in [
        Params {
            pixel_mm: 0.0,
            ..params()
        },
        Params {
            pixel_mm: 6.0,
            ..params()
        },
        Params {
            feed_mm_min: 0.0,
            ..params()
        },
        Params {
            feed_mm_min: f64::NAN,
            ..params()
        },
        Params {
            power_percent: 101.0,
            ..params()
        },
        Params {
            power_percent: -1.0,
            ..params()
        },
        Params {
            overscan_mm: 60.0,
            ..params()
        },
        Params {
            s_max: 0,
            ..params()
        },
        Params {
            origin: (f64::INFINITY, 0.0),
            ..params()
        },
    ] {
        assert!(engrave(&d, &bad).is_err(), "{bad:?}");
    }
    assert_eq!(
        Params {
            s_max: 255,
            power_percent: 100.0,
            ..params()
        }
        .power(),
        255
    );
}

#[test]
fn the_origin_moves_the_whole_engraving() {
    let e = engrave(
        &dots(&["#"]),
        &Params {
            origin: (10.0, 20.0),
            ..params()
        },
    )
    .unwrap();
    assert!(
        e.gcode.contains("G0 X10 Y20") && e.gcode.contains("G1 X11 S500"),
        "{}",
        e.gcode
    );
}

#[test]
fn the_time_is_the_distance_at_the_feed() {
    let e = engrave(&dots(&["######"]), &params()).unwrap();
    assert!(
        (e.seconds - 0.06).abs() < 1e-9,
        "6 mm at 100 mm/s: {}",
        e.seconds
    );
}

#[test]
fn the_calibration_grid_has_a_square_for_every_pair() {
    let g = grid(
        &[20.0, 80.0],
        &[1000.0, 2000.0, 3000.0],
        &Grid {
            square_mm: 2.0,
            gap_mm: 1.0,
            line_spacing_mm: 0.5,
            s_max: 1000,
        },
    )
    .unwrap();
    assert_eq!(g.matches("(power").count(), 6);
    assert!(g.contains("(power 20% feed 1000)") && g.contains("(power 80% feed 3000)"));
    assert!(g.contains("S200") && g.contains("S800"));
    assert!(
        g.contains("G0 X3 Y6"),
        "the third feed, the second power: {g}"
    );
    assert!(g.starts_with("G21\nG90\nM4 S0\n") && g.ends_with("M5\nM2\n"));
}

#[test]
fn the_calibration_grid_refuses_what_makes_no_sense() {
    let ok = Grid::default();
    assert!(grid(&[], &[1000.0], &ok).is_err() && grid(&[10.0], &[], &ok).is_err());
    assert!(grid(&[101.0], &[1000.0], &ok).is_err() && grid(&[-1.0], &[1000.0], &ok).is_err());
    assert!(grid(&[10.0], &[0.5], &ok).is_err() && grid(&[10.0], &[f64::NAN], &ok).is_err());
    assert!(grid(&[10.0; 13], &[1000.0], &ok).is_err());
    assert!(grid(
        &[10.0],
        &[1000.0],
        &Grid {
            line_spacing_mm: 0.0,
            ..ok
        }
    )
    .is_err());
    assert!(grid(
        &[10.0],
        &[1000.0],
        &Grid {
            square_mm: 0.5,
            ..ok
        }
    )
    .is_err());
}
