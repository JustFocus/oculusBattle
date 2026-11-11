//! Renders the game's art into `assets/`.
//!
//! Usage: `cargo run -p artgen --release` from the repo root. Outputs are committed, so this only
//! needs to run after changing an art source.
//!
//! - `art/base.svg`, `art/unit.svg`: hand-authored sprites, rasterized with transparency.
//! - `map.rs`: procedural parchment map, generated as SVG from a seed, then rasterized.

mod map;

use std::path::Path;

/// Cinzel, shared with the game, so map labels match the UI.
const FONT: &[u8] = include_bytes!("../../../assets/fonts/Cinzel.ttf");

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let art = root.join("art");
    let assets = root.join("assets");

    for (src, out, size) in [("base.svg", "base.png", 256), ("unit.svg", "unit.png", 128)] {
        let svg = std::fs::read_to_string(art.join(src)).expect("read sprite svg");
        let img = render_svg(&svg, size, size);
        img.save(assets.join(out)).expect("write sprite png");
        println!("wrote assets/{out} ({size}x{size})");
    }

    let svg = map::generate(map::SEED);
    std::fs::write(art.join("map.generated.svg"), &svg).expect("write map svg");
    let img = render_svg(&svg, map::WIDTH, map::HEIGHT);
    let rgb = image::DynamicImage::ImageRgba8(img).to_rgb8();
    let file = std::fs::File::create(assets.join("map.jpg")).expect("create map.jpg");
    let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(file, 86);
    enc.encode_image(&rgb).expect("encode map.jpg");
    println!("wrote assets/map.jpg ({}x{})", map::WIDTH, map::HEIGHT);
}

/// Rasterizes an SVG document to exactly `width` x `height` pixels (scaled to fit).
pub fn render_svg(svg: &str, width: u32, height: u32) -> image::RgbaImage {
    let mut opt = resvg::usvg::Options::default();
    opt.fontdb_mut().load_font_data(FONT.to_vec());
    let tree = resvg::usvg::Tree::from_str(svg, &opt).expect("parse svg");
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height).expect("pixmap");
    let size = tree.size();
    let transform = resvg::tiny_skia::Transform::from_scale(
        width as f32 / size.width(),
        height as f32 / size.height(),
    );
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    // tiny-skia stores premultiplied alpha; PNG wants straight alpha.
    let mut img = image::RgbaImage::new(width, height);
    for (dst, src) in img.pixels_mut().zip(pixmap.pixels()) {
        let c = src.demultiply();
        *dst = image::Rgba([c.red(), c.green(), c.blue(), c.alpha()]);
    }
    img
}
