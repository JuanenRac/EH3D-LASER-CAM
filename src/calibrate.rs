//! A test grid to find the power and the feed that suit a material: a square for every pair of power and feed, burned line by line.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grid {
    pub square_mm: f64,
    pub gap_mm: f64,
    pub line_spacing_mm: f64,
    pub s_max: u32,
}

impl Default for Grid {
    fn default() -> Self {
        Grid {
            square_mm: 8.0,
            gap_mm: 3.0,
            line_spacing_mm: 0.1,
            s_max: 1000,
        }
    }
}

pub const MAX_CELLS: usize = 12;

fn n(v: f64) -> String {
    let s = format!("{v:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" || s.is_empty() {
        "0".to_string()
    } else {
        s.to_string()
    }
}

/// The G-code of the grid: one column for each power (a percent of full power) and one row for each feed (mm/min).
pub fn grid(powers_percent: &[f64], feeds_mm_min: &[f64], g: &Grid) -> Result<String, String> {
    if powers_percent.is_empty()
        || feeds_mm_min.is_empty()
        || powers_percent.len() > MAX_CELLS
        || feeds_mm_min.len() > MAX_CELLS
    {
        return Err(format!(
            "the grid needs from 1 to {MAX_CELLS} powers and from 1 to {MAX_CELLS} feeds"
        ));
    }
    if powers_percent
        .iter()
        .any(|p| !p.is_finite() || !(0.0..=100.0).contains(p))
    {
        return Err("a power must be a percent from 0 to 100".into());
    }
    if feeds_mm_min
        .iter()
        .any(|f| !f.is_finite() || !(1.0..=30_000.0).contains(f))
    {
        return Err("a feed must be from 1 to 30000 mm/min".into());
    }
    if !(g.square_mm.is_finite()
        && g.square_mm >= 1.0
        && g.square_mm <= 50.0
        && g.gap_mm.is_finite()
        && g.gap_mm >= 0.0
        && g.gap_mm <= 50.0
        && g.line_spacing_mm.is_finite()
        && g.line_spacing_mm >= 0.02
        && g.line_spacing_mm <= g.square_mm
        && g.s_max > 0
        && g.s_max <= 100_000)
    {
        return Err(
            "the size of the squares, the gap, the spacing of the lines or s_max are out of range"
                .into(),
        );
    }
    let mut out = String::from("G21\nG90\nM4 S0\n");
    let lines = (g.square_mm / g.line_spacing_mm).floor() as usize + 1;
    for (row, feed) in feeds_mm_min.iter().enumerate() {
        for (col, power) in powers_percent.iter().enumerate() {
            let x0 = col as f64 * (g.square_mm + g.gap_mm);
            let y0 = row as f64 * (g.square_mm + g.gap_mm);
            let s = (f64::from(g.s_max) * power / 100.0).round() as u32;
            out.push_str(&format!(
                "(power {}% feed {})\nG0 X{} Y{}\n",
                n(*power),
                n(*feed),
                n(x0),
                n(y0)
            ));
            for k in 0..lines {
                let y = y0 + (k as f64 * g.line_spacing_mm).min(g.square_mm);
                let (a, b) = if k % 2 == 0 {
                    (x0, x0 + g.square_mm)
                } else {
                    (x0 + g.square_mm, x0)
                };
                out.push_str(&format!("G1 X{} Y{} S0 F{}\n", n(a), n(y), n(*feed)));
                out.push_str(&format!("G1 X{} S{s} F{}\n", n(b), n(*feed)));
            }
        }
    }
    out.push_str("M5\nM2\n");
    Ok(out)
}
