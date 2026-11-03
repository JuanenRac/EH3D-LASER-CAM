# Changelog

All notable changes to this project are documented here.

## [Unreleased]
- **Shared checks updated:** the validator also compares the badge and the Version row of the README with the manifest, fails when one of its checks cannot run, and the CHANGELOG step of the workflow reads the base branch from the environment.
- **Every G-code file opens with a standard header:** comment lines with the generator and its version, the units (mm, G21), the picture it was made from with its SHA-256 and the tools (the laser and its power); the calibration grid says it has no source. The library functions keep returning the program alone, so the header belongs to the files the command line writes.
- **One way to write a number, and no `NaN` or `inf` in a file:** the engraving and the calibration grid write every number with the same function (`gcode_out::number`: three decimals at most, no `-0`), and a value that is not finite is an error. The SHA-256 used by the header is written in the program itself (no new dependency) and checked against the known test vectors.
- **The reader of pictures comes from EH3D-COMMON (`eh3d-image`):** the program no longer keeps its own copy of the PGM and PNG reader; it takes it by path from the sibling repository, which must be cloned beside this one. `eh3d_*::image` keeps working as before. The manifest names EH3D-COMMON in `depends_on` and the CI checks it out.
- **More tests:** the G-code read back into the dots it was made from, the limits of the head and of every value, the calibration grid square by square, and the command line with its errors and file names in any script.
- **The workflows are hardened:** every action is pinned by its commit with its version in a comment, and the checkouts keep no credentials.
- **More tests of the limits:** every value of the parameters at its edges, what the engraving reports, and the shape of the lines at the ends.

## [0.0.2] - Dry run for engraving

- **Strict command line:** an option that is written wrong (`--powr`) or a word too many is an error that names it; before, it was ignored and the output was made with the default values.
- **A dot size that makes no sense is refused before anything is reserved:** `--dot-mm 0` or a tiny one, or an image that would be larger than the limit, gives an error instead of an allocation failure or an overflow.
- **`engrave --dry-run`:** works the engraving out and counts it (dots, lines, runs, burnt length, time), without writing the output file (which may then be left out).
- **One shared helper for the command-line arguments** (`src/cli_args.rs`: flag, switch, num and a strict finish), the same file in every small utility of the suite; the copies this program had behaved the same, so nothing changes for a person.
- **Tests that run what the README and the scripts promise:** the sample of `run.sh`, `run.bat` and the README is the same command, and it is run on the files of `examples/`.
- **The banner of the README** shows the role and the technologies of this program (it showed the text of the shared library).

## [0.0.1] - An image engraved by a laser: dithering, raster G-code with dynamic power and a calibration grid

- **The image:** PNG (grey, colour, transparent, 8 or 16 bits) and PGM, with the size limited; `resize` averages the pixels.
- **The dots:** threshold, Floyd-Steinberg and Atkinson, checked to keep the darkness of a grey.
- **The laser:** `M4` with `S0` while it travels, alternating directions, an overscan fixed or worked out from the acceleration, and the top of the image as the highest Y.
- **The calibration grid:** a square for every pair of a power and a feed.
- **The command line** `eh3d-lasercam` with `engrave`, `calibrate` and `info`, and a sample gradient.
- **18 tests** run on a computer, with damaged input.
