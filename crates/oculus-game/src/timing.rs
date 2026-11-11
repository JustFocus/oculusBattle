//! Fixed-timestep accumulator (pure, no GL).

/// Longest frame time we are willing to simulate, in seconds. A backgrounded tab can report a
/// huge delta; clamping stops the game from fast-forwarding when it returns.
pub const MAX_FRAME_DT: f32 = 0.25;

/// Converts variable frame times into a whole number of fixed simulation steps.
#[derive(Debug, Default)]
pub struct Stepper {
    acc: f32,
}

impl Stepper {
    pub fn new() -> Self {
        Self::default()
    }

    /// Clamps `dt` to [`MAX_FRAME_DT`], adds it to the accumulator and returns how many steps of
    /// length `step` are due. The remainder carries over to the next frame.
    pub fn advance(&mut self, dt: f32, step: f32) -> u32 {
        self.acc += clamp_dt(dt);
        let n = (self.acc / step).floor();
        self.acc -= n * step;
        n as u32
    }

    /// Drops any partial step (used when the simulation was paused).
    pub fn reset(&mut self) {
        self.acc = 0.0;
    }
}

/// Clamps a raw frame time into `[0, MAX_FRAME_DT]`.
pub fn clamp_dt(dt: f32) -> f32 {
    if dt.is_finite() {
        dt.clamp(0.0, MAX_FRAME_DT)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STEP: f32 = 1.0 / 60.0;

    #[test]
    fn exact_frames_give_one_step_each() {
        let mut s = Stepper::new();
        let total: u32 = (0..60).map(|_| s.advance(STEP, STEP)).sum();
        assert!((59..=60).contains(&total));
    }

    #[test]
    fn fast_monitor_averages_to_sixty_hz() {
        let mut s = Stepper::new();
        let total: u32 = (0..144).map(|_| s.advance(1.0 / 144.0, STEP)).sum();
        assert!((59..=60).contains(&total), "got {total}");
    }

    #[test]
    fn slow_frames_catch_up() {
        let mut s = Stepper::new();
        assert_eq!(s.advance(0.1, STEP), 6);
    }

    #[test]
    fn huge_dt_is_clamped() {
        let mut s = Stepper::new();
        let n = s.advance(600.0, STEP);
        assert!(n <= (MAX_FRAME_DT / STEP).ceil() as u32);
        assert!(n >= 14);
    }

    #[test]
    fn bad_dt_is_ignored() {
        let mut s = Stepper::new();
        assert_eq!(s.advance(f32::NAN, STEP), 0);
        assert_eq!(s.advance(-1.0, STEP), 0);
    }

    #[test]
    fn remainder_carries_over() {
        let mut s = Stepper::new();
        assert_eq!(s.advance(STEP * 0.6, STEP), 0);
        assert_eq!(s.advance(STEP * 0.6, STEP), 1);
    }
}
