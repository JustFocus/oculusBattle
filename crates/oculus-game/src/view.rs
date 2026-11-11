//! Letterboxed camera and screen <-> world conversion (pure math plus one camera builder).

use macroquad::prelude::*;
use oculus_core::{WORLD_HEIGHT, WORLD_WIDTH};

/// Maps the fixed 800x500 world into a window, keeping aspect ratio and centering it below an
/// optional HUD strip. All values are in logical (DPI-independent) screen pixels, the same unit
/// as `screen_width()` and `mouse_position()`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    /// Logical pixels per world unit.
    pub scale: f32,
    /// Screen position of the world origin (top-left corner of the board).
    pub origin: Vec2,
    pub screen: Vec2,
    pub top_inset: f32,
}

impl View {
    /// Fits the world into the window area below `top_inset` pixels.
    pub fn fit(screen_w: f32, screen_h: f32, top_inset: f32) -> Self {
        let avail_h = (screen_h - top_inset).max(1.0);
        let avail_w = screen_w.max(1.0);
        let scale = (avail_w / WORLD_WIDTH).min(avail_h / WORLD_HEIGHT);
        let origin = vec2(
            (avail_w - WORLD_WIDTH * scale) * 0.5,
            top_inset + (avail_h - WORLD_HEIGHT * scale) * 0.5,
        );
        Self {
            scale,
            origin,
            screen: vec2(screen_w, screen_h),
            top_inset,
        }
    }

    pub fn to_world(self, screen: Vec2) -> Vec2 {
        (screen - self.origin) / self.scale
    }

    pub fn to_screen(self, world: Vec2) -> Vec2 {
        world * self.scale + self.origin
    }

    /// The board in screen space.
    pub fn board_rect(&self) -> Rect {
        Rect::new(
            self.origin.x,
            self.origin.y,
            WORLD_WIDTH * self.scale,
            WORLD_HEIGHT * self.scale,
        )
    }

    /// A camera whose world space is the 800x500 board, drawing over the whole window (so
    /// world-space content outside the board is visible until the bars are painted).
    pub fn camera(&self) -> Camera2D {
        let tl = self.to_world(Vec2::ZERO);
        let (w, h) = (self.screen.x / self.scale, self.screen.y / self.scale);
        // Negative height: with a plain positive rect macroquad renders the scene upside down
        // on the default framebuffer.
        Camera2D::from_display_rect(Rect::new(tl.x, tl.y + h, w, -h))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < 1e-3
    }

    #[test]
    fn exact_aspect_fills_window() {
        let v = View::fit(1000.0, 625.0, 0.0);
        assert!((v.scale - 1.25).abs() < 1e-6);
        assert!(close(v.origin, Vec2::ZERO));
    }

    #[test]
    fn wide_window_has_side_bars() {
        let v = View::fit(1280.0, 720.0, 0.0);
        assert!((v.scale - 1.44).abs() < 1e-6);
        assert!((v.origin.x - 64.0).abs() < 1e-3);
        assert!(v.origin.y.abs() < 1e-3);
    }

    #[test]
    fn narrow_window_has_top_bottom_bars() {
        let v = View::fit(400.0, 800.0, 0.0);
        assert!((v.scale - 0.5).abs() < 1e-6);
        assert!(v.origin.x.abs() < 1e-3);
        assert!((v.origin.y - 275.0).abs() < 1e-3);
    }

    #[test]
    fn inset_shifts_board_down() {
        let v = View::fit(1000.0, 625.0, 30.0);
        assert!((v.scale - 1.19).abs() < 1e-6);
        assert!(v.origin.y >= 30.0);
        let r = v.board_rect();
        assert!(r.bottom() <= 625.0 + 1e-3);
    }

    #[test]
    fn round_trip_and_known_points() {
        for (w, h) in [(1000.0, 625.0), (1280.0, 720.0), (400.0, 800.0), (3.0, 3.0)] {
            let v = View::fit(w, h, 34.0);
            for p in [vec2(75.0, 250.0), vec2(725.0, 250.0), vec2(0.0, 0.0)] {
                assert!(close(v.to_world(v.to_screen(p)), p));
            }
            assert!(close(
                v.to_world(v.board_rect().center()),
                vec2(400.0, 250.0)
            ));
        }
    }

    #[test]
    fn degenerate_window_is_finite() {
        let v = View::fit(0.0, 0.0, 34.0);
        assert!(v.scale.is_finite() && v.scale > 0.0);
    }
}
