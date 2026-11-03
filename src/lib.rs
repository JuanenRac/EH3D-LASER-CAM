// EH3D-LASER-CAM - laser engraving from an image.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
#![forbid(unsafe_code)]
//! An image becomes grey levels, the grey levels become dots (dithering), and the dots become the G-code of a laser that burns line by line with dynamic power.

/// The reader of PGM and PNG pictures, kept in EH3D-COMMON because the other programs that work from a picture read them the same way.
pub use eh3d_image as image;
pub mod calibrate;
pub mod dither;
pub mod gcode_out;
pub mod raster;
