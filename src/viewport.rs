//! Zoom and pan of the drawing in the window, in plain numbers: no screen is needed to test it. The board is in millimetres with the y axis up; the screen has the y axis down.

/// The smallest and the largest zoom, in pixels for each millimetre.
pub const MIN_SCALE: f64 = 0.01;
pub const MAX_SCALE: f64 = 20_000.0;

/// The part of the board in the middle of the picture and how much it is magnified.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub centre_x: f64,
    pub centre_y: f64,
    pub px_per_mm: f64,
}

impl Default for Viewport {
    fn default() -> Self {
        Viewport {
            centre_x: 0.0,
            centre_y: 0.0,
            px_per_mm: 10.0,
        }
    }
}

impl Viewport {
    /// The view that shows the whole box (`min` and `max` corners, in mm) in a picture of `width_px` x `height_px`, with a margin of `margin_px` all round.
    pub fn fit(
        min: (f64, f64),
        max: (f64, f64),
        width_px: f64,
        height_px: f64,
        margin_px: f64,
    ) -> Viewport {
        let all_finite = [min.0, min.1, max.0, max.1].iter().all(|v| v.is_finite());
        if !all_finite || min.0 > max.0 || min.1 > max.1 {
            return Viewport::default();
        }
        let w = (max.0 - min.0).max(1e-6);
        let h = (max.1 - min.1).max(1e-6);
        let room_w = (width_px - 2.0 * margin_px).max(1.0);
        let room_h = (height_px - 2.0 * margin_px).max(1.0);
        Viewport {
            centre_x: (min.0 + max.0) / 2.0,
            centre_y: (min.1 + max.1) / 2.0,
            px_per_mm: (room_w / w).min(room_h / h).clamp(MIN_SCALE, MAX_SCALE),
        }
    }

    /// Where a point of the board falls in the picture (pixels from its top left corner).
    pub fn to_screen(&self, x: f64, y: f64, width_px: f64, height_px: f64) -> (f64, f64) {
        (
            width_px / 2.0 + (x - self.centre_x) * self.px_per_mm,
            height_px / 2.0 - (y - self.centre_y) * self.px_per_mm,
        )
    }

    /// The point of the board under a place of the picture.
    pub fn to_world(&self, sx: f64, sy: f64, width_px: f64, height_px: f64) -> (f64, f64) {
        (
            self.centre_x + (sx - width_px / 2.0) / self.px_per_mm,
            self.centre_y - (sy - height_px / 2.0) / self.px_per_mm,
        )
    }

    /// Magnifies by `factor` (above 1 closer, below 1 farther) keeping still the point of the board that is under the pointer.
    pub fn zoom_at(&mut self, sx: f64, sy: f64, width_px: f64, height_px: f64, factor: f64) {
        if !factor.is_finite() || factor <= 0.0 {
            return;
        }
        let (wx, wy) = self.to_world(sx, sy, width_px, height_px);
        self.px_per_mm = (self.px_per_mm * factor).clamp(MIN_SCALE, MAX_SCALE);
        self.centre_x = wx - (sx - width_px / 2.0) / self.px_per_mm;
        self.centre_y = wy + (sy - height_px / 2.0) / self.px_per_mm;
    }

    /// Drags the drawing by `dx`, `dy` pixels (to the right and down are positive).
    pub fn pan_px(&mut self, dx: f64, dy: f64) {
        self.centre_x -= dx / self.px_per_mm;
        self.centre_y += dy / self.px_per_mm;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_middle_of_the_box_is_the_middle_of_the_picture() {
        let v = Viewport::fit((0.0, 0.0), (100.0, 50.0), 800.0, 600.0, 20.0);
        let (x, y) = v.to_screen(50.0, 25.0, 800.0, 600.0);
        assert!((x - 400.0).abs() < 1e-9 && (y - 300.0).abs() < 1e-9);
    }

    #[test]
    fn the_whole_box_fits_inside_the_margin() {
        let v = Viewport::fit((-10.0, -5.0), (30.0, 15.0), 400.0, 300.0, 10.0);
        let (x0, y0) = v.to_screen(-10.0, 15.0, 400.0, 300.0);
        let (x1, y1) = v.to_screen(30.0, -5.0, 400.0, 300.0);
        assert!(x0 >= 10.0 - 1e-9 && y0 >= 10.0 - 1e-9);
        assert!(x1 <= 390.0 + 1e-9 && y1 <= 290.0 + 1e-9);
    }

    #[test]
    fn the_y_axis_points_up_on_the_board_and_down_on_the_screen() {
        let v = Viewport::default();
        let (_, low) = v.to_screen(0.0, 1.0, 100.0, 100.0);
        let (_, high) = v.to_screen(0.0, -1.0, 100.0, 100.0);
        assert!(low < high);
    }

    #[test]
    fn to_world_undoes_to_screen() {
        let v = Viewport {
            centre_x: 12.5,
            centre_y: -3.0,
            px_per_mm: 7.5,
        };
        let (sx, sy) = v.to_screen(4.0, 9.0, 640.0, 480.0);
        let (x, y) = v.to_world(sx, sy, 640.0, 480.0);
        assert!((x - 4.0).abs() < 1e-9 && (y - 9.0).abs() < 1e-9);
    }

    #[test]
    fn zooming_keeps_the_point_under_the_pointer_still() {
        let mut v = Viewport::fit((0.0, 0.0), (100.0, 100.0), 500.0, 500.0, 10.0);
        let before = v.to_world(120.0, 340.0, 500.0, 500.0);
        v.zoom_at(120.0, 340.0, 500.0, 500.0, 2.5);
        let after = v.to_world(120.0, 340.0, 500.0, 500.0);
        assert!((before.0 - after.0).abs() < 1e-9 && (before.1 - after.1).abs() < 1e-9);
    }

    #[test]
    fn the_zoom_has_limits_and_ignores_nonsense() {
        let mut v = Viewport::default();
        v.zoom_at(10.0, 10.0, 100.0, 100.0, 1e12);
        assert_eq!(v.px_per_mm, MAX_SCALE);
        v.zoom_at(10.0, 10.0, 100.0, 100.0, 1e-12);
        assert_eq!(v.px_per_mm, MIN_SCALE);
        let before = v;
        v.zoom_at(10.0, 10.0, 100.0, 100.0, f64::NAN);
        v.zoom_at(10.0, 10.0, 100.0, 100.0, -2.0);
        assert_eq!(v, before);
    }

    #[test]
    fn dragging_moves_the_drawing_with_the_pointer() {
        let mut v = Viewport::default();
        let before = v.to_screen(1.0, 1.0, 200.0, 200.0);
        v.pan_px(30.0, -12.0);
        let after = v.to_screen(1.0, 1.0, 200.0, 200.0);
        assert!((after.0 - before.0 - 30.0).abs() < 1e-9);
        assert!((after.1 - before.1 + 12.0).abs() < 1e-9);
    }

    #[test]
    fn an_empty_or_broken_box_gives_the_default_view() {
        assert_eq!(
            Viewport::fit((1.0, 1.0), (0.0, 0.0), 100.0, 100.0, 5.0),
            Viewport::default()
        );
        assert_eq!(
            Viewport::fit((f64::NAN, 0.0), (1.0, 1.0), 100.0, 100.0, 5.0),
            Viewport::default()
        );
        assert_eq!(
            Viewport::fit(
                (f64::INFINITY, f64::INFINITY),
                (f64::NEG_INFINITY, f64::NEG_INFINITY),
                100.0,
                100.0,
                5.0
            ),
            Viewport::default()
        );
    }

    #[test]
    fn a_single_point_still_gives_a_usable_view() {
        let v = Viewport::fit((5.0, 5.0), (5.0, 5.0), 100.0, 100.0, 5.0);
        assert!(v.px_per_mm.is_finite() && v.px_per_mm <= MAX_SCALE);
    }
}
