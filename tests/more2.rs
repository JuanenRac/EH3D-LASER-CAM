//! The limits of the engraving: every value of the parameters at its edge, the numbers the engraving reports (rows, runs, length, time) and the shape of the lines at the ends.

use eh3d_laser_cam::dither::Dots;
use eh3d_laser_cam::raster::{engrave, overscan_for, Params};

fn dots(rows: &[&str]) -> Dots {
    let (h, w) = (rows.len(), rows[0].len());
    Dots {
        w,
        h,
        burn: rows.iter().flat_map(|r| r.chars().map(|c| c == '#')).collect(),
    }
}

fn p() -> Params {
    Params {
        pixel_mm: 1.0,
        feed_mm_min: 600.0,
        s_max: 1000,
        power_percent: 50.0,
        overscan_mm: 0.0,
        origin: (0.0, 0.0),
    }
}

fn with(f: impl Fn(&mut Params)) -> Params {
    let mut q = p();
    f(&mut q);
    q
}

#[test]
fn every_value_of_the_parameters_is_accepted_at_its_edges_and_refused_one_step_past_them() {
    for ok in [
        with(|q| q.pixel_mm = 0.01),
        with(|q| q.pixel_mm = 5.0),
        with(|q| q.feed_mm_min = 1.0),
        with(|q| q.feed_mm_min = 30_000.0),
        with(|q| q.power_percent = 0.0),
        with(|q| q.power_percent = 100.0),
        with(|q| q.overscan_mm = 50.0),
        with(|q| q.origin = (-100_000.0, 100_000.0)),
        with(|q| q.s_max = 1),
        with(|q| q.s_max = 100_000),
    ] {
        assert!(ok.validate().is_ok(), "{ok:?}");
    }
    for (bad, name) in [
        (with(|q| q.pixel_mm = 0.0099), "pixel_mm"),
        (with(|q| q.pixel_mm = 5.01), "pixel_mm"),
        (with(|q| q.feed_mm_min = 0.99), "feed_mm_min"),
        (with(|q| q.feed_mm_min = 30_000.1), "feed_mm_min"),
        (with(|q| q.power_percent = -0.1), "power_percent"),
        (with(|q| q.power_percent = 100.1), "power_percent"),
        (with(|q| q.overscan_mm = -0.1), "overscan_mm"),
        (with(|q| q.overscan_mm = 50.1), "overscan_mm"),
        (with(|q| q.origin.0 = 100_001.0), "origin x"),
        (with(|q| q.origin.1 = -100_001.0), "origin y"),
        (with(|q| q.s_max = 0), "s_max"),
        (with(|q| q.s_max = 100_001), "s_max"),
    ] {
        let e = bad.validate().unwrap_err();
        assert!(e.contains(name), "{name}: {e}");
    }
}

#[test]
fn a_value_that_is_not_a_number_is_refused_in_every_field() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for q in [
            with(|q| q.pixel_mm = bad),
            with(|q| q.feed_mm_min = bad),
            with(|q| q.power_percent = bad),
            with(|q| q.overscan_mm = bad),
            with(|q| q.origin.0 = bad),
            with(|q| q.origin.1 = bad),
        ] {
            assert!(q.validate().is_err(), "{q:?}");
        }
    }
}

#[test]
fn the_power_is_zero_at_zero_percent_the_whole_s_max_at_a_hundred_and_rounds_in_between() {
    assert_eq!(with(|q| q.power_percent = 0.0).power(), 0);
    assert_eq!(with(|q| q.power_percent = 100.0).power(), 1000);
    assert_eq!(with(|q| { q.s_max = 255; q.power_percent = 50.0 }).power(), 128);
    assert_eq!(with(|q| { q.s_max = 1; q.power_percent = 49.0 }).power(), 0);
    assert_eq!(with(|q| { q.s_max = 1; q.power_percent = 50.0 }).power(), 1);
    assert_eq!(Params::default().power(), 500);
}

#[test]
fn nothing_to_burn_is_a_program_that_only_sets_up_and_ends_and_counts_nothing() {
    let e = engrave(&dots(&["...", "..."]), &p()).unwrap();
    assert_eq!((e.rows, e.runs), (0, 0));
    assert_eq!((e.burn_mm, e.seconds), (0.0, 0.0));
    assert_eq!(e.gcode, "G21\nG90\nM4 S0\nF600\nM5\nM2\n");
}

#[test]
fn a_picture_that_is_all_burning_is_one_run_in_every_row_and_the_time_is_the_burning_at_the_feed() {
    let e = engrave(&dots(&["####", "####", "####"]), &p()).unwrap();
    assert_eq!((e.rows, e.runs), (3, 3));
    assert!((e.burn_mm - 12.0).abs() < 1e-9);
    // 600 mm/min is 10 mm/s; no overscan; the head goes back to the start of each row by burning, so only the travel to the first dot counts
    assert!(e.seconds >= 1.2 - 1e-9, "{}", e.seconds);
}

#[test]
fn a_single_dot_is_a_run_of_one_pixel_long() {
    let e = engrave(&dots(&[".#."]), &p()).unwrap();
    assert_eq!((e.rows, e.runs), (1, 1));
    assert!((e.burn_mm - 1.0).abs() < 1e-9);
    assert!(e.gcode.contains("G0 X1 Y0\n"), "{}", e.gcode);
    assert!(e.gcode.contains("G1 X2 S500\n"), "{}", e.gcode);
}

#[test]
fn two_runs_in_a_row_are_two_burning_moves_with_a_move_with_the_laser_off_between() {
    let e = engrave(&dots(&["##..##"]), &p()).unwrap();
    assert_eq!((e.rows, e.runs), (1, 2));
    let lines: Vec<&str> = e.gcode.lines().collect();
    let at = lines.iter().position(|l| *l == "G1 X2 S500").expect("first run");
    assert_eq!(lines[at + 1], "G1 X4 S0");
    assert_eq!(lines[at + 2], "G1 X6 S500");
}

#[test]
fn the_overscan_is_added_at_both_ends_of_every_line_and_only_when_it_is_more_than_zero() {
    let with_overscan = engrave(&dots(&["..##.."]), &with(|q| q.overscan_mm = 3.0)).unwrap();
    // the approach is 3 mm before the first dot, and the head runs 3 mm past the last
    assert!(with_overscan.gcode.contains("G0 X-1 Y0\n"), "{}", with_overscan.gcode);
    assert!(with_overscan.gcode.contains("G1 X2 S0\n"));
    assert!(with_overscan.gcode.contains("G1 X7 S0\n"), "{}", with_overscan.gcode);
    let none = engrave(&dots(&["..##.."]), &p()).unwrap();
    assert!(!none.gcode.contains("G1 X7 S0"));
    assert!(with_overscan.seconds > none.seconds);
}

#[test]
fn the_rows_without_dots_are_skipped_and_the_direction_alternates_only_over_the_rows_that_burn() {
    let e = engrave(&dots(&["##", "..", "..", "##", "##"]), &p()).unwrap();
    assert_eq!(e.rows, 3);
    let starts: Vec<&str> = e.gcode.lines().filter(|l| l.starts_with("G0 ")).collect();
    assert_eq!(starts.len(), 3);
    // the first row goes to the right (it starts at x = 0), the second to the left (it starts at x = 2), the third to the right again
    assert!(starts[0].starts_with("G0 X0 "));
    assert!(starts[1].starts_with("G0 X2 "));
    assert!(starts[2].starts_with("G0 X0 "));
}

#[test]
fn the_top_row_of_the_picture_is_the_highest_y_and_the_origin_moves_everything() {
    let e = engrave(&dots(&["#", ".", "#"]), &with(|q| q.origin = (10.0, 20.0))).unwrap();
    let ys: Vec<&str> = e.gcode.lines().filter(|l| l.starts_with("G0 ")).collect();
    assert_eq!(ys[0], "G0 X10 Y22");
    assert_eq!(ys[1], "G0 X11 Y20");
}

#[test]
fn a_pixel_of_a_fraction_of_a_millimetre_keeps_its_positions_without_noise() {
    let e = engrave(&dots(&["###"]), &with(|q| q.pixel_mm = 0.1)).unwrap();
    assert!(e.gcode.contains("G1 X0.3 S500\n"), "{}", e.gcode);
    assert!(!e.gcode.contains("0.30000000000000004"));
    assert!(!e.gcode.contains("e-"));
}

#[test]
fn a_parameter_that_is_refused_stops_the_engraving_before_any_code_is_made() {
    assert!(engrave(&dots(&["#"]), &with(|q| q.feed_mm_min = 0.0)).is_err());
    assert!(engrave(&dots(&["#"]), &with(|q| q.s_max = 0)).is_err());
}

#[test]
fn the_run_up_grows_with_the_square_of_the_feed_and_falls_with_the_acceleration() {
    let a = overscan_for(3000.0, 1000.0).unwrap();
    assert!((a - 1.25).abs() < 1e-9, "{a}");
    assert!((overscan_for(6000.0, 1000.0).unwrap() - 4.0 * a).abs() < 1e-9);
    assert!((overscan_for(3000.0, 2000.0).unwrap() - a / 2.0).abs() < 1e-9);
    assert!(overscan_for(0.0, 1000.0).is_err());
    assert!(overscan_for(3000.0, 0.0).is_err());
    assert!(overscan_for(f64::NAN, 1000.0).is_err());
}

#[test]
fn the_program_always_starts_with_the_units_the_absolute_mode_and_dynamic_power_and_ends_with_the_laser_off() {
    let e = engrave(&dots(&["#.#", ".#."]), &p()).unwrap();
    assert!(e.gcode.starts_with("G21\nG90\nM4 S0\nF600\n"));
    assert!(e.gcode.ends_with("M5\nM2\n"));
    assert_eq!(e.gcode.matches("M4").count(), 1);
    assert_eq!(e.gcode.matches("M5").count(), 1);
}

#[test]
fn the_laser_is_off_whenever_the_head_moves_without_burning() {
    let e = engrave(&dots(&["#..#", ".##.", "#..#"]), &with(|q| q.overscan_mm = 2.0)).unwrap();
    for line in e.gcode.lines() {
        if line.starts_with("G0 ") {
            assert!(!line.contains(" S"), "a rapid with a power: {line}");
        }
    }
}
