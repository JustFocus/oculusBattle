//! Screen-space UI: crisp text, HUD strip, title and intermission overlays, base values.
//!
//! All text is drawn with the default (pixel-perfect) camera, so macroquad rasterizes glyphs at
//! the real device pixel size instead of scaling a small bitmap.

use crate::assets::Assets;
use crate::effects::Effects;
use crate::render::{team_color, BAR_COLOR, FULL_PULSE_GROWTH};
use crate::view::View;
use macroquad::prelude::*;
use oculus_core::{Game, Outcome, Phase, Team};
use std::cell::RefCell;
use std::collections::HashSet;

/// Height of the HUD strip above the board, in logical pixels.
pub const HUD_HEIGHT: f32 = 36.0;

const PARCHMENT: Color = Color::new(0.96, 0.92, 0.82, 1.0);
const SHADOW: Color = Color::new(0.02, 0.015, 0.035, 0.9);
/// Secondary HUD text (the timer).
const PARCHMENT_DIM: Color = Color::new(0.80, 0.76, 0.66, 1.0);

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
    /// A single soft drop shadow under the glyphs, for text drawn over the board.
    pub shadow: bool,
    /// Extra multiplier applied through font_scale (used for the capture pop).
    pub scale: f32,
}

impl TextStyle {
    pub fn new(size: f32, color: Color) -> Self {
        Self {
            size,
            color,
            shadow: true,
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
    if style.shadow {
        // One copy straight below, snapped to whole device pixels. (A ring of offset copies
        // as a fake outline leaves stair-stepped ghosts on every serif and corner.) It fades
        // with the fill so pulsing text does not leave a dark shape behind.
        let o = snap((style.size * style.scale / 20.0).max(1.0), dpi);
        draw(0.0, o, alpha(SHADOW, style.color.a));
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
pub fn draw_base_values(assets: &Assets, game: &Game, fx: &Effects, view: &View, time: f64) {
    for b in game.bases() {
        let c = view.to_screen(vec2(b.pos.x, b.pos.y));
        let size = (b.radius * view.scale * 0.72).max(11.0);
        let mut style = TextStyle::new(size, WHITE);
        style.scale = fx.pop_scale(b.id) * (1.0 + FULL_PULSE_GROWTH * fx.full_pulse(b.id, time));
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

/// `m:ss`, or `h:mm:ss` past an hour.
pub fn format_time(seconds: f32) -> String {
    let t = seconds.max(0.0).floor() as u32;
    let (h, m, s) = (t / 3600, t / 60 % 60, t % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

/// Text whose digits all take the width of the widest digit, so a running clock does not
/// shift sideways every second. Other characters keep their own width.
struct Tabular {
    size: u16,
    digit_w: f32,
    /// Vertical placement shared by every character, from the full digit set.
    height: f32,
    offset_y: f32,
}

impl Tabular {
    fn new(font: &Font, size: u16) -> Self {
        const SET: &str = "0123456789:";
        ensure_glyphs(font, SET, size);
        let digit_w = ('0'..='9')
            .map(|d| measure_text(d.to_string(), Some(font), size, 1.0).width)
            .fold(0.0, f32::max);
        let all = measure_text(SET, Some(font), size, 1.0);
        Self {
            size,
            digit_w,
            height: all.height,
            offset_y: all.offset_y,
        }
    }

    fn char_w(&self, font: &Font, ch: char) -> f32 {
        if ch.is_ascii_digit() {
            self.digit_w
        } else {
            measure_text(ch.to_string(), Some(font), self.size, 1.0).width
        }
    }

    fn width(&self, font: &Font, text: &str) -> f32 {
        text.chars().map(|c| self.char_w(font, c)).sum()
    }

    /// Draws `text` starting at `left`, vertically centered on `cy`.
    fn draw(&self, font: &Font, text: &str, left: f32, cy: f32, color: Color) {
        let dpi = screen_dpi_scale();
        let y = snap(cy - self.height * 0.5 + self.offset_y, dpi);
        let mut x = left;
        for ch in text.chars() {
            let cell = self.char_w(font, ch);
            let s = ch.to_string();
            let w = measure_text(&s, Some(font), self.size, 1.0).width;
            draw_text_ex(
                &s,
                snap(x + (cell - w) * 0.5, dpi),
                y,
                TextParams {
                    font: Some(font),
                    font_size: self.size,
                    color,
                    ..Default::default()
                },
            );
            x += cell;
        }
    }
}

pub fn draw_hud(assets: &Assets, game: &Game, view: &View) {
    let font = &assets.font;
    let cy = HUD_HEIGHT * 0.5;
    let dpi = screen_dpi_scale();

    // Centered group: "Level N", a small dot, then the level clock.
    let level_text = format!("Level {}", game.level());
    ensure_glyphs(font, &level_text, 22);
    let level_w = measure_text(&level_text, Some(font), 22, 1.0).width;
    let clock = Tabular::new(font, 20);
    let time_text = format_time(game.elapsed_seconds());
    let time_w = clock.width(font, &time_text);
    let gap = 26.0;
    let left_edge = snap(view.screen.x * 0.5 - (level_w + gap + time_w) * 0.5, dpi);
    let mut level = TextStyle::new(22.0, PARCHMENT);
    level.shadow = false;
    draw_centered(font, &level_text, left_edge + level_w * 0.5, cy, &level);
    draw_circle(
        snap(left_edge + level_w + gap * 0.5, dpi),
        cy,
        2.0,
        alpha(PARCHMENT_DIM, 0.7),
    );
    clock.draw(
        font,
        &time_text,
        left_edge + level_w + gap,
        cy,
        PARCHMENT_DIM,
    );

    let pad = 12.0;
    let left = view.board_rect().x.max(0.0) + pad;
    let right = view.board_rect().right().min(view.screen.x) - pad;
    let dpi = screen_dpi_scale();
    for (team, label, align_right) in [(Team::Player, "Blue", false), (Team::Cpu, "Red", true)] {
        let text = format!("{label} {}", team_totals(game, team));
        let mut style = TextStyle::new(16.0, team_color(team).lighten_for_text());
        style.shadow = false;
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
    let y0 = b.y + b.h * 0.43;
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

    // Shortcuts, quieter than the rules above. One size for both lines so they read as a pair.
    let hints = [
        "Shift-click a base: attack from all your bases",
        "Ctrl-click: select several    A: select all",
    ];
    let hint = hints
        .iter()
        .map(|h| fit_size(font, h, (17.0 * s).max(12.0), max_w))
        .fold(f32::MAX, f32::min);
    let hy = y0 + 2.0 * line_h + body * 1.9;
    for (i, h) in hints.iter().enumerate() {
        draw_centered(
            font,
            h,
            cx,
            hy + i as f32 * hint * 1.6,
            &TextStyle::new(hint, PARCHMENT_DIM),
        );
    }

    let prompt = "Click to start";
    let ps = fit_size(font, prompt, (30.0 * s).max(18.0), max_w);
    let st = TextStyle::new(
        ps,
        alpha(Color::new(1.0, 0.86, 0.45, 1.0), prompt_pulse(time)),
    );
    draw_centered(font, prompt, cx, b.y + b.h * 0.86, &st);
}

pub fn draw_intermission(assets: &Assets, game: &Game, view: &View, now: f64) {
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

    let time = format_time(game.elapsed_seconds());
    let (head, head_color, timing, sub) = match outcome {
        Outcome::Victory => (
            "Victory!",
            Color::new(1.0, 0.86, 0.45, 1.0),
            format!("Level {} won in {time}", game.level()),
            Some("The computer grows stronger..."),
        ),
        Outcome::Defeat => (
            "Game Over!",
            team_color(Team::Cpu).lighten_for_text(),
            format!("Held out for {time} on level {}", game.level()),
            None,
        ),
    };
    // Without a subtitle, drop the block lower so it does not leave a hole above the prompt.
    let lift = if sub.is_some() { 0.0 } else { 0.06 };
    let hs = fit_size(font, head, (60.0 * s).max(26.0), max_w);
    draw_centered(
        font,
        head,
        cx,
        b.y + b.h * (0.30 + lift),
        &TextStyle::new(hs, head_color),
    );
    let ts = fit_size(font, &timing, (22.0 * s).max(14.0), max_w);
    draw_centered(
        font,
        &timing,
        cx,
        b.y + b.h * (0.43 + lift),
        &TextStyle::new(ts, PARCHMENT_DIM),
    );
    if let Some(sub) = sub {
        let ss = fit_size(font, sub, (26.0 * s).max(15.0), max_w);
        draw_centered(font, sub, cx, b.y + b.h * 0.55, &TextStyle::new(ss, WHITE));
    }
    let prompt = format!("Click to begin level {next_level}");
    let ps = fit_size(font, &prompt, (30.0 * s).max(18.0), max_w);
    let st = TextStyle::new(ps, alpha(PARCHMENT, prompt_pulse(now)));
    draw_centered(font, &prompt, cx, b.y + b.h * 0.72, &st);
}

#[cfg(test)]
mod tests {
    use super::format_time;

    #[test]
    fn clock_format() {
        assert_eq!(format_time(0.0), "0:00");
        assert_eq!(format_time(59.99), "0:59");
        assert_eq!(format_time(61.0), "1:01");
        assert_eq!(format_time(600.0), "10:00");
        assert_eq!(format_time(3725.0), "1:02:05");
        assert_eq!(format_time(-1.0), "0:00");
    }
}
