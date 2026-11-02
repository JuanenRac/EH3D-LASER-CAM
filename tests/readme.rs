//! What the README and the scripts run.sh / run.bat promise can be run: the three say the same, and the command works on the files of examples/.

use std::path::PathBuf;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_eh3d-lasercam");

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(name: &str) -> String {
    std::fs::read_to_string(root().join(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn words(s: &str) -> Vec<String> {
    s.split_whitespace().map(str::to_string).collect()
}

fn of_run_sh() -> Vec<String> {
    let t = read("run.sh");
    let line = t
        .lines()
        .find(|l| l.contains("set -- "))
        .expect("run.sh has the sample");
    words(
        line.split("set -- ")
            .nth(1)
            .unwrap()
            .split(';')
            .next()
            .unwrap(),
    )
}

fn of_run_bat() -> Vec<String> {
    let t = read("run.bat");
    let line = t
        .lines()
        .find(|l| l.contains("set \"ARGS=") && !l.contains("%*"))
        .expect("run.bat has the sample");
    words(
        line.split("set \"ARGS=")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap(),
    )
}

fn of_readme() -> Vec<Vec<String>> {
    read("README.md")
        .lines()
        .filter_map(|l| l.trim().strip_prefix("cargo run --release -- ").map(words))
        .collect()
}

/// Runs the command in the folder of the repository; a file of examples/ that does not exist yet is an output, which goes to a temporary folder instead.
fn run(args: &[String], tag: &str) -> (bool, String, String) {
    let dir =
        std::env::temp_dir().join(format!("eh3d_lasercam_readme_{tag}_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let args: Vec<String> = args
        .iter()
        .map(|a| {
            if a.starts_with("examples/") && !root().join(a).exists() {
                dir.join(a.replace('/', "_")).to_string_lossy().into_owned()
            } else {
                a.clone()
            }
        })
        .collect();
    let out = Command::new(BIN)
        .args(&args)
        .current_dir(root())
        .output()
        .expect("the program runs");
    let _ = std::fs::remove_dir_all(&dir);
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn the_scripts_and_the_readme_run_the_same_sample() {
    let sh = of_run_sh();
    assert!(!sh.is_empty());
    assert_eq!(sh, of_run_bat(), "run.sh and run.bat differ");
    assert!(
        of_readme().contains(&sh),
        "the README does not show the sample of run.sh"
    );
}

#[test]
fn the_sample_command_works() {
    for (i, args) in of_readme().iter().enumerate() {
        let (ok, out, err) = run(args, &i.to_string());
        assert!(ok, "{args:?}: {err}");
        assert!(!out.trim().is_empty(), "{args:?} printed nothing");
    }
}

#[test]
fn the_help_runs() {
    let (ok, out, _) = run(&["help".to_string()], "help");
    assert!(ok && out.contains("eh3d-"), "{out}");
}
