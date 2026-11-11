//! Particle effects: a pure simulation core, a thin draw layer and game presets.
//!
//! Renderer usage (all positions are WORLD coordinates, 800x500; the caller sets the camera):
//!
//! ```ignore
//! let renderer = ParticleRenderer::new();          // once, after the window exists
//! let mut sim = ParticleSim::new();                // once
//! // per frame, with the real frame time:
//! presets::unit_trail(&mut sim, unit_pos, unit_vel_px_per_s, team_color, dt);
//! sim.update(dt);
//! renderer.draw(&sim);                             // inside the world camera
//! ```
//!
//! Layers: [`ParticleSim`] has no GL calls and is unit tested. [`ParticleRenderer`] owns the
//! glow texture and the additive material. [`presets`] spawn tuned effects into a sim and use
//! `macroquad::rand` (visual only; not deterministic).

use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation};
use macroquad::prelude::*;
use std::collections::VecDeque;

/// Hard cap on live particles. When full, the oldest particle is recycled.
pub const MAX_PARTICLES: usize = 4096;

/// How a particle is composited.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blend {
    /// Normal alpha blending (dark smoke, soft shadows).
    Alpha,
    /// Additive glow (`src.rgb * src.a + dst`). Looks like light; only brightens.
    Additive,
}

/// Everything needed to spawn one particle. Colors are straight (non-premultiplied) RGBA and
/// are linearly interpolated from `color_start` to `color_end` over the particle's life.
#[derive(Clone, Copy, Debug)]
pub struct ParticleSpec {
    pub pos: Vec2,
    /// World units per second.
    pub vel: Vec2,
    /// Constant acceleration in world units per second squared (negative y is up).
    pub gravity: Vec2,
    /// Exponential velocity damping per second: `v *= exp(-drag * dt)`. 0 = none.
    pub drag: f32,
    /// Lifetime in seconds (must be > 0).
    pub life: f32,
    /// Sprite diameter in world units at birth and at death.
    pub size_start: f32,
    pub size_end: f32,
    pub color_start: Color,
    pub color_end: Color,
    pub blend: Blend,
}

impl Default for ParticleSpec {
    fn default() -> Self {
        Self {
            pos: Vec2::ZERO,
            vel: Vec2::ZERO,
            gravity: Vec2::ZERO,
            drag: 0.0,
            life: 1.0,
            size_start: 8.0,
            size_end: 0.0,
            color_start: WHITE,
            color_end: Color::new(1.0, 1.0, 1.0, 0.0),
            blend: Blend::Additive,
        }
    }
}

/// A live particle. Read via [`ParticleSim::particles`].
#[derive(Clone, Copy, Debug)]
pub struct Particle {
    pub pos: Vec2,
    pub vel: Vec2,
    spec_gravity: Vec2,
    drag: f32,
    age: f32,
    life: f32,
    size_start: f32,
    size_end: f32,
    color_start: Color,
    color_end: Color,
    pub blend: Blend,
}

impl Particle {
    /// Normalized age in `[0, 1]`.
    pub fn t(&self) -> f32 {
        (self.age / self.life).clamp(0.0, 1.0)
    }
    /// Current sprite diameter.
    pub fn size(&self) -> f32 {
        self.size_start + (self.size_end - self.size_start) * self.t()
    }
    /// Current color (including alpha).
    pub fn color(&self) -> Color {
        lerp_color(self.color_start, self.color_end, self.t())
    }
}

fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

/// Pooled particle simulation. No GL, no randomness: fully unit testable.
pub struct ParticleSim {
    // Ordered oldest (front) to newest (back).
    particles: VecDeque<Particle>,
    capacity: usize,
}

impl Default for ParticleSim {
    fn default() -> Self {
        Self::new()
    }
}

impl ParticleSim {
    pub fn new() -> Self {
        Self::with_capacity(MAX_PARTICLES)
    }

    /// Simulation with a custom hard cap (mostly for tests).
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            particles: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// Spawns a particle; if the pool is full the oldest particle is dropped.
    pub fn spawn(&mut self, s: ParticleSpec) {
        if self.capacity == 0 || s.life <= 0.0 {
            return;
        }
        if self.particles.len() >= self.capacity {
            self.particles.pop_front();
        }
        self.particles.push_back(Particle {
            pos: s.pos,
            vel: s.vel,
            spec_gravity: s.gravity,
            drag: s.drag,
            age: 0.0,
            life: s.life,
            size_start: s.size_start,
            size_end: s.size_end,
            color_start: s.color_start,
            color_end: s.color_end,
            blend: s.blend,
        });
    }

    /// Advances all particles by `dt` seconds and removes expired ones.
    pub fn update(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        self.particles.retain_mut(|p| {
            p.age += dt;
            if p.age >= p.life {
                return false;
            }
            p.vel += p.spec_gravity * dt;
            if p.drag > 0.0 {
                p.vel *= (-p.drag * dt).exp();
            }
            p.pos += p.vel * dt;
            true
        });
    }

    pub fn len(&self) -> usize {
        self.particles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.particles.is_empty()
    }

    pub fn clear(&mut self) {
        self.particles.clear();
    }

    /// Live particles, oldest first.
    pub fn particles(&self) -> impl Iterator<Item = &Particle> {
        self.particles.iter()
    }
}

/// Accumulates fractional emissions so a rate in particles/second yields the same total
/// count regardless of frame rate. Keep one per continuous emitter.
#[derive(Clone, Copy, Debug, Default)]
pub struct RateAccumulator {
    carry: f32,
}

impl RateAccumulator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns how many whole particles to emit this frame.
    pub fn step(&mut self, rate_per_sec: f32, dt: f32) -> u32 {
        self.carry += rate_per_sec.max(0.0) * dt.max(0.0);
        let n = self.carry.floor();
        self.carry -= n;
        n as u32
    }
}

// ---------------------------------------------------------------------------------------------
// Drawing
// ---------------------------------------------------------------------------------------------

const TEX_SIZE: u16 = 64;

const VERTEX: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;

varying lowp vec2 uv;
varying lowp vec4 color;

uniform mat4 Model;
uniform mat4 Projection;

void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    color = color0 / 255.0;
    uv = texcoord;
}"#;

const FRAGMENT: &str = r#"#version 100
varying lowp vec4 color;
varying lowp vec2 uv;

uniform sampler2D Texture;

void main() {
    gl_FragColor = color * texture2D(Texture, uv);
}"#;

/// Soft radial falloff in the alpha channel, white RGB (so bilinear filtering has no dark fringe).
fn glow_pixels() -> Vec<u8> {
    let n = TEX_SIZE as usize;
    let mut px = Vec::with_capacity(n * n * 4);
    for y in 0..n {
        for x in 0..n {
            let dx = (x as f32 + 0.5) / n as f32 * 2.0 - 1.0;
            let dy = (y as f32 + 0.5) / n as f32 * 2.0 - 1.0;
            let r = (dx * dx + dy * dy).sqrt().min(1.0);
            let s = 1.0 - r;
            // Smooth, gaussian-like: bright core, long soft tail, exactly 0 at the edge.
            let a = s * s * (3.0 - 2.0 * s);
            let a = a * a.sqrt();
            px.extend_from_slice(&[255, 255, 255, (a * 255.0).round() as u8]);
        }
    }
    px
}

/// Owns the GPU resources (glow texture + additive material). Create once after the window
/// exists, then call [`ParticleRenderer::draw`] each frame inside the world camera.
pub struct ParticleRenderer {
    texture: Texture2D,
    additive: Material,
}

impl ParticleRenderer {
    /// Panics if the material fails to compile (a programmer error, same as macroquad's own
    /// default shader).
    pub fn new() -> Self {
        let texture = Texture2D::from_rgba8(TEX_SIZE, TEX_SIZE, &glow_pixels());
        texture.set_filter(FilterMode::Linear);
        let additive = load_material(
            ShaderSource::Glsl {
                vertex: VERTEX,
                fragment: FRAGMENT,
            },
            MaterialParams {
                pipeline_params: PipelineParams {
                    color_blend: Some(BlendState::new(
                        Equation::Add,
                        BlendFactor::Value(BlendValue::SourceAlpha),
                        BlendFactor::One,
                    )),
                    // Leave destination alpha untouched.
                    alpha_blend: Some(BlendState::new(
                        Equation::Add,
                        BlendFactor::Zero,
                        BlendFactor::One,
                    )),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .expect("particle additive material");
        Self { texture, additive }
    }

    /// Draws all particles: alpha-blended ones first, then additive ones in a single
    /// material batch. Restores the default material afterwards.
    pub fn draw(&self, sim: &ParticleSim) {
        self.draw_pass(sim, Blend::Alpha);
        gl_use_material(&self.additive);
        self.draw_pass(sim, Blend::Additive);
        gl_use_default_material();
    }

    fn draw_pass(&self, sim: &ParticleSim, blend: Blend) {
        for p in sim.particles().filter(|p| p.blend == blend) {
            let size = p.size();
            if size <= 0.0 {
                continue;
            }
            draw_texture_ex(
                &self.texture,
                p.pos.x - size * 0.5,
                p.pos.y - size * 0.5,
                p.color(),
                DrawTextureParams {
                    dest_size: Some(vec2(size, size)),
                    ..Default::default()
                },
            );
        }
    }
}

impl Default for ParticleRenderer {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------------------------
// Presets
// ---------------------------------------------------------------------------------------------

/// Game effect presets. All take world positions and an RGB team color (alpha ignored).
pub mod presets {
    use super::*;
    use macroquad::rand::gen_range;

    fn rnd(lo: f32, hi: f32) -> f32 {
        gen_range(lo, hi)
    }

    fn mix_white(c: Color, t: f32) -> Color {
        lerp_color(Color::new(c.r, c.g, c.b, 1.0), WHITE, t)
    }

    fn with_alpha(c: Color, a: f32) -> Color {
        Color::new(c.r, c.g, c.b, a)
    }

    /// Spawns a two-layer glow: an alpha-blended, saturated team-colored halo (reads on the light
    /// parchment, where pure additive light washes out) and, if `core` is true, a smaller
    /// additive near-white core that pops on dark backgrounds. `s.color_*` carry the team
    /// color with the desired halo alpha; the core reuses them lightened.
    fn glow(sim: &mut ParticleSim, s: ParticleSpec, team: Color, core: bool) {
        sim.spawn(ParticleSpec {
            color_start: lerp_color(s.color_start, with_alpha(team, s.color_start.a), 0.75),
            blend: Blend::Alpha,
            ..s
        });
        if core {
            sim.spawn(ParticleSpec {
                size_start: s.size_start * 0.55,
                size_end: s.size_end * 0.55,
                color_start: with_alpha(mix_white(s.color_start, 0.55), s.color_start.a * 0.9),
                color_end: with_alpha(s.color_end, 0.0),
                blend: Blend::Additive,
                ..s
            });
        }
    }

    fn dir(angle: f32) -> Vec2 {
        vec2(angle.cos(), angle.sin())
    }

    /// Subtle short-lived glow behind a moving unit. Call every frame for every
    /// moving unit with the real frame time `dt`; emission is stochastic (about 30 per second)
    /// so no per-unit state is needed and the density is frame-rate independent on average.
    /// `vel` is the unit's velocity in world units per second.
    pub fn unit_trail(sim: &mut ParticleSim, pos: Vec2, vel: Vec2, color: Color, dt: f32) {
        const RATE: f32 = 30.0;
        let expect = RATE * dt.max(0.0);
        let mut n = expect.floor() as u32;
        if rnd(0.0, 1.0) < expect.fract() {
            n += 1;
        }
        let back = -vel.normalize_or_zero();
        for _ in 0..n {
            // Spawn somewhere along the path travelled this frame, slightly behind the unit.
            let along = rnd(0.0, 1.0);
            let p = pos - vel * dt * along
                + back * 2.0
                + dir(rnd(0.0, std::f32::consts::TAU)) * rnd(0.0, 1.5);
            glow(
                sim,
                ParticleSpec {
                    pos: p,
                    vel: back * rnd(4.0, 14.0)
                        + dir(rnd(0.0, std::f32::consts::TAU)) * rnd(0.0, 5.0),
                    drag: 2.0,
                    life: rnd(0.28, 0.5),
                    size_start: rnd(9.0, 12.0),
                    size_end: 2.0,
                    color_start: with_alpha(color, 0.5),
                    color_end: with_alpha(color, 0.0),
                    ..Default::default()
                },
                color,
                rnd(0.0, 1.0) < 0.35,
            );
        }
    }

    /// Small burst of sparks when a unit hits a base. `impact_dir` is the direction the unit
    /// was travelling (any length); sparks splash back against it. `pos` is the contact point.
    pub fn hit_sparks(sim: &mut ParticleSim, pos: Vec2, impact_dir: Vec2, color: Color) {
        let back = (-impact_dir).normalize_or_zero();
        let base_angle = if back == Vec2::ZERO {
            rnd(0.0, std::f32::consts::TAU)
        } else {
            back.y.atan2(back.x)
        };
        let spread = if back == Vec2::ZERO {
            std::f32::consts::PI
        } else {
            1.1
        };
        // Brief soft flash.
        glow(
            sim,
            ParticleSpec {
                pos,
                life: 0.18,
                size_start: 20.0,
                size_end: 6.0,
                color_start: with_alpha(mix_white(color, 0.5), 0.55),
                color_end: with_alpha(color, 0.0),
                ..Default::default()
            },
            color,
            true,
        );
        for _ in 0..9 {
            let a = base_angle + rnd(-spread, spread);
            glow(
                sim,
                ParticleSpec {
                    pos,
                    vel: dir(a) * rnd(50.0, 130.0),
                    drag: 3.5,
                    life: rnd(0.3, 0.55),
                    size_start: rnd(8.0, 12.0),
                    size_end: 1.0,
                    color_start: with_alpha(mix_white(color, 0.6), 0.9),
                    color_end: with_alpha(color, 0.0),
                    ..Default::default()
                },
                color,
                true,
            );
        }
    }

    /// Celebratory effect when a base of world radius `radius` changes team: a flash, an
    /// expanding ring of glow and slowly rising motes. `color` is the new owner's color.
    pub fn capture_burst(sim: &mut ParticleSim, pos: Vec2, radius: f32, color: Color) {
        // Central flash.
        glow(
            sim,
            ParticleSpec {
                pos,
                life: 0.45,
                size_start: radius * 2.4,
                size_end: radius * 3.4,
                color_start: with_alpha(mix_white(color, 0.45), 0.6),
                color_end: with_alpha(color, 0.0),
                ..Default::default()
            },
            color,
            true,
        );
        // Expanding ring: starts on the rim and drifts out ~0.6 * radius over its life.
        let n = ((radius * 0.9) as usize).clamp(28, 64);
        let phase = rnd(0.0, std::f32::consts::TAU);
        for i in 0..n {
            let a = phase + i as f32 / n as f32 * std::f32::consts::TAU;
            let d = dir(a);
            let life = 0.75;
            let drag = 3.0;
            // Total travel with drag is v/drag; pick v so it travels 0.6 * radius.
            let v = radius * 0.6 * drag;
            glow(
                sim,
                ParticleSpec {
                    pos: pos + d * radius * 0.95,
                    vel: d * v * rnd(0.9, 1.1),
                    drag,
                    life,
                    size_start: 11.0,
                    size_end: 3.0,
                    color_start: with_alpha(mix_white(color, 0.35), 0.8),
                    color_end: with_alpha(color, 0.0),
                    ..Default::default()
                },
                color,
                true,
            );
        }
        // Rising motes inside the base.
        for _ in 0..14 {
            let off = dir(rnd(0.0, std::f32::consts::TAU)) * rnd(0.0, radius * 0.9);
            glow(
                sim,
                ParticleSpec {
                    pos: pos + off,
                    vel: vec2(rnd(-6.0, 6.0), rnd(-22.0, -10.0)),
                    drag: 0.8,
                    life: rnd(0.9, 1.5),
                    size_start: rnd(6.0, 10.0),
                    size_end: 1.0,
                    color_start: with_alpha(mix_white(color, 0.7), 0.85),
                    color_end: with_alpha(color, 0.0),
                    ..Default::default()
                },
                color,
                true,
            );
        }
    }

    /// Soft, small glint for a friendly unit arriving at its own base.
    pub fn reinforce_glint(sim: &mut ParticleSim, pos: Vec2, color: Color) {
        glow(
            sim,
            ParticleSpec {
                pos,
                life: 0.3,
                size_start: 20.0,
                size_end: 4.0,
                color_start: with_alpha(mix_white(color, 0.4), 0.45),
                color_end: with_alpha(color, 0.0),
                ..Default::default()
            },
            color,
            true,
        );
        for _ in 0..3 {
            glow(
                sim,
                ParticleSpec {
                    pos: pos + dir(rnd(0.0, std::f32::consts::TAU)) * rnd(0.0, 3.0),
                    vel: vec2(rnd(-8.0, 8.0), rnd(-26.0, -12.0)),
                    drag: 1.5,
                    life: rnd(0.4, 0.7),
                    size_start: rnd(6.0, 9.0),
                    size_end: 1.0,
                    color_start: with_alpha(mix_white(color, 0.7), 0.7),
                    color_end: with_alpha(color, 0.0),
                    ..Default::default()
                },
                color,
                true,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(life: f32) -> ParticleSpec {
        ParticleSpec {
            life,
            ..Default::default()
        }
    }

    #[test]
    fn expires_after_lifetime() {
        let mut sim = ParticleSim::new();
        sim.spawn(spec(0.5));
        sim.update(0.4);
        assert_eq!(sim.len(), 1);
        sim.update(0.2);
        assert!(sim.is_empty());
    }

    #[test]
    fn cap_recycles_oldest() {
        let mut sim = ParticleSim::with_capacity(3);
        for i in 0..5 {
            sim.spawn(ParticleSpec {
                pos: vec2(i as f32, 0.0),
                ..spec(10.0)
            });
        }
        assert_eq!(sim.len(), 3);
        let xs: Vec<f32> = sim.particles().map(|p| p.pos.x).collect();
        assert_eq!(xs, vec![2.0, 3.0, 4.0]);
    }

    #[test]
    fn default_cap_is_hard() {
        let mut sim = ParticleSim::new();
        for _ in 0..MAX_PARTICLES + 100 {
            sim.spawn(spec(5.0));
        }
        assert_eq!(sim.len(), MAX_PARTICLES);
    }

    #[test]
    fn interpolation_endpoints() {
        let mut sim = ParticleSim::new();
        sim.spawn(ParticleSpec {
            size_start: 10.0,
            size_end: 2.0,
            color_start: Color::new(1.0, 0.0, 0.0, 1.0),
            color_end: Color::new(0.0, 0.0, 1.0, 0.0),
            ..spec(1.0)
        });
        let p = *sim.particles().next().unwrap();
        assert_eq!(p.size(), 10.0);
        assert_eq!(p.color().r, 1.0);
        sim.update(0.5);
        let p = *sim.particles().next().unwrap();
        assert!((p.size() - 6.0).abs() < 1e-5);
        assert!((p.color().a - 0.5).abs() < 1e-5);
        sim.update(0.4999);
        let p = *sim.particles().next().unwrap();
        assert!((p.size() - 2.0).abs() < 0.01);
        assert!(p.color().a < 0.01);
    }

    #[test]
    fn drag_slows_and_gravity_accelerates() {
        let mut sim = ParticleSim::new();
        sim.spawn(ParticleSpec {
            vel: vec2(100.0, 0.0),
            drag: 2.0,
            ..spec(10.0)
        });
        sim.spawn(ParticleSpec {
            gravity: vec2(0.0, 10.0),
            ..spec(10.0)
        });
        for _ in 0..60 {
            sim.update(1.0 / 60.0);
        }
        let ps: Vec<Particle> = sim.particles().copied().collect();
        // v = 100 * exp(-2) after 1 s.
        assert!((ps[0].vel.x - 100.0 * (-2.0f32).exp()).abs() < 0.1);
        assert!((ps[1].vel.y - 10.0).abs() < 0.01);
        assert!(ps[1].pos.y > 4.0 && ps[1].pos.y < 6.0);
    }

    #[test]
    fn drag_is_frame_rate_independent() {
        let run = |steps: u32| {
            let mut sim = ParticleSim::new();
            sim.spawn(ParticleSpec {
                vel: vec2(100.0, 0.0),
                drag: 3.0,
                ..spec(10.0)
            });
            for _ in 0..steps {
                sim.update(1.0 / steps as f32);
            }
            let x = sim.particles().next().unwrap().vel.x;
            x
        };
        assert!((run(30) - run(240)).abs() < 1e-3);
    }

    #[test]
    fn rate_accumulation_is_frame_rate_independent() {
        for fps in [24.0f32, 30.0, 60.0, 144.0, 240.0] {
            let mut acc = RateAccumulator::new();
            let frames = fps as u32 * 5;
            let total: u32 = (0..frames).map(|_| acc.step(37.0, 1.0 / fps)).sum();
            // 37/s for 5 s = 185, within one particle of carry.
            assert!((184..=185).contains(&total), "fps {fps}: {total}");
        }
    }

    #[test]
    fn presets_spawn_and_stay_bounded() {
        let mut sim = ParticleSim::new();
        let c = Color::new(0.2, 0.4, 1.0, 1.0);
        presets::hit_sparks(&mut sim, vec2(10.0, 10.0), vec2(1.0, 0.0), c);
        presets::hit_sparks(&mut sim, vec2(10.0, 10.0), Vec2::ZERO, c);
        presets::capture_burst(&mut sim, vec2(100.0, 100.0), 45.0, c);
        presets::reinforce_glint(&mut sim, vec2(5.0, 5.0), c);
        assert!(sim.len() > 20);
        for _ in 0..200 {
            presets::unit_trail(&mut sim, vec2(1.0, 1.0), vec2(180.0, 0.0), c, 1.0 / 60.0);
            sim.update(1.0 / 60.0);
        }
        assert!(sim.len() <= MAX_PARTICLES);
        assert!(!sim.is_empty());
    }

    #[test]
    fn glow_texture_is_soft() {
        let px = glow_pixels();
        let n = TEX_SIZE as usize;
        assert_eq!(px.len(), n * n * 4);
        let a = |x: usize, y: usize| px[(y * n + x) * 4 + 3];
        assert_eq!(a(0, 0), 0);
        assert!(a(n / 2, n / 2) > 240);
    }
}
