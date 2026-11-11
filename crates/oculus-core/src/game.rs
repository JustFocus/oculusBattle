//! The simulation: state, fixed-tick stepping and click handling.
use crate::level::generate_bases;
use crate::rng::Rng;
use crate::{
    ai, Base, BaseId, ClickMode, GameEvent, Outcome, Phase, Point, Team, Unit, TICK_SECONDS,
    UNIT_RADIUS, UNIT_SPEED,
};

/// Milliseconds per tick; the AI timer is expressed in ms like the original.
const TICK_MS: f32 = TICK_SECONDS * 1000.0;
/// Base AI delay in ms at level 1, divided by the level.
const AI_BASE_DELAY_MS: f32 = 8000.0;
/// Random extra AI delay in ms, re-rolled every tick.
const AI_JITTER_MS: f32 = 4000.0;
/// A full refill (start value) takes this many ticks at multiplier 1.
const GROWTH_TICKS: f32 = 1000.0;
/// Cpu growth speeds up by this fraction per level above 1.
const CPU_LEVEL_GROWTH: f32 = 0.1;

/// The whole simulation. Construct with a seed for determinism.
#[derive(Debug, Clone)]
pub struct Game {
    bases: Vec<Base>,
    units: Vec<Unit>,
    level: u32,
    phase: Phase,
    /// Selected player bases, in selection order, without duplicates.
    selected: Vec<BaseId>,
    /// Ticks played in the current level; frozen during an intermission.
    level_ticks: u64,
    /// Milliseconds since the AI last acted.
    ai_timer: f32,
    rng: Rng,
    events: Vec<GameEvent>,
}

impl Game {
    /// Starts level 1 with freshly generated bases.
    pub fn new(seed: u64) -> Self {
        let mut game = Self {
            bases: Vec::new(),
            units: Vec::new(),
            level: 1,
            phase: Phase::Playing,
            selected: Vec::new(),
            level_ticks: 0,
            ai_timer: 0.0,
            rng: Rng::new(seed),
            events: Vec::new(),
        };
        game.start_level(1);
        game
    }

    /// Advances the simulation by exactly one tick ([`crate::TICK_SECONDS`]).
    /// Does nothing during an intermission.
    pub fn tick(&mut self) {
        if self.phase != Phase::Playing {
            return;
        }
        self.level_ticks += 1;
        self.ai_move();
        self.move_units();
        self.resolve_arrivals();
        self.grow_bases();
        self.check_level_end();
    }

    /// A plain click: same as `click_with(at, ClickMode::Single)`.
    pub fn click(&mut self, at: Point) {
        self.click_with(at, ClickMode::Single);
    }

    /// Handles a primary click at a world position: selection/sending while
    /// playing, or starting the next level during an intermission (any mode).
    /// See [`ClickMode`] for what each mode does.
    pub fn click_with(&mut self, at: Point, mode: ClickMode) {
        if let Phase::Intermission { next_level, .. } = self.phase {
            self.start_level(next_level);
            return;
        }
        let Some(id) = self.base_at(at) else {
            // A ctrl-click that misses keeps the selection being built; other misses clear it.
            if mode != ClickMode::Toggle {
                self.clear_selection();
            }
            return;
        };
        match mode {
            ClickMode::All => {
                let senders: Vec<BaseId> = self.player_bases().filter(|&b| b != id).collect();
                self.clear_selection();
                self.send_all(&senders, id);
            }
            ClickMode::Toggle if self.bases[id].team == Team::Player => {
                if self.selected.contains(&id) {
                    self.deselect(id);
                } else {
                    self.select(id);
                }
            }
            ClickMode::Single | ClickMode::Toggle => {
                if self.selected.is_empty() {
                    if self.bases[id].team == Team::Player {
                        self.select(id);
                    }
                } else if self.selected == [id] {
                    self.clear_selection();
                } else {
                    let senders: Vec<BaseId> =
                        self.selected.iter().copied().filter(|&b| b != id).collect();
                    self.clear_selection();
                    self.send_all(&senders, id);
                }
            }
        }
    }

    /// Selects every player base (keyboard shortcut). No-op outside play.
    pub fn select_all(&mut self) {
        if self.phase != Phase::Playing {
            return;
        }
        let owned: Vec<BaseId> = self.player_bases().collect();
        for id in owned {
            if !self.selected.contains(&id) {
                self.select(id);
            }
        }
    }

    /// Drops the whole selection (keyboard shortcut).
    pub fn clear_selection(&mut self) {
        if !self.selected.is_empty() {
            self.selected.clear();
            self.events.push(GameEvent::SelectionCleared);
        }
    }

    /// Returns and clears events produced since the last call.
    pub fn drain_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.events)
    }

    /// All bases; ids are indices into this slice.
    pub fn bases(&self) -> &[Base] {
        &self.bases
    }

    /// Units currently in flight.
    pub fn units(&self) -> &[Unit] {
        &self.units
    }

    /// Current level (the original's `ai.power`), starting at 1.
    ///
    /// During an intermission this is still the level just played; the level
    /// to be played next is in [`Phase::Intermission`].
    pub fn level(&self) -> u32 {
        self.level
    }

    /// Whether a level is running or the game is between levels.
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// The selected player bases, in the order they were selected.
    pub fn selected(&self) -> &[BaseId] {
        &self.selected
    }

    /// Seconds played in the current level. Stops when the level ends, so
    /// during an intermission it is the time the finished level took.
    pub fn elapsed_seconds(&self) -> f32 {
        self.level_ticks as f32 * TICK_SECONDS
    }

    /// Ids of all bases the player owns.
    pub fn player_bases(&self) -> impl Iterator<Item = BaseId> + '_ {
        self.bases
            .iter()
            .filter(|b| b.team == Team::Player)
            .map(|b| b.id)
    }

    /// The base whose circle contains `at`, if any (used for hover/click).
    pub fn base_at(&self, at: Point) -> Option<BaseId> {
        self.bases
            .iter()
            .find(|b| b.pos.distance(at) < b.radius)
            .map(|b| b.id)
    }

    /// Test-only constructor for hand-built scenarios (level 1, playing).
    #[cfg(test)]
    pub(crate) fn from_bases(bases: Vec<Base>, seed: u64) -> Self {
        Self {
            bases,
            units: Vec::new(),
            level: 1,
            phase: Phase::Playing,
            selected: Vec::new(),
            level_ticks: 0,
            ai_timer: 0.0,
            rng: Rng::new(seed),
            events: Vec::new(),
        }
    }

    /// Begins `level` with fresh bases and no units, selection or timers.
    fn start_level(&mut self, level: u32) {
        self.level = level;
        self.phase = Phase::Playing;
        self.bases = generate_bases(&mut self.rng);
        self.units.clear();
        self.selected.clear();
        self.level_ticks = 0;
        self.ai_timer = 0.0;
        self.events.push(GameEvent::LevelStarted { level });
    }

    fn select(&mut self, id: BaseId) {
        self.selected.push(id);
        self.events.push(GameEvent::BaseSelected { base: id });
    }

    /// Removes one base from the selection; reports `SelectionCleared` when it was the last.
    fn deselect(&mut self, id: BaseId) {
        let before = self.selected.len();
        self.selected.retain(|&b| b != id);
        if self.selected.len() == before {
            return;
        }
        self.events.push(if self.selected.is_empty() {
            GameEvent::SelectionCleared
        } else {
            GameEvent::BaseDeselected { base: id }
        });
    }

    fn send_all(&mut self, senders: &[BaseId], to: BaseId) {
        for &from in senders {
            self.send_units(from, to);
        }
    }

    fn ai_move(&mut self) {
        self.ai_timer += TICK_MS;
        // Re-rolled every tick, as in the original.
        let threshold = self.rng.next_f32() * AI_JITTER_MS + AI_BASE_DELAY_MS / self.level as f32;
        if self.ai_timer > threshold {
            self.ai_timer = 0.0;
            if let Some((from, to)) = ai::choose_attack(&self.bases, &mut self.rng) {
                self.send_units(from, to);
            }
        }
    }

    /// Sends half of `from`'s units (floor) toward `to`.
    fn send_units(&mut self, from: BaseId, to: BaseId) {
        let origin = &self.bases[from];
        let (team, pos, radius) = (origin.team, origin.pos, origin.radius);
        let count = (origin.value / 2.0).floor() as u32;
        if count == 0 {
            return;
        }
        self.bases[from].value -= count as f32;
        let target_pos = self.bases[to].pos;
        for _ in 0..count {
            // Random distance in [radius, radius + count] at a random angle.
            let distance = (self.rng.next_f32() * (count + 1) as f32).floor() + radius;
            let angle = std::f32::consts::TAU * self.rng.next_f32();
            let unit_pos = Point::new(
                distance * angle.sin() + pos.x,
                distance * angle.cos() + pos.y,
            );
            self.units.push(Unit {
                team,
                pos: unit_pos,
                vel: velocity(unit_pos, target_pos),
                target: to,
            });
        }
        self.events.push(GameEvent::UnitsSent {
            from,
            to,
            team,
            count,
        });
    }

    fn move_units(&mut self) {
        for unit in &mut self.units {
            unit.pos.x += unit.vel.x;
            unit.pos.y += unit.vel.y;
        }
    }

    /// Applies every unit touching its target base, then drops those units.
    fn resolve_arrivals(&mut self) {
        let units = std::mem::take(&mut self.units);
        let mut in_flight = Vec::with_capacity(units.len());
        for unit in units {
            let target = &self.bases[unit.target];
            if unit.pos.distance(target.pos) < UNIT_RADIUS + target.radius {
                self.apply_arrival(&unit);
            } else {
                in_flight.push(unit);
            }
        }
        self.units = in_flight;
    }

    fn apply_arrival(&mut self, unit: &Unit) {
        let base = &mut self.bases[unit.target];
        let reinforced = base.team == unit.team;
        base.value += if reinforced { 1.0 } else { -1.0 };
        self.events.push(GameEvent::UnitArrived {
            base: unit.target,
            team: unit.team,
            pos: unit.pos,
            reinforced,
        });
        if base.value <= 0.0 {
            let from = base.team;
            base.team = unit.team;
            base.value = 1.0;
            self.events.push(GameEvent::BaseCaptured {
                base: unit.target,
                from,
                to: unit.team,
            });
            // A captured base must not stay selected, or a later click would
            // send the enemy's units.
            if unit.team != Team::Player {
                self.deselect(unit.target);
            }
        }
    }

    fn grow_bases(&mut self) {
        for base in &mut self.bases {
            if base.team == Team::Neutral || base.is_full() {
                continue;
            }
            let multiplier = match base.team {
                Team::Cpu => 1.0 + CPU_LEVEL_GROWTH * (self.level - 1) as f32,
                _ => 1.0,
            };
            base.value += base.start_value / (GROWTH_TICKS / multiplier);
        }
    }

    fn check_level_end(&mut self) {
        let owns = |team| self.bases.iter().any(|b| b.team == team);
        // If both sides vanish at once, the cpu wins, as in the original.
        let outcome = match (owns(Team::Player), owns(Team::Cpu)) {
            (true, true) => return,
            (true, false) => Outcome::Victory,
            (false, _) => Outcome::Defeat,
        };
        let next_level = match outcome {
            Outcome::Victory => self.level + 1,
            Outcome::Defeat => 1,
        };
        self.clear_selection();
        self.units.clear();
        self.bases.clear();
        self.ai_timer = 0.0;
        self.phase = Phase::Intermission {
            outcome,
            next_level,
        };
        self.events.push(GameEvent::LevelEnded { outcome });
    }
}

/// Velocity of magnitude [`UNIT_SPEED`] pointing from `from` to `to`.
/// Zero if the points coincide, so it is never NaN.
fn velocity(from: Point, to: Point) -> Point {
    let (dx, dy) = (to.x - from.x, to.y - from.y);
    let length = dx.hypot(dy);
    if length > f32::EPSILON {
        Point::new(dx / length * UNIT_SPEED, dy / length * UNIT_SPEED)
    } else {
        Point::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::make_base;

    fn base(id: usize, x: f32, y: f32, team: Team, value: f32) -> Base {
        make_base(id, Point::new(x, y), team, value)
    }

    /// Player at (100,250), cpu at (700,250), neutral at (400,100).
    fn scenario(player: f32, cpu: f32) -> Game {
        Game::from_bases(
            vec![
                base(0, 100.0, 250.0, Team::Player, player),
                base(1, 700.0, 250.0, Team::Cpu, cpu),
                base(2, 400.0, 100.0, Team::Neutral, 20.0),
            ],
            1,
        )
    }

    fn at(game: &Game, id: usize) -> Point {
        game.bases[id].pos
    }

    fn unit_at(game: &Game, team: Team, target: usize) -> Unit {
        Unit {
            team,
            pos: at(game, target),
            vel: Point::default(),
            target,
        }
    }

    #[test]
    fn new_emits_level_started() {
        let mut game = Game::new(1);
        assert_eq!(game.level(), 1);
        assert_eq!(game.phase(), Phase::Playing);
        assert_eq!(
            game.drain_events(),
            vec![GameEvent::LevelStarted { level: 1 }]
        );
        assert!(game.drain_events().is_empty());
    }

    #[test]
    fn same_seed_gives_identical_state() {
        let run = |seed| {
            let mut game = Game::new(seed);
            let mut log = Vec::new();
            for t in 0..4000u32 {
                if t % 45 == 0 {
                    let id = (t / 45) as usize % game.bases().len();
                    game.click(game.bases()[id].pos);
                }
                game.tick();
                log.extend(game.drain_events());
            }
            (
                game.bases().to_vec(),
                game.units().to_vec(),
                game.phase(),
                log,
            )
        };
        assert_eq!(run(99), run(99));
        assert_ne!(run(99).0, run(100).0);
    }

    #[test]
    fn growth_rates_for_player_and_cpu() {
        // Cpu base under 15 never triggers the AI, so only growth is observed.
        for (level, mult) in [(1u32, 1.0f32), (3, 1.2)] {
            let mut game = scenario(20.0, 10.0);
            game.level = level;
            game.tick();
            let player_gain = game.bases[0].value - 20.0;
            let cpu_gain = game.bases[1].value - 10.0;
            assert!((player_gain - 20.0 / 1000.0).abs() < 1e-5);
            assert!((cpu_gain - 10.0 * mult / 1000.0).abs() < 1e-5);
            assert_eq!(game.bases[2].value, 20.0, "neutral never grows");
        }
    }

    #[test]
    fn growth_stops_at_cap() {
        // Cpu cap (14) stays under the AI attack threshold, so it never sends.
        let mut game = scenario(10.0, 7.0);
        for _ in 0..3000 {
            game.tick();
        }
        for b in &game.bases[..2] {
            let step = b.start_value / 1000.0;
            assert!(b.value >= b.max_value() && b.value < b.max_value() + step + 1e-4);
        }
    }

    #[test]
    fn sending_halves_value_and_spawns_units() {
        let mut game = scenario(31.0, 10.0);
        game.drain_events();
        game.click(at(&game, 0));
        game.click(at(&game, 2));
        assert_eq!(game.bases[0].value, 16.0);
        assert_eq!(game.units().len(), 15);
        let origin = &game.bases[0];
        for u in game.units() {
            assert_eq!(u.team, Team::Player);
            assert_eq!(u.target, 2);
            let d = u.pos.distance(origin.pos);
            assert!(d >= origin.radius - 1e-3 && d <= origin.radius + 15.0 + 1e-3);
            assert!((u.vel.x.hypot(u.vel.y) - UNIT_SPEED).abs() < 1e-4);
        }
        assert_eq!(
            game.drain_events(),
            vec![
                GameEvent::BaseSelected { base: 0 },
                GameEvent::SelectionCleared,
                GameEvent::UnitsSent {
                    from: 0,
                    to: 2,
                    team: Team::Player,
                    count: 15
                },
            ]
        );
    }

    #[test]
    fn units_fly_to_the_target_and_arrive() {
        let mut game = scenario(31.0, 10.0);
        game.click(at(&game, 0));
        game.click(at(&game, 2));
        game.drain_events();
        for _ in 0..400 {
            game.tick();
        }
        assert!(game.units().is_empty());
        assert_eq!(game.bases[2].value, 5.0);
    }

    #[test]
    fn arrival_reinforces_damages_and_captures() {
        let mut game = scenario(10.0, 10.0);
        game.units.push(unit_at(&game, Team::Player, 0));
        game.units.push(unit_at(&game, Team::Cpu, 0));
        game.units.push(unit_at(&game, Team::Cpu, 0));
        game.drain_events();
        game.resolve_arrivals();
        // All three were processed in one pass (no remove-while-iterating skips).
        assert!(game.units.is_empty());
        assert_eq!(game.bases[0].value, 9.0);
        let events = game.drain_events();
        assert!(matches!(
            events[0],
            GameEvent::UnitArrived {
                reinforced: true,
                ..
            }
        ));
        assert!(matches!(
            events[1],
            GameEvent::UnitArrived {
                reinforced: false,
                ..
            }
        ));

        game.bases[1].value = 1.0;
        game.units.push(unit_at(&game, Team::Player, 1));
        game.resolve_arrivals();
        assert_eq!(game.bases[1].team, Team::Player);
        assert_eq!(game.bases[1].value, 1.0);
        assert!(game.drain_events().contains(&GameEvent::BaseCaptured {
            base: 1,
            from: Team::Cpu,
            to: Team::Player
        }));
    }

    #[test]
    fn unit_only_hits_its_own_target() {
        let mut game = scenario(10.0, 10.0);
        // Sitting on base 0 but aimed at base 1: no arrival.
        let mut unit = unit_at(&game, Team::Cpu, 1);
        unit.pos = at(&game, 0);
        game.units.push(unit);
        game.resolve_arrivals();
        assert_eq!(game.units.len(), 1);
        assert_eq!(game.bases[0].value, 10.0);
    }

    #[test]
    fn click_state_machine() {
        let mut game = scenario(30.0, 10.0);
        game.drain_events();
        let empty = Point::new(400.0, 400.0);

        // Empty click with nothing selected: no-op, no event.
        game.click(empty);
        assert!(game.selected().is_empty());
        assert!(game.drain_events().is_empty());

        // First click on non-owned bases does nothing.
        game.click(at(&game, 1));
        game.click(at(&game, 2));
        assert!(game.selected().is_empty());
        assert!(game.drain_events().is_empty());

        // First click on own base selects.
        game.click(at(&game, 0));
        assert_eq!(game.selected(), [0]);
        assert_eq!(
            game.drain_events(),
            vec![GameEvent::BaseSelected { base: 0 }]
        );

        // Same base again deselects.
        game.click(at(&game, 0));
        assert!(game.selected().is_empty());
        assert_eq!(game.drain_events(), vec![GameEvent::SelectionCleared]);

        // Empty space deselects.
        game.click(at(&game, 0));
        game.drain_events();
        game.click(empty);
        assert!(game.selected().is_empty());
        assert_eq!(game.drain_events(), vec![GameEvent::SelectionCleared]);
        assert!(game.units().is_empty());

        // Another base sends and deselects.
        game.click(at(&game, 0));
        game.click(at(&game, 2));
        assert!(game.selected().is_empty());
        assert_eq!(game.units().len(), 15);
    }

    #[test]
    fn selecting_a_tiny_base_sends_nothing_but_deselects() {
        let mut game = scenario(1.0, 10.0);
        game.click(at(&game, 0));
        game.click(at(&game, 2));
        assert!(game.units().is_empty());
        assert_eq!(game.bases[0].value, 1.0);
        assert!(game.selected().is_empty());
    }

    #[test]
    fn capture_of_selected_base_clears_selection() {
        let mut game = scenario(1.0, 10.0);
        game.click(at(&game, 0));
        assert_eq!(game.selected(), [0]);
        game.drain_events();
        game.units.push(unit_at(&game, Team::Cpu, 0));
        game.resolve_arrivals();
        assert_eq!(game.bases[0].team, Team::Cpu);
        assert!(game.selected().is_empty());
        assert!(game.drain_events().contains(&GameEvent::SelectionCleared));
        // A later click cannot send from the captured base.
        game.click(at(&game, 0));
        assert!(game.selected().is_empty());
    }

    #[test]
    fn cpu_attacks_on_its_own_timer() {
        let mut game = scenario(10.0, 30.0);
        let mut sent = false;
        for _ in 0..1000 {
            game.tick();
            if game.drain_events().iter().any(|e| {
                matches!(
                    e,
                    GameEvent::UnitsSent {
                        team: Team::Cpu,
                        ..
                    }
                )
            }) {
                sent = true;
                break;
            }
        }
        assert!(sent, "cpu should attack within ~12s at level 1");
    }

    #[test]
    fn victory_advances_the_level() {
        let mut game = scenario(30.0, 1.0);
        game.level = 2;
        game.drain_events();
        game.units.push(unit_at(&game, Team::Player, 1));
        game.tick();
        assert_eq!(
            game.phase(),
            Phase::Intermission {
                outcome: Outcome::Victory,
                next_level: 3
            }
        );
        assert!(game.units().is_empty());
        let events = game.drain_events();
        assert_eq!(
            events.last(),
            Some(&GameEvent::LevelEnded {
                outcome: Outcome::Victory
            })
        );

        // Ticking is inert; a click starts the next level.
        game.tick();
        game.click(Point::new(0.0, 0.0));
        assert_eq!(game.level(), 3);
        assert_eq!(game.phase(), Phase::Playing);
        assert!(game.bases().len() >= 5);
        assert!(game.units().is_empty());
        assert!(game.selected().is_empty());
        assert_eq!(
            game.drain_events(),
            vec![GameEvent::LevelStarted { level: 3 }]
        );
    }

    #[test]
    fn defeat_resets_to_level_one() {
        let mut game = scenario(1.0, 30.0);
        game.level = 4;
        game.units.push(unit_at(&game, Team::Cpu, 0));
        game.tick();
        assert_eq!(
            game.phase(),
            Phase::Intermission {
                outcome: Outcome::Defeat,
                next_level: 1
            }
        );
        game.click(Point::new(10.0, 10.0));
        assert_eq!(game.level(), 1);
        assert_eq!(game.phase(), Phase::Playing);
    }

    #[test]
    fn ending_a_level_clears_selection() {
        let mut game = scenario(30.0, 1.0);
        game.click(at(&game, 0));
        game.drain_events();
        game.units.push(unit_at(&game, Team::Player, 1));
        game.tick();
        assert!(game.selected().is_empty());
        let events = game.drain_events();
        assert!(events.contains(&GameEvent::SelectionCleared));
    }

    #[test]
    fn velocities_are_finite_for_vertical_and_horizontal_targets() {
        for (tx, ty) in [(100.0, 50.0), (100.0, 450.0), (400.0, 250.0), (0.0, 250.0)] {
            let mut game = Game::from_bases(
                vec![
                    base(0, 100.0, 250.0, Team::Player, 30.0),
                    base(1, tx, ty, Team::Neutral, 10.0),
                ],
                5,
            );
            game.click(at(&game, 0));
            game.click(at(&game, 1));
            assert!(!game.units().is_empty());
            for u in game.units() {
                assert!(u.vel.x.is_finite() && u.vel.y.is_finite());
                assert!((u.vel.x.hypot(u.vel.y) - UNIT_SPEED).abs() < 1e-4);
                // Velocity points toward the target.
                let to_target = Point::new(tx - u.pos.x, ty - u.pos.y);
                assert!(u.vel.x * to_target.x + u.vel.y * to_target.y > 0.0);
            }
        }
    }

    #[test]
    fn ai_with_no_player_units_does_not_produce_nan() {
        let mut game = Game::from_bases(
            vec![
                base(0, 100.0, 250.0, Team::Neutral, 10.0),
                base(1, 700.0, 250.0, Team::Cpu, 30.0),
            ],
            3,
        );
        for _ in 0..2000 {
            game.tick();
            for u in game.units() {
                assert!(u.pos.x.is_finite() && u.vel.x.is_finite());
            }
            for b in game.bases() {
                assert!(b.value.is_finite());
            }
        }
    }

    #[test]
    fn base_at_uses_circle_containment_in_order() {
        let game = scenario(30.0, 10.0);
        assert_eq!(game.base_at(Point::new(100.0, 250.0)), Some(0));
        assert_eq!(game.base_at(Point::new(100.0 + 44.0, 250.0)), Some(0));
        assert_eq!(game.base_at(Point::new(100.0 + 46.0, 250.0)), None);
    }

    /// Two player bases (0, 1), a cpu base (2) and a neutral (3).
    fn two_player_bases() -> Game {
        let mut game = Game::from_bases(
            vec![
                base(0, 100.0, 150.0, Team::Player, 20.0),
                base(1, 100.0, 350.0, Team::Player, 10.0),
                base(2, 700.0, 250.0, Team::Cpu, 10.0),
                base(3, 400.0, 250.0, Team::Neutral, 10.0),
            ],
            1,
        );
        game.drain_events();
        game
    }

    fn sent(events: &[GameEvent]) -> Vec<(BaseId, BaseId, u32)> {
        events
            .iter()
            .filter_map(|e| match *e {
                GameEvent::UnitsSent {
                    from, to, count, ..
                } => Some((from, to, count)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn toggle_builds_a_multi_selection_that_sends_together() {
        let mut game = two_player_bases();
        game.click_with(at(&game, 0), ClickMode::Toggle);
        game.click_with(at(&game, 1), ClickMode::Toggle);
        assert_eq!(game.selected(), [0, 1]);
        // Toggle on a non-owned base never selects it; it sends, like a plain click.
        game.click_with(at(&game, 2), ClickMode::Toggle);
        assert_eq!(sent(&game.drain_events()), vec![(0, 2, 10), (1, 2, 5)]);
        assert!(game.selected().is_empty());
        assert_eq!(game.bases[0].value, 10.0);
        assert_eq!(game.bases[1].value, 5.0);
        assert_eq!(game.units().len(), 15);
    }

    #[test]
    fn toggle_removes_and_reports_deselection() {
        let mut game = two_player_bases();
        game.click_with(at(&game, 0), ClickMode::Toggle);
        game.click_with(at(&game, 1), ClickMode::Toggle);
        game.drain_events();
        game.click_with(at(&game, 0), ClickMode::Toggle);
        assert_eq!(game.selected(), [1]);
        assert_eq!(
            game.drain_events(),
            vec![GameEvent::BaseDeselected { base: 0 }]
        );
        game.click_with(at(&game, 1), ClickMode::Toggle);
        assert!(game.selected().is_empty());
        assert_eq!(game.drain_events(), vec![GameEvent::SelectionCleared]);
    }

    #[test]
    fn toggle_click_on_empty_ground_keeps_the_selection() {
        let mut game = two_player_bases();
        game.click_with(at(&game, 0), ClickMode::Toggle);
        game.drain_events();
        game.click_with(Point::new(400.0, 470.0), ClickMode::Toggle);
        assert_eq!(game.selected(), [0]);
        assert!(game.drain_events().is_empty());
    }

    #[test]
    fn plain_click_on_a_selected_base_reinforces_it_from_the_others() {
        let mut game = two_player_bases();
        game.select_all();
        game.drain_events();
        game.click(at(&game, 1));
        assert_eq!(sent(&game.drain_events()), vec![(0, 1, 10)]);
        assert!(game.selected().is_empty());
    }

    #[test]
    fn plain_click_on_own_base_after_toggle_sends() {
        // Ctrl-select one base, then plain-click another own base: that is a send, as before.
        let mut game = two_player_bases();
        game.click_with(at(&game, 0), ClickMode::Toggle);
        game.click(at(&game, 1));
        assert_eq!(sent(&game.drain_events()), vec![(0, 1, 10)]);
    }

    #[test]
    fn all_mode_sends_from_every_player_base_ignoring_selection() {
        let mut game = two_player_bases();
        game.click(at(&game, 1));
        game.click_with(at(&game, 3), ClickMode::All);
        assert_eq!(sent(&game.drain_events()), vec![(0, 3, 10), (1, 3, 5)]);
        assert!(game.selected().is_empty());

        // Targeting an own base pulls from all the others into it.
        let mut game = two_player_bases();
        game.click_with(at(&game, 0), ClickMode::All);
        assert_eq!(sent(&game.drain_events()), vec![(1, 0, 5)]);

        // Empty ground just clears.
        let mut game = two_player_bases();
        game.click(at(&game, 0));
        game.click_with(Point::new(400.0, 470.0), ClickMode::All);
        assert!(game.selected().is_empty());
        assert!(game.units().is_empty());
    }

    #[test]
    fn select_all_picks_only_player_bases_once() {
        let mut game = two_player_bases();
        game.click(at(&game, 1));
        game.drain_events();
        game.select_all();
        assert_eq!(game.selected(), [1, 0]);
        assert_eq!(
            game.drain_events(),
            vec![GameEvent::BaseSelected { base: 0 }]
        );
        game.select_all();
        assert!(game.drain_events().is_empty());
        game.clear_selection();
        assert!(game.selected().is_empty());
    }

    #[test]
    fn capturing_one_of_several_selected_bases_keeps_the_rest() {
        let mut game = two_player_bases();
        game.bases[0].value = 1.0;
        game.select_all();
        game.drain_events();
        game.units.push(unit_at(&game, Team::Cpu, 0));
        game.resolve_arrivals();
        assert_eq!(game.selected(), [1]);
        assert!(game
            .drain_events()
            .contains(&GameEvent::BaseDeselected { base: 0 }));
    }

    #[test]
    fn timer_counts_play_and_freezes_between_levels() {
        let mut game = scenario(30.0, 1.0);
        for _ in 0..120 {
            game.tick();
        }
        assert!((game.elapsed_seconds() - 2.0).abs() < 1e-4);
        game.bases[1].value = 1.0; // undo two seconds of growth so one hit captures
        game.units.push(unit_at(&game, Team::Player, 1));
        game.tick();
        assert!(matches!(game.phase(), Phase::Intermission { .. }));
        let final_time = game.elapsed_seconds();
        assert!((final_time - 121.0 / 60.0).abs() < 1e-4);
        for _ in 0..100 {
            game.tick();
        }
        assert_eq!(game.elapsed_seconds(), final_time);
        game.click(Point::new(0.0, 0.0));
        assert_eq!(game.elapsed_seconds(), 0.0);
    }

    #[test]
    fn full_means_at_or_over_the_cap() {
        let mut game = scenario(10.0, 10.0);
        assert!(!game.bases[0].is_full());
        game.bases[0].value = 20.0;
        assert!(game.bases[0].is_full());
        game.bases[0].value = 23.0; // reinforcements can push past the cap
        assert!(game.bases[0].is_full());
    }
}
