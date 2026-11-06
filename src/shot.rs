//! The picture mode of the window (`--shot FILE.png`): what is asked for, read and checked without a screen. The window opens at the size asked, shows the files given with `--open`,
//! waits a number of frames so that the drawing is complete, saves what it drew as a PNG file and closes by itself. In this mode the window keeps no settings, so a run leaves
//! nothing of the person's changed.

use crate::lang::Lang;
use std::path::PathBuf;

/// The smallest side of the window of a picture, in pixels.
pub const SIZE_MIN: u32 = 320;
/// The largest side of the window of a picture, in pixels.
pub const SIZE_MAX: u32 = 8192;
/// The size of the window of a picture when `--size` is not given.
pub const DEFAULT_SIZE: (u32, u32) = (1180, 760);
/// The frames drawn before the picture is taken when `--wait` is not given.
pub const DEFAULT_WAIT: u32 = 6;
/// The most frames `--wait` may ask for.
pub const WAIT_MAX: u32 = 3600;
/// The frames after the one asked for in which the picture may still come back; after them the run fails.
pub const PATIENCE_FRAMES: u32 = 600;

/// The look of the window in the picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShotTheme {
    /// Amber on charcoal, the look the window starts with.
    #[default]
    Forge,
    /// The dark look of egui.
    Dark,
    /// The light look of egui.
    Light,
}

impl ShotTheme {
    /// The word that names the theme on the command line.
    pub fn name(self) -> &'static str {
        match self {
            ShotTheme::Forge => "forge",
            ShotTheme::Dark => "dark",
            ShotTheme::Light => "light",
        }
    }

    /// The theme a word names, in any case.
    pub fn parse(text: &str) -> Option<ShotTheme> {
        let word = text.trim().to_ascii_lowercase();
        [ShotTheme::Forge, ShotTheme::Dark, ShotTheme::Light]
            .into_iter()
            .find(|t| t.name() == word)
    }
}

/// What the picture mode was asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShotRequest {
    /// The PNG file to write.
    pub output: PathBuf,
    /// True when a file that exists may be replaced; otherwise it is an error.
    pub replace: bool,
    /// The size of the window in pixels, width and height.
    pub size: (u32, u32),
    /// The look of the window.
    pub theme: ShotTheme,
    /// The frames drawn before the picture is taken.
    pub wait: u32,
}

/// What the window was asked for on the command line.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WindowArgs {
    /// The files to open at once (bare arguments and `--open FILE`).
    pub files: Vec<PathBuf>,
    /// The language of the window; none takes the one of the system.
    pub lang: Option<Lang>,
    /// The picture mode: the window opens, saves a PNG file and closes by itself.
    pub shot: Option<ShotRequest>,
}

/// Reads a size written `WIDTHxHEIGHT` (`1280x720`, also with a capital X), each side between [`SIZE_MIN`] and [`SIZE_MAX`].
pub fn parse_size(text: &str) -> Option<(u32, u32)> {
    let (width, height) = text.trim().split_once(['x', 'X'])?;
    let width: u32 = width.trim().parse().ok()?;
    let height: u32 = height.trim().parse().ok()?;
    let ok = |side: u32| (SIZE_MIN..=SIZE_MAX).contains(&side);
    (ok(width) && ok(height)).then_some((width, height))
}

/// Whether the arguments ask for the picture mode.
pub fn wants_picture(args: &[String]) -> bool {
    args.iter().any(|a| a == "--shot")
}

/// The options of the picture as they are read, before they are checked together.
#[derive(Default)]
struct PictureOptions {
    size: Option<(u32, u32)>,
    theme: Option<ShotTheme>,
    wait: Option<u32>,
    replace: bool,
}

/// Reads the arguments of the window (after an optional leading `gui`): `--shot FILE.png`, `--size WxH`, `--theme`, `--wait N`, `--replace`, `--lang CODE`, `--open FILE` and bare
/// file names, of which at most `max_files` are accepted. The answer for a mistake is the reason.
pub fn parse_window_args(args: &[String], max_files: usize) -> Result<WindowArgs, String> {
    let args = if args.first().is_some_and(|a| a == "gui") {
        &args[1..]
    } else {
        args
    };
    let mut parsed = WindowArgs::default();
    let mut output: Option<PathBuf> = None;
    let mut picture = PictureOptions::default();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--shot" => {
                let path = iter.next().ok_or("--shot needs a file to write")?;
                if !path.to_ascii_lowercase().ends_with(".png") {
                    return Err("--shot writes a PNG file: the name must end in .png".to_owned());
                }
                output = Some(PathBuf::from(path));
            }
            "--size" => {
                let text = iter.next().ok_or("--size needs WIDTHxHEIGHT")?;
                picture.size = Some(parse_size(text).ok_or(
                    "--size is WIDTHxHEIGHT in pixels, each side from 320 to 8192, as 1280x720",
                )?);
            }
            "--theme" => {
                let text = iter.next().ok_or("--theme needs forge, dark or light")?;
                picture.theme =
                    Some(ShotTheme::parse(text).ok_or("--theme is forge, dark or light")?);
            }
            "--wait" => {
                let text = iter.next().ok_or("--wait needs a number of frames")?;
                picture.wait = Some(
                    text.parse::<u32>()
                        .ok()
                        .filter(|n| *n <= WAIT_MAX)
                        .ok_or("--wait is a number of frames from 0 to 3600")?,
                );
            }
            "--lang" => {
                let tag = iter.next().ok_or("--lang needs a language code")?;
                parsed.lang = Some(Lang::from_code(tag).ok_or("unknown language code")?);
            }
            "--replace" => picture.replace = true,
            "--open" => {
                let path = iter.next().ok_or("--open needs a file")?;
                add_file(&mut parsed, path, max_files)?;
            }
            other if other.starts_with("--") => return Err(format!("unknown option {other}")),
            other => add_file(&mut parsed, other, max_files)?,
        }
    }
    parsed.shot = match output {
        Some(output) => Some(ShotRequest {
            output,
            replace: picture.replace,
            size: picture.size.unwrap_or(DEFAULT_SIZE),
            theme: picture.theme.unwrap_or_default(),
            wait: picture.wait.unwrap_or(DEFAULT_WAIT),
        }),
        None => {
            if picture.size.is_some()
                || picture.theme.is_some()
                || picture.wait.is_some()
                || picture.replace
            {
                return Err("--size, --theme, --wait and --replace need --shot".to_owned());
            }
            None
        }
    };
    Ok(parsed)
}

fn add_file(parsed: &mut WindowArgs, path: &str, max_files: usize) -> Result<(), String> {
    if parsed.files.len() >= max_files {
        return Err(if max_files == 1 {
            "only one file can be opened".to_owned()
        } else {
            format!("at most {max_files} files can be opened")
        });
    }
    parsed.files.push(PathBuf::from(path));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn a_size_is_two_sides_inside_the_limits() {
        assert_eq!(parse_size("1280x720"), Some((1280, 720)));
        assert_eq!(parse_size(" 800 X 600 "), Some((800, 600)));
        assert_eq!(parse_size("320x8192"), Some((320, 8192)));
        for bad in [
            "", "1280", "x720", "1280x", "319x600", "800x8193", "-1x600", "1e3x600", "10x20x30",
            "800*600",
        ] {
            assert_eq!(parse_size(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn a_theme_is_read_in_any_case_and_names_read_back() {
        for theme in [ShotTheme::Forge, ShotTheme::Dark, ShotTheme::Light] {
            assert_eq!(ShotTheme::parse(theme.name()), Some(theme));
        }
        assert_eq!(ShotTheme::parse(" Light "), Some(ShotTheme::Light));
        assert_eq!(ShotTheme::parse("blue"), None);
        assert_eq!(ShotTheme::default(), ShotTheme::Forge);
    }

    #[test]
    fn only_the_shot_option_asks_for_the_picture() {
        assert!(wants_picture(&args(&["--shot", "a.png"])));
        assert!(wants_picture(&args(&["gui", "x.pgm", "--shot", "a.png"])));
        assert!(!wants_picture(&args(&["gui", "x.pgm"])));
        assert!(!wants_picture(&[]));
    }

    #[test]
    fn no_arguments_open_the_window_alone() {
        assert_eq!(parse_window_args(&[], 1), Ok(WindowArgs::default()));
        assert_eq!(
            parse_window_args(&args(&["gui"]), 1),
            Ok(WindowArgs::default())
        );
    }

    #[test]
    fn files_and_a_language_are_read_in_any_order() {
        let parsed = parse_window_args(
            &args(&["gui", "--lang", "pt", "a.pgm", "--open", "b.svg"]),
            3,
        )
        .unwrap();
        assert_eq!(
            parsed.files,
            [PathBuf::from("a.pgm"), PathBuf::from("b.svg")]
        );
        assert_eq!(parsed.lang, Some(Lang::Pt));
        assert_eq!(parsed.shot, None);
    }

    #[test]
    fn the_picture_mode_has_its_defaults() {
        let parsed = parse_window_args(&args(&["--shot", "start.png"]), 1).unwrap();
        assert_eq!(
            parsed.shot,
            Some(ShotRequest {
                output: PathBuf::from("start.png"),
                replace: false,
                size: DEFAULT_SIZE,
                theme: ShotTheme::Forge,
                wait: DEFAULT_WAIT,
            })
        );
        let upper = parse_window_args(&args(&["--shot", "B.PNG", "x.pgm"]), 1).unwrap();
        assert_eq!(upper.shot.unwrap().output, PathBuf::from("B.PNG"));
    }

    #[test]
    fn every_option_of_the_picture_is_read() {
        let parsed = parse_window_args(
            &args(&[
                "--shot",
                "a.png",
                "--size",
                "1280x720",
                "--theme",
                "Light",
                "--wait",
                "12",
                "--replace",
                "--open",
                "x.pgm",
                "--lang",
                "de",
            ]),
            1,
        )
        .unwrap();
        assert_eq!(parsed.lang, Some(Lang::De));
        assert_eq!(parsed.files, [PathBuf::from("x.pgm")]);
        assert_eq!(
            parsed.shot,
            Some(ShotRequest {
                output: PathBuf::from("a.png"),
                replace: true,
                size: (1280, 720),
                theme: ShotTheme::Light,
                wait: 12,
            })
        );
    }

    #[test]
    fn a_wait_of_zero_and_of_the_limit_are_allowed() {
        for (text, wait) in [("0", 0), ("3600", 3600)] {
            let parsed = parse_window_args(&args(&["--shot", "a.png", "--wait", text]), 1);
            assert_eq!(parsed.unwrap().shot.unwrap().wait, wait);
        }
    }

    #[test]
    fn a_mistake_gives_its_reason() {
        for (list, reason) in [
            (vec!["--shot"], "needs a file to write"),
            (vec!["--shot", "a.jpg"], "must end in .png"),
            (vec!["--shot", "a.png", "--size"], "needs WIDTHxHEIGHT"),
            (
                vec!["--shot", "a.png", "--size", "10x10"],
                "from 320 to 8192",
            ),
            (
                vec!["--shot", "a.png", "--theme", "blue"],
                "forge, dark or light",
            ),
            (vec!["--shot", "a.png", "--wait", "-1"], "from 0 to 3600"),
            (vec!["--shot", "a.png", "--wait", "3601"], "from 0 to 3600"),
            (vec!["--size", "800x600"], "need --shot"),
            (vec!["--replace"], "need --shot"),
            (vec!["--lang"], "needs a language code"),
            (vec!["--lang", "xx"], "unknown language code"),
            (vec!["--open"], "needs a file"),
            (vec!["--nonsense"], "unknown option --nonsense"),
            (vec!["a.pgm", "b.pgm"], "only one file"),
        ] {
            let error = parse_window_args(&args(&list), 1).unwrap_err();
            assert!(error.contains(reason), "{list:?}: {error}");
        }
        let error = parse_window_args(&args(&["a", "b", "c"]), 2).unwrap_err();
        assert!(error.contains("at most 2 files"), "{error}");
    }
}
