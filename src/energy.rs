// EH3D-PHOENIX-LASER - the energy a layer puts into the material.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
//! How much energy the beam puts into the material, worked out from the settings of a layer and the optical power of the laser (`laser_watts` in `[machine]`). It is plain arithmetic:
//! the power of the beam is the optical power times the percent of the layer, the beam goes at the speed of the layer, so every millimetre of a line gets the power divided by the speed in
//! millimetres a second. A hatch puts that energy on a strip as wide as the distance between its lines. It says nothing of how a material answers to it, and a laser whose optical power
//! is not known (`laser_watts = 0`) gets no numbers.

use crate::vconfig::{LayerMode, LayerParams};

/// The energy for each millimetre of a line of the layer, in joules, for one pass. None when the optical power is not known or the layer has no speed or no power.
pub fn line_energy_j_per_mm(laser_watts: f64, p: &LayerParams) -> Option<f64> {
    let watts = laser_watts * p.power_percent / 100.0;
    let speed_mm_s = p.speed_mm_min / 60.0;
    let ok = laser_watts.is_finite()
        && laser_watts > 0.0
        && p.power_percent > 0.0
        && speed_mm_s.is_finite()
        && speed_mm_s > 0.0;
    ok.then(|| watts / speed_mm_s)
}

/// The energy for each square millimetre of a hatch, in joules, for one pass: the energy of a millimetre of line over the distance between the lines. None as
/// [`line_energy_j_per_mm`], or when the lines are not apart.
pub fn hatch_energy_j_per_mm2(laser_watts: f64, p: &LayerParams) -> Option<f64> {
    if !(p.hatch_spacing_mm.is_finite() && p.hatch_spacing_mm > 0.0) {
        return None;
    }
    line_energy_j_per_mm(laser_watts, p).map(|j| j / p.hatch_spacing_mm)
}

/// The words that follow the line of a layer in the list of layers: the energy of one pass for each millimetre (cut) or each square millimetre (engrave); empty for a layer that is
/// skipped or when the optical power is not known.
pub fn describe(laser_watts: f64, p: &LayerParams) -> String {
    match p.mode {
        LayerMode::Cut => line_energy_j_per_mm(laser_watts, p)
            .map(|j| format!(", {j:.3} J/mm a pass"))
            .unwrap_or_default(),
        LayerMode::Engrave => hatch_energy_j_per_mm2(laser_watts, p)
            .map(|j| format!(", {j:.3} J/mm2 a pass"))
            .unwrap_or_default(),
        LayerMode::Skip => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layer(mode: LayerMode, power: f64, speed: f64, spacing: f64) -> LayerParams {
        LayerParams {
            mode,
            power_percent: power,
            speed_mm_min: speed,
            hatch_spacing_mm: spacing,
            ..LayerParams::default()
        }
    }

    #[test]
    fn a_line_gets_the_power_over_the_speed() {
        // 10 W at 50 % is 5 W; 600 mm/min is 10 mm/s; 5 / 10 = 0.5 J/mm
        let p = layer(LayerMode::Cut, 50.0, 600.0, 0.1);
        assert!((line_energy_j_per_mm(10.0, &p).unwrap() - 0.5).abs() < 1e-12);
        // 40 W at 100 % and 1200 mm/min (20 mm/s) is 2 J/mm
        let p = layer(LayerMode::Cut, 100.0, 1200.0, 0.1);
        assert!((line_energy_j_per_mm(40.0, &p).unwrap() - 2.0).abs() < 1e-12);
    }

    #[test]
    fn a_hatch_spreads_the_line_over_the_distance_between_its_lines() {
        // 0.5 J/mm over lines 0.1 mm apart is 5 J/mm2; 2 J/mm over 0.2 mm is 10 J/mm2
        let p = layer(LayerMode::Engrave, 50.0, 600.0, 0.1);
        assert!((hatch_energy_j_per_mm2(10.0, &p).unwrap() - 5.0).abs() < 1e-9);
        let p = layer(LayerMode::Engrave, 100.0, 1200.0, 0.2);
        assert!((hatch_energy_j_per_mm2(40.0, &p).unwrap() - 10.0).abs() < 1e-9);
    }

    #[test]
    fn nothing_is_said_when_a_number_is_not_known() {
        let p = layer(LayerMode::Cut, 50.0, 600.0, 0.1);
        assert_eq!(line_energy_j_per_mm(0.0, &p), None);
        assert_eq!(line_energy_j_per_mm(f64::NAN, &p), None);
        assert_eq!(line_energy_j_per_mm(-5.0, &p), None);
        let no_power = layer(LayerMode::Cut, 0.0, 600.0, 0.1);
        assert_eq!(line_energy_j_per_mm(10.0, &no_power), None);
        let no_speed = layer(LayerMode::Cut, 50.0, 0.0, 0.1);
        assert_eq!(line_energy_j_per_mm(10.0, &no_speed), None);
        let no_gap = layer(LayerMode::Engrave, 50.0, 600.0, 0.0);
        assert_eq!(hatch_energy_j_per_mm2(10.0, &no_gap), None);
        assert_eq!(describe(0.0, &p), "");
        let skipped = layer(LayerMode::Skip, 50.0, 600.0, 0.1);
        assert_eq!(describe(10.0, &skipped), "");
    }

    #[test]
    fn the_words_name_the_unit_of_the_kind_of_layer() {
        let cut = layer(LayerMode::Cut, 50.0, 600.0, 0.1);
        assert_eq!(describe(10.0, &cut), ", 0.500 J/mm a pass");
        let engrave = layer(LayerMode::Engrave, 50.0, 600.0, 0.1);
        assert_eq!(describe(10.0, &engrave), ", 5.000 J/mm2 a pass");
    }
}
