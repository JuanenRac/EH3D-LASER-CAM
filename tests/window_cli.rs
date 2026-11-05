//! The program is a command line first: without the `gui` feature it never opens a window, and the `gui` word explains why. (With the feature, no arguments open the window, so these
//! tests are for the build the CI makes.)
#![cfg(not(feature = "gui"))]

use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_phoenix-laser");

#[test]
fn no_arguments_print_the_help_when_the_build_has_no_window() {
    let out = Command::new(BIN).output().expect("the program runs");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("laser engraving from an image"));
    assert!(text.contains("[gui]"));
}

#[test]
fn the_gui_word_says_that_this_build_has_no_window() {
    let out = Command::new(BIN)
        .arg("gui")
        .output()
        .expect("the program runs");
    assert!(!out.status.success());
    let text = String::from_utf8_lossy(&out.stderr);
    assert!(text.contains("--features gui"), "{text}");
}

#[test]
fn a_single_picture_without_a_command_is_an_unknown_command_in_this_build() {
    let picture = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/gradient.pgm");
    let out = Command::new(BIN)
        .arg(picture)
        .output()
        .expect("the program runs");
    assert!(!out.status.success());
}
