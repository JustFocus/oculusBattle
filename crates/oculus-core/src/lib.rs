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

    /// At or over the growth cap. A full player or cpu base produces nothing until it drops
    /// below the cap again (neutral bases never grow at all).
    pub fn is_full(&self) -> bool {
        self.value >= self.max_value()
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

/// How a click interprets the base under the cursor (see [`Game::click_with`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ClickMode {
    /// Select an own base; with a selection, send from every selected base to the clicked
    /// one. Clicking the only selected base, or empty ground, deselects.
    #[default]
    Single,
    /// Add or remove an own base from the selection (ctrl/cmd-click). On any other base it
    /// behaves like `Single`; on empty ground it does nothing, so a near miss keeps the
    /// selection.
    Toggle,
    /// Send from every player base to the clicked base at once (shift-click), regardless of
    /// the current selection.
    All,
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
    /// One base left a selection that still has others in it.
    BaseDeselected {
        base: BaseId,
    },
    /// The selection became empty.
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
