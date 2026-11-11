//! Oculus Battle game rules.
//!
//! Pure and deterministic: no rendering, no I/O, no global randomness. The
//! simulation advances in fixed ticks of [`TICK_SECONDS`] and reports what
//! happened through [`GameEvent`]s so a frontend can animate it.
//!
//! This file is the API contract between `oculus-core` and `oculus-game`.
//! Public item names and signatures here are stable; implementations live in
//! the submodules.

mod ai;
mod game;
mod geom;
mod level;
mod rng;

pub use game::Game;
pub use geom::Point;

/// World width in world units (the original canvas size).
pub const WORLD_WIDTH: f32 = 800.0;
/// World height in world units.
pub const WORLD_HEIGHT: f32 = 500.0;
/// Length of one simulation tick in seconds.
pub const TICK_SECONDS: f32 = 1.0 / 60.0;
/// Radius of a unit in world units.
pub const UNIT_RADIUS: f32 = 5.0;
/// Unit speed in world units per tick.
pub const UNIT_SPEED: f32 = 3.0;

/// Index into [`Game::bases`]. Stable for the duration of a level.
pub type BaseId = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Team {
    Player,
    Cpu,
    Neutral,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Base {
    pub id: BaseId,
    pub pos: Point,
    pub radius: f32,
    pub team: Team,
    /// Current unit count. Fractional because bases grow every tick;
    /// display with `value.round()` (the original showed `Math.round`).
    pub value: f32,
    /// Initial value; growth caps at `2 * start_value`.
    pub start_value: f32,
}

impl Base {
    pub fn max_value(&self) -> f32 {
        self.start_value * 2.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Unit {
    pub team: Team,
    pub pos: Point,
    /// Velocity in world units per tick.
    pub vel: Point,
    pub target: BaseId,
}

/// High-level state of the match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// A level is in progress.
    Playing,
    /// The level ended; waiting for a click to start `next_level`.
    Intermission { outcome: Outcome, next_level: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Victory,
    Defeat,
}

/// Things that happened during a tick or a click, for the frontend to animate.
#[derive(Debug, Clone, PartialEq)]
pub enum GameEvent {
    LevelStarted {
        level: u32,
    },
    BaseSelected {
        base: BaseId,
    },
    SelectionCleared,
    /// `count` units left `from` heading for `to`.
    UnitsSent {
        from: BaseId,
        to: BaseId,
        team: Team,
        count: u32,
    },
    /// A unit reached its target. `reinforced` is true when the base was friendly.
    UnitArrived {
        base: BaseId,
        team: Team,
        pos: Point,
        reinforced: bool,
    },
    BaseCaptured {
        base: BaseId,
        from: Team,
        to: Team,
    },
    LevelEnded {
        outcome: Outcome,
    },
}
