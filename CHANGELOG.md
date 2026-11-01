# Changelog

All notable changes to this project are documented here.

## [Unreleased]

- **A dot size that makes no sense is refused before anything is reserved:** `--dot-mm 0` or a tiny one, or an image that would be larger than the limit, gives an error instead of an allocation failure or an overflow.

## [0.0.1] - An image engraved by a laser: dithering, raster G-code with dynamic power and a calibration grid

- **The image:** PNG (grey, colour, transparent, 8 or 16 bits) and PGM, with the size limited; `resize` averages the pixels.
- **The dots:** threshold, Floyd-Steinberg and Atkinson, checked to keep the darkness of a grey.
- **The laser:** `M4` with `S0` while it travels, alternating directions, an overscan fixed or worked out from the acceleration, and the top of the image as the highest Y.
- **The calibration grid:** a square for every pair of a power and a feed.
- **The command line** `eh3d-lasercam` with `engrave`, `calibrate` and `info`, and a sample gradient.
- **18 tests** run on a computer, with damaged input.
