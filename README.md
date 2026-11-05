<p align="center">
  <img src="images/EH3D_PHOENIX_LASER_BANNER.svg" alt="EH3D-PHOENIX-LASER banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-PHOENIX-LASER
<p align="center">🇺🇸 <b>English</b> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### Laser engraving from an image and cutting from a drawing: dithering, raster G-code, SVG and DXF contours and a calibration grid

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.3-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Honesty check - what actually runs today:** a Rust library and a command line program (`phoenix-laser`) with 294 tests that run on a computer: turns an image into dots by dithering and writes the G-code of a laser that burns line by line with dynamic power and overscan, reads SVG and DXF drawings and writes the G-code that cuts their contours (the inside ones first, the width of the beam taken into account, bridges, several passes) and engraves their shapes with lines, with the settings of each layer in a file, and a power-against-feed test grid. It has a window (built with `--features gui`) that opens a picture or a drawing, shows the path of the laser, lets the settings be changed and exports the G-code, but the window has not been looked at on a screen yet; it has not been tried on a machine and it has not been checked against a collection of real files.

---

> **Safety and security status:** this program does not control any machine and is not a safety system. What it produces must be checked by a person before anything is run on a machine; nothing here has run on real hardware.


## 📋 At a glance

| | |
|---|---|
| Version | `0.0.3` |
| Maturity | Scaffolding |
| Tests | 294 |
| Stack | Rust |
| On real hardware | Not tried: nothing here has run on a machine |

**Jump to:** [Overview](#-overview) · [Architecture](#-architecture) · [Quick start](#-quick-start) · [Testing](#-testing) · [Structure](#-repository-structure) · [Related](#-related-projects)

## 🚦 Project status

Each line is one part of this repository and the light says how far it is today. It is written from what the repository can show (tests, builds, runs), not from what is planned.

🟢 Done and verified · 🟡 Works, with known limits · 🟠 Partly done or not verified yet · 🔴 Not done or not working

| Area | State | What it means here |
|---|---|---|
| Core | 🟡 | turns an image into dots by dithering and writes the G-code of a laser that burns line by line with dynamic power and overscan, reads SVG and DXF drawings and cuts or engraves them by layer (ordered contours, beam width, bridges, passes, hatch), and a power-against-feed test grid; tested on a computer with normal and damaged input, not with a collection of real files. |
| Command line | 🟡 | `phoenix-laser` (`engrave`, `calibrate`, `info`, `cut`, `vector`, `layers`) runs the same functions as the library and prints the result; every error comes back as a message and a non-zero exit code. |
| Tests | 🟡 | 294 tests pass (`cargo test`), with normal cases, edge cases and damaged input. |
| Security | 🟡 | Plain safe Rust (`forbid(unsafe_code)`): nothing panics on the files it is given and a bad value is an error with its reason; no independent review. |
| Window | 🟠 | Written (egui, `--features gui`): open or drop a picture or an SVG or DXF drawing, see the path of the laser (the cut in one colour, the travel dim), edit the table of layers, a calibration grid, zoom and pan, estimate the time, export; its logic is tested, but it has not been built or looked at on a screen yet. |
| Real hardware | 🔴 | Not tried: nothing here has been run on a machine. |
| Documentation | 🟡 | README in eight languages, the changelog and the two pictures drawn from `docs/`. |

## 🎯 Overview

* **The image** (`eh3d-image`): PNG (grey, colour, transparent, 8 or 16 bits) and PGM (text and binary) become grey levels, with transparent parts as white and the size limited so that a hostile file cannot ask for gigabytes; `resize` averages the pixels so that one dot is as big as the beam.
* **The dots** (`src/dither.rs`): a threshold, Floyd-Steinberg or Atkinson error diffusion; the tests check that the darkness of a grey is kept (a mid grey burns about half of the dots) and that white burns nothing and black burns everything.
* **The laser** (`src/raster.rs`, `src/calibrate.rs`): one line after another, left to right and then right to left, `M4` with `S0` while it travels and the chosen power where it burns, an overscan fixed or worked out from the acceleration (v squared over 2a), and a grid of squares with a power for each column and a feed for each row to find what suits a material.
* **The header and the numbers of every G-code file** (`src/gcode_out.rs`): the file opens with comment lines that say the generator and its version, the units (mm, G21), the picture it was made from with its SHA-256, and the tools (the laser and its power); a calibration grid says it has no source. A number is written one way only (at most three decimals, no `-0`) and a value that is not finite is refused instead of being written as `NaN` or `inf`.
* **`--dry-run`:** `engrave` works the result out and counts it (dots, lines, runs, burnt length, time) without writing the output file, which may then be left out.
* **The drawings** (`src/svg.rs`, `src/dxf.rs`, `src/flatten.rs`, `src/vector.rs`): a safe subset of SVG (paths with all their commands, shapes, transforms, groups, units) and the entities of a DXF (lines, polylines with arcs, arcs, circles, splines, units) become shapes in millimetres; scripts, images, `use`, declared entities and anything unknown are skipped or refused, the size, the depth, the number of elements and of points are limited by the settings, and curves become straight pieces within a tolerance.
* **Cutting and engraving** (`src/order.rs`, `src/offset.rs`, `src/cut.rs`): the shapes are grouped by layer (DXF) or colour (SVG); each group is cut (loose lines joined, the contours inside another one first, the shortest travel, the width of the beam taken off on the right side, bridges left uncut, several passes with the focus moving on) or engraved (closed shapes filled with lines at an angle, by the even-odd rule) with its own settings.
* **The settings file** (`src/vconfig.rs`, `docs/vector-settings.md`): every value that is tuned (the limits of the readers, the machine, the settings of each layer) is in a plain text file that is checked line by line; none of it is in the code.
* **A checked G-code:** the program of a drawing is checked line by line before it is given: no laser on during a rapid move or at no speed, no `S` above full power, no move outside of the bed, `M5` at the end; the file is never written over the drawing or the settings it comes from.

## 🧩 Role in EH3D

EH3D-PHOENIX-LASER is one program of the EH3D Fabrication Suite. It works as a library and a command line and, built with `--features gui`, as a window in the look the other programs of the suite share. It reads and writes plain files, so it can be used on its own or between the other programs.

## 🏛️ Architecture

<p align="center">
  <img src="images/architecture.svg" alt="EH3D-PHOENIX-LASER architecture: who uses it, what it holds and what it depends on" width="100%">
</p>


How the parts of this repository fit together: who uses it, what it holds and what it depends on. The picture is drawn from `docs/architecture.json` and checked against the code.

## ✨ Features

* **A library and a command line:** every command of `phoenix-laser` is a function of the library, so another program of the suite can call it without starting a process.
* **Strict command line:** an option that is written wrong (`--powr`) or a word too many is an error that names it, not something to ignore; the program would otherwise make its output with the default values.
* **Tests with damaged input:** the tests also feed empty files, garbage, huge numbers and cut files, and none of them may panic.
* **No guessing:** a value that does not make sense is an error that says which value and why, never a result that only looks right.
* **No unsafe code:** `#![forbid(unsafe_code)]` in the library and the program.

## 🔒 Security & Safety

| Threat | Defense |
|---|---|
| A damaged, truncated or empty file | no panic: the tests cut valid input at every position and feed garbage |
| A number that makes no sense (negative, not finite, absurdly large) | refused with an error that names the value; nothing is rounded into something plausible |
| Unsafe memory use | plain safe Rust with `forbid(unsafe_code)` |
| A hostile drawing (scripts, declared entities, deep nesting, millions of points, huge numbers) | scripts, images and links are never run or fetched; a declared entity is an error; the size, the depth, the elements, the shapes, the points and the pieces of a curve are limited; a shape with a point out of range is left out |
| A program that could turn the laser on at no speed or above full power | every program is checked line by line before it is given, and one that fails is not written |
| The output file chosen over the drawing or the settings it is made from | refused, in the window and in the commands |

**What is not defended:** a result that was produced cleanly is not a verified result. Nothing here knows the limits of a machine, and the program has had no independent review. Report a vulnerability privately (see [SECURITY.md](SECURITY.md)).

## 🔌 Interfaces

The library is `eh3d_phoenix_laser` (`use eh3d_phoenix_laser::...`); the command line is `phoenix-laser`. The commands are these; each prints its result on the standard output and reports an error on the standard error with a non-zero exit code. There is no network access.

```text
phoenix-laser engrave IMAGE OUT.nc --width-mm 50 [--dot-mm 0.1] [--feed 3000] [--power 50] [--smax 1000]
              [--dither fs|atkinson|threshold] [--threshold 128] [--overscan MM|auto] [--accel 1000] [--origin X,Y] [--dry-run]
phoenix-laser calibrate OUT.nc --powers 20,40,60,80 --feeds 1000,2000,3000 [--smax 1000]
phoenix-laser info IMAGE
phoenix-laser cut DRAWING OUT.nc [--config FILE] [--power P] [--speed MM/MIN] [--passes N] [--focus-step MM] [--kerf MM] [--kerf-side none|auto|inside|outside]
              [--tabs N] [--tab-width MM] [--smax N] [--overscan MM] [--origin X,Y] [--bed X,Y] [--dpi N] [--tolerance MM] [--dry-run]
phoenix-laser vector DRAWING OUT.nc [the options of cut] [--angle DEG] [--spacing MM]
phoenix-laser layers DRAWING [--config FILE] [--dpi N]
phoenix-laser [gui] [FILE]                       the window (a build with --features gui); with no arguments it opens
```

`DRAWING` is an SVG or a DXF file. `cut` follows the contours (the inside ones first); `vector` fills the closed shapes with lines and follows the open ones; both take the settings of every layer or colour from `--config FILE` (see [docs/vector-settings.md](docs/vector-settings.md)) and an option of the command line changes them for all the layers. `layers` lists the layers of a drawing with what would be done with each.

## ⚙️ Configuration

The engraving of a picture is set with arguments of the command line (see Interfaces) and every default is written in the help of the command (`phoenix-laser help`). Cutting and engraving a drawing is set with a settings file (`--config FILE`, or "Load settings" in the window): the limits of the readers, how a file is read, the order of the cuts, the machine and, for each layer or colour, the mode (cut, engrave or skip), the power, the speed, the passes, the step of the focus, the width of the beam and its side, the bridges and the hatch. Every key, its default and its range are in [docs/vector-settings.md](docs/vector-settings.md) and an example is in `examples/plate.cfg`; a key that is not known or a value out of range is an error that names its line. The fixed limits of the picture reader live in the code as named constants and are covered by tests.

## 🚀 Quick start

You need a stable Rust toolchain (1.88 or newer). The repository EH3D-COMMON must be cloned beside this one: the reader of pictures is taken from it by path.

```bash
cargo build --release
cargo test
cargo run --release -- engrave examples/gradient.pgm examples/gradient.nc --width-mm 16 --dot-mm 0.5 --overscan auto
cargo run --release -- cut examples/plate.svg examples/plate.nc --config examples/plate.cfg
cargo run --release --features gui   # the window
```

## 🧪 Testing

The suite has 294 tests (`cargo test`, run on Windows and Linux in the CI). They check the normal cases, the edge cases and damaged input: empty files, garbage and cut files never panic. The CI also runs `cargo fmt --check`, `cargo clippy` with warnings as errors and `python -I tools/ci_validate.py`.

## 🏗️ Build & Release

The version lives in `Cargo.toml` and in `eh3d.project.json`; each version has its entry in [CHANGELOG.md](CHANGELOG.md). `build.bat` / `build.sh` build the program, `test.bat` / `test.sh` run the tests and `run.bat` / `run.sh` run it with a sample. Releases are published on GitHub.

**Downloads:** Ready-made programs for every system are published with each release of this repository: Windows (64 and 32 bits), Linux (64 bits) and macOS (Apple silicon and Intel). Each package, `phoenix-laser-<version>-<system>.zip` or `.tar.gz`, holds the program, the licence, this README, the changelog, the icon and the example files, and comes with its `.sha256` file; the release also lists every file in `SHA256SUMS` and `build-manifest.json`. No release has been published yet.

**Signed and checked:** the packages of a release are signed with the release key of EH3D, and the `.sig` file of each package is published next to it (an OpenSSH signature, namespace `eh3d-release`). Check a download before running it: compare its SHA-256 (`sha256sum -c phoenix-laser-<version>-<system>.tar.gz.sha256`, or `Get-FileHash` on Windows) and verify the signature with `ssh-keygen -Y verify -f allowed_signers -I eh3d-release -n eh3d-release -s <package>.sig < <package>`, where `allowed_signers` holds the line `eh3d-release <key type> <public key of the releases of EH3D>`.

## 📂 Repository Structure

<p align="center">
  <img src="images/repository-structure.svg" alt="Repository structure of EH3D-PHOENIX-LASER" width="100%">
</p>

<details>
<summary>Text version of the tree</summary>

```text
EH3D-PHOENIX-LASER/
├── src/
│   ├── lib.rs                                   the library: what every command uses
│   ├── main.rs                                  the command line: engrave, calibrate, info, cut, vector and layers
│   ├── calibrate.rs                             a part of the logic
│   ├── cli_args.rs                              the arguments of the command line, the same file in every small utility
│   ├── dither.rs                                a part of the logic
│   ├── gcode_out.rs                             the one way a G-code number is written and the standard header
│   ├── raster.rs                                a part of the logic
│   ├── vector.rs                                a drawing once read: shapes in mm, layers, limits and the small geometry
│   ├── flatten.rs                               curves cut into straight pieces within a tolerance (Bezier, arcs)
│   ├── svg.rs                                   the SVG reader: a safe subset, no scripts, images or entities
│   ├── dxf.rs                                   the DXF reader: lines, polylines, arcs, circles and splines, with units
│   ├── offset.rs                                moving a contour sideways by the width of the beam, checked edge by edge
│   ├── order.rs                                 the order of the cuts: loose lines joined, inside first, short travel
│   ├── cut.rs                                   from a drawing to G-code: cut, tabs, hatch, passes and the safety check
│   ├── vconfig.rs                               the settings file of the vector work: limits, machine and each layer
│   ├── browse.rs                                the list of files of the window that opens a file
│   ├── gui.rs                                   the window (egui, with the gui feature): draws the state and passes on what the person does
│   ├── lang.rs                                  the words of the window in the eight languages
│   ├── viewport.rs                              zoom and pan of the picture, in plain numbers
│   └── window.rs                                the state of the window without a screen: picture or drawing, settings, layers, passes and export
├── tests/
│   ├── dry_run.rs                               tests of --dry-run: it counts the same and writes nothing
│   ├── laser.rs                                 tests of the library and the command line
│   ├── readme.rs                                tests that run what the README and the run scripts promise
│   ├── strict.rs                                tests of the command line: options and words it does not know
│   ├── vector.rs                                tests of cutting from drawings: commands, examples, hostile files and the check of the G-code
│   └── window_cli.rs                            tests that a build without the gui feature stays a command line
├── examples/
│   ├── gradient.pgm                             a sample to try the program with
│   ├── plate.svg                                a plate with a hole and a panel to engrave (SVG)
│   ├── plate.dxf                                the same plate as a DXF
│   └── plate.cfg                                the settings of its layers: power, speed, passes, beam width, tabs and hatch
├── docs/
│   ├── architecture.json                        data of the Architecture picture
│   ├── structure.json                           data of the Repository Structure picture
│   └── vector-settings.md                       the settings file of the vector work, key by key
├── tools/                                       CI check, README check and the picture tools
├── images/                                      banner and the Architecture and Repository Structure pictures
├── .github/                                     CI and release workflows, issue and pull-request templates
├── eh3d.project.json                            manifest: version, role, maturity and build command
├── Cargo.toml Cargo.lock                        the workspace and its locked dependencies
├── CHANGELOG.md                                 what changed in each version
├── SECURITY.md SUPPORT.md                       how to report a vulnerability and where to ask
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   how to contribute, the code of conduct and the licence
```

</details>

## 🗺️ Roadmap & Known limits

**Known limits today**

* The window has not been looked at on a screen yet.
* It has not been tried on a machine nor with a collection of real files, and it has had no independent review.
* A drawing is read in a safe subset (no `use`, text, gradients or styles with selectors; no ellipses, hatches or blocks in a DXF); a contour narrower than the beam is not cut and the program says so; the laser is the one of a GRBL controller with `M4`, and nothing has been tried on a real laser or material.

**Next**

* Show the power density on the board before burning and keep the power constant in the corners of a cut.
* Look at the window on a screen and finish it.
* Try it with real drawings and with the other programs of the suite.

## 🌍 Languages

README and texts exist in English, Spanish, French, Italian, German, Chinese, Japanese and European Portuguese.

## 🔗 Related Projects

* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): the shared crates, among them the reader of pictures
* [EH3D-STENCILCAM](https://github.com/JuanenRac/EH3D-STENCILCAM): solder paste stencils
* [EH3D-CAM-DEBUGGER](https://github.com/JuanenRac/EH3D-CAM-DEBUGGER): the checking of G-code before it reaches a machine
* [EH3D-TOOL-MANAGER](https://github.com/JuanenRac/EH3D-TOOL-MANAGER): the tool crib shared by the CAM programs
* [EH3D-MOTIONCNC](https://github.com/JuanenRac/EH3D-MOTIONCNC): the machine control program
* [EH3D-SUITE-LAUNCHER](https://github.com/JuanenRac/EH3D-SUITE-LAUNCHER): the hub that finds and starts the programs of the suite

## 📚 Documentation

* [Changelog](CHANGELOG.md)
* [Architecture and structure data](docs/architecture.json): the data the two pictures are drawn from.
* [Settings of the vector work](docs/vector-settings.md): every key of the settings file with its default and its range.

## 🤝 Contributing & Community

* Before a pull request run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` and `python -I tools/ci_validate.py`: all must pass. See [CONTRIBUTING.md](CONTRIBUTING.md) and the [code of conduct](CODE_OF_CONDUCT.md).
* The README exists in eight languages: a change to the text is made in all eight files, and the two pictures are drawn from `docs/architecture.json` and `docs/structure.json` by `tools/make_architecture.py` and `tools/make_structure.py`.
* Report a vulnerability privately: [SECURITY.md](SECURITY.md). Questions, ideas and reports: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTHOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENSE

Software: GPL-3.0-or-later. See [LICENSE](LICENSE). Hardware designs of the suite use CERN OHL-S.
