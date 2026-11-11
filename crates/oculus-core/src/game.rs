use crate::{Base, BaseId, GameEvent, Phase, Point, Unit};

/// The whole simulation. Construct with a seed for determinism.
#[derive(Debug, Clone)]
pub struct Game {
    // Implementation-defined fields.
}

impl Game {
    /// Starts level 1 with freshly generated bases.
    pub fn new(seed: u64) -> Self {
        let _ = seed;
        todo!("task 1")
    }

    /// Advances the simulation by exactly one tick ([`crate::TICK_SECONDS`]).
    /// Does nothing during an intermission.
    pub fn tick(&mut self) {
        todo!("task 1")
    }

    /// Handles a primary click at a world position: selection/sending while
    /// playing, or starting the next level during an intermission.
    pub fn click(&mut self, at: Point) {
        let _ = at;
        todo!("task 1")
    }

    /// Returns and clears events produced since the last call.
    pub fn drain_events(&mut self) -> Vec<GameEvent> {
        todo!("task 1")
    }

    pub fn bases(&self) -> &[Base] {
        todo!("task 1")
    }

    pub fn units(&self) -> &[Unit] {
        todo!("task 1")
    }

    /// Current level (the original's `ai.power`), starting at 1.
    pub fn level(&self) -> u32 {
        todo!("task 1")
    }

    pub fn phase(&self) -> Phase {
        todo!("task 1")
    }

    /// The base the player has selected, if any.
    pub fn selected(&self) -> Option<BaseId> {
        todo!("task 1")
    }

    /// The base whose circle contains `at`, if any (used for hover/click).
    pub fn base_at(&self, at: Point) -> Option<BaseId> {
        let _ = at;
        todo!("task 1")
    }
}
