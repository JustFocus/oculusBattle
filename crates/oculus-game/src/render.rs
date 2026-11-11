//! Board drawing in world space (background, bases, units, targeting arrow).
//! The world camera must be active (see `View::camera`).

use crate::assets::Assets;
use crate::effects::Effects;
use crate::view::View;
use macroquad::prelude::*;
use oculus_core::{BaseId, ClickMode, Game, Point, Team, UNIT_RADIUS, WORLD_HEIGHT, WORLD_WIDTH};

pub const BAR_COLOR: Color = Color::new(0.035, 0.035, 0.055, 1.0);
const SELECT_COLOR: Color = Color::new(0.62, 0.86, 1.0, 1.0);
/// On-screen diameter of a unit sprite in world units (collision radius is 5).
const UNIT_SPRITE: f32 = 13.0;

pub fn team_color(team: Team) -> Color {
    match team {
        Team::Player => Color::new(0.22, 0.52, 1.0, 1.0),
        Team::Cpu => Color::new(0.95, 0.22, 0.22, 1.0),
        Team::Neutral => Color::new(0.30, 0.30, 0.33, 1.0),
    }
}

fn alpha(c: Color, a: f32) -> Color {
    Color::new(c.r, c.g, c.b, a)
}

fn toward_white(c: Color, k: f32) -> Color {
    Color::new(
        c.r + (1.0 - c.r) * k,
        c.g + (1.0 - c.g) * k,
        c.b + (1.0 - c.b) * k,
        c.a,
    )
}

/// How much a full base grows at the peak of its pulse (fraction of its radius).
pub const FULL_PULSE_GROWTH: f32 = 0.05;

/// Ring thickness in world units: grows with value like the original, capped.
pub fn ring_thickness(team: Team, value: f32) -> f32 {
    if team == Team::Neutral {
        2.0
    } else {
        ((value / 5.0).floor() + 1.0).min(7.0)
    }
}

fn sides(radius_world: f32, scale: f32) -> u8 {
    ((radius_world * scale * 1.6) as i32).clamp(24, 255) as u8
}

fn disc(c: Vec2, r: f32, scale: f32, color: Color) {
    draw_poly(c.x, c.y, sides(r, scale), r, 0.0, color);
}

/// Ring occupying radii `inner..inner + thickness`.
fn ring(c: Vec2, inner: f32, thickness: f32, scale: f32, color: Color) {
    draw_poly_lines(
        c.x,
        c.y,
        sides(inner + thickness, scale),
        inner,
        0.0,
        thickness,
        color,
    );
}

pub struct BoardState {
    /// Base under the cursor that a click would act on (None when not interactive).
    pub hover: Option<BaseId>,
    /// Bases that would send units if the hovered base were clicked: the selection, or every
    /// player base while the send-all modifier is held.
    pub armed: Vec<BaseId>,
    /// Whether a click on the hovered base would toggle it in the selection instead of sending.
    pub toggling: bool,
    pub time: f64,
}

impl BoardState {
    /// What the cursor would do right now with modifier `mode`.
    pub fn new(game: &Game, world: Vec2, mode: ClickMode, time: f64) -> Self {
        let armed = match mode {
            ClickMode::All => game.player_bases().collect(),
            _ => game.selected().to_vec(),
        };
        let hover = game.base_at(Point::new(world.x, world.y)).filter(|&id| {
            let own = game.bases()[id].team == Team::Player;
            match mode {
                ClickMode::All => armed.iter().any(|&b| b != id),
                ClickMode::Toggle if own => true,
                _ if armed.is_empty() => own,
                // Clicking the only selected base just deselects it.
                _ => armed != [id],
            }
        });
        let toggling = mode == ClickMode::Toggle
            && hover.is_some_and(|id| game.bases()[id].team == Team::Player);
        Self {
            hover,
            armed,
            toggling,
            time,
        }
    }

    /// Nothing hovered or armed (title screen, intermission).
    pub fn idle(time: f64) -> Self {
        Self {
            hover: None,
            armed: Vec::new(),
            toggling: false,
            time,
        }
    }
}

pub fn draw_world(
    assets: &Assets,
    game: &Game,
    fx: &Effects,
    view: &View,
    state: &BoardState,
    dpi: f32,
) {
    draw_background(assets);
    draw_bases(assets, game, fx, view, state, dpi);
    draw_targeting(game, state);
    draw_units(assets, game, dpi * view.scale);
    fx.draw();
}

fn draw_background(assets: &Assets) {
    // tools/artgen renders the map at exactly the board's aspect, so it maps 1:1 onto the world.
    draw_texture_ex(
        &assets.map,
        0.0,
        0.0,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(WORLD_WIDTH, WORLD_HEIGHT)),
            ..Default::default()
        },
    );
    draw_rectangle(
        0.0,
        0.0,
        WORLD_WIDTH,
        WORLD_HEIGHT,
        Color::new(0.04, 0.03, 0.07, 0.58),
    );
    draw_texture_ex(
        &assets.vignette,
        0.0,
        0.0,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(WORLD_WIDTH, WORLD_HEIGHT)),
            ..Default::default()
        },
    );
}

fn draw_bases(
    assets: &Assets,
    game: &Game,
    fx: &Effects,
    view: &View,
    state: &BoardState,
    dpi: f32,
) {
    let s = view.scale;
    let pulse = (state.time * 4.0).sin() as f32 * 0.5 + 0.5;
    for b in game.bases() {
        let c = vec2(b.pos.x, b.pos.y);
        let pop = fx.pop_scale(b.id);
        // A full base breathes (slightly larger, brighter ring, wider glow): it is not growing.
        let full = fx.full_pulse(b.id, state.time);
        let r = b.radius * (1.0 + (pop - 1.0) * 0.25) * (1.0 + FULL_PULSE_GROWTH * full);
        let t = ring_thickness(b.team, b.value);
        let col = toward_white(team_color(b.team), 0.35 * full);

        // Soft drop shadow so the sprite lifts off the map.
        disc(
            c + vec2(0.0, 2.5),
            r + t + 1.0,
            s,
            Color::new(0.0, 0.0, 0.0, 0.35),
        );

        if b.team == Team::Neutral {
            ring(c, r - 0.5, t, s, alpha(col, 0.95));
        } else {
            ring(
                c,
                r - 0.5,
                t + 3.0 + 4.0 * full,
                s,
                alpha(col, 0.22 + 0.18 * full),
            );
            ring(c, r - 0.5, t, s, col);
        }
        let sprite = if b.team == Team::Neutral {
            &assets.base_neutral
        } else {
            &assets.base
        };
        sprite.draw(c, r * 2.0, WHITE, dpi * s);

        if state.hover == Some(b.id) {
            disc(c, r, s, Color::new(1.0, 1.0, 1.0, 0.08));
            ring(c, r + t, 1.6, s, Color::new(1.0, 1.0, 1.0, 0.55));
        }
        if state.armed.contains(&b.id) {
            let a = 0.55 + 0.45 * pulse;
            ring(c, r + t + 3.0 + pulse * 2.0, 2.5, s, alpha(SELECT_COLOR, a));
            ring(
                c,
                r + t + 2.0,
                6.0 + pulse * 2.0,
                s,
                alpha(SELECT_COLOR, 0.10 * a),
            );
        }
    }
}

/// Dashed arrows from every armed base to the hovered target.
fn draw_targeting(game: &Game, state: &BoardState) {
    let Some(hov) = state.hover else {
        return;
    };
    if state.toggling {
        return;
    }
    for &sel in &state.armed {
        draw_arrow(game, state, sel, hov);
    }
}

fn draw_arrow(game: &Game, state: &BoardState, sel: BaseId, hov: BaseId) {
    let (a, b) = (&game.bases()[sel], &game.bases()[hov]);
    if a.value < 2.0 || sel == hov {
        return;
    }
    let (pa, pb) = (vec2(a.pos.x, a.pos.y), vec2(b.pos.x, b.pos.y));
    let dir = (pb - pa).normalize_or_zero();
    let start = pa + dir * (a.radius + ring_thickness(a.team, a.value) + 7.0);
    let tip = pb - dir * (b.radius + ring_thickness(b.team, b.value) + 4.0);
    let len = (tip - start).dot(dir);
    if len < 14.0 {
        return;
    }
    let color = if b.team == Team::Player {
        SELECT_COLOR
    } else {
        Color::new(1.0, 0.62, 0.42, 1.0)
    };
    const HEAD: f32 = 14.0;
    let (dash, gap) = (9.0, 6.0);
    let shaft = len - HEAD + 2.0;
    let phase = (state.time as f32 * 22.0) % (dash + gap);
    let mut d = -phase;
    while d < shaft {
        let (d0, d1) = (d.max(0.0), (d + dash).min(shaft));
        if d1 > d0 {
            let p0 = start + dir * d0;
            let p1 = start + dir * d1;
            draw_line(p0.x, p0.y, p1.x, p1.y, 4.5, Color::new(0.0, 0.0, 0.0, 0.35));
            draw_line(p0.x, p0.y, p1.x, p1.y, 2.5, alpha(color, 0.9));
        }
        d += dash + gap;
    }
    let normal = vec2(-dir.y, dir.x);
    let base = tip - dir * HEAD;
    let tri = [tip, base + normal * 7.0, base - normal * 7.0];
    let grow = |k: f32| {
        [
            tip + dir * k,
            base + normal * (7.0 + k) - dir * k,
            base - normal * (7.0 + k) - dir * k,
        ]
    };
    let o = grow(1.6);
    draw_triangle(o[0], o[1], o[2], Color::new(0.0, 0.0, 0.0, 0.35));
    draw_triangle(tri[0], tri[1], tri[2], alpha(color, 0.95));
}

fn draw_units(assets: &Assets, game: &Game, px_per_unit: f32) {
    for u in game.units() {
        let c = vec2(u.pos.x, u.pos.y);
        let col = team_color(u.team);
        draw_poly(c.x, c.y, 24, UNIT_RADIUS + 4.5, 0.0, alpha(col, 0.20));
        draw_poly(c.x, c.y, 24, UNIT_RADIUS + 2.0, 0.0, alpha(col, 0.45));
        assets.unit.draw(c, UNIT_SPRITE, WHITE, px_per_unit);
    }
}
