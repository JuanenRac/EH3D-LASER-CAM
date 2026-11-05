// EH3D-PHOENIX-LASER - laser engraving from an image.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
#![forbid(unsafe_code)]
//! An image becomes grey levels, the grey levels become dots (dithering), and the dots become the G-code of a laser that burns line by line with dynamic power. A vector drawing (SVG or DXF) has its contours cut and its shapes engraved with the settings of each layer.

/// The reader of PGM and PNG pictures, kept in EH3D-COMMON because the other programs that work from a picture read them the same way.
pub use eh3d_image as image;
pub mod browse;
pub mod calibrate;
pub mod cut;
pub mod dither;
pub mod dxf;
pub mod flatten;
pub mod gcode_out;
#[cfg(feature = "gui")]
pub mod gui;
pub mod lang;
pub mod offset;
pub mod order;
pub mod raster;
pub mod svg;
pub mod vconfig;
pub mod vector;
pub mod viewport;
pub mod window;
