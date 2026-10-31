//! The G-code of a laser that burns line by line: `M4` (dynamic power: the power follows the speed in the corners), `S0` while it travels and `S<power>` where it burns, one line
//! left to right, the next right to left. Before and after every line the head runs past the work (the overscan) so that it is at full speed when the laser turns on.

use crate::dither::Dots;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Params {
    /// The size of one dot, in mm.
    pub pixel_mm: f64,
    pub feed_mm_min: f64,
    /// The spindle value that is full power for the controller (`$30` of GRBL, usually 1000).
    pub s_max: u32,
    /// The part of full power that is used, from 0 to 100.
    pub power_percent: f64,
    /// How far the head runs past the work at each end of a line.
    pub overscan_mm: f64,
    /// Where the lower left corner of the engraving goes.
    pub origin: (f64, f64),
}

impl Default for Params {
    fn default() -> Self {
        Params {
            pixel_mm: 0.1,
            feed_mm_min: 3000.0,
            s_max: 1000,
            power_percent: 50.0,
            overscan_mm: 2.0,
            origin: (0.0, 0.0),
        }
    }
}

impl Params {
    pub fn validate(&self) -> Result<(), String> {
        let range = |name: &str, v: f64, lo: f64, hi: f64| {
            if v.is_finite() && v >= lo && v <= hi {
                Ok(())
            } else {
                Err(format!("{name} is {v}: it must be from {lo} to {hi}"))
            }
        };
        range("pixel_mm", self.pixel_mm, 0.01, 5.0)?;
        range("feed_mm_min", self.feed_mm_min, 1.0, 30_000.0)?;
        range("power_percent", self.power_percent, 0.0, 100.0)?;
        range("overscan_mm", self.overscan_mm, 0.0, 50.0)?;
        range("origin x", self.origin.0, -100_000.0, 100_000.0)?;
        range("origin y", self.origin.1, -100_000.0, 100_000.0)?;
        if self.s_max == 0 || self.s_max > 100_000 {
            return Err(format!(
                "s_max is {}: it must be from 1 to 100000",
                self.s_max
            ));
        }
        Ok(())
    }

    pub fn power(&self) -> u32 {
        (f64::from(self.s_max) * self.power_percent / 100.0).round() as u32
    }
}

/// The run-up that a head needs to be at `feed_mm_min` with an acceleration of `accel_mm_s2`: v^2 / 2a.
pub fn overscan_for(feed_mm_min: f64, accel_mm_s2: f64) -> Result<f64, String> {
    if !(feed_mm_min.is_finite()
        && feed_mm_min > 0.0
        && accel_mm_s2.is_finite()
        && accel_mm_s2 > 0.0)
    {
        return Err("the feed and the acceleration must be more than 0".into());
    }
    let v = feed_mm_min / 60.0;
    Ok(v * v / (2.0 * accel_mm_s2))
}

#[derive(Debug, Clone, PartialEq)]
pub struct Engraving {
    pub gcode: String,
    /// Lines of dots that have something to burn.
    pub rows: usize,
    /// Runs of dots that are burned.
    pub runs: usize,
    pub burn_mm: f64,
    /// The time of the burning and the travelling at the feed, without the corners.
    pub seconds: f64,
}

fn n(v: f64) -> String {
    let s = format!("{v:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" || s.is_empty() {
        "0".to_string()
    } else {
        s.to_string()
    }
}

pub fn engrave(dots: &Dots, p: &Params) -> Result<Engraving, String> {
    p.validate()?;
    let s = p.power();
    let mut g = format!("G21\nG90\nM4 S0\nF{}\n", n(p.feed_mm_min));
    let (mut rows, mut runs, mut burn_mm, mut travel_mm) = (0usize, 0usize, 0.0f64, 0.0f64);
    let mut head_x = p.origin.0;
    for y in 0..dots.h {
        // runs of burning dots in the order of the x axis
        let mut found: Vec<(usize, usize)> = Vec::new();
        let mut x = 0;
        while x < dots.w {
            if dots.at(x, y) {
                let a = x;
                while x < dots.w && dots.at(x, y) {
                    x += 1;
                }
                found.push((a, x));
            } else {
                x += 1;
            }
        }
        if found.is_empty() {
            continue;
        }
        let right_to_left = rows % 2 == 1;
        rows += 1;
        let ypos = p.origin.1 + (dots.h - 1 - y) as f64 * p.pixel_mm;
        let at = |i: usize| p.origin.0 + i as f64 * p.pixel_mm;
        let ordered: Vec<(f64, f64)> = if right_to_left {
            found.iter().rev().map(|&(a, b)| (at(b), at(a))).collect()
        } else {
            found.iter().map(|&(a, b)| (at(a), at(b))).collect()
        };
        let dir = if right_to_left { -1.0 } else { 1.0 };
        let first = ordered[0].0;
        let approach = first - dir * p.overscan_mm;
        travel_mm += (approach - head_x).abs();
        g.push_str(&format!("G0 X{} Y{}\n", n(approach), n(ypos)));
        if p.overscan_mm > 0.0 {
            g.push_str(&format!("G1 X{} S0\n", n(first)));
            travel_mm += p.overscan_mm;
        }
        let mut here = first;
        for &(from, to) in &ordered {
            if (from - here).abs() > 1e-9 {
                g.push_str(&format!("G1 X{} S0\n", n(from)));
                travel_mm += (from - here).abs();
            }
            g.push_str(&format!("G1 X{} S{s}\n", n(to)));
            burn_mm += (to - from).abs();
            runs += 1;
            here = to;
        }
        let end = here + dir * p.overscan_mm;
        if p.overscan_mm > 0.0 {
            g.push_str(&format!("G1 X{} S0\n", n(end)));
            travel_mm += p.overscan_mm;
        }
        head_x = end;
    }
    g.push_str("M5\nM2\n");
    Ok(Engraving {
        gcode: g,
        rows,
        runs,
        burn_mm,
        seconds: (burn_mm + travel_mm) / (p.feed_mm_min / 60.0),
    })
}
