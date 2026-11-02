# Changelog

All notable changes to this project are documented here.

## [Unreleased]

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
