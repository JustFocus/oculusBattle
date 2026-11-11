//! Turns core `GameEvent`s and unit motion into particles and short-lived animations.

use crate::particles::{presets, ParticleRenderer, ParticleSim};
use crate::render::team_color;
use macroquad::prelude::*;
use oculus_core::{BaseId, Game, GameEvent, Team, TICK_SECONDS};
use std::collections::HashMap;

/// How long the ownership "pop" on a base lasts, in seconds.
const POP_SECONDS: f32 = 0.55;
/// Time for the "full" pulse to fade in or out when a base reaches or leaves its cap.
const FULL_FADE_SECONDS: f32 = 0.35;
/// One breath of the "full" pulse, in seconds.
const FULL_PERIOD_SECONDS: f64 = 1.6;

pub struct Effects {
    sim: ParticleSim,
    renderer: ParticleRenderer,
    /// Remaining pop animation per base, 1.0 (just captured) down to 0.0.
    pops: HashMap<BaseId, f32>,
    /// How strongly each base shows the "full" pulse, 0.0..=1.0, eased in and out.
    full: HashMap<BaseId, f32>,
}

impl Effects {
    /// Needs a live GL context (creates the particle material).
    pub fn new() -> Self {
        Self {
            sim: ParticleSim::new(),
            renderer: ParticleRenderer::new(),
            pops: HashMap::new(),
            full: HashMap::new(),
        }
    }

    /// Reacts to events drained from `game` (call after every tick and click).
    pub fn handle_events(&mut self, events: &[GameEvent], game: &Game) {
        for ev in events {
            match *ev {
                GameEvent::LevelStarted { .. } => {
                    self.sim.clear();
                    self.pops.clear();
                    self.full.clear();
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

    /// Eases each owned base's "full" pulse toward on (at the growth cap) or off.
    pub fn track_full(&mut self, game: &Game, dt: f32) {
        let step = dt / FULL_FADE_SECONDS;
        for b in game.bases() {
            let target = if b.team != Team::Neutral && b.is_full() {
                1.0
            } else {
                0.0
            };
            let e = self.full.entry(b.id).or_insert(0.0);
            *e = if target > *e {
                (*e + step).min(target)
            } else {
                (*e - step).max(target)
            };
        }
    }

    /// 0.0..=1.0: how far a full base is into its current breath (0 when not full).
    pub fn full_pulse(&self, base: BaseId, time: f64) -> f32 {
        let env = self.full.get(&base).copied().unwrap_or(0.0);
        if env == 0.0 {
            return 0.0;
        }
        let wave = 0.5 - 0.5 * (time * std::f64::consts::TAU / FULL_PERIOD_SECONDS).cos();
        env * wave as f32
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
