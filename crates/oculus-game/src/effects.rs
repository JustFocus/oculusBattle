//! Turns core `GameEvent`s and unit motion into particles and short-lived animations.

use crate::particles::{presets, ParticleRenderer, ParticleSim};
use crate::render::team_color;
use macroquad::prelude::*;
use oculus_core::{BaseId, Game, GameEvent, TICK_SECONDS};
use std::collections::HashMap;

/// How long the ownership "pop" on a base lasts, in seconds.
const POP_SECONDS: f32 = 0.55;

pub struct Effects {
    sim: ParticleSim,
    renderer: ParticleRenderer,
    /// Remaining pop animation per base, 1.0 (just captured) down to 0.0.
    pops: HashMap<BaseId, f32>,
}

impl Effects {
    /// Needs a live GL context (creates the particle material).
    pub fn new() -> Self {
        Self {
            sim: ParticleSim::new(),
            renderer: ParticleRenderer::new(),
            pops: HashMap::new(),
        }
    }

    /// Reacts to events drained from `game` (call after every tick and click).
    pub fn handle_events(&mut self, events: &[GameEvent], game: &Game) {
        for ev in events {
            match *ev {
                GameEvent::LevelStarted { .. } => {
                    self.sim.clear();
                    self.pops.clear();
                }
                GameEvent::UnitArrived {
                    base,
                    team,
                    pos,
                    reinforced,
                } => {
                    let p = vec2(pos.x, pos.y);
                    let color = team_color(team);
                    if reinforced {
                        presets::reinforce_glint(&mut self.sim, p, color);
                    } else if let Some(b) = game.bases().get(base) {
                        let toward_base = vec2(b.pos.x, b.pos.y) - p;
                        presets::hit_sparks(&mut self.sim, p, toward_base, color);
                    }
                }
                GameEvent::BaseCaptured { base, to, .. } => {
                    if let Some(b) = game.bases().get(base) {
                        presets::capture_burst(
                            &mut self.sim,
                            vec2(b.pos.x, b.pos.y),
                            b.radius,
                            team_color(to),
                        );
                        self.pops.insert(base, 1.0);
                    }
                }
                _ => {}
            }
        }
    }

    /// Emits a trail behind every unit in flight. `dt` is the real frame time.
    pub fn trails(&mut self, game: &Game, dt: f32) {
        for u in game.units() {
            // Core velocities are per tick; particles want per second.
            let vel = vec2(u.vel.x, u.vel.y) / TICK_SECONDS;
            presets::unit_trail(
                &mut self.sim,
                vec2(u.pos.x, u.pos.y),
                vel,
                team_color(u.team),
                dt,
            );
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.sim.update(dt);
        self.pops.retain(|_, t| {
            *t -= dt / POP_SECONDS;
            *t > 0.0
        });
    }

    /// Scale multiplier (>= 1) for a base's value text and sprite after a capture.
    pub fn pop_scale(&self, base: BaseId) -> f32 {
        let t = self.pops.get(&base).copied().unwrap_or(0.0);
        1.0 + 0.4 * t * t
    }

    /// Draws particles in world space; the world camera must be active.
    pub fn draw(&self) {
        self.renderer.draw(&self.sim);
    }
}
