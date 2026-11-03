//! `engrave --dry-run` works out and counts the engraving, and writes nothing.

use std::path::Path;
use std::process::Command;

fn run(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_phoenix-laser"))
        .args(args)
        .output()
        .expect("the program starts");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn sample() -> String {
    format!("{}/examples/gradient.pgm", env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn a_dry_run_counts_the_same_and_writes_nothing() {
    let out = format!("{}/dry-run-out.nc", env!("CARGO_TARGET_TMPDIR"));
    let _ = std::fs::remove_file(&out);
    let base = ["--width-mm", "16", "--dot-mm", "0.5"];
    let img = sample();
    let mut real = vec!["engrave", &img, &out];
    real.extend(base);
    let (ok, real_out, e) = run(&real);
    assert!(ok, "{e}");
    assert!(Path::new(&out).exists());
    std::fs::remove_file(&out).unwrap();
    let mut dry = vec!["engrave", &img, &out, "--dry-run"];
    dry.extend(base);
    let (ok, dry_out, e) = run(&dry);
    assert!(ok, "{e}");
    assert!(!Path::new(&out).exists(), "a dry run writes nothing");
    assert!(
        dry_out.starts_with("dry run, nothing written: "),
        "{dry_out}"
    );
    assert!(
        dry_out.ends_with(real_out.as_str()),
        "{dry_out} / {real_out}"
    );
}

#[test]
fn a_dry_run_needs_no_output_file_but_still_needs_the_image_and_the_width() {
    let img = sample();
    let (ok, out, e) = run(&[
        "engrave",
        &img,
        "--dry-run",
        "--width-mm",
        "16",
        "--dot-mm",
        "0.5",
    ]);
    assert!(ok && out.contains("dots"), "{out} {e}");
    let (ok, _, e) = run(&["engrave", "--dry-run", "--width-mm", "16"]);
    assert!(!ok && e.contains("needs an image"), "{e}");
    let (ok, _, e) = run(&["engrave", &img, "--dry-run"]);
    assert!(!ok && e.contains("--width-mm is needed"), "{e}");
    let (ok, _, e) = run(&[
        "engrave",
        "no-such-image.pgm",
        "--dry-run",
        "--width-mm",
        "16",
    ]);
    assert!(!ok && e.contains("no-such-image.pgm"), "{e}");
}
