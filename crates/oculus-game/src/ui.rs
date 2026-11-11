//! Screen-space UI: crisp text, HUD strip, title and intermission overlays, base values.
//!
//! All text is drawn with the default (pixel-perfect) camera, so macroquad rasterizes glyphs at
//! the real device pixel size instead of scaling a small bitmap.

use crate::assets::Assets;
use crate::effects::Effects;
use crate::render::{team_color, BAR_COLOR};
use crate::view::View;
use macroquad::prelude::*;
use oculus_core::{Game, Outcome, Phase, Team};
use std::cell::RefCell;
use std::collections::HashSet;

/// Height of the HUD strip above the board, in logical pixels.
pub const HUD_HEIGHT: f32 = 36.0;

const PARCHMENT: Color = Color::new(0.96, 0.92, 0.82, 1.0);
const OUTLINE: Color = Color::new(0.03, 0.02, 0.05, 0.92);

thread_local! {
    static CACHED: RefCell<HashSet<(char, u16)>> = RefCell::new(HashSet::new());
}

/// Makes sure every glyph of `text` is in the font atlas before anything is drawn with it.
/// When the atlas has to grow, macroquad deletes the old atlas texture immediately, which
/// would invalidate draw calls already batched this frame. Flushing the batch first (switching
/// to the default camera does that) avoids the "deleted texture" GL error.
fn ensure_glyphs(font: &Font, text: &str, size: u16) {
    let phys = (size as f32 * screen_dpi_scale()).ceil() as u16;
    let missing: Vec<char> = CACHED.with(|c| {
        let mut c = c.borrow_mut();
        text.chars().filter(|&ch| c.insert((ch, phys))).collect()
    });
    if !missing.is_empty() {
        set_default_camera();
        font.populate_font_cache(&missing, phys);
        // Macroquad only swaps in a regrown atlas texture on the next text draw. Do that now
        // (fully transparent, off screen) while the batch is empty.
        draw_text_ex(
            missing[0].to_string(),
            -1000.0,
            -1000.0,
            TextParams {
                font: Some(font),
                font_size: size,
                color: Color::new(0.0, 0.0, 0.0, 0.0),
                ..Default::default()
            },
        );
        set_default_camera();
    }
}

/// Rounds a logical coordinate to a whole physical pixel so glyph edges stay sharp.
fn snap(v: f32, dpi: f32) -> f32 {
    (v * dpi).round() / dpi
}

pub struct TextStyle {
    pub size: f32,
    pub color: Color,
    pub outline: bool,
    /// Extra multiplier applied through font_scale (used for the capture pop).
    pub scale: f32,
}

impl TextStyle {
    pub fn new(size: f32, color: Color) -> Self {
        Self {
            size,
            color,
            outline: true,
            scale: 1.0,
        }
    }
}

/// Largest size <= `size` at which `text` fits in `max_w`.
fn fit_size(font: &Font, text: &str, size: f32, max_w: f32) -> f32 {
    let px = size.round().max(1.0) as u16;
    ensure_glyphs(font, text, px);
    let w = measure_text(text, Some(font), px, 1.0).width;
    if w > max_w && w > 0.0 {
        (size * max_w / w).floor().max(8.0)
    } else {
        size
    }
}

/// Draws `text` centered on `(cx, cy)` (cy is the vertical middle of the glyph box).
pub fn draw_centered(font: &Font, text: &str, cx: f32, cy: f32, style: &TextStyle) {
    let dpi = screen_dpi_scale();
    let size = style.size.round().max(1.0) as u16;
    ensure_glyphs(font, text, size);
    let dims = measure_text(text, Some(font), size, 1.0);
    let w = dims.width * style.scale;
    let h = dims.height * style.scale;
    let x = snap(cx - w * 0.5, dpi);
    let y = snap(cy - h * 0.5 + dims.offset_y * style.scale, dpi);
    let draw = |dx: f32, dy: f32, color: Color| {
        draw_text_ex(
            text,
            x + dx,
            y + dy,
            TextParams {
                font: Some(font),
                font_size: size,
                font_scale: style.scale,
                color,
                ..Default::default()
            },
        );
    };
    if style.outline {
        // Fade the outline with the fill so pulsing text does not turn into a gray fill inside
        // an opaque dark halo.
        let fade = style.color.a;
        let outline = alpha(OUTLINE, fade);
        let o = (style.size / 14.0).max(1.0);
        let d = o * std::f32::consts::FRAC_1_SQRT_2;
        draw(0.0, o * 0.6, alpha(outline, 0.5));
        for (dx, dy) in [
            (o, 0.0),
            (-o, 0.0),
            (0.0, o),
            (0.0, -o),
            (d, d),
            (-d, d),
            (d, -d),
            (-d, -d),
        ] {
            draw(dx, dy, outline);
        }
    }
    draw(0.0, 0.0, style.color);
}

fn alpha(c: Color, a: f32) -> Color {
    Color::new(c.r, c.g, c.b, c.a * a)
}

/// Paints the dark bars around the board (covering any world-space spill) and the HUD strip.
pub fn draw_bars(view: &View) {
    let b = view.board_rect();
    let (sw, sh) = (view.screen.x, view.screen.y);
    draw_rectangle(0.0, 0.0, sw, b.y, BAR_COLOR);
    draw_rectangle(0.0, b.bottom(), sw, (sh - b.bottom()).max(0.0), BAR_COLOR);
    draw_rectangle(0.0, b.y, b.x, b.h, BAR_COLOR);
    draw_rectangle(b.right(), b.y, (sw - b.right()).max(0.0), b.h, BAR_COLOR);
}

/// Rounded base values, centered on each base.
pub fn draw_base_values(assets: &Assets, game: &Game, fx: &Effects, view: &View) {
    for b in game.bases() {
        let c = view.to_screen(vec2(b.pos.x, b.pos.y));
        let size = (b.radius * view.scale * 0.72).max(11.0);
        let mut style = TextStyle::new(size, WHITE);
        style.scale = fx.pop_scale(b.id);
        draw_centered(
            &assets.font,
            &format!("{}", b.value.round() as i32),
            c.x,
            c.y,
            &style,
        );
    }
}

fn team_totals(game: &Game, team: Team) -> u32 {
    let in_bases: f32 = game
        .bases()
        .iter()
        .filter(|b| b.team == team)
        .map(|b| b.value)
        .sum();
    let flying = game.units().iter().filter(|u| u.team == team).count() as f32;
    (in_bases + flying).round() as u32
}

pub fn draw_hud(assets: &Assets, game: &Game, view: &View) {
    let font = &assets.font;
    let cy = HUD_HEIGHT * 0.5;
    let mut level = TextStyle::new(22.0, PARCHMENT);
    level.outline = false;
    draw_centered(
        font,
        &format!("Level {}", game.level()),
        view.screen.x * 0.5,
        cy,
        &level,
    );

    let pad = 12.0;
    let left = view.board_rect().x.max(0.0) + pad;
    let right = view.board_rect().right().min(view.screen.x) - pad;
    let dpi = screen_dpi_scale();
    for (team, label, align_right) in [(Team::Player, "Blue", false), (Team::Cpu, "Red", true)] {
        let text = format!("{label} {}", team_totals(game, team));
        let mut style = TextStyle::new(16.0, team_color(team).lighten_for_text());
        style.outline = false;
        ensure_glyphs(font, &text, 16);
        let w = measure_text(&text, Some(font), 16, 1.0).width;
        let cx = if align_right {
            right - w * 0.5
        } else {
            left + w * 0.5
        };
        draw_centered(font, &text, snap(cx, dpi), cy, &style);
    }
}

trait Lighten {
    fn lighten_for_text(self) -> Color;
}

impl Lighten for Color {
    fn lighten_for_text(self) -> Color {
        Color::new(
            self.r + (1.0 - self.r) * 0.35,
            self.g + (1.0 - self.g) * 0.35,
            self.b + (1.0 - self.b) * 0.35,
            1.0,
        )
    }
}

fn dim_board(view: &View, a: f32) {
    let b = view.board_rect();
    draw_rectangle(b.x, b.y, b.w, b.h, Color::new(0.02, 0.015, 0.04, a));
}

/// Gentle 0.6..1.0 pulse for call-to-action text.
fn prompt_pulse(time: f64) -> f32 {
    0.8 + 0.2 * (time * 3.0).sin() as f32
}

pub fn draw_title(assets: &Assets, view: &View, time: f64) {
    dim_board(view, 0.78);
    let font = &assets.font;
    let b = view.board_rect();
    let cx = b.center().x;
    let max_w = b.w * 0.94;
    let s = view.scale;

    let title = "Oculus Battle";
    let ts = fit_size(font, title, (62.0 * s).max(26.0), max_w);
    draw_centered(
        font,
        title,
        cx,
        b.y + b.h * 0.25,
        &TextStyle::new(ts, PARCHMENT),
    );

    let lines = [
        "You are blue. Click your base.",
        "Click another base.",
        "Repeat until red is gone.",
    ];
    let body = (22.0 * s).max(14.0);
    let line_h = body * 1.55;
    let y0 = b.y + b.h * 0.47;
    for (i, l) in lines.iter().enumerate() {
        let sz = fit_size(font, l, body, max_w);
        draw_centered(
            font,
            l,
            cx,
            y0 + i as f32 * line_h,
            &TextStyle::new(sz, WHITE),
        );
    }

    let prompt = "Click to start";
    let ps = fit_size(font, prompt, (30.0 * s).max(18.0), max_w);
    let mut st = TextStyle::new(
        ps,
        alpha(Color::new(1.0, 0.86, 0.45, 1.0), prompt_pulse(time)),
    );
    st.outline = true;
    draw_centered(font, prompt, cx, b.y + b.h * 0.84, &st);
}

pub fn draw_intermission(assets: &Assets, game: &Game, view: &View, time: f64) {
    let Phase::Intermission {
        outcome,
        next_level,
    } = game.phase()
    else {
        return;
    };
    dim_board(view, 0.55);
    let font = &assets.font;
    let b = view.board_rect();
    let cx = b.center().x;
    let max_w = b.w * 0.94;
    let s = view.scale;

    let (head, head_color, sub) = match outcome {
        Outcome::Victory => (
            "Victory!",
            Color::new(1.0, 0.86, 0.45, 1.0),
            Some("The computer grows stronger..."),
        ),
        Outcome::Defeat => ("Game Over!", team_color(Team::Cpu).lighten_for_text(), None),
    };
    let hs = fit_size(font, head, (60.0 * s).max(26.0), max_w);
    draw_centered(
        font,
        head,
        cx,
        b.y + b.h * 0.33,
        &TextStyle::new(hs, head_color),
    );
    if let Some(sub) = sub {
        let ss = fit_size(font, sub, (26.0 * s).max(15.0), max_w);
        draw_centered(font, sub, cx, b.y + b.h * 0.50, &TextStyle::new(ss, WHITE));
    }
    let prompt = format!("Click to begin level {next_level}");
    let ps = fit_size(font, &prompt, (30.0 * s).max(18.0), max_w);
    let st = TextStyle::new(ps, alpha(PARCHMENT, prompt_pulse(time)));
    draw_centered(font, &prompt, cx, b.y + b.h * 0.72, &st);
}
