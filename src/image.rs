//! Grey levels read from a PGM (P2, P5) or a PNG file. Sizes are limited so that a hostile file cannot ask for gigabytes.

use std::io::Cursor;

pub const MAX_SIDE: usize = 20_000;
pub const MAX_PIXELS: usize = 25_000_000;

/// 0 is black (burns most) and 255 is white (does not burn).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gray {
    pub w: usize,
    pub h: usize,
    pub data: Vec<u8>,
}

impl Gray {
    pub fn new(w: usize, h: usize, data: Vec<u8>) -> Result<Gray, String> {
        if w == 0 || h == 0 || w > MAX_SIDE || h > MAX_SIDE || w * h > MAX_PIXELS {
            return Err(format!("{w} x {h} pixels: each side must be from 1 to {MAX_SIDE} and the image at most {MAX_PIXELS} pixels"));
        }
        if data.len() != w * h {
            return Err(format!(
                "{w} x {h} pixels need {} values, not {}",
                w * h,
                data.len()
            ));
        }
        Ok(Gray { w, h, data })
    }

    pub fn at(&self, x: usize, y: usize) -> u8 {
        self.data[y * self.w + x]
    }

    /// The image scaled with a box filter to `w` x `h` pixels (used to make the pixel of the engraving as big as the dot of the laser).
    pub fn resize(&self, w: usize, h: usize) -> Result<Gray, String> {
        let mut out = vec![0u8; w.max(1) * h.max(1)];
        let (w, h) = (w.max(1), h.max(1));
        Gray::new(w, h, vec![0; w * h])?;
        for y in 0..h {
            let (y0, y1) = (
                y * self.h / h,
                ((y + 1) * self.h)
                    .div_ceil(h)
                    .min(self.h)
                    .max(y * self.h / h + 1),
            );
            for x in 0..w {
                let (x0, x1) = (
                    x * self.w / w,
                    ((x + 1) * self.w)
                        .div_ceil(w)
                        .min(self.w)
                        .max(x * self.w / w + 1),
                );
                let mut sum = 0u64;
                for yy in y0..y1 {
                    for xx in x0..x1 {
                        sum += u64::from(self.at(xx, yy));
                    }
                }
                out[y * w + x] = (sum / ((y1 - y0) * (x1 - x0)) as u64) as u8;
            }
        }
        Gray::new(w, h, out)
    }
}

fn skip_blank_and_comments(bytes: &[u8], i: &mut usize) {
    loop {
        while *i < bytes.len() && bytes[*i].is_ascii_whitespace() {
            *i += 1;
        }
        if *i < bytes.len() && bytes[*i] == b'#' {
            while *i < bytes.len() && bytes[*i] != b'\n' {
                *i += 1;
            }
        } else {
            return;
        }
    }
}

fn number(bytes: &[u8], i: &mut usize) -> Result<usize, String> {
    skip_blank_and_comments(bytes, i);
    let start = *i;
    while *i < bytes.len() && bytes[*i].is_ascii_digit() {
        *i += 1;
    }
    std::str::from_utf8(&bytes[start..*i])
        .ok()
        .filter(|s| !s.is_empty() && s.len() < 10)
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| "the PGM header has something that is not a number".to_string())
}

/// A PGM file, P2 (text) or P5 (binary), 8 bits (a maximum value up to 255).
pub fn from_pgm(bytes: &[u8]) -> Result<Gray, String> {
    if bytes.len() < 2 || bytes[0] != b'P' || !(bytes[1] == b'2' || bytes[1] == b'5') {
        return Err("not a PGM file (it must begin with P2 or P5)".into());
    }
    let binary = bytes[1] == b'5';
    let mut i = 2;
    let (w, h, max) = (
        number(bytes, &mut i)?,
        number(bytes, &mut i)?,
        number(bytes, &mut i)?,
    );
    if max == 0 || max > 255 {
        return Err(format!(
            "the maximum value {max} is not from 1 to 255 (only 8 bits are read)"
        ));
    }
    if w == 0 || h == 0 || w > MAX_SIDE || h > MAX_SIDE || w * h > MAX_PIXELS {
        return Err(format!("{w} x {h} pixels is too large or empty"));
    }
    let scale = |v: usize| (v.min(max) * 255 / max) as u8;
    let mut data = Vec::with_capacity(w * h);
    if binary {
        i += 1; // the single whitespace byte after the maximum value
        let raw = bytes
            .get(i..i + w * h)
            .ok_or("the PGM file is shorter than its header says")?;
        data.extend(raw.iter().map(|&b| scale(usize::from(b))));
    } else {
        for _ in 0..w * h {
            data.push(scale(number(bytes, &mut i)?));
        }
    }
    Gray::new(w, h, data)
}

/// A PNG file in grey, grey with alpha, colour (RGB or RGBA) or with a palette, at 8 or 16 bits; transparent parts count as white.
pub fn from_png(bytes: &[u8]) -> Result<Gray, String> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder
        .read_info()
        .map_err(|e| format!("the PNG cannot be read: {e}"))?;
    let (w, h) = (reader.info().width as usize, reader.info().height as usize);
    if w == 0 || h == 0 || w > MAX_SIDE || h > MAX_SIDE || w * h > MAX_PIXELS {
        return Err(format!("{w} x {h} pixels is too large or empty"));
    }
    let mut buf = vec![0; reader.output_buffer_size()];
    let frame = reader
        .next_frame(&mut buf)
        .map_err(|e| format!("the PNG cannot be read: {e}"))?;
    let (channels, has_alpha, colour) = match frame.color_type {
        png::ColorType::Grayscale => (1, false, false),
        png::ColorType::GrayscaleAlpha => (2, true, false),
        png::ColorType::Rgb => (3, false, true),
        png::ColorType::Rgba => (4, true, true),
        png::ColorType::Indexed => return Err("a PNG with a palette was not expanded".into()),
    };
    let mut data = Vec::with_capacity(w * h);
    for px in buf[..frame.buffer_size()].chunks_exact(channels) {
        let luma = if colour {
            (299 * u32::from(px[0]) + 587 * u32::from(px[1]) + 114 * u32::from(px[2])) / 1000
        } else {
            u32::from(px[0])
        };
        let alpha = if has_alpha {
            u32::from(px[channels - 1])
        } else {
            255
        };
        data.push(((luma * alpha + 255 * (255 - alpha)) / 255) as u8);
    }
    Gray::new(w, h, data)
}

/// A file of either kind, told apart by its first bytes.
pub fn from_bytes(bytes: &[u8]) -> Result<Gray, String> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        from_png(bytes)
    } else {
        from_pgm(bytes)
    }
}
