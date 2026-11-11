//! Embedded art and fonts. This is the single place to swap art later.
//!
//! Sprites are stored as a small mip chain (100, 50, 25 px ...) built at load time with a
//! premultiplied box filter, so heavy downscales (a 100 px eye drawn at ~13 px) stay smooth
//! instead of aliasing under plain bilinear sampling.

use macroquad::prelude::*;

const MAP_JPG: &[u8] = include_bytes!("../../../assets/map.jpg");
const BEHOLDER_PNG: &[u8] = include_bytes!("../../../assets/beholder.png");
const EYE_PNG: &[u8] = include_bytes!("../../../assets/eye.png");
/// Cinzel by the Cinzel Project Authors, SIL Open Font License 1.1 (assets/fonts/OFL.txt).
const FONT_TTF: &[u8] = include_bytes!("../../../assets/fonts/Cinzel.ttf");

/// Intrinsic aspect of the background art, used to cover-crop it to the board.
pub const MAP_SIZE: Vec2 = vec2(1920.0, 1080.0);

pub struct Assets {
    pub font: Font,
    pub map: Texture2D,
    pub vignette: Texture2D,
    pub beholder: Sprite,
    /// Desaturated, dimmed beholder for neutral bases.
    pub beholder_neutral: Sprite,
    pub eye: Sprite,
}

impl Assets {
    pub fn load() -> Self {
        let font = load_ttf_font_from_bytes(FONT_TTF).expect("embedded font");
        pregrow_font_atlas(&font);
        let map = Texture2D::from_file_with_format(MAP_JPG, None);
        map.set_filter(FilterMode::Linear);
        let beholder_img = key_out_background(&decode_png(BEHOLDER_PNG));
        Self {
            font,
            map,
            vignette: make_vignette(),
            beholder_neutral: Sprite::from_image(&desaturate(&beholder_img, 0.8, 0.72)),
            beholder: Sprite::from_image(&beholder_img),
            eye: Sprite::from_image(&decode_png(EYE_PNG)),
        }
    }
}

/// Macroquad regrows its glyph atlas on demand and deletes the old texture while draw calls
/// batched earlier in the frame can still reference it, which logs GL "deleted texture" errors.
/// Growing the atlas up front, before any frame is drawn, avoids that. The junk glyphs are
/// large so a handful of sizes pushes the atlas to 1024x1024.
fn pregrow_font_atlas(font: &Font) {
    for size in (96..=170u16).step_by(2) {
        font.populate_font_cache(&['W', 'M', 'Q', 'O'], size);
    }
    // The atlas texture itself is only swapped in by the next text draw.
    draw_text_ex(
        "W",
        -1000.0,
        -1000.0,
        TextParams {
            font: Some(font),
            font_size: 96,
            color: Color::new(0.0, 0.0, 0.0, 0.0),
            ..Default::default()
        },
    );
}

fn decode_png(bytes: &[u8]) -> Image {
    Image::from_file_with_format(bytes, Some(ImageFormat::Png)).expect("embedded png")
}

/// A sprite with several pre-filtered sizes, largest first.
pub struct Sprite {
    levels: Vec<Texture2D>,
}

impl Sprite {
    fn from_image(img: &Image) -> Self {
        let mut levels = Vec::new();
        let mut cur = Image {
            bytes: img.bytes.clone(),
            width: img.width,
            height: img.height,
        };
        loop {
            let t = Texture2D::from_image(&cur);
            t.set_filter(FilterMode::Linear);
            levels.push(t);
            if cur.width < 24 || cur.height < 24 {
                break;
            }
            cur = downscale_half(&cur);
        }
        Self { levels }
    }

    /// Draws centered at `center`, `diameter` logical pixels across. `dpi` is the window's
    /// device pixel ratio, used to pick the closest pre-filtered level.
    pub fn draw(&self, center: Vec2, diameter: f32, tint: Color, dpi: f32) {
        let tex = self.level_for(diameter * dpi);
        draw_texture_ex(
            tex,
            center.x - diameter * 0.5,
            center.y - diameter * 0.5,
            tint,
            DrawTextureParams {
                dest_size: Some(vec2(diameter, diameter)),
                ..Default::default()
            },
        );
    }

    /// The smallest level that is still at least `physical_px` wide (or the smallest overall).
    fn level_for(&self, physical_px: f32) -> &Texture2D {
        self.levels
            .iter()
            .rev()
            .find(|t| t.width() >= physical_px)
            .unwrap_or(&self.levels[0])
    }
}

/// Halves an RGBA image with a premultiplied-alpha 2x2 box filter.
pub fn downscale_half(src: &Image) -> Image {
    let (sw, sh) = (src.width as usize, src.height as usize);
    let (dw, dh) = ((sw / 2).max(1), (sh / 2).max(1));
    let mut out = vec![0u8; dw * dh * 4];
    for y in 0..dh {
        for x in 0..dw {
            let mut acc = [0f32; 4];
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let sx = (x * 2 + dx).min(sw - 1);
                let sy = (y * 2 + dy).min(sh - 1);
                let i = (sy * sw + sx) * 4;
                let a = src.bytes[i + 3] as f32;
                acc[0] += src.bytes[i] as f32 * a;
                acc[1] += src.bytes[i + 1] as f32 * a;
                acc[2] += src.bytes[i + 2] as f32 * a;
                acc[3] += a;
            }
            let o = (y * dw + x) * 4;
            if acc[3] > 0.0 {
                out[o] = (acc[0] / acc[3]).round() as u8;
                out[o + 1] = (acc[1] / acc[3]).round() as u8;
                out[o + 2] = (acc[2] / acc[3]).round() as u8;
                out[o + 3] = (acc[3] / 4.0).round() as u8;
            }
        }
    }
    Image {
        bytes: out,
        width: dw as u16,
        height: dh as u16,
    }
}

/// The beholder art ships on an opaque white square. Flood-fills near-white pixels connected to
/// the image border and makes them transparent, leaving the round medallion.
fn key_out_background(src: &Image) -> Image {
    let (w, h) = (src.width as usize, src.height as usize);
    let mut bytes = src.bytes.clone();
    let is_bg = |b: &[u8], x: usize, y: usize| {
        let i = (y * w + x) * 4;
        b[i + 3] > 0 && b[i].min(b[i + 1]).min(b[i + 2]) >= 215
    };
    let mut stack: Vec<(usize, usize)> = Vec::new();
    for x in 0..w {
        stack.push((x, 0));
        stack.push((x, h - 1));
    }
    for y in 0..h {
        stack.push((0, y));
        stack.push((w - 1, y));
    }
    while let Some((x, y)) = stack.pop() {
        if !is_bg(&bytes, x, y) {
            continue;
        }
        bytes[(y * w + x) * 4 + 3] = 0;
        if x > 0 {
            stack.push((x - 1, y));
        }
        if y > 0 {
            stack.push((x, y - 1));
        }
        if x + 1 < w {
            stack.push((x + 1, y));
        }
        if y + 1 < h {
            stack.push((x, y + 1));
        }
    }
    Image {
        bytes,
        width: src.width,
        height: src.height,
    }
}

/// Blends colors toward their luminance by `amount` (0..1) and scales brightness.
fn desaturate(src: &Image, amount: f32, brightness: f32) -> Image {
    let mut bytes = src.bytes.clone();
    for px in bytes.as_chunks_mut::<4>().0 {
        let (r, g, b) = (px[0] as f32, px[1] as f32, px[2] as f32);
        let lum = 0.299 * r + 0.587 * g + 0.114 * b;
        for (c, v) in px[..3].iter_mut().zip([r, g, b]) {
            *c = ((v + (lum - v) * amount) * brightness).clamp(0.0, 255.0) as u8;
        }
    }
    Image {
        bytes,
        width: src.width,
        height: src.height,
    }
}

/// A soft elliptical darkening toward the edges, stretched over the board.
fn make_vignette() -> Texture2D {
    const W: usize = 128;
    const H: usize = 80;
    let mut bytes = vec![0u8; W * H * 4];
    for y in 0..H {
        for x in 0..W {
            let nx = (x as f32 + 0.5) / W as f32 * 2.0 - 1.0;
            let ny = (y as f32 + 0.5) / H as f32 * 2.0 - 1.0;
            let d = (nx * nx + ny * ny).sqrt() / std::f32::consts::SQRT_2;
            let t = ((d - 0.45) / 0.55).clamp(0.0, 1.0);
            let a = t * t * (3.0 - 2.0 * t) * 0.65;
            let i = (y * W + x) * 4;
            bytes[i] = 4;
            bytes[i + 1] = 3;
            bytes[i + 2] = 8;
            bytes[i + 3] = (a * 255.0) as u8;
        }
    }
    let tex = Texture2D::from_image(&Image {
        bytes,
        width: W as u16,
        height: H as u16,
    });
    tex.set_filter(FilterMode::Linear);
    tex
}

#[cfg(test)]
mod tests {
    use super::*;

    fn img(w: u16, h: u16, px: [u8; 4]) -> Image {
        Image {
            bytes: px
                .iter()
                .copied()
                .cycle()
                .take(w as usize * h as usize * 4)
                .collect(),
            width: w,
            height: h,
        }
    }

    #[test]
    fn downscale_halves_and_keeps_color() {
        let d = downscale_half(&img(8, 6, [200, 100, 50, 255]));
        assert_eq!((d.width, d.height), (4, 3));
        assert_eq!(&d.bytes[..4], &[200, 100, 50, 255]);
    }

    #[test]
    fn background_key_clears_border_but_keeps_interior_white() {
        // 5x5 white image with a dark ring around a white center pixel.
        let mut s = img(5, 5, [255, 255, 255, 255]);
        for (x, y) in [
            (1, 1),
            (2, 1),
            (3, 1),
            (1, 2),
            (3, 2),
            (1, 3),
            (2, 3),
            (3, 3),
        ] {
            let i = (y * 5 + x) * 4;
            s.bytes[i..i + 3].copy_from_slice(&[20, 20, 20]);
        }
        let k = key_out_background(&s);
        assert_eq!(k.bytes[3], 0, "corner cleared");
        assert_eq!(k.bytes[(2 * 5 + 2) * 4 + 3], 255, "enclosed white kept");
        assert_eq!(k.bytes[(5 + 1) * 4 + 3], 255, "dark ring kept");
    }

    #[test]
    fn transparent_pixels_do_not_bleed_color() {
        let mut s = img(2, 2, [0, 0, 0, 0]);
        s.bytes[..4].copy_from_slice(&[255, 0, 0, 255]);
        let d = downscale_half(&s);
        assert_eq!(&d.bytes[..3], &[255, 0, 0]);
        assert_eq!(d.bytes[3], 64);
    }
}
