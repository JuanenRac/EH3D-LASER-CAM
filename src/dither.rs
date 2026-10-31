//! From grey levels to dots: a dot is where the laser burns.

use crate::image::Gray;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// Every pixel darker than the threshold burns.
    Threshold(u8),
    FloydSteinberg,
    /// Keeps only three quarters of the error: lighter greys come out cleaner, at the price of pure black and white areas.
    Atkinson,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dots {
    pub w: usize,
    pub h: usize,
    /// True where the laser burns.
    pub burn: Vec<bool>,
}

impl Dots {
    pub fn at(&self, x: usize, y: usize) -> bool {
        self.burn[y * self.w + x]
    }
    pub fn count(&self) -> usize {
        self.burn.iter().filter(|b| **b).count()
    }
}

pub fn dither(img: &Gray, method: Method) -> Dots {
    let (w, h) = (img.w, img.h);
    let mut burn = vec![false; w * h];
    match method {
        Method::Threshold(t) => {
            for (b, &v) in burn.iter_mut().zip(&img.data) {
                *b = v < t;
            }
        }
        Method::FloydSteinberg | Method::Atkinson => {
            let mut v: Vec<i32> = img.data.iter().map(|&b| i32::from(b)).collect();
            let spread: &[(isize, isize, i32, i32)] = match method {
                Method::FloydSteinberg => {
                    &[(1, 0, 7, 16), (-1, 1, 3, 16), (0, 1, 5, 16), (1, 1, 1, 16)]
                }
                _ => &[
                    (1, 0, 1, 8),
                    (2, 0, 1, 8),
                    (-1, 1, 1, 8),
                    (0, 1, 1, 8),
                    (1, 1, 1, 8),
                    (0, 2, 1, 8),
                ],
            };
            for y in 0..h {
                for x in 0..w {
                    let old = v[y * w + x].clamp(0, 255);
                    let new = if old < 128 { 0 } else { 255 };
                    burn[y * w + x] = new == 0;
                    let err = old - new;
                    for &(dx, dy, num, den) in spread {
                        let (nx, ny) = (x as isize + dx, y as isize + dy);
                        if nx >= 0 && (nx as usize) < w && (ny as usize) < h {
                            v[ny as usize * w + nx as usize] += err * num / den;
                        }
                    }
                }
            }
        }
    }
    Dots { w, h, burn }
}
